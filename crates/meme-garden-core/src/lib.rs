//! Meme Garden core simulation engine.
//!
//! Public surface:
//! - [`Simulation`] — the canonical tick loop owner.
//! - [`SimConfig`] — TOML-deserialized parameters.
//! - [`Metrics`] — per-tick snapshot.
//! - [`ai`] — provider seams (meme namer, run analyst).
//!
//! All randomness flows through [`rng::SimRng`]. No `std::time` or `thread_rng`
//! in this crate — determinism for a given seed is a hard invariant.

pub mod ai;
pub mod agent;
pub mod config;
pub mod meme;
pub mod metrics;
pub mod rng;
pub mod world;

pub use config::{ConfigError, SimConfig};
pub use metrics::Metrics;
pub use world::Simulation;
