//! Random animal placement and region growth around the animals.

use rand::Rng;
use rand::seq::SliceRandom;

use territories_core::Cell;

use crate::grid::neighbors;

/// A random valid placement: one animal per row and column, none touching.
/// `animals[row] = col`. Requires `n == 1` or `n >= 4`.
pub fn random_animals(n: usize, rng: &mut impl Rng) -> Vec<usize> {
    fn place(n: usize, cols: &mut Vec<usize>, used: u32, rng: &mut impl Rng) -> bool {
        if cols.len() == n {
            return true;
        }
        let mut order: Vec<usize> = (0..n).collect();
        order.shuffle(rng);
        for c in order {
            let touches = cols.last().is_some_and(|&p: &usize| p.abs_diff(c) <= 1);
            if used & (1 << c) != 0 || touches {
                continue;
            }
            cols.push(c);
            if place(n, cols, used | (1 << c), rng) {
                return true;
            }
            cols.pop();
        }
        false
    }
    let mut cols = Vec::with_capacity(n);
    assert!(place(n, &mut cols, 0, rng), "no animal placement for size {n}");
    cols
}

type Shape = &'static [(i32, i32)];

/// Small polyominoes a free region may start from before growing.
const SEEDS: &[Shape] = &[
    &[(0, 0), (0, 1)],
    &[(0, 0), (0, 1), (0, 2)],
    &[(0, 0), (1, 0), (1, 1)],
    &[(0, 0), (0, 1), (1, 0), (1, 1)],
    &[(0, 0), (1, 0), (2, 0), (2, 1)],
];

/// Recognizable "signature" shapes. A region stamped with one is locked:
/// it never grows and repairs never change it.
const SIGNATURES: &[Shape] = &[
    &[(0, 0), (0, 1), (0, 2)],                                 // I3
    &[(0, 0), (0, 1), (0, 2), (0, 3)],                         // I4
    &[(0, 0), (0, 1), (0, 2), (0, 3), (0, 4)],                 // I5
    &[(0, 0), (1, 0), (1, 1)],                                 // small L
    &[(0, 0), (1, 0), (2, 0), (2, 1)],                         // L
    &[(0, 0), (1, 0), (2, 0), (2, 1), (2, 2)],                 // big corner
    &[(0, 0), (0, 1), (0, 2), (1, 1)],                         // T
    &[(0, 0), (0, 1), (0, 2), (1, 1), (2, 1)],                 // tall T
    &[(0, 1), (1, 0), (1, 1), (1, 2), (2, 1)],                 // plus
    &[(0, 0), (0, 1), (1, 0), (1, 1)],                         // 2x2 square
    &[(0, 0), (0, 1), (0, 2), (1, 0), (1, 1), (1, 2)],         // 2x3 block
    &[(0, 0), (1, 0), (1, 1), (1, 2), (0, 2)],                 // U
    &[(0, 0), (0, 1), (1, 1), (1, 2)],                         // S
    &[(0, 0), (1, 0), (1, 1), (2, 1), (2, 2)],                 // stairs
];

/// Chance each region is stamped with a locked signature shape.
const SIGNATURE_RATE: f64 = 0.25;

pub struct Layout {
    /// `grid[row][col]` = region.
    pub grid: Vec<Vec<usize>>,
    /// Regions whose shape must not change.
    pub locked: Vec<bool>,
}

