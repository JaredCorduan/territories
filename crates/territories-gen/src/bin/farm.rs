//! Long-running puzzle farm. Generates puzzles on all cores and saves every
//! new one to the permanent collection (`data/puzzles.json`), never removing
//! any. Every few seconds it publishes the site's copy (`web/puzzles.json`),
//! capped per size/level. Ctrl-C is safe: saves are atomic, so at most the
//! last few seconds of finds are lost.
//!
//! `--stars 2` farms the two-animal game instead, into its own collection
//! (`data/puzzles-2.json`, published to `web/puzzles-2.json`).
//!
//! With `--target N` it only fills size/level combinations that hold fewer
//! than N puzzles, and stops by itself once every one has N. Threads with no
//! easy/medium/hard combination left to fill join the brutal hunt.
//!
//! The two-animal game has two kinds of brutal puzzle, and the site shows
//! a set share of each. With `--target`, each kind fills to its share.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};
use std::time::{Duration, Instant};

use clap::Parser;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::Serialize;
use serde::de::DeserializeOwned;

use territories_core::Level;
use territories_gen::game::{Game, OneStar, TwoStar};
use territories_gen::output::Graded;
use territories_gen::store::Store;

#[derive(Parser)]
struct Args {
    /// Animals per row, column, and territory: which game to farm (1 or 2).
    #[arg(long, default_value_t = 1)]
    stars: usize,
    /// Defaults to 7,8,9,10 (one animal) or 9,10,11 (two).
    #[arg(long, value_delimiter = ',')]
    sizes: Option<Vec<usize>>,
    /// Puzzles per size/level copied to the site (the collection keeps all).
    #[arg(long, default_value_t = 1000)]
    publish_cap: usize,
    /// Just publish the site's copy from the collection, then exit.
    #[arg(long)]
    publish_only: bool,
    /// Stop after this many minutes (runs until Ctrl-C if omitted).
    #[arg(long)]
    minutes: Option<f64>,
    /// Only add to size/level combinations holding fewer puzzles than this,
    /// and stop once all of them (brutal included) reach it.
    #[arg(long)]
    target: Option<usize>,
    /// Worker threads (defaults to all cores).
    #[arg(long)]
    threads: Option<usize>,
    /// Threads hunting brutal puzzles (defaults to a quarter of them).
    #[arg(long)]
    brutal_threads: Option<usize>,
    /// The permanent collection (defaults to the game's file in `data/`).
    #[arg(long)]
    store: Option<PathBuf>,
    /// Where the site loads puzzles from (defaults to the game's file in `web/`).
    #[arg(long)]
    publish: Option<PathBuf>,
}

/// A game's collection: its solution type differs between games.
type StoreOf<G> = Store<<<G as Game>::Found as Graded>::Solution>;

/// Where a run reads and writes, after the game's defaults are filled in.
struct Paths {
    store: PathBuf,
    publish: PathBuf,
    publish_cap: usize,
}

/// Per (size, level): puzzles held, and whether more are wanted.
type Counts = BTreeMap<(usize, Level), (usize, bool)>;

/// Levels the regular workers aim for; brutal has its own workers.
const TARGETS: [Level; 3] = [Level::Easy, Level::Medium, Level::Hard];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.stars {
        1 => run::<OneStar>(args),
        2 => run::<TwoStar>(args),
        n => Err(format!("--stars {n}: there are games with 1 and 2 animals").into()),
    }
}

