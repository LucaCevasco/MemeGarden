use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::meme::MemeKind;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize config: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("config value out of range: {field} = {value} (expected {expected})")]
    OutOfRange {
        field: &'static str,
        value: String,
        expected: &'static str,
    },
    #[error("unknown starter meme name: '{0}' (see configs/presets for valid names)")]
    UnknownStarterMeme(String),
    #[error("invalid scarcity level: '{0}' (expected one of low, mid, high, custom)")]
    InvalidScarcityLevel(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimConfig {
    pub world: WorldConfig,
    pub agents: AgentConfig,
    pub food: FoodConfig,
    pub scarcity: ScarcityConfig,
    pub cognition: CognitionConfig,
    pub transmission: TransmissionConfig,
    pub mutation: MutationConfig,
    pub reproduction: ReproductionConfig,
    pub attack: AttackConfig,
    pub sharing: SharingConfig,
    pub memes: MemePoolConfig,
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
    pub max_age: u32,
    /// Probabilities summing to 1.0 in `AgentTrait::ALL` order:
    /// [Generous, Cautious, Aggressive, Conformist]
    pub initial_traits_dist: [f32; 4],
    pub trait_mutation_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoodConfig {
    pub initial_density: f32,
    pub regrowth_rate: f32,
    pub energy_per_food: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScarcityConfig {
    /// `"low" | "mid" | "high" | "custom"`. `"custom"` leaves `food.*` untouched.
    #[serde(default = "default_scarcity_level")]
    pub level: String,
}

fn default_scarcity_level() -> String {
    "custom".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitionConfig {
    pub inventory_cap: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransmissionConfig {
    pub base_rate: f32,
    pub social_copying_bias_mean: f32,
    pub social_copying_bias_std: f32,
    pub prestige_boost: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationConfig {
    pub strength_jitter_max: f32,
    pub enum_swap_probability: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproductionConfig {
    pub energy_threshold: f32,
    pub offspring_energy_cost: f32,
    pub inherit_meme_prob: f32,
    pub min_age: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackConfig {
    pub energy_cost_attacker: f32,
    pub energy_steal: f32,
    pub retaliation_chance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingConfig {
    pub share_threshold: f32,
    pub share_amount: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemePoolConfig {
    pub seed: Vec<SeedMemeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedMemeEntry {
    pub name: String,
    pub carrier_fraction: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    pub seed: u64,
    pub horizon: u32,
    #[serde(default)]
    pub stop_on_extinction: bool,
    #[serde(default = "default_cluster_snapshot_every")]
    pub cluster_snapshot_every: u32,
    #[serde(default = "default_metrics_emit_every")]
    pub metrics_emit_every: u32,
    #[serde(default = "default_survival_threshold")]
    pub survival_threshold: f32,
}

fn default_cluster_snapshot_every() -> u32 {
    50
}
fn default_metrics_emit_every() -> u32 {
    1
}
fn default_survival_threshold() -> f32 {
    0.05
}

impl SimConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, ConfigError> {
        // Try the strict schema first. On failure, fall back to the legacy POC shape
        // and adapt with documented defaults (logged at WARN). This adapter is
        // explicitly temporary — Phase 6 Polish removes it once every shipped config
        // is upgraded.
        match toml::from_str::<SimConfig>(s) {
            Ok(cfg) => Ok(cfg),
            Err(strict_err) => match toml::from_str::<LegacySimConfig>(s) {
                Ok(legacy) => {
                    tracing::warn!(
                        "loaded legacy POC config; adapting with MVP defaults. \
                         Update the config to the schema in \
                         specs/001-meme-garden-mvp/contracts/config.schema.md."
                    );
                    Ok(legacy.into())
                }
                Err(_legacy_err) => Err(strict_err.into()),
            },
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_toml_str(&text)
    }

    pub fn to_toml_string(&self) -> Result<String, ConfigError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Apply scarcity preset to `food.*` and return whether anything changed.
    /// "low" = 1.0x (no-op), "mid" = 0.5x, "high" = 0.2x, "custom" = no-op.
    /// Multipliers apply to both `initial_density` and `regrowth_rate`.
    pub fn apply_scarcity(&mut self) -> Result<(), ConfigError> {
        let mult: f32 = match self.scarcity.level.as_str() {
            "low" => 1.0,
            "mid" => 0.5,
            "high" => 0.2,
            "custom" => return Ok(()),
            other => return Err(ConfigError::InvalidScarcityLevel(other.to_string())),
        };
        self.food.initial_density *= mult;
        self.food.regrowth_rate *= mult;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        in_unit("food.initial_density", self.food.initial_density)?;
        in_unit("food.regrowth_rate", self.food.regrowth_rate)?;
        in_unit("transmission.base_rate", self.transmission.base_rate)?;
        in_unit(
            "transmission.social_copying_bias_mean",
            self.transmission.social_copying_bias_mean,
        )?;
        in_unit(
            "transmission.prestige_boost",
            self.transmission.prestige_boost,
        )?;
        in_unit(
            "mutation.enum_swap_probability",
            self.mutation.enum_swap_probability,
        )?;
        in_unit(
            "reproduction.inherit_meme_prob",
            self.reproduction.inherit_meme_prob,
        )?;
        in_unit("attack.retaliation_chance", self.attack.retaliation_chance)?;
        in_unit(
            "agents.trait_mutation_rate",
            self.agents.trait_mutation_rate,
        )?;
        in_unit("run.survival_threshold", self.run.survival_threshold)?;

        non_negative(
            "mutation.strength_jitter_max",
            self.mutation.strength_jitter_max,
        )?;
        non_negative(
            "transmission.social_copying_bias_std",
            self.transmission.social_copying_bias_std,
        )?;

        positive_u32("agents.count", self.agents.count)?;
        positive_u32("agents.max_age", self.agents.max_age)?;
        positive_u32("cognition.inventory_cap", self.cognition.inventory_cap)?;
        positive_u32("run.horizon", self.run.horizon)?;
        if self.run.metrics_emit_every == 0 {
            return Err(ConfigError::OutOfRange {
                field: "run.metrics_emit_every",
                value: "0".to_string(),
                expected: ">= 1",
            });
        }

        let sum: f32 = self.agents.initial_traits_dist.iter().sum();
        if (sum - 1.0).abs() > 1e-3 {
            return Err(ConfigError::OutOfRange {
                field: "agents.initial_traits_dist",
                value: format!("sum={}", sum),
                expected: "sum == 1.0 (±1e-3)",
            });
        }
        for (i, p) in self.agents.initial_traits_dist.iter().enumerate() {
            in_unit_arr("agents.initial_traits_dist", i, *p)?;
        }

        for entry in &self.memes.seed {
            in_unit_named("memes.seed[*].carrier_fraction", entry.carrier_fraction)?;
            if crate::starters::lookup(&entry.name).is_none() {
                return Err(ConfigError::UnknownStarterMeme(entry.name.clone()));
            }
        }

        if self.agents.starting_energy <= 0.0 {
            return Err(ConfigError::OutOfRange {
                field: "agents.starting_energy",
                value: format!("{}", self.agents.starting_energy),
                expected: "> 0.0",
            });
        }
        if self.agents.max_energy < self.agents.starting_energy {
            return Err(ConfigError::OutOfRange {
                field: "agents.max_energy",
                value: format!("{}", self.agents.max_energy),
                expected: ">= agents.starting_energy",
            });
        }

        // Scarcity level must parse — apply_scarcity is the canonical check.
        match self.scarcity.level.as_str() {
            "low" | "mid" | "high" | "custom" => {}
            other => return Err(ConfigError::InvalidScarcityLevel(other.to_string())),
        }
        Ok(())
    }

    pub fn carrier_fraction(&self, kind: MemeKind) -> f32 {
        self.memes
            .seed
            .iter()
            .filter_map(|e| {
                let ctor = crate::starters::lookup(&e.name)?;
                if ctor().kind == kind {
                    Some(e.carrier_fraction)
                } else {
                    None
                }
            })
            .sum()
    }
}

fn in_unit(field: &'static str, v: f32) -> Result<(), ConfigError> {
    if !(0.0..=1.0).contains(&v) || !v.is_finite() {
        return Err(ConfigError::OutOfRange {
            field,
            value: format!("{}", v),
            expected: "[0.0, 1.0] finite",
        });
    }
    Ok(())
}

fn in_unit_named(field: &'static str, v: f32) -> Result<(), ConfigError> {
    in_unit(field, v)
}

fn in_unit_arr(field: &'static str, _idx: usize, v: f32) -> Result<(), ConfigError> {
    in_unit(field, v)
}

fn non_negative(field: &'static str, v: f32) -> Result<(), ConfigError> {
    if v < 0.0 || !v.is_finite() {
        return Err(ConfigError::OutOfRange {
            field,
            value: format!("{}", v),
            expected: ">= 0.0 finite",
        });
    }
    Ok(())
}

fn positive_u32(field: &'static str, v: u32) -> Result<(), ConfigError> {
    if v == 0 {
        return Err(ConfigError::OutOfRange {
            field,
            value: format!("{}", v),
            expected: "> 0",
        });
    }
    Ok(())
}

// ----- Legacy adapter (Phase 2 transitional; removed in Phase 6 Polish) -----

#[derive(Debug, Clone, Deserialize)]
struct LegacySimConfig {
    world: WorldConfig,
    agents: LegacyAgentConfig,
    food: LegacyFoodConfig,
    meme: LegacyMemeConfig,
    run: LegacyRunConfig,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyAgentConfig {
    count: u32,
    starting_energy: f32,
    metabolism: f32,
    max_energy: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyFoodConfig {
    initial_density: f32,
    regrowth_rate: f32,
    energy_per_food: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyMemeConfig {
    initial_carrier_fraction: f32,
    transmissibility: f32,
    share_threshold: f32,
    share_amount: f32,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyRunConfig {
    seed: u64,
}

impl From<LegacySimConfig> for SimConfig {
    fn from(l: LegacySimConfig) -> Self {
        SimConfig {
            world: l.world,
            agents: AgentConfig {
                count: l.agents.count,
                starting_energy: l.agents.starting_energy,
                metabolism: l.agents.metabolism,
                max_energy: l.agents.max_energy,
                max_age: 800,
                initial_traits_dist: [0.35, 0.20, 0.20, 0.25],
                trait_mutation_rate: 0.01,
            },
            food: FoodConfig {
                initial_density: l.food.initial_density,
                regrowth_rate: l.food.regrowth_rate,
                energy_per_food: l.food.energy_per_food,
            },
            scarcity: ScarcityConfig {
                level: "custom".into(),
            },
            cognition: CognitionConfig { inventory_cap: 8 },
            transmission: TransmissionConfig {
                base_rate: l.meme.transmissibility,
                social_copying_bias_mean: 0.5,
                social_copying_bias_std: 0.15,
                prestige_boost: 0.10,
            },
            mutation: MutationConfig {
                strength_jitter_max: 0.10,
                enum_swap_probability: 0.20,
            },
            reproduction: ReproductionConfig {
                energy_threshold: 40.0,
                offspring_energy_cost: 15.0,
                inherit_meme_prob: 0.5,
                min_age: 50,
            },
            attack: AttackConfig {
                energy_cost_attacker: 3.0,
                energy_steal: 5.0,
                retaliation_chance: 0.5,
            },
            sharing: SharingConfig {
                share_threshold: l.meme.share_threshold,
                share_amount: l.meme.share_amount,
            },
            memes: MemePoolConfig {
                seed: vec![SeedMemeEntry {
                    name: "share_with_allies".into(),
                    carrier_fraction: l.meme.initial_carrier_fraction,
                }],
            },
            run: RunConfig {
                seed: l.run.seed,
                horizon: 1000,
                stop_on_extinction: false,
                cluster_snapshot_every: 50,
                metrics_emit_every: 1,
                survival_threshold: 0.05,
            },
        }
    }
}
