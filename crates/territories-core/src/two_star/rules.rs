//! Human-style deduction rules for two animals per unit. Every rule looks
//! at the board as it stands: none of them place a mark and follow where
//! it leads.
//!
//! Most rules are one idea. Take some cells known to hold exactly `k`
//! animals, and list the ways `k` animals fit there without touching. A
//! cell every way uses is an animal. A cell where an animal would leave no
//! way at all is crossed out.

use std::ops::RangeInclusive;

use serde::{Deserialize, Serialize};

use super::ANIMALS;
use crate::board::{Cell, Mark, Puzzle, State, Unit};
use crate::level::Level;

/// Largest supported grid; cell sets are stored as `u128` bitmasks.
pub const MAX_SIZE: usize = 11;

/// Largest territory a `SmallSqueeze` reasons about, in open cells.
const SMALL: u32 = 4;

/// Largest animal count a leftover is reasoned about with.
const MAX_LEFTOVER: u32 = 4;

/// Most units a `MixedBand` fits others within.
const MAX_MIXED: usize = 3;

/// Longest run of lines a `Loose` shares out.
const MAX_LOOSE: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rule {
    /// An animal crosses out the cells touching it.
    AnimalShadow,
    /// A unit with both its animals: cross out the rest of it.
    FullUnit,
    /// A unit with as many open cells as animals it still needs gets them.
    LastSpots,
    /// Territories whose open cells lie in one line, needing just as many
    /// animals as it does: cross out the rest of the line.
    ClaimedLine,
    /// One line's open cells lie in territories needing just as many
    /// animals as it does: cross out the rest of those territories.
    ClaimedRegion,
    /// `Squeeze` on a unit with at most `SMALL` open cells.
    SmallSqueeze,
    /// The ways one unit's open cells can hold its animals: a cell in all
    /// of them is an animal, and a cell where an animal would leave no way
    /// is crossed out.
    Squeeze,
    /// `Squeeze`, also counting the room other units have left.
    CrowdedSqueeze,
    /// `ClaimedLine` with two lines.
    #[serde(rename = "region_band_2")]
    RegionBand2,
    /// `ClaimedRegion` with two lines.
    #[serde(rename = "line_band_2")]
    LineBand2,
    /// `ClaimedLine` with three or more lines.
    #[serde(rename = "region_band_3")]
    RegionBand3,
    /// `ClaimedRegion` with three or more lines.
    #[serde(rename = "line_band_3")]
    LineBand3,
    /// `CrowdedSqueeze` on a leftover: what one line shares with the
    /// territories around it holds a known number of animals.
    Leftover,
    /// `Leftover` across a run of two or more neighboring lines.
    WideLeftover,
    /// Units of any kinds that share no open cell, inside units of any
    /// kinds that need just as many animals: cross out the rest of those.
    MixedBand,
    /// A squeeze on a unit that also counts a leftover: the unit takes no
    /// more animals from the leftover's cells than they hold.
    LeftoverCap,
    /// Some lines' animals are shared among the territories they cross. A
    /// territory the others can't cover for must take some of them, so the
    /// rest of it holds fewer: a squeeze on another unit counts that.
    Loose,
}

impl Rule {
    pub const ALL: [Rule; 17] = [
        Rule::AnimalShadow,
        Rule::FullUnit,
        Rule::LastSpots,
        Rule::ClaimedLine,
        Rule::ClaimedRegion,
        Rule::SmallSqueeze,
        Rule::Squeeze,
        Rule::CrowdedSqueeze,
        Rule::RegionBand2,
        Rule::LineBand2,
        Rule::RegionBand3,
        Rule::LineBand3,
        Rule::Leftover,
        Rule::WideLeftover,
        Rule::MixedBand,
        Rule::LeftoverCap,
        Rule::Loose,
    ];

    pub fn level(self) -> Level {
        match self {
            Rule::AnimalShadow
            | Rule::FullUnit
            | Rule::LastSpots
            | Rule::ClaimedLine
            | Rule::ClaimedRegion
            | Rule::SmallSqueeze => Level::Easy,
            Rule::Squeeze | Rule::CrowdedSqueeze | Rule::RegionBand2 | Rule::LineBand2 => Level::Medium,
            Rule::RegionBand3 | Rule::LineBand3 | Rule::Leftover => Level::Hard,
            Rule::WideLeftover | Rule::MixedBand | Rule::LeftoverCap | Rule::Loose => Level::Brutal,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Rule::AnimalShadow => "animal_shadow",
            Rule::FullUnit => "full_unit",
            Rule::LastSpots => "last_spots",
            Rule::ClaimedLine => "claimed_line",
            Rule::ClaimedRegion => "claimed_region",
            Rule::SmallSqueeze => "small_squeeze",
            Rule::Squeeze => "squeeze",
            Rule::CrowdedSqueeze => "crowded_squeeze",
            Rule::RegionBand2 => "region_band_2",
            Rule::LineBand2 => "line_band_2",
            Rule::RegionBand3 => "region_band_3",
            Rule::LineBand3 => "line_band_3",
            Rule::Leftover => "leftover",
            Rule::WideLeftover => "wide_leftover",
            Rule::MixedBand => "mixed_band",
            Rule::LeftoverCap => "leftover_cap",
            Rule::Loose => "loose",
        }
    }
}

