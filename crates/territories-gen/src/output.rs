//! The puzzle file format shared by the generator, the farm, and the site.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use territories_core::{Level, Rule};

use crate::generate::Generated;

#[derive(Serialize, Deserialize)]
pub struct Output {
    pub version: u32,
    pub puzzles: Vec<PuzzleOut>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PuzzleOut {
    pub id: String,
    pub level: Level,
    pub size: usize,
    /// `regions[row][col]` = region index.
    pub regions: Vec<Vec<usize>>,
    /// `solution[row]` = animal's column.
    pub solution: Vec<usize>,
    /// Steps the grader took, by rule.
    pub moves: BTreeMap<String, usize>,
}

impl PuzzleOut {
    pub fn new(g: &Generated, id: String) -> PuzzleOut {
        let mut moves = BTreeMap::new();
        for rule in Rule::ALL {
            let used = g.trace.steps.iter().filter(|d| d.rule == rule).count();
            if used > 0 {
                moves.insert(rule.id().to_string(), used);
            }
        }
        PuzzleOut {
            id,
            level: g.level(),
            size: g.puzzle.size(),
            regions: g.puzzle.grid(),
            solution: g.solution.clone(),
            moves,
        }
    }
}