/// Partitions the grid into `n` 4-connected regions, region `i` containing
/// the animal at `(i, animals[i])`. `None` if locked shapes wall off cells
/// that no free region can reach.
pub fn grow_regions(animals: &[usize], rng: &mut impl Rng) -> Option<Layout> {
    let n = animals.len();
    let mut grid: Vec<Vec<Option<usize>>> = vec![vec![None; n]; n];
    for (r, &c) in animals.iter().enumerate() {
        grid[r][c] = Some(r);
    }

    let mut locked = vec![false; n];
    let mut order: Vec<usize> = (0..n).collect();
    order.shuffle(rng);
    for &region in &order {
        let animal = (region, animals[region]);
        if rng.random_bool(SIGNATURE_RATE) && stamp_shape(&mut grid, region, animal, SIGNATURES, rng) {
            locked[region] = true;
        } else if rng.random_bool(0.5) {
            stamp_shape(&mut grid, region, animal, SEEDS, rng);
        }
    }

    // Per-region personality: how often it grabs a cell, and how strongly
    // it prefers cells that keep it compact (0 = snaky, high = blobby).
    let speed: Vec<f64> = (0..n).map(|_| rng.random_range(0.3..=1.0)).collect();
    let compactness: Vec<i32> = (0..n).map(|_| rng.random_range(0..=3)).collect();
    let mut sizes: Vec<usize> = (0..n)
        .map(|i| grid.iter().flatten().filter(|&&g| g == Some(i)).count())
        .collect();

    let mut unassigned = grid.iter().flatten().filter(|c| c.is_none()).count();
    while unassigned > 0 {
        let mut any_frontier = false;
        order.shuffle(rng);
        for &region in &order {
            if locked[region] {
                continue;
            }
            let frontier = frontier(&grid, region);
            any_frontier |= !frontier.is_empty();
            // Single-cell regions always grow, so they rarely stay single.
            if frontier.is_empty() || (sizes[region] > 1 && !rng.random_bool(speed[region])) {
                continue;
            }
            let weights: Vec<f64> = frontier
                .iter()
                .map(|&cell| {
                    let same = neighbors(n, cell)
                        .filter(|&(r, c)| grid[r][c] == Some(region))
                        .count();
                    (1.0 + same as f64).powi(compactness[region])
                })
                .collect();
            let (r, c) = frontier[weighted_index(&weights, rng)];
            grid[r][c] = Some(region);
            sizes[region] += 1;
            unassigned -= 1;
        }
        if !any_frontier {
            return None;
        }
    }
    let grid = grid
        .into_iter()
        .map(|row| row.into_iter().map(Option::unwrap).collect())
        .collect();
    Some(Layout { grid, locked })
}

/// Stamps a random rotation/reflection of one of `shapes` over `animal`.
fn stamp_shape(
    grid: &mut [Vec<Option<usize>>],
    region: usize,
    animal: Cell,
    shapes: &[Shape],
    rng: &mut impl Rng,
) -> bool {
    let n = grid.len() as i32;
    for _ in 0..8 {
        let shape = shapes[rng.random_range(0..shapes.len())];
        let sym = rng.random_range(0..8);
        let cells: Vec<(i32, i32)> = shape
            .iter()
            .map(|&(r, c)| {
                let (r, c) = if sym & 1 != 0 { (c, r) } else { (r, c) };
                let r = if sym & 2 != 0 { -r } else { r };
                let c = if sym & 4 != 0 { -c } else { c };
                (r, c)
            })
            .collect();
        // Anchor the shape so one of its cells lands on the animal.
        let (ar, ac) = cells[rng.random_range(0..cells.len())];
        let placed: Vec<(i32, i32)> = cells
            .iter()
            .map(|&(r, c)| (r - ar + animal.0 as i32, c - ac + animal.1 as i32))
            .collect();
        let fits = placed.iter().all(|&(r, c)| {
            (0..n).contains(&r)
                && (0..n).contains(&c)
                && (grid[r as usize][c as usize].is_none() || (r as usize, c as usize) == animal)
        });
        if fits {
            for (r, c) in placed {
                grid[r as usize][c as usize] = Some(region);
            }
            return true;
        }
    }
    false
}

fn frontier(grid: &[Vec<Option<usize>>], region: usize) -> Vec<Cell> {
    let n = grid.len();
    let mut out = Vec::new();
    for r in 0..n {
        for c in 0..n {
            if grid[r][c].is_none()
                && neighbors(n, (r, c)).any(|(nr, nc)| grid[nr][nc] == Some(region))
            {
                out.push((r, c));
            }
        }
    }
    out
}

fn weighted_index(weights: &[f64], rng: &mut impl Rng) -> usize {
    let total: f64 = weights.iter().sum();
    let mut x = rng.random_range(0.0..total);
    for (i, w) in weights.iter().enumerate() {
        if x < *w {
            return i;
        }
        x -= w;
    }
    weights.len() - 1
}