/// One application of a rule. `focus`, `units`, and `count` explain *why*
/// (for hint highlighting); `animals` and `crosses` are *what* to mark.
///
/// `units` reads differently by rule:
/// - claimed and bands: the units that fit, then the units they fit within
///   (territories in lines for `ClaimedLine` and the `RegionBand`s, lines
///   in territories for `ClaimedRegion` and the `LineBand`s).
/// - leftovers: lines and then the territories they touch (the leftover is
///   the rest of those territories), or territories and then the lines
///   they lie within (the leftover is the rest of those lines).
///
/// - crowded squeeze: the squeezed unit, then the units with too little
///   room for some of its arrangements.
/// - mixed band: the `inner` units that fit, then the units they fit within.
/// - leftover cap: the leftover's units (as for leftovers), then the
///   squeezed unit. `focus` is the leftover and `count` what it holds.
/// - loose: the `inner` lines sharing their animals out, then the territory
///   that must take some, then the squeezed unit. `focus` is the rest of
///   that territory and `count` the most it holds. `line_need` is what
///   the lines still need, and `filling` what the territory must take.
///   `others` is the other territories crossing the lines.
///
/// `count` is the number of animals the rule counted: those the squeezed
/// unit still needs, those a band claims, or those a leftover holds.
///
/// `filled` is for a crowded squeeze's crosses: the lines holding them
/// that the squeezed unit's animals fill, in the arrangements where no
/// animal touches the crossed cells. `filling` is how many of its animals
/// it takes to fill each of those lines (0 if they differ, or none do).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Deduction {
    pub rule: Rule,
    pub animals: Vec<Cell>,
    pub crosses: Vec<Cell>,
    pub focus: Vec<Cell>,
    pub units: Vec<Unit>,
    pub count: usize,
    pub filled: Vec<Unit>,
    pub filling: usize,
    /// For a leftover: the animals its named lines still need. The
    /// territories named need `count` more than that (lines first) or
    /// `count` fewer (territories first).
    pub line_need: usize,
    /// How many of `units` come first, for the rules that say so.
    pub inner: usize,
    /// For a loose leftover: the other territories with open cells in its
    /// lines.
    pub others: Vec<Unit>,
}

/// A set of cells, as a bitmask over row-major cell numbers.
pub(crate) type Set = u128;

pub(crate) fn bit(i: usize) -> Set {
    1 << i
}

pub(crate) fn cells(mut s: Set) -> impl Iterator<Item = usize> {
    std::iter::from_fn(move || {
        if s == 0 {
            return None;
        }
        let i = s.trailing_zeros() as usize;
        s &= s - 1;
        Some(i)
    })
}

/// Animals still needed per unit: rows, then columns, then territories.
type Caps = [u8; 3 * MAX_SIZE];

/// A puzzle's geometry as cell sets.
pub(crate) struct Board {
    pub n: usize,
    pub region: Vec<usize>,
    /// Cells of each unit: rows `0..n`, columns `n..2n`, territories `2n..3n`.
    pub units: Vec<Set>,
    /// Cells touching each cell, including diagonally.
    pub nbrs: Vec<Set>,
}

impl Board {
    /// Panics if the puzzle is larger than `MAX_SIZE`.
    pub fn new(puzzle: &Puzzle) -> Board {
        let n = puzzle.size();
        assert!(n <= MAX_SIZE, "size {n} is above {MAX_SIZE}");
        let region: Vec<usize> = puzzle.cells().map(|c| puzzle.region(c)).collect();
        let mut units = vec![0; 3 * n];
        let mut nbrs = vec![0; n * n];
        for c in 0..n * n {
            let (r, k) = (c / n, c % n);
            units[r] |= bit(c);
            units[n + k] |= bit(c);
            units[2 * n + region[c]] |= bit(c);
            for rr in r.saturating_sub(1)..=(r + 1).min(n - 1) {
                for kk in k.saturating_sub(1)..=(k + 1).min(n - 1) {
                    if (rr, kk) != (r, k) {
                        nbrs[c] |= bit(rr * n + kk);
                    }
                }
            }
        }
        Board { n, region, units, nbrs }
    }

