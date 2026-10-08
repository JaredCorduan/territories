//! The puzzle file format shared by the generators, the farm, and the site.
//! `S` is a game's solution: `solution[row]` is the animal's column in the
//! one-animal game, and both animals' columns in the two-animal game.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use territories_core::{Level, Puzzle};

#[derive(Serialize, Deserialize)]
pub struct Output<S> {
    pub version: u32,
    pub puzzles: Vec<PuzzleOut<S>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PuzzleOut<S> {
    pub id: String,
    pub level: Level,
    pub size: usize,
    /// `regions[row][col]` = region index.
    pub regions: Vec<Vec<usize>>,
    pub solution: S,
    /// Steps the grader took, by rule.
    pub moves: BTreeMap<String, usize>,
}

/// A generated puzzle with its grade, as either game produces it.
pub trait Graded {
    type Solution: Clone;

    fn puzzle(&self) -> &Puzzle;
    fn level(&self) -> Level;
    fn solution(&self) -> &Self::Solution;
    /// Steps the grader took, by rule id.
    fn moves(&self) -> BTreeMap<String, usize>;
}

impl<S: Clone> PuzzleOut<S> {
    pub fn new(g: &impl Graded<Solution = S>, id: String) -> PuzzleOut<S> {
        PuzzleOut {
            id,
            level: g.level(),
            size: g.puzzle().size(),
            regions: g.puzzle().grid(),
            solution: g.solution().clone(),
            moves: g.moves(),
        }
    }
}