fn run<G: Game>(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let sizes = args.sizes.clone().unwrap_or_else(|| G::SIZES.to_vec());
    if let Some(&bad) = sizes.iter().find(|&&n| !G::SIZE_LIMITS.contains(&n)) {
        return Err(format!("size {bad} must be in {:?}", G::SIZE_LIMITS).into());
    }
    let paths = Paths {
        store: args.store.clone().unwrap_or_else(|| G::STORE.into()),
        publish: args.publish.clone().unwrap_or_else(|| G::PUBLISH.into()),
        publish_cap: args.publish_cap,
    };
    let mut store: StoreOf<G> = Store::load(&paths.store, G::ID_PREFIX, G::MIX)?;
    eprintln!("loaded {} puzzles from {}", store.len(), paths.store.display());
    if args.publish_only {
        return save(&mut store, &paths);
    }

    let threads = args
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let brutal_threads = args.brutal_threads.unwrap_or(threads / 4).min(threads);

    // Per (size, level), refreshed at each save: the collection's size, and
    // whether it is short of the target. Workers aim for combinations that
    // are short, favoring whichever has the fewest puzzles.
    let cap = args.target.unwrap_or(usize::MAX);
    let counts: Arc<RwLock<Counts>> = Arc::new(RwLock::new(BTreeMap::new()));
    let refresh_counts = |store: &StoreOf<G>, counts: &RwLock<Counts>| {
        let mut c = counts.write().unwrap();
        for &n in &sizes {
            for level in Level::ALL {
                c.insert((n, level), (store.count(n, level), store.missing(n, level, cap) > 0));
            }
        }
    };
    refresh_counts(&store, &counts);

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let seed0: u64 = rand::rng().random();
    for t in 0..threads {
        let (stop, counts, tx, sizes) = (stop.clone(), counts.clone(), tx.clone(), sizes.clone());
        let brutal = t < brutal_threads;
        std::thread::spawn(move || {
            let mut rng = StdRng::seed_from_u64(seed0 ^ (t as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            while !stop.load(Ordering::Relaxed) {
                // Combinations still short of the target, as of the last save.
                let wanted = |levels: &[Level]| -> Vec<(usize, Level)> {
                    let counts = counts.read().unwrap();
                    let short = |b: &(usize, Level)| counts.get(b).is_none_or(|&(_, short)| short);
                    sizes.iter().flat_map(|&n| levels.iter().map(move |&l| (n, l))).filter(short).collect()
                };
                let regular = if brutal { Vec::new() } else { wanted(&TARGETS) };
                let found = if regular.is_empty() {
                    let open = wanted(&[Level::Brutal]);
                    if open.is_empty() {
                        std::thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    G::generate_brutal(open[rng.random_range(0..open.len())].0, &mut rng)
                } else {
                    let (n, target) = pick_bucket(&regular, &counts.read().unwrap(), &mut rng);
                    G::generate(n, target, &mut rng)
                };
                if let Some(g) = found
                    && tx.send(g).is_err()
                {
                    return;
                }
            }
        });
    }
    drop(tx);

    let start = Instant::now();
    let deadline = args.minutes.map(|m| start + Duration::from_secs_f64(m * 60.0));
    eprintln!(
        "farming sizes {:?} with {threads} threads ({brutal_threads} on brutal), publishing up to {} per size/level{}{}",
        sizes,
        args.publish_cap,
        args.target.map_or(String::new(), |n| format!(", filling each size/level to {n}")),
        args.minutes.map_or(" — Ctrl-C to stop".to_string(), |m| format!(", for {m} minutes")),
    );
    let (mut added, mut dupes, mut unsaved) = (0usize, 0usize, 0usize);
    let mut last_save = Instant::now();
    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        if sizes.iter().all(|&n| Level::ALL.iter().all(|&l| store.missing(n, l, cap) == 0)) {
            eprintln!("every size/level has reached the target");
            break;
        }
        if let Ok(g) = rx.recv_timeout(Duration::from_millis(500)) {
            if !store.wants(&g, cap) {
                // Came out easier than aimed for, or of a kind, into a
                // combination that's full.
            } else if store.add(&g) {
                added += 1;
                unsaved += 1;
            } else {
                dupes += 1;
            }
        }
        if unsaved > 0 && last_save.elapsed() >= Duration::from_secs(10) {
            save(&mut store, &paths)?;
            refresh_counts(&store, &counts);
            unsaved = 0;
            last_save = Instant::now();
            report::<G>(&store, &sizes, added, dupes, start);
        }
    }
    stop.store(true, Ordering::Relaxed);
    save(&mut store, &paths)?;
    report::<G>(&store, &sizes, added, dupes, start);
    Ok(())
}

/// One of `options` (size, target level) to aim for, weighted toward
/// combinations with fewer puzzles so thin ones catch up.
fn pick_bucket(
    options: &[(usize, Level)],
    counts: &Counts,
    rng: &mut impl Rng,
) -> (usize, Level) {
    let weights: Vec<f64> = options
        .iter()
        .map(|b| 1.0 / (1.0 + counts.get(b).map_or(0, |&(held, _)| held) as f64))
        .collect();
    let mut x = rng.random_range(0.0..weights.iter().sum::<f64>());
    for (b, w) in options.iter().zip(&weights) {
        if x < *w {
            return *b;
        }
        x -= w;
    }
    options[options.len() - 1]
}

fn save<S: Clone + Serialize + DeserializeOwned>(
    store: &mut Store<S>,
    paths: &Paths,
) -> Result<(), Box<dyn std::error::Error>> {
    store.save(&paths.store)?;
    store.publish(&paths.publish, paths.publish_cap)
}

fn report<G: Game>(store: &StoreOf<G>, sizes: &[usize], added: usize, dupes: usize, start: Instant) {
    let mins = start.elapsed().as_secs() / 60;
    let secs = start.elapsed().as_secs() % 60;
    eprintln!("[{mins}:{secs:02}] {} puzzles saved, +{added} this run ({dupes} duplicates skipped)", store.len());
    for &n in sizes {
        let row: Vec<String> = Level::ALL.iter().map(|&l| format!("{} {:>4}", l.name(), store.count(n, l))).collect();
        let rare = if G::MIX.is_some() { format!(" ({} rare)", store.rare(n)) } else { String::new() };
        eprintln!("    {n:>2}x{n:<2} {}{rare}", row.join("   "));
    }
}