    fn units_of(&self, c: usize) -> [usize; 3] {
        [c / self.n, self.n + c % self.n, 2 * self.n + self.region[c]]
    }

    fn unit(&self, u: usize) -> Unit {
        match u / self.n {
            0 => Unit::Row(u),
            1 => Unit::Col(u - self.n),
            _ => Unit::Region(u - 2 * self.n),
        }
    }

    fn list(&self, s: Set) -> Vec<Cell> {
        cells(s).map(|c| (c / self.n, c % self.n)).collect()
    }
}

/// Room left for animals: in each unit, and in one more set of cells
/// (`extra`, empty for none) that takes at most `most`.
#[derive(Clone, Copy)]
struct Room {
    caps: Caps,
    extra: Set,
    most: u8,
}

impl Room {
    /// Room for two animals in every unit.
    const FREE: Room = Room { caps: [ANIMALS as u8; 3 * MAX_SIZE], extra: 0, most: 0 };

    /// The room left once `c` takes an animal, and the cells that leaves
    /// with no room. `None` if `c` had no room itself.
    fn take(mut self, b: &Board, c: usize) -> Option<(Room, Set)> {
        let mut full = 0;
        for u in b.units_of(c) {
            self.caps[u] = self.caps[u].checked_sub(1)?;
            if self.caps[u] == 0 {
                full |= b.units[u];
            }
        }
        if self.extra & bit(c) != 0 {
            self.most = self.most.checked_sub(1)?;
            if self.most == 0 {
                full |= self.extra;
            }
        }
        Some((self, full))
    }
}

/// Whether `m` more animals fit in `s`: none touching, and nothing taking
/// more than `room` allows.
fn fit(b: &Board, room: &Room, s: Set, m: u32) -> bool {
    if m == 0 {
        return true;
    }
    if s.count_ones() < m {
        return false;
    }
    let c = s.trailing_zeros() as usize;
    let rest = s & !b.nbrs[c] & !bit(c);
    room.take(b, c).is_some_and(|(left, full)| fit(b, &left, rest & !full, m - 1)) || fit(b, room, s & !bit(c), m)
}

/// A position, with each unit's remaining need worked out.
pub(crate) struct View<'a> {
    b: &'a Board,
    open: Set,
    stars: Set,
    caps: Caps,
}

impl<'a> View<'a> {
    pub fn new(b: &'a Board, state: &State) -> View<'a> {
        let n = b.n;
        let (mut open, mut stars) = (0, 0);
        for c in 0..n * n {
            match state.get((c / n, c % n)) {
                Mark::Empty => open |= bit(c),
                Mark::Animal => stars |= bit(c),
                Mark::Cross => {}
            }
        }
        let mut caps = [0; 3 * MAX_SIZE];
        for (u, &cells) in b.units.iter().enumerate() {
            caps[u] = (ANIMALS as u8).saturating_sub((stars & cells).count_ones() as u8);
        }
        View { b, open, stars, caps }
    }

    /// Every animal is placed.
    pub fn solved(&self) -> bool {
        self.stars.count_ones() as usize == ANIMALS * self.b.n
    }

    fn need(&self, u: usize) -> u32 {
        self.caps[u] as u32
    }

    fn cands(&self, u: usize) -> Set {
        self.open & self.b.units[u]
    }

    fn open_units(&self) -> impl Iterator<Item = usize> + '_ {
        (0..3 * self.b.n).filter(|&u| self.need(u) > 0)
    }

    fn deduction(&self, rule: Rule, animals: Set, crosses: Set, focus: Set, units: Vec<usize>, count: u32) -> Deduction {
        Deduction {
            rule,
            animals: self.b.list(animals),
            crosses: self.b.list(crosses),
            focus: self.b.list(focus),
            units: units.into_iter().map(|u| self.b.unit(u)).collect(),
            count: count as usize,
            filled: Vec::new(),
            filling: 0,
            line_need: 0,
            inner: 0,
            others: Vec::new(),
        }
    }

    fn room(&self) -> Room {
        Room { caps: self.caps, ..Room::FREE }
    }

    pub fn animal_shadow(&self) -> Option<Deduction> {
        let casting = cells(self.stars)
            .filter(|&c| self.b.nbrs[c] & self.open != 0)
            .fold(0, |acc, c| acc | bit(c));
        let crosses = cells(casting).fold(0, |acc, c| acc | self.b.nbrs[c]) & self.open;
        (crosses != 0).then(|| self.deduction(Rule::AnimalShadow, 0, crosses, casting, vec![], 0))
    }

