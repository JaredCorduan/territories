//! Finds a small, real example position for each rule, for the help page.
//! Each example is the state just before the grader first used that rule
//! on some generated puzzle, so the pictured move is exactly what the
//! engine (and the hint button) would say there.

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::Serialize;

use territories_core::{Deduction, Level, Mark, Puzzle, Rule, State, UnitKind, solver};
use territories_gen::output::Output;
use territories_gen::generate::{generate, generate_brutal};

#[derive(Serialize)]
struct Example {
    rule: Rule,
    size: usize,
    regions: Vec<Vec<usize>>,
    solution: Vec<usize>,
    /// Row-major: `.` empty, `x` cross, `A` animal.
    marks: String,
    deduction: Deduction,
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
    let collection: Output = serde_json::from_str(&std::fs::read_to_string("data/puzzles.json")?)?;
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
