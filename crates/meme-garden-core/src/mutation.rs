//! Bounded mutation + recombination of memes.
//!
//! Mutation operates only on `trigger`, `target`, `effect`, and `strength`. The
//! per-field enum swap probability and the scalar strength jitter are bounded by
//! the `MutationConfig`. `transmissibility`, `mutation_rate`, and `cognitive_cost`
//! are held fixed per the MVP design (see `research.md D-006`). A meme's `kind`
//! tracks its *behavior*: when a mutation swaps the `effect` field, or when two
//! parents recombine, the resulting kind is re-derived from the effect via
//! `MemeKind::from_effect`. This keeps the cooperative/aggressive split honest
//! and stops recombinants from accumulating in a privileged `Mutant` bucket.

use smallvec::smallvec;

use crate::config::MutationConfig;
use crate::lineage::{LineageGraph, LineageOrigin};
use crate::meme::{Effect, Meme, MemeId, MemeKind, TargetSelector, Trigger};
use crate::rng::SimRng;

/// Result of a mutation attempt. `field` is `None` if no field changed.
#[derive(Debug, Clone, Copy)]
pub struct MutationOutcome {
    pub mutated: bool,
    pub field: Option<crate::metrics::MutatedField>,
}

/// Apply mutation in place. Returns whether the meme actually changed and which
/// field was rolled (lineage updates are the caller's responsibility — the
/// caller knows the current tick).
pub fn mutate_in_place(meme: &mut Meme, rng: &mut SimRng, cfg: &MutationConfig) -> MutationOutcome {
    use crate::metrics::MutatedField;

    // Pick exactly one field deterministically; this keeps the search space
    // sane and makes the resulting Event::Mutation cleanly attributable.
    let pick = rng.gen_range_usize(0, 4);
    let mut changed = false;
    let mut field = None;

    match pick {
        0 => {
            if rng.gen_bool(cfg.enum_swap_probability) {
                let new = pick_other_variant(&Trigger::ALL, meme.trigger, rng);
                meme.trigger = new;
                changed = true;
                field = Some(MutatedField::Trigger);
            }
        }
        1 => {
            if rng.gen_bool(cfg.enum_swap_probability) {
                let new = pick_other_variant(&TargetSelector::ALL, meme.target, rng);
                meme.target = new;
                changed = true;
                field = Some(MutatedField::Target);
            }
        }
        2 => {
            if rng.gen_bool(cfg.enum_swap_probability) {
                let new = pick_other_variant(&Effect::ALL, meme.effect, rng);
                meme.effect = new;
                changed = true;
                field = Some(MutatedField::Effect);
            }
        }
        _ => {
            // Strength jitter is always attempted (no separate gate); the magnitude
            // is bounded by cfg.strength_jitter_max. We still record only meaningful
            // jitters as mutations.
            let max = cfg.strength_jitter_max;
            if max > 0.0 {
                // Uniform in [-max, +max].
                let u = (rng.gen_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0;
                let delta = u * max;
                let before = meme.strength;
                meme.strength = (meme.strength + delta).clamp(0.0, 1.0);
                if (meme.strength - before).abs() > f32::EPSILON {
                    changed = true;
                    field = Some(MutatedField::Strength);
                }
            }
        }
    }

    // Kind tracks behavior, not founding lineage: an effect swap can flip a
    // cooperative meme into an aggressive one (or vice versa), so we re-derive
    // kind from the new effect. Trigger/target/strength changes don't touch what
    // the meme *does*, so they leave kind alone.
    if field == Some(MutatedField::Effect) {
        meme.kind = MemeKind::from_effect(meme.effect);
    }
    MutationOutcome {
        mutated: changed,
        field,
    }
}

/// Recombine two parents into a child meme by picking each field independently
/// from one of the two parents.
pub fn recombine(
    a: &Meme,
    b: &Meme,
    new_meme_id: MemeId,
    rng: &mut SimRng,
    lineage: &mut LineageGraph,
    tick: u64,
) -> Meme {
    fn pick<T: Copy>(rng: &mut SimRng, x: T, y: T) -> T {
        if rng.gen_bool(0.5) {
            x
        } else {
            y
        }
    }

    let trigger = pick(rng, a.trigger, b.trigger);
    let target = pick(rng, a.target, b.target);
    let effect = pick(rng, a.effect, b.effect);
    // Kind follows behavior: a recombinant that ends up Sharing is cooperative,
    // one that ends up Attacking is aggressive. This re-subjects hybrids to
    // conflict resolution instead of letting them hide in a conflict-exempt
    // Mutant bucket. (Computed after `effect` so the RNG sequence is unchanged.)
    let kind = MemeKind::from_effect(effect);
    let strength = if rng.gen_bool(0.5) {
        a.strength
    } else {
        b.strength
    };
    let transmissibility = if rng.gen_bool(0.5) {
        a.transmissibility
    } else {
        b.transmissibility
    };
    let mutation_rate = if rng.gen_bool(0.5) {
        a.mutation_rate
    } else {
        b.mutation_rate
    };
    let cognitive_cost = if rng.gen_bool(0.5) {
        a.cognitive_cost
    } else {
        b.cognitive_cost
    };

    let lineage_id = lineage.add(
        smallvec![a.lineage_id, b.lineage_id],
        tick,
        LineageOrigin::Recombination,
    );

    Meme {
        id: new_meme_id,
        lineage_id,
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

fn pick_other_variant<T: Copy + PartialEq>(all: &[T], current: T, rng: &mut SimRng) -> T {
    if all.len() <= 1 {
        return current;
    }
    loop {
        let idx = rng.gen_range_usize(0, all.len());
        if all[idx] != current {
            return all[idx];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starters;

    fn cfg(max: f32, swap: f32) -> MutationConfig {
        MutationConfig {
            strength_jitter_max: max,
            enum_swap_probability: swap,
        }
    }

    #[test]
    fn mutation_keeps_strength_in_range() {
        let mut rng = SimRng::from_seed(1);
        let mut m = starters::share_with_allies();
        for _ in 0..1000 {
            mutate_in_place(&mut m, &mut rng, &cfg(0.5, 1.0));
            assert!((0.0..=1.0).contains(&m.strength), "strength out of range");
        }
    }

    #[test]
    fn mutation_does_not_change_static_fields() {
        let mut rng = SimRng::from_seed(2);
        let mut m = starters::avoid_strangers();
        let trans = m.transmissibility;
        let rate = m.mutation_rate;
        let cost = m.cognitive_cost;
        for _ in 0..200 {
            mutate_in_place(&mut m, &mut rng, &cfg(0.2, 1.0));
        }
        assert_eq!(m.transmissibility, trans);
        assert_eq!(m.mutation_rate, rate);
        assert_eq!(m.cognitive_cost, cost);
    }
}
