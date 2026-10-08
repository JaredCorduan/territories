//! Finds a small, real example position for each rule, for the help page.
//! Each example is the state just before the grader first used that rule
//! on some generated puzzle, so the pictured move is exactly what the
//! engine (and the hint button) would say there.
//!
//! `--stars 2` does the same for the two-animal game's rules.

use clap::Parser;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::Serialize;

use territories_core::one_star::{Deduction, Rule, solver};
use territories_core::{Level, Mark, Puzzle, State, UnitKind, two_star};
use territories_gen::game::{Game, TwoStar};
use territories_gen::output::{Output, PuzzleOut};
use territories_gen::one_star::generate::{generate, generate_brutal};

#[derive(Parser)]
struct Args {
    /// Animals per row, column, and territory: which game's rules (1 or 2).
    #[arg(long, default_value_t = 1)]
    stars: usize,
}

/// `R`, `S`, and `D` are a game's rule, solution, and deduction.
#[derive(Serialize)]
struct Example<R = Rule, S = Vec<usize>, D = Deduction> {
    rule: R,
    size: usize,
    regions: Vec<Vec<usize>>,
    solution: S,
    /// Row-major: `.` empty, `x` cross, `A` animal.
    marks: String,
    deduction: D,
}

fn marks(state: &State) -> String {
    let n = state.size();
    (0..n * n)
        .map(|i| match state.get((i / n, i % n)) {
            Mark::Empty => '.',
            Mark::Cross => 'x',
            Mark::Animal => 'A',
        })
        .collect()
}

/// Lower is better: small boards, few marks already down, and the
/// region-centric form of each rule (the one players think of first).
fn score(size: usize, state: &State, d: &Deduction) -> usize {
    let n = state.size();
    let marked = (0..n * n).filter(|&i| state.get((i / n, i % n)) != Mark::Empty).count();
    let region_first = d.units.first().is_none_or(|u| u.kind() == UnitKind::Region);
    size * size * 3 + marked + if region_first { 0 } else { 100 }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match Args::parse().stars {
        1 => one_animal(),
        2 => two_animals(),
        n => Err(format!("--stars {n}: there are games with 1 and 2 animals").into()),
    }
}

/// The two-animal game: every rule shows up in ordinary 9x9 puzzles.
/// Lower scores are better: few marks already down, few units and cells
/// to take in.
type TwoExample = Example<two_star::Rule, two_star::Solution, two_star::Deduction>;

fn two_animals() -> Result<(), Box<dyn std::error::Error>> {
    use two_star::Rule;
    const SIZE: usize = 9;
    let mut rng = StdRng::seed_from_u64(2026);
    let mut best: Vec<Option<(usize, TwoExample)>> = Rule::ALL.iter().map(|_| None).collect();
    let offer = |best: &mut Vec<Option<(usize, TwoExample)>>, puzzle: &Puzzle, solution: &two_star::Solution, steps: &[two_star::Deduction]| {
        let mut state = State::new(SIZE);
        for d in steps {
            let slot = Rule::ALL.iter().position(|&r| r == d.rule).unwrap();
            let marked = puzzle.cells().filter(|&c| state.get(c) != Mark::Empty).count();
            let mut s = marked + 20 * d.units.len() + d.focus.len() + d.animals.len() + d.crosses.len();
            // A leftover reads best with territories in it, and the wide one
            // with an animal already down, so the counts aren't all twos.
            if matches!(d.rule, Rule::Leftover | Rule::WideLeftover) {
                s += if d.units.iter().any(|u| u.kind() == UnitKind::Region) { 0 } else { 500 };
                s += if d.rule == Rule::WideLeftover && d.line_need % 2 == 0 { 250 } else { 0 };
            }
            if best[slot].as_ref().is_none_or(|(b, _)| s < *b) {
                best[slot] = Some((s, Example {
                    rule: d.rule,
                    size: SIZE,
                    regions: puzzle.grid(),
                    solution: solution.clone(),
                    marks: marks(&state),
                    deduction: d.clone(),
                }));
            }
            two_star::solver::apply(&mut state, d);
        }
    };
    for _ in 0..600 {
        let target = Level::ALL[rng.random_range(0..Level::ALL.len())];
        let Some(g) = territories_gen::two_star::generate::generate(SIZE, target, &mut rng) else { continue };
        offer(&mut best, &g.puzzle, &g.solution, &g.trace.steps);
    }
    // The rare brutal rules turn up too seldom to count on above: take
    // them from the collection's puzzles that need them.
    if let Ok(text) = std::fs::read_to_string(TwoStar::STORE) {
        let stored: Output<two_star::Solution> = serde_json::from_str(&text)?;
        let rare = |p: &&PuzzleOut<two_star::Solution>| {
            p.size == SIZE && two_star::solver::RARE.iter().any(|rule| p.moves.contains_key(rule.id()))
        };
        for p in stored.puzzles.iter().filter(rare).take(400) {
            let puzzle = Puzzle::new(&p.regions)?;
            let trace = two_star::solver::solve(&puzzle, &State::new(SIZE), Level::Brutal);
            offer(&mut best, &puzzle, &p.solution, &trace.steps);
        }
    }
    let missing: Vec<&str> = Rule::ALL.iter().zip(&best).filter(|(_, b)| b.is_none()).map(|(r, _)| r.id()).collect();
    if !missing.is_empty() {
        eprintln!("no example found for {missing:?}");
    }
    let examples: Vec<_> = best.into_iter().flatten().map(|(_, e)| e).collect();
    println!("{}", serde_json::to_string(&examples)?);
    Ok(())
}

