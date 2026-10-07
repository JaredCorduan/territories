//! Exhaustive search, used for uniqueness checks and for finding the
//! alternate solutions the generator repairs away. Never used for grading.

use crate::board::{Mark, Puzzle, State};

/// Up to `limit` solutions consistent with `state` (crosses are excluded,
/// animals are required). Each solution is the animal's column for each row.
pub fn solutions(puzzle: &Puzzle, state: &State, limit: usize) -> Vec<Vec<usize>> {
    let n = puzzle.size();
    let mut out = Vec::new();
    let mut cols = Vec::with_capacity(n);
    search(puzzle, state, limit, 0, 0, &mut cols, &mut out);
    out
}

fn search(
    puzzle: &Puzzle,
    state: &State,
    limit: usize,
    used_cols: u32,
    used_regions: u32,
    cols: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    let n = puzzle.size();
    let r = cols.len();
    if r == n {
        out.push(cols.clone());
        return;
    }
    let forced = (0..n).find(|&c| state.get((r, c)) == Mark::Animal);
    for c in 0..n {
        if out.len() >= limit {
            return;
        }
        if forced.is_some_and(|f| f != c) || state.get((r, c)) == Mark::Cross {
            continue;
        }
        let region = puzzle.region((r, c));
        if used_cols & (1 << c) != 0 || used_regions & (1 << region) != 0 {
            continue;
        }
        if cols.last().is_some_and(|&prev| prev.abs_diff(c) <= 1) {
            continue;
        }
        cols.push(c);
        search(
            puzzle,
            state,
            limit,
            used_cols | (1 << c),
            used_regions | (1 << region),
            cols,
            out,
        );
        cols.pop();
    }
}
