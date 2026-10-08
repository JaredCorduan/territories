//! Core logic for the Territories puzzle: the board model shared by both
//! games, and for each game its human-style deduction rules, a rule-based
//! solver/grader, hints, and a brute-force solution counter.
//!
//! Kept free of randomness and I/O so it can compile to WASM for hints.

pub mod board;
pub mod level;
pub mod one_star;
pub mod two_star;

pub use board::{Cell, Mark, Puzzle, PuzzleError, State, Unit, UnitKind};
pub use level::Level;
