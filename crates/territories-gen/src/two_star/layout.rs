//! Random animal placement and region growth around the animals.

use rand::Rng;
use rand::seq::SliceRandom;

use territories_core::Cell;
use territories_core::two_star::Solution;

use crate::grid::neighbors;

/// Rows a placement search may visit before starting over.
const MAX_TRIES: u32 = 20_000;

/// A random valid placement: two animals per row and column, none
/// touching. `None` if the search gave up, or `n` is too small to have one.
pub fn random_animals(n: usize, rng: &mut impl Rng) -> Option<Solution> {
    fn place(n: usize, rows: &mut Solution, used: &mut [u8], tries: &mut u32, rng: &mut impl Rng) -> bool {
        let r = rows.len();
        if r == n {
            return true;
        }
        *tries += 1;
        // A column takes at most every other row left.
        let left = n - r;
        let above = |c: usize| rows.last().is_some_and(|prev| prev.contains(&c));
        if (0..n).any(|c| (2 - used[c]) as usize > (left + 1 - above(c) as usize) / 2) {
            return false;
        }
        let mut pairs: Vec<[usize; 2]> = (0..n).flat_map(|a| (a + 2..n).map(move |z| [a, z])).collect();
        pairs.shuffle(rng);
        for pair in pairs {
            if *tries > MAX_TRIES {
                return false;
            }
            let touches = |c: usize| rows.last().is_some_and(|prev| prev.iter().any(|p| p.abs_diff(c) <= 1));
            if pair.iter().any(|&c| used[c] == 2 || touches(c)) {
                continue;
            }
            pair.iter().for_each(|&c| used[c] += 1);
            rows.push(pair);
            if place(n, rows, used, tries, rng) {
                return true;
            }
            rows.pop();
            pair.iter().for_each(|&c| used[c] -= 1);
        }
        false
    }
    let mut rows = Vec::with_capacity(n);
    place(n, &mut rows, &mut vec![0; n], &mut 0, rng).then_some(rows)
}

/// Partitions the grid into `n` 4-connected regions with two animals each:
/// grows a piece around every animal, then merges neighboring pieces in
/// pairs. `None` if the pieces can't all be paired up. Returns
/// `grid[row][col]` = region.
pub fn grow_regions(animals: &Solution, rng: &mut impl Rng) -> Option<Vec<Vec<usize>>> {
    let n = animals.len();
    let pieces = 2 * n;
    let mut grid: Vec<Vec<Option<usize>>> = vec![vec![None; n]; n];
    for (r, cols) in animals.iter().enumerate() {
        for (i, &c) in cols.iter().enumerate() {
            grid[r][c] = Some(2 * r + i);
        }
    }

    // Per-piece personality: how often it grabs a cell, and how strongly
    // it prefers cells that keep it compact (0 = snaky, high = blobby).
    let speed: Vec<f64> = (0..pieces).map(|_| rng.random_range(0.3..=1.0)).collect();
    let compactness: Vec<i32> = (0..pieces).map(|_| rng.random_range(0..=3)).collect();
    let mut order: Vec<usize> = (0..pieces).collect();
    let mut unassigned = n * n - pieces;
    while unassigned > 0 {
        order.shuffle(rng);
        for &piece in &order {
            if !rng.random_bool(speed[piece]) {
                continue;
            }
            let frontier: Vec<(Cell, f64)> = (0..n)
                .flat_map(|r| (0..n).map(move |c| (r, c)))
                .filter(|&(r, c)| grid[r][c].is_none())
                .filter_map(|cell| {
                    let same = neighbors(n, cell).filter(|&(r, c)| grid[r][c] == Some(piece)).count();
                    (same > 0).then(|| (cell, (1.0 + same as f64).powi(compactness[piece])))
                })
                .collect();
            if frontier.is_empty() {
                continue;
            }
            let mut x = rng.random_range(0.0..frontier.iter().map(|f| f.1).sum());
            let pick = frontier.iter().find(|f| {
                x -= f.1;
                x < 0.0
            });
            let (r, c) = pick.unwrap_or(&frontier[0]).0;
            grid[r][c] = Some(piece);
            unassigned -= 1;
        }
    }
    let piece_at = |(r, c): Cell| grid[r][c].expect("every cell is assigned");

    // Pieces that share an edge, as bitmasks.
    let mut touching = vec![0u32; pieces];
    for r in 0..n {
        for c in 0..n {
            for nb in neighbors(n, (r, c)) {
                if piece_at(nb) != piece_at((r, c)) {
                    touching[piece_at((r, c))] |= 1 << piece_at(nb);
                }
            }
        }
    }
    let mut pairs = Vec::with_capacity(n);
    if !pair_up(&touching, (1 << pieces) - 1, &mut pairs, rng) {
        return None;
    }
    let mut region = vec![0; pieces];
    for (i, &(p, q)) in pairs.iter().enumerate() {
        region[p] = i;
        region[q] = i;
    }
    Some((0..n).map(|r| (0..n).map(|c| region[piece_at((r, c))]).collect()).collect())
}

/// Pairs every piece in `free` with a touching one, at random. The free
/// piece with the fewest free neighbors goes first.
fn pair_up(touching: &[u32], free: u32, pairs: &mut Vec<(usize, usize)>, rng: &mut impl Rng) -> bool {
    let is_free = |p: &usize| free & (1 << p) != 0;
    let Some(p) = (0..touching.len())
        .filter(is_free)
        .min_by_key(|&p| (touching[p] & free).count_ones())
    else {
        return true;
    };
    let mut options: Vec<usize> = (0..touching.len()).filter(|q| is_free(q) && touching[p] & (1 << q) != 0).collect();
    options.shuffle(rng);
    for q in options {
        pairs.push((p, q));
        if pair_up(touching, free & !(1 << p) & !(1 << q), pairs, rng) {
            return true;
        }
        pairs.pop();
    }
    false
}
