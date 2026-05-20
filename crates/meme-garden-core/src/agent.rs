use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

use crate::meme::{Meme, MemeKind};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct AgentId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTrait {
    Generous,
    Cautious,
    Aggressive,
    Conformist,
}

impl AgentTrait {
    pub const ALL: [AgentTrait; 4] = [
        AgentTrait::Generous,
        AgentTrait::Cautious,
        AgentTrait::Aggressive,
        AgentTrait::Conformist,
    ];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            AgentTrait::Generous => "generous",
            AgentTrait::Cautious => "cautious",
            AgentTrait::Aggressive => "aggressive",
            AgentTrait::Conformist => "conformist",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMemory {
    pub last_attacker: Option<AgentId>,
    pub last_attacked_tick: Option<u64>,
    pub saw_agent_gain_energy: Option<AgentId>,
}

pub type TrustMap = SmallVec<[(AgentId, f32); 8]>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub position: Position,
    pub energy: f32,
    pub age: u32,
    pub alive: bool,
    pub social_copying_bias: f32,
    pub traits: SmallVec<[AgentTrait; 4]>,
    pub memory: AgentMemory,
    pub trust: TrustMap,
    pub inventory: Vec<Meme>,
}

impl Agent {
    pub fn new(id: AgentId, position: Position, energy: f32) -> Self {
        Self {
            id,
            position,
            energy,
            age: 0,
            alive: true,
            social_copying_bias: 0.5,
            traits: SmallVec::new(),
            memory: AgentMemory::default(),
            trust: TrustMap::new(),
            inventory: Vec::new(),
        }
    }

    pub fn has_kind(&self, kind: MemeKind) -> bool {
        self.inventory.iter().any(|m| m.kind == kind)
    }

    pub fn has_trait(&self, t: AgentTrait) -> bool {
        self.traits.contains(&t)
    }

    pub fn trust_of(&self, other: AgentId) -> f32 {
        self.trust
            .iter()
            .find(|(id, _)| *id == other)
            .map(|(_, v)| *v)
            .unwrap_or(0.0)
    }

    /// Adjust trust toward `other` by `delta`, clamping to [-1.0, 1.0]. Adds an entry
    /// if missing; entries are kept in insertion order (deterministic).
    pub fn adjust_trust(&mut self, other: AgentId, delta: f32) {
        for entry in self.trust.iter_mut() {
            if entry.0 == other {
                entry.1 = (entry.1 + delta).clamp(-1.0, 1.0);
                return;
            }
        }
        // SmallVec inline cap is 8; if we exceed it we still push (heap-spills),
        // but bounded growth is fine in practice — TrustMap grows only on novel
        // neighbors, and decays each tick.
        self.trust.push((other, delta.clamp(-1.0, 1.0)));
    }
}
