use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub world: WorldConfig,
    pub agents: AgentConfig,
    pub food: FoodConfig,
    pub meme: MemeConfig,
    pub run: RunConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldConfig {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub count: u32,
    pub starting_energy: f32,
    pub metabolism: f32,
    pub max_energy: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodConfig {
    /// Fraction of cells initialized with food, 0.0..=1.0
    pub initial_density: f32,
    /// Probability per tick that a random empty cell sprouts food.
    pub regrowth_rate: f32,
    /// Energy gained when an agent eats a food cell.
    pub energy_per_food: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemeConfig {
    /// Fraction of initial agents seeded with the POC meme.
    pub initial_carrier_fraction: f32,
    /// Probability of meme transmission on a successful share.
    pub transmissibility: f32,
    /// Energy threshold: agent shares only if its own energy is above this.
    pub share_threshold: f32,
    /// Energy transferred from sharer to recipient.
    pub share_amount: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub seed: u64,
}

impl SimConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        Ok(toml::from_str(s)?)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_toml_str(&text)
    }
}
