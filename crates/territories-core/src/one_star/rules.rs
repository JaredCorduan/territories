//! Human-style deduction rules for one animal per unit. Every rule is a direct pattern a player can
//! spot on the board; none of them guess or search for contradictions.
//! See MOVES.md for the player-facing descriptions.

use serde::{Deserialize, Serialize};

use crate::board::{Cell, MAX_SIZE, Mark, Puzzle, State, Unit, UnitKind};
use crate::level::Level;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// An animal crosses out every cell it attacks.
    AnimalShadow,
    /// A row, column, or region with one open cell left gets its animal there.
    LastSpot,
    /// A region's open cells lie in one row/column: cross out the rest of it.
    ClaimedLine,
    /// A row's/column's open cells lie in one region: cross out the rest of it.
    ClaimedRegion,
    /// A region's open cells are three cells of a 2x2 (an L): cross out the
    /// fourth, which touches all three.
    LCorner,
    /// A cell whose attacks would cover every open cell of some other unit.
    Squeeze,
    /// Two regions whose open cells lie within two lines.
    #[serde(rename = "region_band_2")]
    RegionBand2,
    /// Two lines whose open cells lie within two regions (or two lines).
    #[serde(rename = "line_band_2")]
    LineBand2,
    /// Three or more regions within as many lines.
    #[serde(rename = "region_band_3")]
    RegionBand3,
    /// Three or more lines within as many regions (or lines).
    #[serde(rename = "line_band_3")]
    LineBand3,
    /// Units needing k animals whose open cells fit in k 2x2 blocks: the rest
    /// of those blocks is empty, since a 2x2 block holds at most one animal.
    BlockPacking,
    /// Units of one kind needing k animals whose open cells fit in a mix of
    /// k lines, regions, and 2x2 blocks.
    MixedPacking,
    /// Units of different kinds that share no open cell, needing k animals,
    /// whose open cells fit in k lines or regions.
    MixedBand,
    /// Two units that share no open cell whose open cells fit in one 3x3
    /// window, which holds at most two animals.
    WindowPacking,
}

impl Rule {
    pub const ALL: [Rule; 14] = [
        Rule::AnimalShadow,
        Rule::LastSpot,
        Rule::ClaimedLine,
        Rule::ClaimedRegion,
        Rule::LCorner,
        Rule::Squeeze,
        Rule::RegionBand2,
        Rule::LineBand2,
        Rule::RegionBand3,
        Rule::LineBand3,
        Rule::BlockPacking,
        Rule::MixedPacking,
        Rule::MixedBand,
        Rule::WindowPacking,
    ];

    pub fn level(self) -> Level {
        match self {
            Rule::AnimalShadow | Rule::LastSpot | Rule::ClaimedLine | Rule::ClaimedRegion | Rule::LCorner => {
                Level::Easy
            }
            Rule::Squeeze | Rule::RegionBand2 | Rule::LineBand2 => Level::Medium,
            Rule::RegionBand3 | Rule::LineBand3 => Level::Hard,
            Rule::BlockPacking | Rule::MixedPacking | Rule::MixedBand | Rule::WindowPacking => Level::Brutal,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Rule::AnimalShadow => "animal_shadow",
            Rule::LastSpot => "last_spot",
            Rule::ClaimedLine => "claimed_line",
            Rule::ClaimedRegion => "claimed_region",
            Rule::LCorner => "l_corner",
            Rule::Squeeze => "squeeze",
            Rule::RegionBand2 => "region_band_2",
            Rule::LineBand2 => "line_band_2",
            Rule::RegionBand3 => "region_band_3",
            Rule::LineBand3 => "line_band_3",
            Rule::BlockPacking => "block_packing",
            Rule::MixedPacking => "mixed_packing",
            Rule::MixedBand => "mixed_band",
            Rule::WindowPacking => "window_packing",
        }
    }
}