    pub fn full_unit(&self) -> Option<Deduction> {
        let full: Vec<usize> = (0..3 * self.b.n)
            .filter(|&u| self.need(u) == 0 && self.cands(u) != 0)
            .collect();
        let crosses = full.iter().fold(0, |acc, &u| acc | self.cands(u));
        (crosses != 0).then(|| self.deduction(Rule::FullUnit, 0, crosses, 0, full, ANIMALS as u32))
    }

    pub fn last_spots(&self) -> Option<Deduction> {
        let u = self.open_units().find(|&u| self.cands(u).count_ones() == self.need(u))?;
        Some(self.deduction(Rule::LastSpots, self.cands(u), 0, self.cands(u), vec![u], self.need(u)))
    }

    /// What must hold however `k` animals go in `s`: cells of `s` they
    /// always use (animals), and open cells where an animal would leave `s`
    /// unable to fit them (crosses). With `crowded`, other units' remaining
    /// room counts too; without, only touching does.
    fn certain(&self, s: Set, k: u32, crowded: bool) -> (Set, Set) {
        self.certain_in(s, k, crowded.then(|| self.room()))
    }

    /// `certain`, crowded by the room given (if any).
    fn certain_in(&self, s: Set, k: u32, crowded: Option<Room>) -> (Set, Set) {
        let base = crowded.unwrap_or(Room::FREE);
        let animals = cells(s)
            .filter(|&c| !fit(self.b, &base, s & !bit(c), k))
            .fold(0, |acc, c| acc | bit(c));
        let mut crosses = 0;
        for c in cells(self.open) {
            let taken = if crowded.is_some() { base.take(self.b, c) } else { Some((base, 0)) };
            let Some((room, full)) = taken else {
                crosses |= bit(c);
                continue;
            };
            let killed = self.b.nbrs[c] | bit(c) | full;
            let inside = s & bit(c) != 0;
            if (inside || killed & s != 0) && !fit(self.b, &room, s & !killed, k - inside as u32) {
                crosses |= bit(c);
            }
        }
        (animals, crosses)
    }

    /// `SmallSqueeze`, `Squeeze`, or `CrowdedSqueeze` (as `rule` says) on
    /// the unit with the fewest open cells that gives anything.
    pub fn squeeze(&self, rule: Rule) -> Option<Deduction> {
        let mut units: Vec<usize> = self.open_units().collect();
        units.sort_by_key(|&u| self.cands(u).count_ones());
        units.into_iter().find_map(|u| {
            let (s, k) = (self.cands(u), self.need(u));
            if rule == Rule::SmallSqueeze && s.count_ones() > SMALL {
                return None;
            }
            let (animals, crosses) = self.certain(s, k, rule == Rule::CrowdedSqueeze);
            if animals | crosses == 0 {
                return None;
            }
            Some(match rule {
                Rule::CrowdedSqueeze => self.crowded(u, animals, crosses),
                _ => self.deduction(rule, animals, crosses, s, vec![u], k),
            })
        })
    }

    /// Every way to put `k` animals in `s` with none touching.
    fn arrangements(&self, s: Set, k: u32) -> Vec<Set> {
        if k == 0 {
            return vec![0];
        }
        let mut out = Vec::new();
        let mut rest = s;
        for c in cells(s) {
            rest &= !bit(c);
            let others = self.arrangements(rest & !self.b.nbrs[c], k - 1);
            out.extend(others.into_iter().map(|a| a | bit(c)));
        }
        out
    }

