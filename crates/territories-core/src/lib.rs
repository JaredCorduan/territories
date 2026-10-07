//! Core logic for the Territories puzzle: board model, human-style deduction
//! rules, a rule-based solver/grader, and a brute-force solution counter.
//!
//! Kept free of randomness and I/O so it can compile to WASM for hints.

pub mod board;
pub mod bruteforce;
pub mod hint;
pub mod rules;
pub mod solver;

pub use board::{Cell, Mark, Puzzle, PuzzleError, State, Unit, UnitKind};
pub use hint::Hint;
pub use rules::{Deduction, Level, Rule};
pub use solver::{Status, Trace};
