//! Builds a puzzle around a random animal layout, then repairs regions until
//! the human rules alone solve it (which also proves it is unique).

use rand::Rng;
use rand::seq::{IndexedRandom, SliceRandom};

use territories_core::{Cell, Level, Puzzle, State, Status, Trace, bruteforce, solver};

use crate::layout::{Layout, grow_regions, neighbors, random_animals};

const MAX_REPAIRS: usize = 60;

/// Share of puzzles allowed to keep a single-cell region (a free animal).
const SINGLETON_RATE: f64 = 0.08;

pub struct Generated {
    pub puzzle: Puzzle,
    pub solution: Vec<usize>,
    pub trace: Trace,
}

impl Generated {
    pub fn level(&self) -> Level {
        self.trace.level().unwrap_or(Level::Easy)
    }
}

/// One attempt at a puzzle solvable with rules up to `target`. `None` if
/// repairs ran out or it has a disallowed single-cell region. The result's
/// actual level may be easier than `target`.
pub fn generate(n: usize, target: Level, rng: &mut impl Rng) -> Option<Generated> {
    let allow_singleton = rng.random_bool(SINGLETON_RATE);
    let animals = random_animals(n, rng);
    let Layout { mut grid, locked } = grow_regions(&animals, rng)?;
    // Best stuck position so far while nudging toward `target`: open cells left.
    let mut best: Option<(usize, Vec<Vec<usize>>)> = None;
    for _ in 0..MAX_REPAIRS {
        let puzzle = Puzzle::new(&grid).expect("generated grid is valid");
        let trace = solver::solve(&puzzle, &State::new(n), target);
        match trace.status {
            Status::Solved => {
                let singleton = (0..n).any(|i| puzzle.region_cells(i).len() == 1);
                if singleton && !allow_singleton {
                    return None;
                }
                return Some(Generated {
                    puzzle,
                    solution: animals,
                    trace,
                });
            }
            Status::Broken => unreachable!("rules are sound, intended solution survives"),
            Status::Stuck => {}
        }
        let alt = bruteforce::solutions(&puzzle, &trace.state, 2)
            .into_iter()
            .find(|s| *s != animals);
        let moved = match alt {
            Some(alt) => break_alternate(&mut grid, &locked, &animals, &alt, rng),
            None => {
                // Unique, but too hard for `target`: hill-climb on how far the
                // target rules get, nudging an open non-solution cell.
                let open = puzzle.cells().filter(|&c| trace.state.is_candidate(c)).count();
                match &best {
                    Some((b, prev)) if open > *b => grid = prev.clone(),
                    _ => best = Some((open, grid.clone())),
                }
                let open_cells: Vec<Cell> = puzzle
                    .cells()
                    .filter(|&(r, c)| trace.state.is_candidate((r, c)) && animals[r] != c)
                    .collect();
                move_cell(&mut grid, &locked, open_cells, rng)
            }
        };
        if !moved {
            return None;
        }
    }
    None
}

/// One attempt at a brutal puzzle: one the Hard rules can't finish but the
/// full rule set can. `None` unless the result is truly brutal.
pub fn generate_brutal(n: usize, rng: &mut impl Rng) -> Option<Generated> {
    let (puzzle, animals) = generate_beyond_hard(n, rng)?;
    let full = solver::solve(&puzzle, &State::new(n), Level::Brutal);
    (full.status == Status::Solved).then_some(Generated {
        puzzle,
        solution: animals,
        trace: full,
    })
}

/// One attempt at a puzzle with a unique solution that the Hard rules can't
/// finish (and no single-cell region), with that solution. Repairs use the
/// cheap Hard-level solver. The brutal rules may or may not solve it.
pub fn generate_beyond_hard(n: usize, rng: &mut impl Rng) -> Option<(Puzzle, Vec<usize>)> {
    let animals = random_animals(n, rng);
    let Layout { mut grid, locked } = grow_regions(&animals, rng)?;
    for _ in 0..MAX_REPAIRS {
        let puzzle = Puzzle::new(&grid).expect("generated grid is valid");
        let trace = solver::solve(&puzzle, &State::new(n), Level::Hard);
        if trace.status == Status::Solved {
            return None;
        }
        let alt = bruteforce::solutions(&puzzle, &trace.state, 2)
            .into_iter()
            .find(|s| *s != animals);
        match alt {
            Some(alt) => {
                if !break_alternate(&mut grid, &locked, &animals, &alt, rng) {
                    return None;
                }
            }
            None => {
                let single = (0..n).any(|i| puzzle.region_cells(i).len() == 1);
                return (!single).then_some((puzzle, animals));
            }
        }
    }
    None
}

/// Moves one cell holding an alternate-solution animal into a neighboring
/// region. That region then holds two of the alternate's animals, so the
/// alternate is no longer a solution. Locked regions are left alone, and no
/// region shrinks below two cells. Returns false if no such move exists.
fn break_alternate(
    grid: &mut [Vec<usize>],
    locked: &[bool],
    animals: &[usize],
    alt: &[usize],
    rng: &mut impl Rng,
) -> bool {
    let n = grid.len();
    let cells = (0..n).filter(|&r| alt[r] != animals[r]).map(|r| (r, alt[r])).collect();
    move_cell(grid, locked, cells, rng)
}

/// Moves one of `cells` (tried in random order) into a neighboring region,
/// keeping locked regions intact, every region connected, and no region
/// below two cells.
fn move_cell(grid: &mut [Vec<usize>], locked: &[bool], mut cells: Vec<Cell>, rng: &mut impl Rng) -> bool {
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
fn stays_connected(grid: &[Vec<usize>], region: usize, removed: Cell) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use territories_core::Mark;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    /// Every rule must be sound: whatever state the solver reaches (even on
    /// ambiguous puzzles), all true solutions must still fit it.
    #[test]
    fn rules_never_eliminate_a_real_solution() {
        let mut rng = StdRng::seed_from_u64(7);
        for n in [5, 6, 7, 8, 9] {
            for _ in 0..300 {
                let animals = random_animals(n, &mut rng);
                let Some(layout) = grow_regions(&animals, &mut rng) else { continue };
                let puzzle = Puzzle::new(&layout.grid).unwrap();
                let trace = solver::solve(&puzzle, &State::new(n), Level::Brutal);
                assert_ne!(trace.status, Status::Broken);
                for sol in bruteforce::solutions(&puzzle, &State::new(n), 10_000) {
                    for (r, &c) in sol.iter().enumerate() {
                        assert_ne!(trace.state.get((r, c)), Mark::Cross, "{:?}", puzzle.grid());
                    }
                }
            }
        }
    }

    #[test]
    fn generated_puzzles_are_unique_with_intended_solution() {
        let mut rng = StdRng::seed_from_u64(11);
        let mut made = 0;
        for n in [6, 8, 10] {
            for attempt in 0..1500 {
                let target = Level::ALL[attempt % Level::ALL.len()];
                let Some(g) = generate(n, target, &mut rng) else { continue };
                assert!(g.level() <= target);
                made += 1;
                assert_eq!(bruteforce::solutions(&g.puzzle, &State::new(n), 2), vec![g.solution.clone()]);
                for i in 0..n {
                    assert!(stays_connected(&g.puzzle.grid(), i, (n, n)), "region {i} disconnected");
                }
            }
        }
        assert!(made > 40, "only {made} puzzles");
    }
}