    /// A crowded squeeze on unit `u` as one explainable step, given
    /// everything `certain` found. Either the animals, with the crosses no
    /// arrangement leaves room for; or, failing those, the crosses that
    /// share a reason: in every arrangement they touch an animal or lie in
    /// the same units that the arrangement fills.
    fn crowded(&self, u: usize, animals: Set, crosses: Set) -> Deduction {
        let (s, k) = (self.cands(u), self.need(u));
        let held = |a: Set, v: usize| (a & self.b.units[v]).count_ones();
        let all = self.arrangements(s, k);
        let short: Vec<usize> = (0..3 * self.b.n)
            .filter(|&v| v != u && all.iter().any(|&a| held(a, v) > self.need(v)))
            .collect();
        let left: Vec<Set> = all
            .into_iter()
            .filter(|&a| (0..3 * self.b.n).all(|v| held(a, v) <= self.need(v)))
            .collect();
        let filled = |c: usize| -> Vec<usize> {
            let apart = left.iter().filter(|&&a| a & self.b.nbrs[c] == 0);
            let fills = |v: &usize| apart.clone().any(|&a| held(a, *v) == self.need(*v));
            if s & bit(c) != 0 { Vec::new() } else { self.b.units_of(c).into_iter().filter(fills).collect() }
        };
        let plain = cells(crosses).filter(|&c| filled(c).is_empty()).fold(0, |acc, c| acc | bit(c));
        let (crosses, filled) = if animals | plain != 0 {
            (plain, Vec::new())
        } else {
            let reason = filled(crosses.trailing_zeros() as usize);
            let same = cells(crosses).filter(|&c| filled(c) == reason).fold(0, |acc, c| acc | bit(c));
            (same, reason)
        };
        let units = std::iter::once(u).chain(short).collect();
        let room = filled.first().map_or(0, |&v| self.need(v));
        Deduction {
            filling: if filled.iter().all(|&v| self.need(v) == room) { room as usize } else { 0 },
            filled: filled.into_iter().map(|v| self.b.unit(v)).collect(),
            ..self.deduction(Rule::CrowdedSqueeze, animals, crosses, s, units, k)
        }
    }

    /// Rows (`dir` 0) or columns (1) that still need an animal.
    fn open_lines(&self, dir: usize) -> Vec<usize> {
        let n = self.b.n;
        (dir * n..(dir + 1) * n).filter(|&u| self.need(u) > 0).collect()
    }

    /// For the `lines` picked by `mask`: the open cells of the territories
    /// touching those lines that lie outside them, and how many animals
    /// those cells must hold.
    fn outside(&self, lines: &[usize], mask: u32) -> (Set, u32) {
        let n = self.b.n;
        let (mut inside, mut need_lines) = (0, 0);
        for (i, &u) in lines.iter().enumerate() {
            if mask & (1 << i) != 0 {
                inside |= self.cands(u);
                need_lines += self.need(u);
            }
        }
        let (mut touching, mut need_regions) = (0, 0);
        for u in 2 * n..3 * n {
            if self.cands(u) & inside != 0 {
                touching |= self.cands(u);
                need_regions += self.need(u);
            }
        }
        (touching & !inside, need_regions.saturating_sub(need_lines))
    }

    /// The units behind `outside(lines, mask)`, told the shorter way: the
    /// picked lines and then the territories touching them, or (if most
    /// lines are picked) the territories lying within the other lines and
    /// then those lines. Also the animals the lines named still need.
    fn describe(&self, lines: &[usize], mask: u32) -> (Vec<usize>, u32) {
        let n = self.b.n;
        let picked = |want: bool| {
            lines.iter().enumerate().filter(move |&(i, _)| (mask & (1 << i) != 0) == want).map(|(_, &u)| u)
        };
        let inside = picked(true).fold(0, |acc, u| acc | self.cands(u));
        let touching = |u: &usize| self.cands(*u) & inside != 0;
        let few = mask.count_ones() as usize <= lines.len() / 2;
        let units: Vec<usize> = if few {
            picked(true).chain((2 * n..3 * n).filter(touching)).collect()
        } else {
            let within = (2 * n..3 * n).filter(|u| self.need(*u) > 0 && !touching(u));
            within.chain(picked(false)).collect()
        };
        (units, picked(few).map(|u| self.need(u)).sum())
    }

    /// The claimed and band rules for `sizes` lines, smallest first: lines
    /// whose animals exactly fill the territories they touch, so the rest
    /// of those territories is crossed out (`ClaimedRegion`, `LineBand2`,
    /// `LineBand3`). Seen the other way, the territories within the
    /// remaining lines fill those lines (`ClaimedLine`, `RegionBand2`,
    /// `RegionBand3`). Whichever way names fewer lines is the one reported.
    pub fn band(&self, sizes: RangeInclusive<usize>) -> Option<Deduction> {
        let mut best: Option<(usize, Set, usize, u32)> = None;
        for dir in 0..2 {
            let lines = self.open_lines(dir);
            let m = lines.len();
            for mask in 1..(1u32 << m).saturating_sub(1) {
                let a = mask.count_ones() as usize;
                let k = a.min(m - a);
                if !sizes.contains(&k) || best.is_some_and(|(least, ..)| least <= k) {
                    continue;
                }
                let (crosses, count) = self.outside(&lines, mask);
                if count == 0 && crosses != 0 {
                    best = Some((k, crosses, dir, mask));
                }
            }
        }
        let (k, crosses, dir, mask) = best?;
        let lines = self.open_lines(dir);
        let lines_first = mask.count_ones() as usize <= lines.len() / 2;
        let rule = match (k, lines_first) {
            (1, true) => Rule::ClaimedRegion,
            (1, false) => Rule::ClaimedLine,
            (2, true) => Rule::LineBand2,
            (2, false) => Rule::RegionBand2,
            (_, true) => Rule::LineBand3,
            (_, false) => Rule::RegionBand3,
        };
        let (units, count) = self.describe(&lines, mask);
        Some(self.deduction(rule, 0, crosses, 0, units, count))
    }

