//! The permanent puzzle collection (`data/puzzles.json`): add-only, with
//! stable ids and duplicate detection up to rotation and reflection. The
//! site gets a capped copy (`publish`).

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use territories_core::Level;

use crate::generate::Generated;
use crate::output::{Output, PuzzleOut};

pub struct Store {
    puzzles: Vec<PuzzleOut>,
    keys: HashSet<Vec<u8>>,
    counts: BTreeMap<(usize, Level), usize>,
    /// Next id number per (size, level); ids are never reused or renumbered.
    next: BTreeMap<(usize, Level), usize>,
}

impl Store {
    /// Loads the collection, or starts empty if the file doesn't exist.
    pub fn load(path: &Path) -> Result<Store, Box<dyn std::error::Error>> {
        let mut store = Store {
            puzzles: Vec::new(),
            keys: HashSet::new(),
            counts: BTreeMap::new(),
            next: BTreeMap::new(),
        };
        if path.exists() {
            let out: Output = serde_json::from_str(&std::fs::read_to_string(path)?)?;
            for p in out.puzzles {
                store.insert_existing(p);
            }
        }
        Ok(store)
    }

    fn insert_existing(&mut self, p: PuzzleOut) {
        if !self.keys.insert(canonical(&p.regions)) {
            return;
        }
        let bucket = (p.size, p.level);
        *self.counts.entry(bucket).or_default() += 1;
        let number = id_number(&p.id).unwrap_or(0);
        let next = self.next.entry(bucket).or_insert(1);
        *next = (*next).max(number + 1);
        self.puzzles.push(p);
    }

    pub fn count(&self, size: usize, level: Level) -> usize {
        self.counts.get(&(size, level)).copied().unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.puzzles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.puzzles.is_empty()
    }

    /// Adds a new puzzle unless it duplicates one already stored.
    pub fn add(&mut self, g: &Generated) -> bool {
        let regions = g.puzzle.grid();
        if !self.keys.insert(canonical(&regions)) {
            return false;
        }
        let (size, level) = (g.puzzle.size(), g.level());
        let next = self.next.entry((size, level)).or_insert(1);
        let id = format!("{}-{size}-{:03}", level.name(), *next);
        *next += 1;
        *self.counts.entry((size, level)).or_default() += 1;
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
    /// size/level, so the published set only ever grows.
    pub fn publish(&mut self, path: &Path, cap: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.puzzles
            .sort_by_key(|p| (p.size, p.level, id_number(&p.id).unwrap_or(0)));
        let mut taken: BTreeMap<(usize, Level), usize> = BTreeMap::new();
        let puzzles: Vec<PuzzleOut> = self
            .puzzles
            .iter()
            .filter(|p| {
                let k = taken.entry((p.size, p.level)).or_default();
                *k += 1;
                *k <= cap
            })
            .cloned()
            .collect();
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
    fn canonical_ignores_symmetry_and_labels() {
        let a = vec![vec![0, 0, 1], vec![2, 1, 1], vec![2, 2, 1]];
        // Rotated 90 degrees clockwise, with regions renumbered.
        let b = vec![vec![1, 1, 0], vec![1, 2, 0], vec![2, 2, 2]];
        assert_eq!(canonical(&a), canonical(&b));
        let c = vec![vec![0, 1, 1], vec![2, 1, 1], vec![2, 2, 1]];
        assert_ne!(canonical(&a), canonical(&c));
    }
}
