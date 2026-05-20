//! Meme Garden core simulation engine.
//!
//! Public surface:
//! - [`Simulation`] — the canonical tick loop owner.
//! - [`SimConfig`] — TOML-deserialized parameters.
//! - [`Metrics`] — per-tick aggregate snapshot.
//! - [`metrics::Event`] — per-event records emitted between tick aggregates.
//! - [`ai`] — provider seams (meme namer, experiment designer, run analyst).
//!
//! All randomness flows through [`rng::SimRng`]. No `std::time` or `thread_rng`
//! in this crate — determinism for a given seed is a hard invariant
//! (constitution principle I).

pub mod action;
pub mod agent;
pub mod ai;
pub mod config;
pub mod lineage;
pub mod meme;
pub mod metrics;
pub mod mutation;
pub mod policy;
pub mod rng;
pub mod starters;
pub mod world;

pub use action::{Action, Direction};
pub use agent::{Agent, AgentId, AgentTrait, Position};
pub use config::{ConfigError, SimConfig};
pub use lineage::{LineageGraph, LineageId, LineageNode, LineageOrigin};
pub use meme::{Effect, Meme, MemeId, MemeKind, TargetSelector, Trigger};
pub use metrics::{
    shannon_diversity, top1_fraction, DeathCause, Event, ExtinctionScope, Metrics, MutatedField,
};
pub use world::Simulation;

pub const CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
