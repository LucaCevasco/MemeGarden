use serde::{Deserialize, Serialize};

/// A meme is, in the POC, a small policy modifier. In later iterations this
/// becomes a `Trigger`/`Effect`/`Target`/`strength` quadruple plus mutation and
/// recombination metadata — see `docs/design.md`. For now we only ship the one
/// transmissible norm needed to prove the meme-transmission pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meme {
    pub id: MemeId,
    pub kind: MemeKind,
    /// Probability of transmitting to a recipient on a successful interaction.
    pub transmissibility: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemeId(pub u32);

/// POC has exactly one kind. New kinds slot in next to it; downstream code
/// should match exhaustively so we get a compile error when adding more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemeKind {
    /// "Share with an adjacent low-energy ally, transmit on success."
    SharerNorm,
}

impl Meme {
    /// The single POC meme. `share_threshold` / `share_amount` live in
    /// [`crate::config::MemeConfig`] so they remain tunable from TOML.
    pub fn sharer_norm(transmissibility: f32) -> Self {
        Self {
            id: MemeId(1),
            kind: MemeKind::SharerNorm,
            transmissibility,
        }
    }
}