    /// `Leftover` (one line) or `WideLeftover` (a run of neighboring
    /// lines), smallest first. Both a run and everything but the run give
    /// a leftover: the rest of the territories the picked lines touch.
    pub fn leftover(&self, rule: Rule) -> Option<Deduction> {
        self.leftovers(rule == Rule::WideLeftover).into_iter().find_map(|(cells, count, dir, mask)| {
            let (animals, crosses) = self.certain(cells, count, true);
            (animals | crosses != 0).then(|| {
                let (units, line_need) = self.describe(&self.open_lines(dir), mask);
                Deduction {
                    line_need: line_need as usize,
                    ..self.deduction(rule, animals, crosses, cells, units, count)
                }
            })
        })
    }

    /// Every leftover from one line, or (`wide`) from a run of neighboring
    /// lines, smallest first: its cells, the animals they hold, and the
    /// lines that gave it (a direction and a mask over its open lines).
    fn leftovers(&self, wide: bool) -> Vec<(Set, u32, usize, u32)> {
        let mut sets: Vec<(Set, u32, usize, usize, u32)> = Vec::new();
        for dir in 0..2 {
            let lines = self.open_lines(dir);
            let m = lines.len();
            let full = (1u32 << m) - 1;
            for i in 0..m {
                for j in i..m {
                    let len = j - i + 1;
                    let k = len.min(m - len);
                    if (k == 1) == wide {
                        continue;
                    }
                    let run = (full >> (m - len)) << i;
                    for mask in [run, full & !run] {
                        let (cells, count) = self.outside(&lines, mask);
                        let known = sets.iter().any(|s| (s.0, s.1) == (cells, count));
                        if mask != 0 && cells != 0 && (1..=MAX_LEFTOVER).contains(&count) && !known {
                            sets.push((cells, count, k, dir, mask));
                        }
                    }
                }
            }
        }
        sets.sort_by_key(|&(cells, count, k, ..)| (k, count, cells.count_ones()));
        sets.into_iter().map(|(cells, count, _, dir, mask)| (cells, count, dir, mask)).collect()
    }

    /// A crowded squeeze on some unit outside `skip`, also counting that
    /// the cells of `set` hold at most `most` animals: the unit, and what
    /// its squeeze gives. Units with the fewest open cells go first.
    fn capped(&self, set: Set, most: u32, skip: &[usize]) -> Option<(usize, Set, Set)> {
        let room = Room { extra: set, most: most as u8, ..self.room() };
        let mut units: Vec<usize> = self.open_units().filter(|u| !skip.contains(u)).collect();
        units.sort_by_key(|&u| self.cands(u).count_ones());
        units.into_iter().find_map(|u| {
            let s = self.cands(u);
            // Without room for more of the unit's animals than `most`, the
            // set changes nothing.
            if !fit(self.b, &self.room(), s & set, most + 1) {
                return None;
            }
            let (animals, crosses) = self.certain_in(s, self.need(u), Some(room));
            (animals | crosses != 0).then_some((u, animals, crosses))
        })
    }

    /// `LeftoverCap`: `capped` with a leftover (of any width) as the set.
    pub fn leftover_cap(&self) -> Option<Deduction> {
        let sets = [false, true].into_iter().flat_map(|wide| self.leftovers(wide));
        sets.into_iter().find_map(|(cells, count, dir, mask)| {
            let (u, animals, crosses) = self.capped(cells, count, &[])?;
            let (mut units, line_need) = self.describe(&self.open_lines(dir), mask);
            let inner = units.len();
            units.push(u);
            Some(Deduction {
                line_need: line_need as usize,
                inner,
                ..self.deduction(Rule::LeftoverCap, animals, crosses, cells, units, count)
            })
        })
    }