fn one_animal() -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = StdRng::seed_from_u64(2026);
    let mut best: Vec<Option<(usize, Example)>> = Rule::ALL.iter().map(|_| None).collect();

    // Keeps each step of a solve that beats the best example of its rule so far.
    let offer = |best: &mut Vec<Option<(usize, Example)>>, puzzle: &Puzzle, solution: &[usize], steps: &[Deduction]| {
        let size = puzzle.size();
        let mut state = State::new(size);
        for d in steps {
            let slot = Rule::ALL.iter().position(|&r| r == d.rule).unwrap();
            let s = score(size, &state, d);
            if best[slot].as_ref().is_none_or(|(b, _)| s < *b) {
                best[slot] = Some((s, Example {
                    rule: d.rule,
                    size,
                    regions: puzzle.grid(),
                    solution: solution.to_vec(),
                    marks: marks(&state),
                    deduction: d.clone(),
                }));
            }
            solver::apply(&mut state, d);
        }
    };

    for (size, tries) in [(5, 4000), (6, 4000), (7, 4000), (8, 1500), (9, 800)] {
        for _ in 0..tries {
            let target = [Level::Easy, Level::Medium, Level::Hard][rng.random_range(0..3)];
            if let Some(g) = generate(size, target, &mut rng) {
                offer(&mut best, &g.puzzle, &g.solution, &g.trace.steps);
            }
        }
    }

    // The brutal rules almost never arise in ordinary puzzles; take them
    // from the brutal puzzles in the collection.
    let collection: Output<Vec<usize>> = serde_json::from_str(&std::fs::read_to_string("data/puzzles.json")?)?;
    for p in collection.puzzles.iter().filter(|p| p.level == Level::Brutal) {
        let puzzle = Puzzle::new(&p.regions)?;
        let trace = solver::solve(&puzzle, &State::new(p.size), Level::Brutal);
        offer(&mut best, &puzzle, &p.solution, &trace.steps);
    }
    // Hunt for any brutal rule the collection doesn't show.
    for (size, tries) in [(6, 30_000), (7, 30_000)] {
        for _ in 0..tries {
            if best.iter().all(Option::is_some) {
                break;
            }
            if let Some(g) = generate_brutal(size, &mut rng) {
                offer(&mut best, &g.puzzle, &g.solution, &g.trace.steps);
            }
        }
    }

    let examples: Vec<Example> = best.into_iter().flatten().map(|(_, e)| e).collect();
    let missing: Vec<&str> = Rule::ALL.iter().filter(|r| !examples.iter().any(|e| e.rule == **r)).map(|r| r.id()).collect();
    if !missing.is_empty() {
        // The moves page shows a move without a picture if it has no example.
        eprintln!("no example found for {missing:?}");
    }
    println!("{}", serde_json::to_string(&examples)?);
    Ok(())
}
