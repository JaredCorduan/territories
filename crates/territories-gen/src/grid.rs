//! Territory-grid helpers shared by both games' generators.

use rand::Rng;
use rand::seq::{IndexedRandom, SliceRandom};

use territories_core::Cell;

/// Orthogonal neighbors of `cell` on an `n`×`n` grid.
pub fn neighbors(n: usize, (r, c): Cell) -> impl Iterator<Item = Cell> {
    [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)]
        .into_iter()
        .map(move |(dr, dc)| (r as i32 + dr, c as i32 + dc))
        .filter(move |&(r, c)| r >= 0 && c >= 0 && (r as usize) < n && (c as usize) < n)
        .map(|(r, c)| (r as usize, c as usize))
}

/// Moves one of `cells` (tried in random order) into a neighboring region,
/// keeping locked regions intact, every region connected, and no region
/// below two cells.
pub fn move_cell(grid: &mut [Vec<usize>], locked: &[bool], mut cells: Vec<Cell>, rng: &mut impl Rng) -> bool {
    let n = grid.len();
    cells.shuffle(rng);
    for (r, c) in cells {
        let from = grid[r][c];
        let size = grid.iter().flatten().filter(|&&g| g == from).count();
        if locked[from] || size <= 2 || !stays_connected(grid, from, (r, c)) {
            continue;
        }
        let mut targets: Vec<usize> = neighbors(n, (r, c))
            .map(|(nr, nc)| grid[nr][nc])
            .filter(|&g| g != from && !locked[g])
            .collect();
        targets.dedup();
        if let Some(&to) = targets.choose(rng) {
            grid[r][c] = to;
            return true;
        }
    }
    false
}

/// Whether `region` stays 4-connected (and non-empty) without `removed`.
pub fn stays_connected(grid: &[Vec<usize>], region: usize, removed: Cell) -> bool {
    let n = grid.len();
    let cells: Vec<Cell> = (0..n)
        .flat_map(|r| (0..n).map(move |c| (r, c)))
        .filter(|&(r, c)| grid[r][c] == region && (r, c) != removed)
        .collect();
    let Some(&start) = cells.first() else {
        return false;
    };
    let mut seen = vec![start];
    let mut stack = vec![start];
    while let Some(cell) = stack.pop() {
        for nb in neighbors(n, cell) {
            if nb != removed && grid[nb.0][nb.1] == region && !seen.contains(&nb) {
                seen.push(nb);
                stack.push(nb);
            }
        }
    }
    seen.len() == cells.len()
}
