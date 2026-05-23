use serde::{Deserialize, Serialize};

use crate::lineage::LineageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MemeId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemeKind {
    Cooperative,
    Defensive,
    Imitative,
    Aggressive,
    Punitive,
    Conformist,
    Mutant,
}

impl MemeKind {
    pub const ALL: [MemeKind; 7] = [
        MemeKind::Cooperative,
        MemeKind::Defensive,
        MemeKind::Imitative,
        MemeKind::Aggressive,
        MemeKind::Punitive,
        MemeKind::Conformist,
        MemeKind::Mutant,
    ];

    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn label(self) -> &'static str {
        match self {
            MemeKind::Cooperative => "cooperative",
            MemeKind::Defensive => "defensive",
            MemeKind::Imitative => "imitative",
            MemeKind::Aggressive => "aggressive",
            MemeKind::Punitive => "punitive",
            MemeKind::Conformist => "conformist",
            MemeKind::Mutant => "mutant",
        }
    }
}

/// Two meme kinds conflict when an agent can't sensibly carry both at once.
/// Conflict drives the reject / replace / recombine outcome in `world::Simulation::try_acquire`.
///
/// MVP starts with one pair — Cooperative ↔ Aggressive — because that's the
/// milestone question. Future kinds (Defensive ↔ Imitative, ...) extend the
/// table below; the relation is symmetric so we cover both orderings.
pub fn conflicts(a: MemeKind, b: MemeKind) -> bool {
    use MemeKind::*;
    matches!(
        (a, b),
        (Cooperative, Aggressive) | (Aggressive, Cooperative)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Hungry,
    NearFood,
    NearAlly,
    NearStranger,
    AttackedRecently,
    SawAgentGainEnergy,
}

impl Trigger {
    pub const ALL: [Trigger; 6] = [
        Trigger::Hungry,
        Trigger::NearFood,
        Trigger::NearAlly,
        Trigger::NearStranger,
        Trigger::AttackedRecently,
        Trigger::SawAgentGainEnergy,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetSelector {
    Self_,
    Kin,
    Ally,
    Stranger,
    HighEnergyAgent,
    LowEnergyAgent,
}

impl TargetSelector {
    pub const ALL: [TargetSelector; 6] = [
        TargetSelector::Self_,
        TargetSelector::Kin,
        TargetSelector::Ally,
        TargetSelector::Stranger,
        TargetSelector::HighEnergyAgent,
        TargetSelector::LowEnergyAgent,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    MoveToward,
    MoveAway,
    Share,
    Attack,
    Imitate,
    RefuseInteraction,
    TransmitMeme,
    IncreaseTrust,
    DecreaseTrust,
}

impl Effect {
    pub const ALL: [Effect; 9] = [
        Effect::MoveToward,
        Effect::MoveAway,
        Effect::Share,
        Effect::Attack,
        Effect::Imitate,
        Effect::RefuseInteraction,
        Effect::TransmitMeme,
        Effect::IncreaseTrust,
        Effect::DecreaseTrust,
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meme {
    pub id: MemeId,
    pub lineage_id: LineageId,
    pub kind: MemeKind,
    pub trigger: Trigger,
    pub target: TargetSelector,
    pub effect: Effect,
    pub strength: f32,
    pub transmissibility: f32,
    pub mutation_rate: f32,
    pub cognitive_cost: f32,
}