/// One application of a rule. `focus` and `units` explain *why* (for hint
/// highlighting); `animals` and `crosses` are *what* to mark.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deduction {
    pub rule: Rule,
    pub animals: Vec<Cell>,
    pub crosses: Vec<Cell>,
    pub focus: Vec<Cell>,
    pub units: Vec<Unit>,
}

impl Deduction {
    fn cross(rule: Rule, crosses: Vec<Cell>, focus: Vec<Cell>, units: Vec<Unit>) -> Deduction {
        Deduction {
            rule,
            animals: Vec::new(),
            crosses,
            focus,
            units,
        }
    }
}

/// Open cells and animal presence for every unit, indexed by `UnitKind::index`.
pub(crate) struct Analysis {
    pub cands: [Vec<Vec<Cell>>; 3],
    pub has_animal: [Vec<bool>; 3],
}

impl Analysis {
    pub fn new(puzzle: &Puzzle, state: &State) -> Analysis {
        let n = puzzle.size();
        let mut cands: [Vec<Vec<Cell>>; 3] = std::array::from_fn(|_| vec![Vec::new(); n]);
        let mut has_animal: [Vec<bool>; 3] = std::array::from_fn(|_| vec![false; n]);
        for cell in puzzle.cells() {
            for kind in UnitKind::ALL {
                let i = puzzle.unit_of(kind, cell).index();
                match state.get(cell) {
                    Mark::Empty => cands[kind.index()][i].push(cell),
                    Mark::Animal => has_animal[kind.index()][i] = true,
                    Mark::Cross => {}
                }
            }
        }
        Analysis { cands, has_animal }
    }

    pub fn cands(&self, unit: Unit) -> &[Cell] {
        &self.cands[unit.kind().index()][unit.index()]
    }

    pub fn is_open(&self, unit: Unit) -> bool {
        !self.has_animal[unit.kind().index()][unit.index()]
    }

    pub fn open_units(&self, kind: UnitKind) -> impl Iterator<Item = Unit> + '_ {
        let n = self.has_animal[kind.index()].len();
        (0..n)
            .map(move |i| Unit::new(kind, i))
            .filter(|&u| self.is_open(u))
    }
}

pub(crate) fn animal_shadow(puzzle: &Puzzle, state: &State) -> Option<Deduction> {
    state.animals().into_iter().find_map(|q| {
        let crosses: Vec<Cell> = puzzle
            .cells()
            .filter(|&c| state.is_candidate(c) && puzzle.attacks(q, c))
            .collect();
        (!crosses.is_empty()).then(|| Deduction::cross(Rule::AnimalShadow, crosses, vec![q], vec![]))
    })
}

pub(crate) fn last_spot(a: &Analysis) -> Option<Deduction> {
    UnitKind::ALL.into_iter().find_map(|kind| {
        a.open_units(kind).find_map(|unit| match a.cands(unit) {
            &[cell] => Some(Deduction {
                rule: Rule::LastSpot,
                animals: vec![cell],
                crosses: Vec::new(),
                focus: vec![cell],
                units: vec![unit],
            }),
            _ => None,
        })
    })
}

pub(crate) fn squeeze(puzzle: &Puzzle, state: &State, a: &Analysis) -> Option<Deduction> {
    puzzle.cells().filter(|&c| state.is_candidate(c)).find_map(|cell| {
        UnitKind::ALL.into_iter().find_map(|kind| {
            a.open_units(kind).find_map(|unit| {
                let cands = a.cands(unit);
                let covered = !puzzle.contains(unit, cell)
                    && !cands.is_empty()
                    && cands.iter().all(|&o| puzzle.attacks(cell, o));
                covered.then(|| Deduction::cross(Rule::Squeeze, vec![cell], vec![cell], vec![unit]))
            })
        })
    })
}

