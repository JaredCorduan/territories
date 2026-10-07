//! Hints for a player's in-progress board.

use serde::Serialize;

use crate::board::{Cell, Mark, Puzzle, State};
use crate::rules::{Deduction, Level};
use crate::solver::next_deduction;

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
pub fn hint(puzzle: &Puzzle, state: &State, solution: &[usize], max: Level) -> Hint {
    let is_answer = |(r, c): Cell| solution[r] == c;
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
        .map(|r| (r, solution[r]))
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

    fn puzzle() -> Puzzle {
        // Solution: row r -> col [1, 3, 0, 2].
        Puzzle::new(&[
            vec![0, 0, 1, 1],
            vec![0, 0, 0, 1],
            vec![2, 2, 3, 1],
            vec![2, 2, 3, 3],
        ])
        .unwrap()
    }

    #[test]
    fn reports_mistakes_first() {
        let mut s = State::new(4);
        s.set((0, 0), Mark::Animal);
        s.set((1, 3), Mark::Cross);
        assert_eq!(hint(&puzzle(), &s, &[1, 3, 0, 2], Level::Easy), Hint::Mistake {
            cells: vec![(0, 0), (1, 3)]
        });
    }

    #[test]
    fn gives_a_move_then_solved() {
        let p = puzzle();
        let sol = [1, 3, 0, 2];
        let mut s = State::new(4);
        for _ in 0..100 {
            match hint(&p, &s, &sol, Level::Brutal) {
                Hint::Move(d) => crate::solver::apply(&mut s, &d),
                Hint::Solved => return,
                other => panic!("unexpected {other:?}"),
            }
        }
        panic!("never solved");
    }
}
