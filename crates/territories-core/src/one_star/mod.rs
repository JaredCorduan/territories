//! The one-animal game: one animal per row, column, and territory.

pub mod bruteforce;
pub mod hint;
pub mod rules;
pub mod solver;

pub use hint::Hint;
pub use rules::{Deduction, Rule};
pub use solver::{Status, Trace};
