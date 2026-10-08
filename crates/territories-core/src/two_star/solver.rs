//! Rule-based solver. Always applies the easiest available rule, so the
//! hardest rule in a trace is the puzzle's difficulty.

use super::rules::{Board, Deduction, MAX_SIZE, Rule, View};
use crate::board::{Mark, Puzzle, State};
use crate::level::Level;

#[derive(Clone, Debug)]
pub struct Trace {
    /// Every unit has its animals; otherwise no rule at the allowed level
    /// applied.
    pub solved: bool,
    pub steps: Vec<Deduction>,
    pub state: State,
}

impl Trace {
    /// The hardest level used, or `None` if no steps were needed.
    pub fn level(&self) -> Option<Level> {
        self.steps.iter().map(|d| d.rule.level()).max()
    }
}

/// The brutal rules, in the order they are tried.
pub const BRUTAL: [Rule; 4] = [Rule::WideLeftover, Rule::MixedBand, Rule::LeftoverCap, Rule::Loose];

/// The brutal rules few puzzles need: most brutal puzzles fall to
/// `WideLeftover` alone.
pub const RARE: [Rule; 3] = [Rule::MixedBand, Rule::LeftoverCap, Rule::Loose];

/// The easiest deduction available using rules up to `max`.
pub fn next_deduction(puzzle: &Puzzle, state: &State, max: Level) -> Option<Deduction> {
    next(&View::new(&Board::new(puzzle), state), max, &[])
}

fn next(v: &View, max: Level, skip: &[Rule]) -> Option<Deduction> {
    if let Some(d) = v
        .animal_shadow()
        .or_else(|| v.full_unit())
        .or_else(|| v.last_spots())
        .or_else(|| v.band(1..=1))
        .or_else(|| v.squeeze(Rule::SmallSqueeze))
    {
        return Some(d);
    }
    if max >= Level::Medium
        && let Some(d) = v
            .squeeze(Rule::Squeeze)
            .or_else(|| v.squeeze(Rule::CrowdedSqueeze))
            .or_else(|| v.band(2..=2))
    {
        return Some(d);
    }
    if max >= Level::Hard
        && let Some(d) = v.band(3..=MAX_SIZE).or_else(|| v.leftover(Rule::Leftover))
    {
        return Some(d);
    }
    if max >= Level::Brutal {
        return BRUTAL.iter().filter(|rule| !skip.contains(rule)).find_map(|rule| match rule {
            Rule::WideLeftover => v.leftover(Rule::WideLeftover),
            Rule::MixedBand => v.mixed_band(),
            Rule::LeftoverCap => v.leftover_cap(),
            Rule::Loose => v.loose(),
            _ => unreachable!("not a brutal rule"),
        });
    }
    None
}

pub fn apply(state: &mut State, d: &Deduction) {
    for &c in &d.animals {
        state.set(c, Mark::Animal);
    }
    for &c in &d.crosses {
        if state.get(c) == Mark::Empty {
            state.set(c, Mark::Cross);
        }
    }
}

/// Runs rules up to `max` from `start` until solved or stuck. The rules
/// assume `start` fits some solution.
pub fn solve(puzzle: &Puzzle, start: &State, max: Level) -> Trace {
    solve_without(puzzle, start, max, &[])
}

/// `solve` from an empty board, as a puzzle is graded: with the rare rules
/// only if it can't be solved without them, so its steps say whether it
/// needs them.
pub fn grade(puzzle: &Puzzle, max: Level) -> Trace {
    let start = State::new(puzzle.size());
    let trace = solve(puzzle, &start, max);
    if trace.solved && trace.steps.iter().any(|d| RARE.contains(&d.rule)) {
        let plain = solve_without(puzzle, &start, max, &RARE);
        if plain.solved {
            return plain;
        }
    }
    trace
}

/// `solve`, leaving out the brutal rules in `skip`.
pub fn solve_without(puzzle: &Puzzle, start: &State, max: Level, skip: &[Rule]) -> Trace {
    let board = Board::new(puzzle);
    let mut state = start.clone();
    let mut steps = Vec::new();
    loop {
        let view = View::new(&board, &state);
        let step = if view.solved() { None } else { next(&view, max, skip) };
        let Some(d) = step else {
            return Trace { solved: view.solved(), steps, state };
        };
        apply(&mut state, &d);
        steps.push(d);
    }
}
