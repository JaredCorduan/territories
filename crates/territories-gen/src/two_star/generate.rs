//! Builds a puzzle around a random animal layout, then repairs regions until
//! the human rules alone solve it (which also proves it is unique).

use std::collections::BTreeMap;

use rand::Rng;

use territories_core::two_star::{Solution, Trace, bruteforce, solver};
use territories_core::{Cell, Level, Puzzle};

use super::layout::{grow_regions, random_animals};
use crate::grid::move_cell;
use crate::output::Graded;

const MAX_REPAIRS: usize = 150;

pub struct Generated {
    pub puzzle: Puzzle,
    pub solution: Solution,
    pub trace: Trace,
}

impl Generated {
    pub fn level(&self) -> Level {
        self.trace.level().unwrap_or(Level::Easy)
    }
}

impl Graded for Generated {
    type Solution = Solution;

    fn puzzle(&self) -> &Puzzle {
        &self.puzzle
    }

    fn level(&self) -> Level {
        self.level()
    }

    fn solution(&self) -> &Solution {
        &self.solution
    }

    fn moves(&self) -> BTreeMap<String, usize> {
        let mut moves = BTreeMap::new();
        for d in &self.trace.steps {
            *moves.entry(d.rule.id().to_string()).or_default() += 1;
        }
        moves
    }
}

/// One attempt at a puzzle solvable with rules up to `target`. `None` if
/// there was no layout or repairs ran out. The result's actual level may
/// be easier than `target`.
pub fn generate(n: usize, target: Level, rng: &mut impl Rng) -> Option<Generated> {
    let animals = random_animals(n, rng)?;
    let mut grid = grow_regions(&animals, rng)?;
    // Repairs never move an animal's cell, so no region is ever locked.
    let locked = vec![false; n];
    let is_answer = |&(r, c): &Cell| animals[r].contains(&c);
    // Best stuck position so far while nudging toward `target`: open cells
    // left, the grid, and the open non-solution cells to nudge.
    let mut best: Option<(usize, Vec<Vec<usize>>, Vec<Cell>)> = None;
    for _ in 0..MAX_REPAIRS {
        let puzzle = Puzzle::new(&grid).expect("generated grid is valid");
        let trace = solver::grade(&puzzle, target);
        if trace.solved {
            return Some(Generated {
                puzzle,
                solution: animals,
                trace,
            });
        }
        let alt = bruteforce::solutions(&puzzle, &trace.state, 2)?
            .into_iter()
            .find(|s| *s != animals);
        let cells: Vec<Cell> = match alt {
            // Moving one of the alternate's animals into a neighboring
            // region gives that region three of them.
            Some(alt) => (0..n)
                .flat_map(|r| alt[r].map(|c| (r, c)))
                .filter(|cell| !is_answer(cell))
                .collect(),
            None => {
                // Unique, but too hard for `target`: hill-climb on how far the
                // target rules get, nudging an open non-solution cell.
                let open: Vec<Cell> = puzzle.cells().filter(|&c| trace.state.is_candidate(c)).collect();
                let nudge: Vec<Cell> = open.iter().copied().filter(|cell| !is_answer(cell)).collect();
                match &best {
                    Some((fewest, prev, prev_nudge)) if open.len() > *fewest => {
                        grid = prev.clone();
                        prev_nudge.clone()
                    }
                    _ => {
                        best = Some((open.len(), grid.clone(), nudge.clone()));
                        nudge
                    }
                }
            }
        };
        if !move_cell(&mut grid, &locked, cells, rng) {
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::stays_connected;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use territories_core::{Mark, State};

    /// Every rule must be sound: whatever state the solver reaches (even on
    /// ambiguous puzzles), all true solutions must still fit it.
    #[test]
    fn rules_never_eliminate_a_real_solution() {
        let mut rng = StdRng::seed_from_u64(7);
        let mut checked = 0;
        for n in [9, 10] {
            for _ in 0..150 {
                let Some(animals) = random_animals(n, &mut rng) else { continue };
                let Some(grid) = grow_regions(&animals, &mut rng) else { continue };
                let puzzle = Puzzle::new(&grid).unwrap();
                let trace = solver::solve(&puzzle, &State::new(n), Level::Brutal);
                let Some(solutions) = bruteforce::solutions(&puzzle, &State::new(n), 2000) else { continue };
                assert!(solutions.len() == 2000 || solutions.contains(&animals));
                checked += 1;
                for sol in solutions {
                    for cell in puzzle.cells() {
                        let animal = sol[cell.0].contains(&cell.1);
                        let wrong = if animal { Mark::Cross } else { Mark::Animal };
                        assert_ne!(trace.state.get(cell), wrong, "{:?}", puzzle.grid());
                    }
                }
            }
        }
        assert!(checked > 100, "only {checked} layouts");
    }

    #[test]
    fn generated_puzzles_are_unique_with_intended_solution() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut made = 0;
        for n in [9, 10] {
            for attempt in 0..120 {
                let target = Level::ALL[attempt % Level::ALL.len()];
                let Some(g) = generate(n, target, &mut rng) else { continue };
                assert!(g.level() <= target);
                if g.trace.steps.iter().any(|d| solver::RARE.contains(&d.rule)) {
                    assert!(!solver::solve_without(&g.puzzle, &State::new(n), target, &solver::RARE).solved);
                }
                made += 1;
                assert_eq!(bruteforce::solutions(&g.puzzle, &State::new(n), 2), Some(vec![g.solution.clone()]));
                for i in 0..n {
                    assert!(stays_connected(&g.puzzle.grid(), i, (n, n)), "region {i} disconnected");
                }
            }
        }
        assert!(made > 40, "only {made} puzzles");
    }
}
