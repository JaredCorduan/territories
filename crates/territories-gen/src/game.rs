//! What the farm needs to know about each game.

use std::ops::RangeInclusive;

use rand::rngs::StdRng;
use serde::Serialize;
use serde::de::DeserializeOwned;

use territories_core::Level;
use territories_core::two_star::solver;

use crate::output::Graded;
use crate::store::Mix;
use crate::{one_star, two_star};

pub trait Game {
    type Found: Graded<Solution: Clone + Serialize + DeserializeOwned> + Send + 'static;

    /// Animals per row, column, and territory.
    const ANIMALS: usize;
    /// Starts every puzzle id, so ids never clash between games.
    const ID_PREFIX: &'static str;
    /// Sizes farmed by default, and the sizes the game supports.
    const SIZES: &'static [usize];
    const SIZE_LIMITS: RangeInclusive<usize>;
    /// The permanent collection, and the site's capped copy of it.
    const STORE: &'static str;
    const PUBLISH: &'static str;
    /// The two kinds of brutal puzzle to mix, if the game has them.
    const MIX: Option<Mix>;

    /// One attempt at a puzzle solvable with rules up to `target`. The
    /// result's actual level may be easier.
    fn generate(n: usize, target: Level, rng: &mut StdRng) -> Option<Self::Found>;

    /// One attempt aimed at a brutal puzzle.
    fn generate_brutal(n: usize, rng: &mut StdRng) -> Option<Self::Found>;
}

pub struct OneStar;

impl Game for OneStar {
    type Found = one_star::generate::Generated;

    const ANIMALS: usize = 1;
    const ID_PREFIX: &'static str = "";
    const SIZES: &'static [usize] = &[7, 8, 9, 10];
    const SIZE_LIMITS: RangeInclusive<usize> = 5..=16;
    const STORE: &'static str = "data/puzzles.json";
    const PUBLISH: &'static str = "web/puzzles.json";
    const MIX: Option<Mix> = None;

    fn generate(n: usize, target: Level, rng: &mut StdRng) -> Option<Self::Found> {
        one_star::generate::generate(n, target, rng)
    }

    fn generate_brutal(n: usize, rng: &mut StdRng) -> Option<Self::Found> {
        one_star::generate::generate_brutal(n, rng)
    }
}

pub struct TwoStar;

impl Game for TwoStar {
    type Found = two_star::generate::Generated;

    const ANIMALS: usize = 2;
    const ID_PREFIX: &'static str = "2star-";
    const SIZES: &'static [usize] = &[9, 10, 11];
    const SIZE_LIMITS: RangeInclusive<usize> = 9..=territories_core::two_star::MAX_SIZE;
    const STORE: &'static str = "data/puzzles-2.json";
    const PUBLISH: &'static str = "web/puzzles-2.json";
    /// Most brutal puzzles need only the wide leftover. Two in five on the
    /// site need one of the rare rules, shuffled in fifty at a time.
    const MIX: Option<Mix> = Some(Mix {
        rare: |moves| solver::RARE.iter().any(|rule| moves.contains_key(rule.id())),
        rare_per_block: 20,
        block: 50,
    });

    fn generate(n: usize, target: Level, rng: &mut StdRng) -> Option<Self::Found> {
        two_star::generate::generate(n, target, rng)
    }

    fn generate_brutal(n: usize, rng: &mut StdRng) -> Option<Self::Found> {
        two_star::generate::generate(n, Level::Brutal, rng)
    }
}
