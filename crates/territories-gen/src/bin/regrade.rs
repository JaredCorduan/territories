//! Regrades the two-animal collection after a change to its rules. A
//! puzzle the rules no longer solve, or solve at another level, is removed;
//! the rest keep their ids and get their new move tallies.
//!
//! Without `--write` it only reports what would change. Afterwards, fill
//! the gaps with `farm --stars 2 --target N`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Parser;

use territories_core::two_star::{self, solver};
use territories_core::{Level, Mark, Puzzle};
use territories_gen::game::{Game, TwoStar};
use territories_gen::output::PuzzleOut;
use territories_gen::store::Store;

#[derive(Parser)]
struct Args {
    /// The collection (defaults to the two-animal game's file in `data/`).
    #[arg(long)]
    store: Option<PathBuf>,
    /// Save the regraded collection. Removed puzzles are gone for good.
    #[arg(long)]
    write: bool,
    /// List the id of every puzzle removed.
    #[arg(long)]
    ids: bool,
}

/// A stored puzzle's move tally under the current rules, if they still
/// solve it at its level and to its solution.
fn grade(p: &PuzzleOut<two_star::Solution>) -> Option<BTreeMap<String, usize>> {
    let puzzle = Puzzle::new(&p.regions).ok()?;
    let trace = solver::grade(&puzzle, Level::Brutal);
    let state = &trace.state;
    let animals = |r: usize| (0..p.size).filter(move |&c| state.get((r, c)) == Mark::Animal);
    let mut stored = p.solution.clone();
    stored.iter_mut().for_each(|row| row.sort());
    let same = trace.solved && trace.level() == Some(p.level) && (0..p.size).all(|r| animals(r).eq(stored[r]));
    same.then(|| {
        let mut moves = BTreeMap::new();
        for d in &trace.steps {
            *moves.entry(d.rule.id().to_string()).or_default() += 1;
        }
        moves
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let path = args.store.unwrap_or_else(|| TwoStar::STORE.into());
    let mut store: Store<two_star::Solution> = Store::load(&path, TwoStar::ID_PREFIX, TwoStar::MIX)?;
    let buckets: Vec<(usize, Level)> =
        TwoStar::SIZES.iter().flat_map(|&size| Level::ALL.into_iter().map(move |level| (size, level))).collect();
    let before: Vec<usize> = buckets.iter().map(|&(size, level)| store.count(size, level)).collect();
    let removed = store.regrade(grade);
    println!("{:<8} {:>4} {:>7} {:>7} {:>7}", "level", "size", "stored", "kept", "removed");
    for (&(size, level), &stored) in buckets.iter().zip(&before) {
        let kept = store.count(size, level);
        println!("{:<8} {:>4} {:>7} {:>7} {:>7}", level.name(), size, stored, kept, stored - kept);
    }
    println!("{} puzzles kept, {} removed", store.len(), removed.len());
    if args.ids {
        removed.iter().for_each(|id| println!("{id}"));
    }
    if args.write {
        store.save(&path)?;
        println!("saved {}", path.display());
    } else {
        println!("dry run: nothing saved (pass --write to save)");
    }
    Ok(())
}
