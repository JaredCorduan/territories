//! Difficulty levels, shared by both games. A puzzle's level is the level
//! of the hardest rule its game's solver needed.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Easy,
    Medium,
    Hard,
    Brutal,
}

impl Level {
    pub const ALL: [Level; 4] = [Level::Easy, Level::Medium, Level::Hard, Level::Brutal];

    pub fn name(self) -> &'static str {
        match self {
            Level::Easy => "easy",
            Level::Medium => "medium",
            Level::Hard => "hard",
            Level::Brutal => "brutal",
        }
    }
}
