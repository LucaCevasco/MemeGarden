use serde::{Deserialize, Serialize};

use crate::agent::AgentId;
use crate::meme::MemeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    North,
    South,
    East,
    West,
}

impl Direction {
    pub const ALL: [Direction; 4] = [
        Direction::North,
        Direction::South,
        Direction::East,
        Direction::West,
    ];

    pub fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, -1),
            Direction::South => (0, 1),
            Direction::East => (1, 0),
            Direction::West => (-1, 0),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Move(Direction),
    Eat,
    Share(AgentId),
    Attack(AgentId),
    Imitate(AgentId),
    Transmit(AgentId, MemeId),
    Reproduce(AgentId),
    #[default]
    Idle,
}