    /// `Loose`, for runs of up to `MAX_LOOSE` neighboring lines, shortest
    /// first. The run's animals are shared among the territories crossing
    /// it, and each takes no more than fits in its part of the run. What
    /// the others can't take, a territory must: the rest of that territory
    /// then holds at most what it has left, which `capped` counts.
    pub fn loose(&self) -> Option<Deduction> {
        let n = self.b.n;
        for len in 1..=MAX_LOOSE {
            for dir in 0..2 {
                let lines = self.open_lines(dir);
                for run in lines.windows(len) {
                    let inside = run.iter().fold(0, |acc, &u| acc | self.cands(u));
                    let need: u32 = run.iter().map(|&u| self.need(u)).sum();
                    let parts: Vec<(usize, u32)> = (2 * n..3 * n)
                        .filter(|&v| self.cands(v) & inside != 0)
                        .map(|v| (v, self.most(self.cands(v) & inside, self.need(v))))
                        .collect();
                    let total: u32 = parts.iter().map(|p| p.1).sum();
                    for &(v, most) in &parts {
                        let least = need.saturating_sub(total - most);
                        let rest = self.cands(v) & !inside;
                        if least == 0 || rest == 0 {
                            continue;
                        }
                        let left = self.need(v) - least;
                        let found = if left == 0 { Some((v, 0, rest)) } else { self.capped(rest, left, &[v]) };
                        if let Some((u, animals, crosses)) = found {
                            let units = run.iter().copied().chain([v, u]).collect();
                            let others = parts.iter().filter(|p| p.0 != v).map(|p| self.b.unit(p.0)).collect();
                            return Some(Deduction {
                                line_need: need as usize,
                                filling: least as usize,
                                inner: len,
                                others,
                                ..self.deduction(Rule::Loose, animals, crosses, rest, units, left)
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// The most animals that fit in `s`, up to `limit`.
    fn most(&self, s: Set, limit: u32) -> u32 {
        (1..=limit).take_while(|&m| fit(self.b, &self.room(), s, m)).count() as u32
    }

    /// `MixedBand`, for up to `MAX_MIXED` outer units, fewest first.
    pub fn mixed_band(&self) -> Option<Deduction> {
        let open: Vec<usize> = self.open_units().collect();
        (1..=MAX_MIXED).find_map(|size| self.mixed(&open, size, &mut Vec::new()))
    }

    /// `MixedBand` with `outer` grown to `size` units from `open`, each
    /// later in `open` than the last.
    fn mixed(&self, open: &[usize], size: usize, outer: &mut Vec<usize>) -> Option<Deduction> {
        if outer.len() < size {
            let from = outer.last().map_or(0, |&last| open.iter().position(|&u| u == last).unwrap() + 1);
            for &u in &open[from..] {
                outer.push(u);
                let found = self.mixed(open, size, outer);
                outer.pop();
                if found.is_some() {
                    return found;
                }
            }
            return None;
        }
        // Cells of the outer units, and those two of them share. Both
        // animals of each outer unit go to the units inside only if none
        // sits on a shared cell.
        let (mut within, mut shared, mut need) = (0, 0, 0);
        for &u in outer.iter() {
            shared |= within & self.cands(u);
            within |= self.cands(u);
            need += self.need(u);
        }
        let inside: Vec<usize> = open
            .iter()
            .copied()
            .filter(|u| !outer.contains(u) && self.cands(*u) & !within == 0)
            .collect();
        let (fits, used) = self.neediest(&inside, need, 0)?;
        let crosses = (within & !used) | shared;
        (crosses != 0).then(|| {
            let units = fits.iter().chain(outer.iter()).copied().collect();
            Deduction { inner: fits.len(), ..self.deduction(Rule::MixedBand, 0, crosses, used, units, need) }
        })
    }

    /// Units among `from` sharing no open cell with `used` or each other
    /// and needing `need` animals in all, with the cells they cover.
    fn neediest(&self, from: &[usize], need: u32, used: Set) -> Option<(Vec<usize>, Set)> {
        if need == 0 {
            return Some((Vec::new(), used));
        }
        let (&u, rest) = from.split_first()?;
        if self.cands(u) & used == 0 && self.need(u) <= need
            && let Some((mut units, cells)) = self.neediest(rest, need - self.need(u), used | self.cands(u))
        {
            units.insert(0, u);
            return Some((units, cells));
        }
        self.neediest(rest, need, used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn squeezed(regions: Vec<Vec<usize>>, state: &State) -> Deduction {
        let puzzle = Puzzle::new(&regions).unwrap();
        let board = Board::new(&puzzle);
        View::new(&board, state).squeeze(Rule::Squeeze).unwrap()
    }

    /// A territory down to a cell above a 2x2 block: the top cell is an
    /// animal, the block's top row is crossed out, and so are the two cells
    /// below the block (they touch both cells of its bottom row).
    #[test]
    fn cell_above_a_block() {
        let n = 9;
        let shape = [(2, 3), (3, 3), (3, 4), (4, 3), (4, 4)];
        // Territory 0 is the shape; the rest are too big to give anything.
        let regions: Vec<Vec<usize>> = (0..n)
            .map(|r| (0..n).map(|c| if shape.contains(&(r, c)) { 0 } else { 1 + (r * n + c) % 8 }).collect())
            .collect();
        let d = squeezed(regions, &State::new(n));
        assert_eq!(d.units, vec![Unit::Region(0)]);
        assert_eq!(d.animals, vec![(2, 3)]);
        for cell in [(3, 3), (3, 4), (5, 3), (5, 4)] {
            assert!(d.crosses.contains(&cell), "{cell:?}");
        }
        assert!(!d.crosses.contains(&(4, 3)) && !d.crosses.contains(&(4, 4)));
    }

    /// A line down to four open cells in a run: the cells on both sides of
    /// the run are crossed out.
    #[test]
    fn four_in_a_run() {
        let n = 9;
        let regions: Vec<Vec<usize>> = (0..n).map(|r| vec![r; n]).collect();
        let mut state = State::new(n);
        for r in [0, 1, 6, 7, 8] {
            state.set((r, 4), Mark::Cross);
        }
        let d = squeezed(regions, &state);
        assert_eq!(d.units, vec![Unit::Col(4)]);
        assert!(d.animals.is_empty());
        let sides: Vec<Cell> = (2..=5).flat_map(|r| [(r, 3), (r, 5)]).collect();
        assert_eq!(d.crosses, sides);
    }

    /// A territory with one open cell in the top row and four in the next,
    /// where that next row already holds an animal: the row takes only one
    /// more, so the top cell is an animal. Only the crowded squeeze counts
    /// the row's room.
    #[test]
    fn a_row_with_room_for_one() {
        let n = 9;
        // Teal is 0, blue 1, white 2 (already full); each later row is a territory.
        let mut regions = vec![vec![0, 0, 0, 0, 1, 1, 2, 2, 1], vec![0, 0, 0, 0, 0, 2, 2, 1, 1], vec![1; n]];
        regions.extend((3..n).map(|r| vec![r; n]));
        let puzzle = Puzzle::new(&regions).unwrap();
        let mut state = State::new(n);
        for (r, row) in ["oxxxoxx*x", "ooooxAxxx", "....xxx.."].iter().enumerate() {
            for (c, mark) in row.chars().enumerate() {
                state.set((r, c), match mark {
                    'x' => Mark::Cross,
                    '*' | 'A' => Mark::Animal,
                    _ => Mark::Empty,
                });
            }
        }
        let board = Board::new(&puzzle);
        let view = View::new(&board, &state);
        assert!(view.squeeze(Rule::Squeeze).is_none_or(|d| d.animals.is_empty()));
        let d = view.squeeze(Rule::CrowdedSqueeze).unwrap();
        // Teal, then the row with room for only one more.
        assert_eq!(d.units, vec![Unit::Region(0), Unit::Row(1)]);
        assert_eq!(d.animals, vec![(0, 0)]);
        assert!(d.crosses.contains(&(1, 0)) && d.crosses.contains(&(1, 1)));
        assert!(d.filled.is_empty());
    }

    /// A territory running down the first column with one more cell in the
    /// second: it either uses that cell, which touches the cell above the
    /// column, or puts both animals in the column and fills it.
    #[test]
    fn a_column_it_would_fill() {
        let n = 9;
        // Territory 0 is rows 4-8 of column 0 plus (4, 1); 1 is the rest of
        // the first two columns; each later column is a territory.
        let regions: Vec<Vec<usize>> = (0..n)
            .map(|r| {
                let mut row: Vec<usize> = (0..n).map(|c| c.max(1)).collect();
                row[0] = if r >= 4 { 0 } else { 1 };
                row[1] = if r == 4 { 0 } else { 1 };
                row
            })
            .collect();
        let puzzle = Puzzle::new(&regions).unwrap();
        let board = Board::new(&puzzle);
        // Plain squeezes go first. They never find the cell above the column.
        let mut state = State::new(n);
        while let Some(d) = View::new(&board, &state).squeeze(Rule::Squeeze) {
            assert!(d.animals.is_empty() && !d.crosses.contains(&(3, 0)));
            d.crosses.iter().for_each(|&c| state.set(c, Mark::Cross));
        }
        let view = View::new(&board, &state);
        let d = view.squeeze(Rule::CrowdedSqueeze).unwrap();
        assert_eq!(d.units, vec![Unit::Region(0)]);
        assert!(d.animals.is_empty());
        assert_eq!(d.crosses, vec![(3, 0)]);
        assert_eq!((d.filled, d.filling), (vec![Unit::Col(0)], 2));
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
