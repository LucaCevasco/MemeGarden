//! Starter meme constructors. Each returns a prototype `Meme` with placeholder
//! `id` and `lineage_id`; the simulator overwrites those when seeding.

use crate::lineage::LineageId;
use crate::meme::{Effect, Meme, MemeId, MemeKind, TargetSelector, Trigger};

/// A prototype meme. `id` and `lineage_id` are placeholders — the simulator
/// substitutes real values during seeding.
// why: 8 args is the natural shape of the Meme struct; refactoring into a
// builder would obscure that each starter is just a fully-specified struct
// literal. Allow exceeds the default of 7.
#[allow(clippy::too_many_arguments)]
pub fn prototype(
    kind: MemeKind,
    trigger: Trigger,
    target: TargetSelector,
    effect: Effect,
    strength: f32,
    transmissibility: f32,
    mutation_rate: f32,
    cognitive_cost: f32,
) -> Meme {
    Meme {
        id: MemeId(0),
        lineage_id: LineageId(0),
        kind,
        trigger,
        target,
        effect,
        strength,
        transmissibility,
        mutation_rate,
        cognitive_cost,
    }
}

pub fn share_with_allies() -> Meme {
    prototype(
        MemeKind::Cooperative,
        Trigger::NearAlly,
        TargetSelector::LowEnergyAgent,
        Effect::Share,
        0.7,
        0.45,
        0.05,
        0.02,
    )
}

pub fn avoid_strangers() -> Meme {
    prototype(
        MemeKind::Defensive,
        Trigger::NearStranger,
        TargetSelector::Stranger,
        Effect::MoveAway,
        0.6,
        0.40,
        0.05,
        0.01,
    )
}

pub fn copy_high_energy() -> Meme {
    prototype(
        MemeKind::Imitative,
        Trigger::SawAgentGainEnergy,
        TargetSelector::HighEnergyAgent,
        Effect::Imitate,
        0.5,
        0.50,
        0.05,
        0.02,
    )
}

pub fn attack_low_energy_outsiders() -> Meme {
    prototype(
        MemeKind::Aggressive,
        Trigger::NearStranger,
        TargetSelector::LowEnergyAgent,
        Effect::Attack,
        0.6,
        0.45,
        0.05,
        0.03,
    )
}

pub fn punish_non_sharers() -> Meme {
    prototype(
        MemeKind::Punitive,
        Trigger::SawAgentGainEnergy,
        TargetSelector::Stranger,
        Effect::Attack,
        0.45,
        0.40,
        0.05,
        0.03,
    )
}

pub fn prefer_same_meme() -> Meme {
    prototype(
        MemeKind::Conformist,
        Trigger::NearAlly,
        TargetSelector::Ally,
        Effect::IncreaseTrust,
        0.4,
        0.40,
        0.05,
        0.01,
    )
}

/// Resolve a starter meme name to its constructor. Returns `None` for unknown names.
pub fn lookup(name: &str) -> Option<fn() -> Meme> {
    match name {
        "share_with_allies" => Some(share_with_allies),
        "avoid_strangers" => Some(avoid_strangers),
        "copy_high_energy" => Some(copy_high_energy),
        "attack_low_energy_outsiders" => Some(attack_low_energy_outsiders),
        "punish_non_sharers" => Some(punish_non_sharers),
        "prefer_same_meme" => Some(prefer_same_meme),
        _ => None,
    }
}

/// All six starter memes in a canonical (alphabetical) order.
pub const STARTERS: &[&str] = &[
    "attack_low_energy_outsiders",
    "avoid_strangers",
    "copy_high_energy",
    "prefer_same_meme",
    "punish_non_sharers",
    "share_with_allies",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_starters_resolve() {
        for name in STARTERS {
            assert!(lookup(name).is_some(), "{name} not registered");
        }
    }

    #[test]
    fn unknown_starter_returns_none() {
        assert!(lookup("not_a_real_meme").is_none());
    }

    #[test]
    fn all_starters_have_distinct_kinds() {
        let kinds: Vec<_> = STARTERS.iter().map(|n| lookup(n).unwrap()().kind).collect();
        let unique: std::collections::HashSet<_> = kinds.iter().copied().collect();
        assert_eq!(kinds.len(), unique.len());
    }

    #[test]
    fn starter_meme_fields_in_range() {
        for name in STARTERS {
            let m = lookup(name).unwrap()();
            assert!((0.0..=1.0).contains(&m.strength), "{} strength", name);
            assert!((0.0..=1.0).contains(&m.transmissibility), "{} trans", name);
            assert!((0.0..=1.0).contains(&m.mutation_rate), "{} mut", name);
            assert!(m.cognitive_cost >= 0.0, "{} cog cost", name);
        }
    }
}
