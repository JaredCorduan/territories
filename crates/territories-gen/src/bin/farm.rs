//! Long-running puzzle farm. Generates puzzles on all cores and saves every
//! new one to the permanent collection (`data/puzzles.json`), never removing
//! any. Every few seconds it publishes the site's copy (`web/puzzles.json`),
//! capped per size/level. Ctrl-C is safe: saves are atomic, so at most the
//! last few seconds of finds are lost.
//!
//! With `--target N` it only fills size/level combinations that hold fewer
//! than N puzzles, and stops by itself once every one has N. Threads with no
//! easy/medium/hard combination left to fill join the brutal hunt.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, mpsc};
use std::time::{Duration, Instant};

use clap::Parser;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use territories_core::Level;
use territories_gen::generate::{generate, generate_brutal};
use territories_gen::store::Store;

#[derive(Parser)]
struct Args {
    #[arg(long, value_delimiter = ',', default_value = "7,8,9,10")]
    sizes: Vec<usize>,
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
    #[arg(long, default_value = "data/puzzles.json")]
    store: PathBuf,
    /// Where the site loads puzzles from.
    #[arg(long, default_value = "web/puzzles.json")]
    publish: PathBuf,
}

/// Levels the regular workers aim for; brutal has its own workers.
const TARGETS: [Level; 3] = [Level::Easy, Level::Medium, Level::Hard];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(&bad) = args.sizes.iter().find(|&&n| !(5..=16).contains(&n)) {
        return Err(format!("size {bad} must be in 5..=16").into());
    }
    let mut store = Store::load(&args.store)?;
    eprintln!("loaded {} puzzles from {}", store.len(), args.store.display());
    if args.publish_only {
        return save(&mut store, &args);
    }

    let threads = args
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let brutal_threads = args.brutal_threads.unwrap_or(threads / 4).min(threads);

    // Collection sizes per (size, level), refreshed at each save; workers
    // favor whichever combination has the fewest puzzles.
    let counts: Arc<RwLock<BTreeMap<(usize, Level), usize>>> = Arc::new(RwLock::new(BTreeMap::new()));
    let refresh_counts = |store: &Store, counts: &RwLock<BTreeMap<(usize, Level), usize>>| {
        let mut c = counts.write().unwrap();
        for &n in &args.sizes {
            for level in Level::ALL {
                c.insert((n, level), store.count(n, level));
            }
        }
    };
    refresh_counts(&store, &counts);

    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let seed0: u64 = rand::rng().random();
    for t in 0..threads {
        let (stop, counts, tx, sizes) = (stop.clone(), counts.clone(), tx.clone(), args.sizes.clone());
        let brutal = t < brutal_threads;
        let cap = args.target.unwrap_or(usize::MAX);
        std::thread::spawn(move || {
            let mut rng = StdRng::seed_from_u64(seed0 ^ (t as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15));
            while !stop.load(Ordering::Relaxed) {
                // Combinations still short of the target, as of the last save.
                let wanted = |levels: &[Level]| -> Vec<(usize, Level)> {
                    let counts = counts.read().unwrap();
                    let short = |b: &(usize, Level)| counts.get(b).copied().unwrap_or(0) < cap;
                    sizes.iter().flat_map(|&n| levels.iter().map(move |&l| (n, l))).filter(short).collect()
                };
                let regular = if brutal { Vec::new() } else { wanted(&TARGETS) };
                let found = if regular.is_empty() {
                    let open = wanted(&[Level::Brutal]);
                    if open.is_empty() {
                        std::thread::sleep(Duration::from_millis(200));
                        continue;
                    }
                    generate_brutal(open[rng.random_range(0..open.len())].0, &mut rng)
                } else {
                    let (n, target) = pick_bucket(&regular, &counts.read().unwrap(), &mut rng);
                    generate(n, target, &mut rng)
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
        args.sizes,
        args.publish_cap,
        args.target.map_or(String::new(), |n| format!(", filling each size/level to {n}")),
        args.minutes.map_or(" — Ctrl-C to stop".to_string(), |m| format!(", for {m} minutes")),
    );
    let full = |store: &Store, n: usize, level: Level| args.target.is_some_and(|cap| store.count(n, level) >= cap);
    let (mut added, mut dupes, mut unsaved) = (0usize, 0usize, 0usize);
    let mut last_save = Instant::now();
    loop {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        if args.sizes.iter().all(|&n| Level::ALL.iter().all(|&l| full(&store, n, l))) {
            eprintln!("every size/level has reached the target");
            break;
        }
        if let Ok(g) = rx.recv_timeout(Duration::from_millis(500)) {
            if full(&store, g.puzzle.size(), g.level()) {
                // Came out easier than aimed for, into a combination that's full.
            } else if store.add(&g) {
                added += 1;
                unsaved += 1;
                if g.level() == Level::Brutal {
                    eprintln!("  found a brutal {0}x{0}!", g.puzzle.size());
                }
            } else {
                dupes += 1;
            }
        }
        if unsaved > 0 && last_save.elapsed() >= Duration::from_secs(10) {
            save(&mut store, &args)?;
            refresh_counts(&store, &counts);
            unsaved = 0;
            last_save = Instant::now();
            report(&store, &args.sizes, added, dupes, start);
        }
    }
    stop.store(true, Ordering::Relaxed);
    save(&mut store, &args)?;
    report(&store, &args.sizes, added, dupes, start);
    Ok(())
}

/// One of `options` (size, target level) to aim for, weighted toward
/// combinations with fewer puzzles so thin ones catch up.
fn pick_bucket(
    options: &[(usize, Level)],
    counts: &BTreeMap<(usize, Level), usize>,
    rng: &mut impl Rng,
) -> (usize, Level) {
    let weights: Vec<f64> = options
        .iter()
        .map(|b| 1.0 / (1.0 + counts.get(b).copied().unwrap_or(0) as f64))
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

fn save(store: &mut Store, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    store.save(&args.store)?;
    store.publish(&args.publish, args.publish_cap)
}

fn report(store: &Store, sizes: &[usize], added: usize, dupes: usize, start: Instant) {
    let mins = start.elapsed().as_secs() / 60;
    let secs = start.elapsed().as_secs() % 60;
    eprintln!("[{mins}:{secs:02}] {} puzzles saved, +{added} this run ({dupes} duplicates skipped)", store.len());
    for &n in sizes {
        let row: Vec<String> = Level::ALL.iter().map(|&l| format!("{} {:>4}", l.name(), store.count(n, l))).collect();
        eprintln!("    {n:>2}x{n:<2} {}", row.join("   "));
    }
}
