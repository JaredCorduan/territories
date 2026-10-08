//! Exhaustive search, used for uniqueness checks and for finding the
//! alternate solutions the generator repairs away. Never used for grading.

use super::rules::{Board, MAX_SIZE, Set, bit, cells};
use super::{ANIMALS, Solution};
use crate::board::{Mark, Puzzle, State};

/// Search steps one call may take before giving up.
const BUDGET: u64 = 3_000_000;

/// A row-at-a-time search.
struct Search<'a> {
    b: &'a Board,
    /// Cells that may hold an animal, and cells that must.
    allowed: Set,
    required: Set,
    /// `avail[r][g]`: allowed cells of territory `g` in rows `r..`.
    avail: Vec<[u8; MAX_SIZE]>,
    limit: usize,
    out: Vec<Set>,
    steps: u64,
}

impl Search<'_> {
    fn go(&mut self, r: usize, placed: Set, col: [u8; MAX_SIZE], reg: [u8; MAX_SIZE]) {
        let n = self.b.n;
        let two = ANIMALS as u8;
        if r == n {
            self.out.push(placed);
            return;
        }
        self.steps += 1;
        // The rows left can't finish a territory, or a column (which takes
        // at most every other row).
        if (0..n).any(|g| two - reg[g] > self.avail[r][g]) {
            return;
        }
        let left = n - r;
        for (c, &held) in col.iter().enumerate().take(n) {
            let above = r > 0 && placed & bit((r - 1) * n + c) != 0;
            if (two - held) as usize > (left + 1 - above as usize) / 2 {
                return;
            }
        }
        let must = self.required & self.b.units[r];
        for c1 in 0..n {
            for c2 in c1 + 2..n {
                if self.out.len() >= self.limit || self.steps > BUDGET {
                    return;
                }
                let (a, z) = (r * n + c1, r * n + c2);
                let pair = bit(a) | bit(z);
                let (ga, gz) = (self.b.region[a], self.b.region[z]);
                if pair & !self.allowed != 0
                    || must & !pair != 0
                    || placed & (self.b.nbrs[a] | self.b.nbrs[z]) != 0
                    || col[c1] == two
                    || col[c2] == two
                    || (if ga == gz { reg[ga] > 0 } else { reg[ga] == two || reg[gz] == two })
                {
                    continue;
                }
                let (mut col2, mut reg2) = (col, reg);
                col2[c1] += 1;
                col2[c2] += 1;
                reg2[ga] += 1;
                reg2[gz] += 1;
                self.go(r + 1, placed | pair, col2, reg2);
            }
        }
    }
}

/// Up to `limit` solutions consistent with `state` (crosses are excluded,
/// animals are required). `None` if the search ran out of budget.
pub fn solutions(puzzle: &Puzzle, state: &State, limit: usize) -> Option<Vec<Solution>> {
    let b = Board::new(puzzle);
    let n = b.n;
    let marked = |mark: Mark| {
        (0..n * n)
            .filter(|&c| state.get((c / n, c % n)) == mark)
            .fold(0, |acc, c| acc | bit(c))
    };
    let required = marked(Mark::Animal);
    let allowed = required | marked(Mark::Empty);
    let mut avail = vec![[0u8; MAX_SIZE]; n + 1];
    for r in (0..n).rev() {
        avail[r] = avail[r + 1];
        for c in cells(allowed & b.units[r]) {
            avail[r][b.region[c]] += 1;
        }
    }
    let mut search = Search { b: &b, allowed, required, avail, limit, out: Vec::new(), steps: 0 };
    search.go(0, 0, [0; MAX_SIZE], [0; MAX_SIZE]);
    let rows = |s: Set| -> Solution {
        (0..n)
            .map(|r| {
                let mut cols = cells(s & b.units[r]).map(|c| c % n);
                [cols.next().unwrap(), cols.next().unwrap()]
            })
            .collect()
    };
    (search.steps <= BUDGET).then(|| search.out.into_iter().map(rows).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nine_by_nine_has_664_placements_before_territories_matter() {
        // Territories as rows never rule a placement out.
        let grid: Vec<Vec<usize>> = (0..9).map(|r| vec![r; 9]).collect();
        let puzzle = Puzzle::new(&grid).unwrap();
        assert_eq!(solutions(&puzzle, &State::new(9), 1000).unwrap().len(), 664);
    }
}
