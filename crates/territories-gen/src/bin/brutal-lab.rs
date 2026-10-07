//! Measures the brutal rules. Generates puzzles with a unique solution that
//! the Hard rules can't finish, then reports how many 2x2 packing solves,
//! how many more each extra rule (`solver::EXTRAS`) solves on its own and
//! together, and how many stay out of reach. Read-only: it never touches
//! the puzzle collection.
//!
//! `--check <collection>` instead regrades every stored puzzle and reports
//! any whose level or move tally would differ from what is stored.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use clap::Parser;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use territories_core::{Level, Mark, Puzzle, Rule, State, Status, solver};
use territories_gen::generate::generate_beyond_hard;
use territories_gen::output::Output;

#[derive(Parser)]
struct Args {
    #[arg(long, value_delimiter = ',', default_value = "7,8,9,10")]
    sizes: Vec<usize>,
    #[arg(long, default_value_t = 2.0)]
    minutes: f64,
    /// Worker threads (defaults to all cores).
    #[arg(long)]
    threads: Option<usize>,
    #[arg(long, default_value_t = 1)]
    seed: u64,
    /// Print up to this many puzzles per extra rule that only it solves.
    #[arg(long, default_value_t = 0)]
    show: usize,
    /// Regrade this collection and report differences, then exit.
    #[arg(long)]
    check: Option<PathBuf>,
}

/// How one beyond-hard puzzle fared.
struct Outcome {
    size: usize,
    grid: Vec<Vec<usize>>,
    /// Solved by 2x2 packing alone.
    base: bool,
    /// Solved with only that extra rule added, per `solver::EXTRAS`.
    alone: [bool; 3],
    /// Solved with every rule; its tally of extra-rule steps, and seconds taken.
    all: bool,
    steps: [usize; 3],
    secs: f64,
    /// Seconds the solve without extras took.
    base_secs: f64,
}

fn examine(puzzle: &Puzzle) -> Outcome {
    let n = puzzle.size();
    let solved = |extras: &[Rule]| solver::solve_with(puzzle, &State::new(n), Level::Brutal, extras);
    let start = Instant::now();
    let base = solved(&[]).status == Status::Solved;
    let base_secs = start.elapsed().as_secs_f64();
    let alone = solver::EXTRAS.map(|rule| base || solved(&[rule]).status == Status::Solved);
    let start = Instant::now();
    let full = solved(&solver::EXTRAS);
    let secs = start.elapsed().as_secs_f64();
    let steps = solver::EXTRAS.map(|rule| full.steps.iter().filter(|d| d.rule == rule).count());
    Outcome { size: n, grid: puzzle.grid(), base, alone, all: full.status == Status::Solved, steps, secs, base_secs }
}

#[derive(Default)]
struct Tally {
    total: usize,
    base: usize,
    alone: [usize; 3],
    only: [usize; 3],
    all: usize,
    /// Puzzles whose full solve used each extra rule, and total steps of it.
    used: [usize; 3],
    steps: [usize; 3],
    secs: f64,
    worst: f64,
    base_secs: f64,
}

/// Regrades every puzzle in the collection at `path`; returns how many
/// no longer match their stored level, solution, or move tally.
fn check(path: &PathBuf) -> Result<usize, Box<dyn std::error::Error>> {
    let collection: Output = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    let mut changed = 0;
    for p in &collection.puzzles {
        let puzzle = Puzzle::new(&p.regions)?;
        let trace = solver::solve(&puzzle, &State::new(p.size), Level::Brutal);
        let mut moves: BTreeMap<String, usize> = BTreeMap::new();
        for d in &trace.steps {
            *moves.entry(d.rule.id().to_string()).or_default() += 1;
        }
        let solution: Vec<usize> = (0..p.size)
            .map(|r| (0..p.size).find(|&c| trace.state.get((r, c)) == Mark::Animal).unwrap_or(p.size))
            .collect();
        let same = trace.status == Status::Solved
            && trace.level().unwrap_or(Level::Easy) == p.level
            && solution == p.solution
            && moves == p.moves;
        if !same {
            changed += 1;
            println!("{} now grades differently", p.id);
        }
    }
    println!("{} puzzles checked, {changed} grade differently", collection.puzzles.len());
    Ok(changed)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if let Some(path) = &args.check {
        return match check(path)? {
            0 => Ok(()),
            n => Err(format!("{n} stored puzzles grade differently").into()),
        };
    }
    let threads = args
        .threads
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    for t in 0..threads {
        let (stop, tx, sizes) = (stop.clone(), tx.clone(), args.sizes.clone());
        let seed = args.seed ^ (t as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        std::thread::spawn(move || {
            let mut rng = StdRng::seed_from_u64(seed);
            while !stop.load(Ordering::Relaxed) {
                let n = sizes[rng.random_range(0..sizes.len())];
                if let Some((puzzle, _)) = generate_beyond_hard(n, &mut rng)
                    && tx.send(examine(&puzzle)).is_err()
                {
                    return;
                }
            }
        });
    }
    drop(tx);

    let deadline = Instant::now() + Duration::from_secs_f64(args.minutes * 60.0);
    let mut tallies: BTreeMap<usize, Tally> = BTreeMap::new();
    let mut shown = [0usize; 3];
    while Instant::now() < deadline {
        let Ok(o) = rx.recv_timeout(Duration::from_millis(500)) else { continue };
        let t = tallies.entry(o.size).or_default();
        t.total += 1;
        t.base += o.base as usize;
        t.all += o.all as usize;
        t.secs += o.secs;
        t.base_secs += o.base_secs;
        t.worst = t.worst.max(o.secs);
        for (i, shown) in shown.iter_mut().enumerate() {
            t.alone[i] += (o.alone[i] && !o.base) as usize;
            let only = o.alone[i] && o.alone.iter().filter(|&&b| b).count() == 1;
            t.only[i] += only as usize;
            t.used[i] += (o.all && !o.base && o.steps[i] > 0) as usize;
            t.steps[i] += if o.all && !o.base { o.steps[i] } else { 0 };
            if only && *shown < args.show {
                *shown += 1;
                println!("{} {}", solver::EXTRAS[i].id(), serde_json::to_string(&o.grid)?);
            }
        }
    }
    stop.store(true, Ordering::Relaxed);

    let names = solver::EXTRAS.map(Rule::id);
    println!("unique puzzles the Hard rules can't finish, over {} minutes on {threads} threads:", args.minutes);
    for (n, t) in &tallies {
        println!("{n}x{n}: {} puzzles", t.total);
        println!("  2x2 packing solves            {:>6}", t.base);
        for (i, name) in names.iter().enumerate() {
            println!(
                "  + {:<15} solves      {:>6} more ({} that no other extra rule gets)",
                name, t.alone[i], t.only[i]
            );
        }
        println!("  all rules solve               {:>6} ({} beyond 2x2 packing)", t.all, t.all - t.base);
        for (i, name) in names.iter().enumerate() {
            println!("    {:<15} used in {:>6} of those, {} steps", name, t.used[i], t.steps[i]);
        }
        println!("  still unsolved                {:>6}", t.total - t.all);
        let per = 1000.0 / t.total.max(1) as f64;
        println!(
            "  full solve: {:.1} ms average, {:.0} ms worst (without the extra rules: {:.1} ms average)",
            t.secs * per,
            1000.0 * t.worst,
            t.base_secs * per
        );
    }
    Ok(())
}
