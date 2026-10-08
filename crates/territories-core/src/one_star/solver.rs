//! Rule-based solver. Always applies the easiest available rule, so the
//! hardest rule in a trace is the puzzle's difficulty.

use serde::{Deserialize, Serialize};

use crate::board::{Mark, Puzzle, State, UnitKind};
use super::rules::{self, Analysis, Deduction, Rule};
use crate::level::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Every unit has its animal.
    Solved,
    /// No rule at the allowed level applies.
    Stuck,
    /// The marks contradict the rules (a unit with no room, or clashing animals).
    Broken,
}

#[derive(Clone, Debug)]
pub struct Trace {
    pub status: Status,
    pub steps: Vec<Deduction>,
    pub state: State,
}

impl Trace {
    /// The hardest level used, or `None` if no steps were needed.
    pub fn level(&self) -> Option<Level> {
        self.steps.iter().map(|d| d.rule.level()).max()
    }
}

/// Largest number of units the packing rules consider at once.
const MAX_PACKING: usize = 4;

/// The generalized counting rules, tried (in this order, for each number of
/// units) only once 2x2 packing has nothing.
pub const EXTRAS: [Rule; 3] = [Rule::MixedPacking, Rule::MixedBand, Rule::WindowPacking];

/// The easiest deduction available using rules up to `max`.
pub fn next_deduction(puzzle: &Puzzle, state: &State, max: Level) -> Option<Deduction> {
    next_deduction_with(puzzle, state, max, &EXTRAS)
}

/// As `next_deduction`, with only the given `EXTRAS` enabled.
pub fn next_deduction_with(puzzle: &Puzzle, state: &State, max: Level, extras: &[Rule]) -> Option<Deduction> {
    if let Some(d) = rules::animal_shadow(puzzle, state) {
        return Some(d);
    }
    let a = Analysis::new(puzzle, state);
    if let Some(d) = rules::last_spot(&a)
        .or_else(|| rules::locked(puzzle, state, &a, 1))
        .or_else(|| rules::l_corner(puzzle, state, &a))
    {
        return Some(d);
    }
    if max >= Level::Medium
        && let Some(d) = rules::squeeze(puzzle, state, &a).or_else(|| rules::locked(puzzle, state, &a, 2))
    {
        return Some(d);
    }
    if max >= Level::Hard
        && let Some(d) = (3..=puzzle.size() / 2).find_map(|k| rules::locked(puzzle, state, &a, k))
    {
        return Some(d);
    }
    if max >= Level::Brutal {
        return (2..=MAX_PACKING)
            .find_map(|k| rules::block_packing(puzzle, state, &a, k))
            .or_else(|| {
                (2..=MAX_PACKING).find_map(|k| {
                    EXTRAS
                        .into_iter()
                        .filter(|rule| extras.contains(rule))
                        .find_map(|rule| rules::packing(puzzle, state, &a, k, rule))
                })
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

pub fn status(puzzle: &Puzzle, state: &State) -> Status {
    let animals = state.animals();
    let clash = animals
        .iter()
        .enumerate()
        .any(|(i, &a)| animals[i + 1..].iter().any(|&b| puzzle.attacks(a, b)));
    let a = Analysis::new(puzzle, state);
    let stranded = UnitKind::ALL
        .into_iter()
        .any(|kind| a.open_units(kind).any(|u| a.cands(u).is_empty()));
    if clash || stranded {
        Status::Broken
    } else if animals.len() == puzzle.size() {
        Status::Solved
    } else {
        Status::Stuck
    }
}

/// Runs rules up to `max` from `start` until solved, stuck, or broken.
pub fn solve(puzzle: &Puzzle, start: &State, max: Level) -> Trace {
    solve_with(puzzle, start, max, &EXTRAS)
}

/// As `solve`, with only the given `EXTRAS` enabled.
pub fn solve_with(puzzle: &Puzzle, start: &State, max: Level, extras: &[Rule]) -> Trace {
    let mut state = start.clone();
    let mut steps = Vec::new();
    loop {
        let st = status(puzzle, &state);
        if st != Status::Stuck {
            return Trace { status: st, steps, state };
        }
        match next_deduction_with(puzzle, &state, max, extras) {
            Some(d) => {
                apply(&mut state, &d);
                steps.push(d);
            }
            None => return Trace { status: Status::Stuck, steps, state },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unique puzzle that 2x2 packing can't finish but `rule` can.
    fn needs(rule: Rule, grid: &[&[usize]]) {
        let grid: Vec<Vec<usize>> = grid.iter().map(|row| row.to_vec()).collect();
        let puzzle = Puzzle::new(&grid).unwrap();
        let start = State::new(puzzle.size());
        assert_eq!(solve_with(&puzzle, &start, Level::Brutal, &[]).status, Status::Stuck);
        let trace = solve_with(&puzzle, &start, Level::Brutal, &[rule]);
        assert_eq!(trace.status, Status::Solved);
        assert!(trace.steps.iter().any(|d| d.rule == rule));
        assert_eq!(trace.level(), Some(Level::Brutal));
    }

    #[test]
    fn mixed_packing_solves_what_block_packing_cannot() {
        needs(Rule::MixedPacking, &[
            &[1, 1, 1, 1, 1, 0, 0, 2],
            &[3, 1, 1, 2, 2, 2, 2, 2],
            &[3, 3, 1, 5, 5, 5, 2, 2],
            &[3, 3, 5, 5, 5, 5, 4, 4],
            &[3, 6, 6, 6, 5, 5, 4, 7],
            &[6, 6, 6, 5, 5, 5, 7, 7],
            &[6, 5, 5, 5, 7, 5, 7, 7],
            &[6, 6, 6, 5, 7, 7, 7, 7],
        ]);
    }

    #[test]
    fn mixed_band_solves_what_block_packing_cannot() {
        needs(Rule::MixedBand, &[
            &[2, 0, 0, 0, 0, 1, 1, 4],
            &[2, 2, 2, 2, 0, 0, 1, 4],
            &[2, 5, 2, 5, 0, 0, 3, 4],
            &[2, 5, 2, 5, 3, 3, 3, 4],
            &[2, 5, 5, 5, 6, 4, 4, 4],
            &[7, 7, 5, 6, 6, 6, 6, 6],
            &[7, 7, 7, 7, 7, 6, 6, 6],
            &[7, 7, 7, 6, 6, 6, 6, 6],
        ]);
    }

    #[test]
    fn window_packing_solves_what_block_packing_cannot() {
        needs(Rule::WindowPacking, &[
            &[5, 0, 0, 0, 2, 2, 2, 2, 3],
            &[5, 5, 0, 1, 1, 1, 2, 3, 3],
            &[5, 5, 0, 5, 1, 2, 2, 3, 3],
            &[5, 5, 5, 5, 1, 5, 2, 2, 3],
            &[7, 7, 5, 5, 5, 5, 4, 6, 3],
            &[7, 7, 5, 5, 5, 5, 4, 6, 6],
            &[7, 7, 7, 7, 7, 5, 4, 6, 8],
            &[7, 7, 7, 7, 7, 8, 8, 8, 8],
            &[7, 7, 7, 8, 8, 8, 8, 8, 8],
        ]);
    }
}
