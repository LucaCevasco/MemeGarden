//! Provider seams for AI integrations.
//!
//! These traits exist so call sites are clear and so the symbolic-meme core
//! can stay free of HTTP / LLM deps. No SDK dependency lands in `core`; live
//! providers ship in a downstream crate (e.g. a future `meme-garden-ai`).

use crate::config::SimConfig;
use crate::lineage::LineageGraph;
use crate::meme::Meme;
use crate::metrics::Metrics;

/// Maps natural-language experiment requests to validated configs. Called by
/// `cli experiment design` only — never from the per-tick simulation loop.
pub trait ExperimentDesigner: Send + Sync {
    fn design(&self, prompt: &str) -> Result<SimConfig, AiError>;
}

/// Produces human-readable names for symbolic memes. Called by the CLI when
/// rendering or exporting. Never called from `Simulation::step`.
pub trait MemeNamer: Send + Sync {
    fn name(&self, meme: &Meme) -> String;
}

/// Summarizes a completed run in prose. Called by `cli analyze` post-run.
pub trait RunAnalyst: Send + Sync {
    fn summarize(&self, history: &[Metrics], lineage: &LineageGraph) -> String;
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("ai provider not configured")]
    NotConfigured,
    #[error("ai provider returned invalid config: {0}")]
    InvalidConfig(String),
    #[error("ai provider failed: {0}")]
    Provider(String),
}

/// Deterministic no-op default. Used in tests and as the MVP-shipped impl.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProvider;

impl MemeNamer for NoopProvider {
    fn name(&self, meme: &Meme) -> String {
        format!(
            "meme:{}:{:?}/{:?}",
            meme.kind.label(),
            meme.trigger,
            meme.effect
        )
    }
}

impl ExperimentDesigner for NoopProvider {
    fn design(&self, _prompt: &str) -> Result<SimConfig, AiError> {
        Err(AiError::NotConfigured)
    }
}

impl RunAnalyst for NoopProvider {
    fn summarize(&self, history: &[Metrics], _lineage: &LineageGraph) -> String {
        let Some(last) = history.last() else {
            return "no ticks recorded".to_string();
        };
        format!(
            "ran {} ticks; final alive={}, diversity={:.2}, top1_share={:.2}",
            last.tick + 1,
            last.alive,
            last.diversity_shannon,
            last.dominance_top1_fraction,
        )
    }
}
