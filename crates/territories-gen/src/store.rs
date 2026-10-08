//! A game's permanent puzzle collection (such as `data/puzzles.json`):
//! add-only, with stable ids and duplicate detection up to rotation and
//! reflection. The site gets a capped copy (`publish`).

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use territories_core::Level;

use crate::output::{Graded, Output, PuzzleOut};

/// A game whose brutal puzzles come in two kinds, most of them plain: the
/// site's copy keeps a set share of the rare kind, in a shuffled order.
pub struct Mix {
    /// Whether a brutal puzzle with these moves is the rare kind.
    pub rare: fn(&BTreeMap<String, usize>) -> bool,
    /// Each `block` brutal puzzles published in a row hold this many of
    /// the rare kind, at places that differ from block to block.
    pub rare_per_block: usize,
    pub block: usize,
}

impl Mix {
    /// Which puzzles of a size's block number `index` are the rare kind.
    /// Always the same for the same block, so the site's copy only grows.
    fn kinds(&self, size: usize, index: usize) -> Vec<bool> {
        let mut kinds: Vec<bool> = (0..self.block).map(|i| i < self.rare_per_block).collect();
        // SplitMix64, written out so the order never changes under us.
        let mut seed = (size as u64) << 32 | index as u64;
        let mut random = || {
            seed = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let z = (seed ^ (seed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        };
        for i in (1..kinds.len()).rev() {
            kinds.swap(i, (random() % (i as u64 + 1)) as usize);
        }
        kinds
    }

    /// Whether a size's published puzzle number `slot` is the rare kind.
    fn rare_at(&self, size: usize, slot: usize) -> bool {
        self.kinds(size, slot / self.block)[slot % self.block]
    }

    /// How many of a size's first `total` published puzzles are the rare
    /// kind (or, with `rare` false, the plain kind).
    fn share(&self, size: usize, total: usize, rare: bool) -> usize {
        let per_block = if rare { self.rare_per_block } else { self.block - self.rare_per_block };
        let last = self.kinds(size, total / self.block);
        total / self.block * per_block + last[..total % self.block].iter().filter(|&&r| r == rare).count()
    }
}

pub struct Store<S> {
    /// Starts every id this collection hands out.
    prefix: &'static str,
    mix: Option<Mix>,
    /// Rare brutal puzzles per size (see `Mix`).
    rare: BTreeMap<usize, usize>,
    puzzles: Vec<PuzzleOut<S>>,
    keys: HashSet<Vec<u8>>,
    counts: BTreeMap<(usize, Level), usize>,
    /// Next id number per (size, level); ids are never reused or renumbered.
    next: BTreeMap<(usize, Level), usize>,
}

impl<S: Clone + Serialize + DeserializeOwned> Store<S> {
    /// Loads the collection, or starts empty if the file doesn't exist.
    /// New puzzles get ids starting with `prefix`.
    pub fn load(path: &Path, prefix: &'static str, mix: Option<Mix>) -> Result<Store<S>, Box<dyn std::error::Error>> {
        let mut store = Store {
            prefix,
            mix,
            rare: BTreeMap::new(),
            puzzles: Vec::new(),
            keys: HashSet::new(),
            counts: BTreeMap::new(),
            next: BTreeMap::new(),
        };
        if path.exists() {
            let out: Output<S> = serde_json::from_str(&std::fs::read_to_string(path)?)?;
            for p in out.puzzles {
                store.insert_existing(p);
            }
        }
        Ok(store)
    }

    fn insert_existing(&mut self, p: PuzzleOut<S>) {
        if !self.keys.insert(canonical(&p.regions)) {
            return;
        }
        let bucket = (p.size, p.level);
        *self.counts.entry(bucket).or_default() += 1;
        if self.is_rare(p.level, &p.moves) {
            *self.rare.entry(p.size).or_default() += 1;
        }
        let number = id_number(&p.id).unwrap_or(0);
        let next = self.next.entry(bucket).or_insert(1);
        *next = (*next).max(number + 1);
        self.puzzles.push(p);
    }

    pub fn count(&self, size: usize, level: Level) -> usize {
        self.counts.get(&(size, level)).copied().unwrap_or(0)
    }

    fn is_rare(&self, level: Level, moves: &BTreeMap<String, usize>) -> bool {
        level == Level::Brutal && self.mix.as_ref().is_some_and(|mix| (mix.rare)(moves))
    }

    /// Rare brutal puzzles of a size (none in a game without a `Mix`).
    pub fn rare(&self, size: usize) -> usize {
        self.rare.get(&size).copied().unwrap_or(0)
    }

    /// How many more puzzles like this a size/level takes before it holds
    /// `target`. A mixed brutal level counts each kind toward its share.
    fn room(&self, size: usize, level: Level, rare: bool, target: usize) -> usize {
        let held = self.count(size, level);
        match &self.mix {
            Some(mix) if level == Level::Brutal => {
                let kind = if rare { self.rare(size) } else { held - self.rare(size) };
                mix.share(size, target, rare).saturating_sub(kind)
            }
            _ => target.saturating_sub(held),
        }
    }

    /// How many more puzzles a size/level needs to hold `target`.
    pub fn missing(&self, size: usize, level: Level, target: usize) -> usize {
        let mixed = self.mix.is_some() && level == Level::Brutal;
        self.room(size, level, false, target) + if mixed { self.room(size, level, true, target) } else { 0 }
    }

    /// Whether `g` goes toward its size/level holding `target`.
    pub fn wants(&self, g: &impl Graded<Solution = S>, target: usize) -> bool {
        let rare = self.is_rare(g.level(), &g.moves());
        self.room(g.puzzle().size(), g.level(), rare, target) > 0
    }

    pub fn len(&self) -> usize {
        self.puzzles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.puzzles.is_empty()
    }

    /// Adds a new puzzle unless it duplicates one already stored.
    pub fn add(&mut self, g: &impl Graded<Solution = S>) -> bool {
        let regions = g.puzzle().grid();
        if !self.keys.insert(canonical(&regions)) {
            return false;
        }
        let (size, level) = (g.puzzle().size(), g.level());
        let next = self.next.entry((size, level)).or_insert(1);
        let id = format!("{}{}-{size}-{:03}", self.prefix, level.name(), *next);
        *next += 1;
        *self.counts.entry((size, level)).or_default() += 1;
        if self.is_rare(level, &g.moves()) {
            *self.rare.entry(size).or_default() += 1;
        }
        self.puzzles.push(PuzzleOut::new(g, id));
        true
    }

    /// Writes the collection sorted by size, level, then id number. Writes
    /// to a temporary file first so an interrupted save never corrupts it.
    pub fn save(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        self.puzzles
            .sort_by_key(|p| (p.size, p.level, id_number(&p.id).unwrap_or(0)));
        let json = serde_json::to_string(&Output {
            version: 1,
            puzzles: self.puzzles.clone(),
        })?;
        write_atomic(path, &json)
    }

    /// Writes the site's copy: the first `cap` puzzles (by id) of each
    /// size/level, so the published set only ever grows. A mixed brutal
    /// level takes its two kinds in the mix's order, each by id, and ends
    /// where the next kind due runs out.
    pub fn publish(&mut self, path: &Path, cap: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.puzzles
            .sort_by_key(|p| (p.size, p.level, id_number(&p.id).unwrap_or(0)));
        let mut buckets: BTreeMap<(usize, Level), [Vec<&PuzzleOut<S>>; 2]> = BTreeMap::new();
        for p in &self.puzzles {
            buckets.entry((p.size, p.level)).or_default()[self.is_rare(p.level, &p.moves) as usize].push(p);
        }
        let mut puzzles: Vec<PuzzleOut<S>> = Vec::new();
        for ((size, level), kinds) in buckets {
            let mut kinds = kinds.map(|kind| kind.into_iter());
            let mixed = self.mix.as_ref().filter(|_| level == Level::Brutal);
            for slot in 0..cap {
                let rare = mixed.is_some_and(|mix| mix.rare_at(size, slot));
                let Some(p) = kinds[rare as usize].next() else { break };
                puzzles.push(p.clone());
            }
        }
        write_atomic(path, &serde_json::to_string(&Output { version: 1, puzzles })?)
    }
}

fn write_atomic(path: &Path, json: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// The number at the end of an id like `easy-8-042`.
fn id_number(id: &str) -> Option<usize> {
    id.rsplit('-').next()?.parse().ok()
}

/// A key equal for layouts that differ only by rotation, reflection, or
/// region numbering: the smallest relabeled grid over all 8 symmetries.
pub fn canonical(regions: &[Vec<usize>]) -> Vec<u8> {
    let n = regions.len();
    (0..8)
        .map(|t| {
            let mut labels = vec![u8::MAX; n];
            let mut next = 0u8;
            let mut key = Vec::with_capacity(n * n);
            for r in 0..n {
                for c in 0..n {
                    let (r, c) = if t & 1 != 0 { (c, r) } else { (r, c) };
                    let r = if t & 2 != 0 { n - 1 - r } else { r };
                    let c = if t & 4 != 0 { n - 1 - c } else { c };
                    let g = regions[r][c];
                    if labels[g] == u8::MAX {
                        labels[g] = next;
                        next += 1;
                    }
                    key.push(labels[g]);
                }
            }
            key
        })
        .min()
        .expect("eight symmetries")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mix_shares_out_any_total() {
        let mix = Mix { rare: |_| false, rare_per_block: 20, block: 50 };
        assert_eq!((mix.share(9, 1000, true), mix.share(9, 1000, false)), (400, 600));
        for total in [7, 99, usize::MAX] {
            assert_eq!(mix.share(9, total, true) + mix.share(9, total, false), total);
            assert_eq!(mix.share(9, total.min(99), true), (0..total.min(99)).filter(|&i| mix.rare_at(9, i)).count());
        }
        assert_ne!(mix.kinds(9, 0), mix.kinds(9, 1));
        assert_ne!(mix.kinds(9, 0), mix.kinds(10, 0));
    }

    #[test]
    fn canonical_ignores_symmetry_and_labels() {
        let a = vec![vec![0, 0, 1], vec![2, 1, 1], vec![2, 2, 1]];
        // Rotated 90 degrees clockwise, with regions renumbered.
        let b = vec![vec![1, 1, 0], vec![1, 2, 0], vec![2, 2, 2]];
        assert_eq!(canonical(&a), canonical(&b));
        let c = vec![vec![0, 1, 1], vec![2, 1, 1], vec![2, 2, 1]];
        assert_ne!(canonical(&a), canonical(&c));
    }
}
