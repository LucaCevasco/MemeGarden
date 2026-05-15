use serde::{Deserialize, Serialize};

use crate::meme::Meme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub u32);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub position: Position,
    pub energy: f32,
    pub age: u32,
    pub alive: bool,
    /// POC: zero or one meme. The full inventory (`Vec<Meme>` + cognitive cost)
    /// arrives with the symbolic grammar.
    pub meme: Option<Meme>,
}

impl Agent {
    pub fn new(id: AgentId, position: Position, energy: f32) -> Self {
        Self {
            id,
            position,
            energy,
            age: 0,
            alive: true,
            meme: None,
        }
    }
}
