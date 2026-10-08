//! Hints for a player's in-progress board.

use serde::Serialize;

use super::rules::Deduction;
use super::solver::next_deduction;
use crate::board::{Cell, Mark, Puzzle, State};
use crate::level::Level;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Hint {
    Solved,
    /// Animals off the solution, or crosses on it.
    Mistake { cells: Vec<Cell> },
    /// The easiest move available.
    Move(Deduction),
    /// No rule applies (should not happen for graded puzzles).
    Reveal { cell: Cell },
}

/// Mistakes come first, then the easiest move using rules up to `max`,
/// widening to every rule before falling back to revealing an animal.
pub fn hint(puzzle: &Puzzle, state: &State, solution: &[[usize; 2]], max: Level) -> Hint {
    let is_answer = |(r, c): Cell| solution[r].contains(&c);
    let mistakes: Vec<Cell> = puzzle
        .cells()
        .filter(|&cell| match state.get(cell) {
            Mark::Animal => !is_answer(cell),
            Mark::Cross => is_answer(cell),
            Mark::Empty => false,
        })
        .collect();
    if !mistakes.is_empty() {
        return Hint::Mistake { cells: mistakes };
    }
    let missing = (0..puzzle.size())
        .flat_map(|r| solution[r].map(|c| (r, c)))
        .find(|&cell| state.get(cell) != Mark::Animal);
    let Some(missing) = missing else {
        return Hint::Solved;
    };
    next_deduction(puzzle, state, max)
        .or_else(|| next_deduction(puzzle, state, Level::Brutal))
        .map_or(Hint::Reveal { cell: missing }, Hint::Move)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nine rows as territories, with a solution that has two animals in
    /// every row and column, none touching.
    fn puzzle() -> (Puzzle, Vec<[usize; 2]>) {
        let grid: Vec<Vec<usize>> = (0..9).map(|r| vec![r; 9]).collect();
        let solution = vec![[0, 2], [4, 6], [1, 8], [3, 5], [0, 7], [2, 4], [6, 8], [1, 3], [5, 7]];
        (Puzzle::new(&grid).unwrap(), solution)
    }

    #[test]
    fn reports_mistakes_first() {
        let (p, sol) = puzzle();
        let mut s = State::new(9);
        s.set((0, 1), Mark::Animal);
        s.set((1, 4), Mark::Cross);
        assert_eq!(hint(&p, &s, &sol, Level::Easy), Hint::Mistake { cells: vec![(0, 1), (1, 4)] });
    }

    #[test]
    fn solved_once_every_animal_is_placed() {
        let (p, sol) = puzzle();
        let mut s = State::new(9);
        for (r, cols) in sol.iter().enumerate() {
            for &c in cols {
                assert_ne!(hint(&p, &s, &sol, Level::Brutal), Hint::Solved);
                s.set((r, c), Mark::Animal);
            }
        }
        assert_eq!(hint(&p, &s, &sol, Level::Brutal), Hint::Solved);
    }
}