/// Pairs of unit families checked by the locked-set rules: `k` units of the
/// first kind whose open cells fit inside `k` units of the second kind.
const LOCK_FAMILIES: [(UnitKind, UnitKind); 6] = [
    (UnitKind::Region, UnitKind::Row),
    (UnitKind::Region, UnitKind::Col),
    (UnitKind::Row, UnitKind::Region),
    (UnitKind::Col, UnitKind::Region),
    (UnitKind::Row, UnitKind::Col),
    (UnitKind::Col, UnitKind::Row),
];

/// Claimed line/region (k = 1) and bands (k >= 2): finds `k` units of one
/// kind whose open cells all lie within `k` units of another kind. Those `k`
/// animals then fill the covering units, so any other open cell in the
/// covering units is crossed out.
///
/// Only `k <= open/2` is searched: the complementary set yields the same
/// crosses with the families swapped, so the smaller `k` always describes it.
pub(crate) fn locked(puzzle: &Puzzle, state: &State, a: &Analysis, k: usize) -> Option<Deduction> {
    for (inner, outer) in LOCK_FAMILIES {
        let by_region = inner == UnitKind::Region;
        let rule = match (k, by_region) {
            (1, true) => Rule::ClaimedLine,
            (1, false) => Rule::ClaimedRegion,
            (2, true) => Rule::RegionBand2,
            (2, false) => Rule::LineBand2,
            (_, true) => Rule::RegionBand3,
            (_, false) => Rule::LineBand3,
        };
        if k == 0 || k > a.open_units(inner).count() / 2 {
            continue;
        }
        // Each open inner unit as the bitmask of outer units its cells touch.
        // Units touching a filled outer unit are skipped; AnimalShadow runs
        // first and clears those cells anyway.
        let masks: Vec<(Unit, u32)> = a
            .open_units(inner)
            .filter_map(|unit| {
                let mut mask = 0u32;
                for &cell in a.cands(unit) {
                    let o = puzzle.unit_of(outer, cell);
                    if !a.is_open(o) {
                        return None;
                    }
                    mask |= 1 << o.index();
                }
                Some((unit, mask))
            })
            .collect();
        let mut chosen = Vec::with_capacity(k);
        if let Some(d) = pick(puzzle, state, rule, outer, &masks, k, 0, 0, &mut chosen) {
            return Some(d);
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn pick(
    puzzle: &Puzzle,
    state: &State,
    rule: Rule,
    outer: UnitKind,
    masks: &[(Unit, u32)],
    k: usize,
    start: usize,
    union: u32,
    chosen: &mut Vec<Unit>,
) -> Option<Deduction> {
    if union.count_ones() as usize > k {
        return None;
    }
    if chosen.len() == k {
        if union.count_ones() as usize != k {
            return None;
        }
        let covering: Vec<Unit> = (0..puzzle.size())
            .filter(|i| union & (1 << i) != 0)
            .map(|i| Unit::new(outer, i))
            .collect();
        let crosses: Vec<Cell> = covering
            .iter()
            .flat_map(|&u| puzzle.unit_cells(u))
            .filter(|&c| state.is_candidate(c) && !chosen.iter().any(|&u| puzzle.contains(u, c)))
            .collect();
        if crosses.is_empty() {
            return None;
        }
        let mut units = chosen.clone();
        units.extend(covering);
        return Some(Deduction::cross(rule, crosses, vec![], units));
    }
    for i in start..masks.len() {
        let (unit, mask) = masks[i];
        chosen.push(unit);
        let found = pick(puzzle, state, rule, outer, masks, k, i + 1, union | mask, chosen);
        chosen.pop();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// A region whose open cells are exactly three cells of one 2x2 block: the
/// fourth cell touches all three, so it can't hold an animal.
pub(crate) fn l_corner(puzzle: &Puzzle, state: &State, a: &Analysis) -> Option<Deduction> {
    a.open_units(UnitKind::Region).find_map(|unit| {
        let cands = a.cands(unit);
        if cands.len() != 3 {
            return None;
        }
        let top = cands.iter().map(|c| c.0).min()?;
        let left = cands.iter().map(|c| c.1).min()?;
        if cands.iter().any(|&(r, c)| r > top + 1 || c > left + 1) {
            return None;
        }
        let corner = [(top, left), (top, left + 1), (top + 1, left), (top + 1, left + 1)]
            .into_iter()
            .find(|cell| !cands.contains(cell))?;
        (corner.0 < puzzle.size() && corner.1 < puzzle.size() && state.is_candidate(corner))
            .then(|| Deduction::cross(Rule::LCorner, vec![corner], cands.to_vec(), vec![unit]))
    })
}

/// 2x2 block counting: `k` open units of one kind need exactly `k` animals.
/// If their open cells fit inside `k` 2x2 blocks, those blocks (each holding
/// at most one animal) already hold all `k`, so every other open cell in the
/// blocks is crossed out. `focus` lists the blocks' cells.
pub(crate) fn block_packing(puzzle: &Puzzle, state: &State, a: &Analysis, k: usize) -> Option<Deduction> {
    for kind in UnitKind::ALL {
        let open: Vec<Unit> = a.open_units(kind).collect();
        if k > open.len() {
            continue;
        }
        let mut chosen = Vec::with_capacity(k);
        if let Some(d) = pack_units(puzzle, state, a, &open, k, 0, &mut chosen) {
            return Some(d);
        }
    }
    None
}

fn pack_units(
    puzzle: &Puzzle,
    state: &State,
    a: &Analysis,
    open: &[Unit],
    k: usize,
    start: usize,
    chosen: &mut Vec<Unit>,
) -> Option<Deduction> {
    if chosen.len() == k {
        let cells: Vec<Cell> = chosen.iter().flat_map(|&u| a.cands(u).iter().copied()).collect();
        let mut blocks = Vec::with_capacity(k);
        return cover(puzzle, &cells, k, &mut blocks, &mut |blocks| {
            let block_cells: Vec<Cell> = blocks.iter().flat_map(|&b| block(b)).collect();
            let mut crosses: Vec<Cell> = block_cells
                .iter()
                .copied()
                .filter(|&c| state.is_candidate(c) && !cells.contains(&c))
                .collect();
            crosses.sort();
            crosses.dedup();
            (!crosses.is_empty())
                .then(|| Deduction::cross(Rule::BlockPacking, crosses, block_cells, chosen.clone()))
        });
    }
    for i in start..open.len() {
        chosen.push(open[i]);
        let found = pack_units(puzzle, state, a, open, k, i + 1, chosen);
        chosen.pop();
        if found.is_some() {
            return found;
        }
    }
    None
}

/// The four cells of the 2x2 block with top-left corner `(r, c)`.
fn block((r, c): Cell) -> [Cell; 4] {
    [(r, c), (r, c + 1), (r + 1, c), (r + 1, c + 1)]
}

/// Tries every way to cover `cells` with at most `k` 2x2 blocks (top-left
/// corners in `blocks`), calling `found` on each cover until it returns Some.
fn cover(
    puzzle: &Puzzle,
    cells: &[Cell],
    k: usize,
    blocks: &mut Vec<Cell>,
    found: &mut dyn FnMut(&[Cell]) -> Option<Deduction>,
) -> Option<Deduction> {
    let covered = |c: &Cell| blocks.iter().any(|&b| block(b).contains(c));
    let Some(&(r, c)) = cells.iter().find(|c| !covered(c)) else {
        return found(blocks);
    };
    if blocks.len() == k {
        return None;
    }
    let n = puzzle.size();
    for (dr, dc) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        let (Some(br), Some(bc)) = (r.checked_sub(dr), c.checked_sub(dc)) else {
            continue;
        };
        if br + 1 >= n || bc + 1 >= n {
            continue;
        }
        blocks.push((br, bc));
        let d = cover(puzzle, cells, k, blocks, found);
        blocks.pop();
        if d.is_some() {
            return d;
        }
    }
    None
}

/// Something that holds at most `cap` animals: a line or region (one), a 2x2
/// block (one, since its cells all touch), or a 3x3 window (two: three
/// animals would need three rows and three columns of it, and the middle
/// row's animal would then touch one of the others).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Container {
    Unit(Unit),
    /// Top-left corner of a 2x2 block.
    Block(Cell),
    /// Top-left corner of a 3x3 window.
    Window(Cell),
}

impl Container {
    fn cap(self) -> usize {
        match self {
            Container::Window(_) => 2,
            _ => 1,
        }
    }

    fn cells(self, puzzle: &Puzzle) -> Vec<Cell> {
        let square = |(r, c): Cell, side: usize| (r..r + side).flat_map(|r| (c..c + side).map(move |c| (r, c))).collect();
        match self {
            Container::Unit(u) => puzzle.unit_cells(u),
            Container::Block(b) => square(b, 2),
            Container::Window(w) => square(w, 3),
        }
    }
}

/// A set of cells, as a bitmask over row-major cell numbers.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Cells([u64; Cells::WORDS]);

impl Cells {
    const WORDS: usize = MAX_SIZE * MAX_SIZE / 64;
    const NONE: Cells = Cells([0; Cells::WORDS]);

    fn insert(&mut self, i: usize) {
        self.0[i / 64] |= 1 << (i % 64);
    }

    fn first(&self) -> Option<usize> {
        let w = self.0.iter().position(|&w| w != 0)?;
        Some(w * 64 + self.0[w].trailing_zeros() as usize)
    }

    fn union(&self, other: &Cells) -> Cells {
        Cells(std::array::from_fn(|i| self.0[i] | other.0[i]))
    }

    fn minus(&self, other: &Cells) -> Cells {
        Cells(std::array::from_fn(|i| self.0[i] & !other.0[i]))
    }

    fn overlaps(&self, other: &Cells) -> bool {
        self.0.iter().zip(&other.0).any(|(a, b)| a & b != 0)
    }
}

/// The generalized counting rules (`MixedPacking`, `MixedBand`,
/// `WindowPacking`): `k` open units that share no open cell need `k` animals.
/// If their open cells fit inside containers holding at most `k` animals in
/// all, every container is full, so its other open cells are crossed out.
/// `units` lists the chosen units, then the covering lines and regions;
/// `focus` lists the cells of the covering blocks and windows.
///
/// `WindowPacking` only looks at two units in one window. Windows combined
/// with other containers solve almost nothing more and are slow to search.
pub(crate) fn packing(puzzle: &Puzzle, state: &State, a: &Analysis, k: usize, rule: Rule) -> Option<Deduction> {
    if rule == Rule::WindowPacking && k != 2 {
        return None;
    }
    let packer = Packer::new(puzzle, state, a, rule);
    let kinds: &[&[UnitKind]] = if rule == Rule::MixedPacking {
        &[&[UnitKind::Row], &[UnitKind::Col], &[UnitKind::Region]]
    } else {
        &[&UnitKind::ALL]
    };
    kinds.iter().find_map(|kinds| {
        let open: Vec<(Unit, Cells)> = kinds
            .iter()
            .flat_map(|&kind| a.open_units(kind))
            .filter(|&u| !a.cands(u).is_empty())
            .map(|u| (u, packer.mask(a.cands(u).iter().copied())))
            .collect();
        packer.choose(&open, k, 0, &mut Vec::with_capacity(k), Cells::NONE)
    })
}

struct Packer<'a> {
    puzzle: &'a Puzzle,
    state: &'a State,
    rule: Rule,
    /// Every container `rule` allows, with its cells.
    containers: Vec<(Container, Cells)>,
    /// For each cell, the `containers` holding it, units before blocks.
    at: Vec<Vec<usize>>,
}

impl<'a> Packer<'a> {
    fn new(puzzle: &'a Puzzle, state: &'a State, a: &Analysis, rule: Rule) -> Packer<'a> {
        let n = puzzle.size();
        let squares = |side: usize| {
            let last = (n + 1).saturating_sub(side);
            (0..last).flat_map(move |r| (0..last).map(move |c| (r, c)))
        };
        let units = UnitKind::ALL.into_iter().flat_map(|kind| a.open_units(kind)).map(Container::Unit);
        let list: Vec<Container> = match rule {
            Rule::MixedBand => units.collect(),
            Rule::WindowPacking => squares(3).map(Container::Window).collect(),
            _ => units.chain(squares(2).map(Container::Block)).collect(),
        };
        let mut packer = Packer {
            puzzle,
            state,
            rule,
            containers: Vec::with_capacity(list.len()),
            at: vec![Vec::new(); n * n],
        };
        for (i, c) in list.into_iter().enumerate() {
            let cells = c.cells(puzzle);
            for &(r, col) in &cells {
                packer.at[r * n + col].push(i);
            }
            packer.containers.push((c, packer.mask(cells.into_iter())));
        }
        packer
    }

    fn mask(&self, cells: impl Iterator<Item = Cell>) -> Cells {
        let mut mask = Cells::NONE;
        for (r, c) in cells {
            mask.insert(r * self.puzzle.size() + c);
        }
        mask
    }

    /// Extends `chosen` (whose open cells are `cells`) to `k` units from
    /// `open[start..]` and looks for a deduction.
    fn choose(
        &self,
        open: &[(Unit, Cells)],
        k: usize,
        start: usize,
        chosen: &mut Vec<Unit>,
        cells: Cells,
    ) -> Option<Deduction> {
        // Adding units never makes a cover cheaper, so give up on this
        // branch once the units chosen so far need more than k animals' room.
        let mut used = Vec::with_capacity(k);
        if !self.fill(cells, chosen, k, false, &mut used, &mut |_| true) {
            return None;
        }
        if chosen.len() == k {
            let one_kind = chosen.iter().all(|u| u.kind() == chosen[0].kind());
            if self.rule == Rule::MixedBand && one_kind {
                return None;
            }
            let mut deduction = None;
            self.fill(cells, chosen, k, true, &mut used, &mut |used| {
                deduction = self.deduce(chosen, used);
                deduction.is_some()
            });
            return deduction;
        }
        for (i, &(unit, mask)) in open.iter().enumerate().skip(start) {
            // Units of different kinds hold different animals only if they
            // share no open cell.
            if cells.overlaps(&mask) {
                continue;
            }
            chosen.push(unit);
            let found = self.choose(open, k, i + 1, chosen, cells.union(&mask));
            chosen.pop();
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// Tries ways to cover `uncovered` with containers, calling `found` on
    /// each cover until it returns true (which `fill` then returns). With
    /// `all` true, tries every cover whose capacities add up to exactly
    /// `budget`. With `all` false, only looks for some cover within
    /// `budget`: blocks and windows that reach above the first uncovered
    /// cell are skipped when one starting at its row covers at least as much.
    fn fill(
        &self,
        uncovered: Cells,
        chosen: &[Unit],
        budget: usize,
        all: bool,
        used: &mut Vec<usize>,
        found: &mut dyn FnMut(&[usize]) -> bool,
    ) -> bool {
        let Some(first) = uncovered.first() else {
            return (budget == 0 || !all) && found(used);
        };
        let n = self.puzzle.size();
        for &i in self.at[first].iter().rev() {
            let (container, mask) = self.containers[i];
            // No cell above the first is uncovered, so sliding a square down
            // (while it stays on the board) only covers more.
            let slides = |(top, _): Cell, side: usize| top < first / n && top + side < n;
            let skip = match container {
                Container::Unit(u) => chosen.contains(&u),
                Container::Block(b) => !all && slides(b, 2),
                Container::Window(w) => !all && slides(w, 3),
            };
            if skip || container.cap() > budget {
                continue;
            }
            used.push(i);
            let done = self.fill(uncovered.minus(&mask), chosen, budget - container.cap(), all, used, found);
            used.pop();
            if done {
                return true;
            }
        }
        false
    }

    /// The deduction from `chosen` filling the `used` containers, if it
    /// crosses anything out.
    fn deduce(&self, chosen: &[Unit], used: &[usize]) -> Option<Deduction> {
        let used: Vec<Container> = used.iter().map(|&i| self.containers[i].0).collect();
        let mut crosses: Vec<Cell> = used
            .iter()
            .flat_map(|c| c.cells(self.puzzle))
            .filter(|&c| self.state.is_candidate(c) && !chosen.iter().any(|&u| self.puzzle.contains(u, c)))
            .collect();
        crosses.sort();
        crosses.dedup();
        if crosses.is_empty() {
            return None;
        }
        let mut units = chosen.to_vec();
        let mut focus = Vec::new();
        for c in used {
            match c {
                Container::Unit(u) => units.push(u),
                _ => focus.extend(c.cells(self.puzzle)),
            }
        }
        Some(Deduction::cross(self.rule, crosses, focus, units))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_packing_fires_across_two_blocks() {
        // Regions 0 and 1 each have one open cell in two diagonal 2x2 blocks,
        // so neither squeeze nor a band applies, but packing does.
        let mut grid = vec![vec![0; 6]; 6];
        for (r, row) in grid.iter_mut().enumerate() {
            for (c, g) in row.iter_mut().enumerate() {
                *g = match (r, c) {
                    (0, 0) | (4, 4) => 0,
                    (1, 1) | (5, 5) => 1,
                    _ => 2 + c % 4,
                };
            }
        }
        let p = Puzzle::new(&grid).unwrap();
        let s = State::new(6);
        let a = Analysis::new(&p, &s);
        let d = block_packing(&p, &s, &a, 2).expect("packing applies");
        assert_eq!(d.crosses, vec![(0, 1), (1, 0), (4, 5), (5, 4)]);
        assert_eq!(d.units, vec![Unit::Region(0), Unit::Region(1)]);
    }

    #[test]
    fn a_window_holds_at_most_two_animals() {
        // Three animals in a 3x3 take one column per row with no two touching.
        let fits = |cols: [usize; 3]| {
            cols[0] != cols[1] && cols[0] != cols[2] && cols[1] != cols[2]
                && cols[0].abs_diff(cols[1]) > 1
                && cols[1].abs_diff(cols[2]) > 1
        };
        for i in 0..27 {
            assert!(!fits([i / 9, i / 3 % 3, i % 3]));
        }
        assert_eq!(Container::Window((0, 0)).cap(), 2);
    }

    #[test]
    fn l_corner_crosses_the_fourth_cell() {
        let grid = vec![
            vec![0, 0, 1, 1],
            vec![0, 2, 1, 1],
            vec![2, 2, 3, 3],
            vec![2, 2, 3, 3],
        ];
        let p = Puzzle::new(&grid).unwrap();
        let s = State::new(4);
        let a = Analysis::new(&p, &s);
        let d = l_corner(&p, &s, &a).expect("L-corner applies");
        assert_eq!(d.crosses, vec![(1, 1)]);
        assert_eq!(d.units, vec![Unit::Region(0)]);
    }

    #[test]
    fn serde_names_match_ids() {
        use serde::de::IntoDeserializer;
        for rule in Rule::ALL {
            let de: serde::de::value::StrDeserializer<serde::de::value::Error> = rule.id().into_deserializer();
            assert_eq!(Rule::deserialize(de).unwrap(), rule);
        }
    }
}
