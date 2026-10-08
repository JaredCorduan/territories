//! The two-animal game: two animals per row, column, and territory.

pub mod bruteforce;
pub mod hint;
pub mod rules;
pub mod solver;

pub use hint::Hint;
pub use rules::{Deduction, MAX_SIZE, Rule};
pub use solver::Trace;

/// Animals per row, column, and territory.
pub const ANIMALS: usize = 2;

/// A solved board: the two animals' columns for each row, left one first.
pub type Solution = Vec<[usize; 2]>;
