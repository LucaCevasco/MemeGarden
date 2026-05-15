//! Provider seams for AI integrations.
//!
//! These traits exist now so the call sites are obvious; no SDK / HTTP deps land
//! in `core` until a real provider is wired. A future iteration will add
//! `AnthropicProvider`, `OpenAIProvider`, etc. in a separate crate.

use crate::meme::Meme;
use crate::metrics::Metrics;

/// Produces human-readable names for memes (e.g. "Outsider Caution").
pub trait MemeNamer: Send + Sync {
    fn name(&self, meme: &Meme) -> String;
}

/// Summarizes a completed run in prose.
pub trait RunAnalyst: Send + Sync {
    fn summarize(&self, history: &[Metrics]) -> String;
}

/// Default impls — no network, deterministic, used in tests + the POC.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProvider;

impl MemeNamer for NoopProvider {
    fn name(&self, meme: &Meme) -> String {
        format!("meme:{:?}", meme.kind)
    }
}

impl RunAnalyst for NoopProvider {
    fn summarize(&self, history: &[Metrics]) -> String {
        let Some(last) = history.last() else {
            return "no ticks recorded".to_string();
        };
        format!(
            "ran {} ticks; final alive={}, meme_prevalence={:.2}",
            last.tick + 1,
            last.alive,
            last.meme_prevalence
        )
    }
}
