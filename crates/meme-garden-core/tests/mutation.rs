//! Mutation invariants: mutated memes stay valid symbolic structures.

use meme_garden_core::config::*;
use meme_garden_core::*;

fn cfg() -> SimConfig {
    SimConfig {
        world: WorldConfig {
            width: 30,
            height: 20,
        },
        agents: AgentConfig {
            count: 60,
            starting_energy: 30.0,
            metabolism: 0.3,
            max_energy: 60.0,
            max_age: 1000,
            initial_traits_dist: [0.4, 0.2, 0.2, 0.2],
            trait_mutation_rate: 0.0,
        },
        food: FoodConfig {
            initial_density: 0.30,
            regrowth_rate: 0.010,
            energy_per_food: 10.0,
        },
        scarcity: ScarcityConfig {
            level: "custom".into(),
        },
        cognition: CognitionConfig { inventory_cap: 4 },
        transmission: TransmissionConfig {
            base_rate: 0.9,
            social_copying_bias_mean: 0.9,
            social_copying_bias_std: 0.0,
            prestige_boost: 0.0,
        },
        mutation: MutationConfig {
            strength_jitter_max: 0.20,
            enum_swap_probability: 0.5,
        },
        conflict: ConflictConfig { recombine_share: 0.20 },
        reproduction: ReproductionConfig {
            energy_threshold: 100.0, // effectively disable repro
            offspring_energy_cost: 1.0,
            inherit_meme_prob: 0.0,
            min_age: 1000,
        },
        attack: AttackConfig {
            energy_cost_attacker: 1.0,
            energy_steal: 1.0,
            retaliation_chance: 0.0,
        },
        sharing: SharingConfig {
            share_threshold: 10.0,
            share_amount: 1.0,
        },
        memes: MemePoolConfig {
            seed: vec![SeedMemeEntry {
                name: "share_with_allies".into(),
                carrier_fraction: 1.0,
            }],
        },
        run: RunConfig {
            seed: 5,
            horizon: 500,
            stop_on_extinction: false,
            cluster_snapshot_every: 0,
            metrics_emit_every: 1,
            survival_threshold: 0.05,
        },
    }
}

#[test]
fn mutated_memes_stay_in_enum_ranges() {
    let mut sim = Simulation::new(cfg(), Some(123));
    // Force the per-meme mutation rate to 1.0 by mutating starter inventories.
    for a in sim.agents.iter_mut() {
        for m in a.inventory.iter_mut() {
            m.mutation_rate = 1.0;
        }
    }
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
        for a in &sim.agents {
            if !a.alive {
                continue;
            }
            for m in &a.inventory {
                assert!(
                    (0.0..=1.0).contains(&m.strength),
                    "strength out of range: {}",
                    m.strength
                );
                assert!(
                    (0.0..=1.0).contains(&m.transmissibility),
                    "trans out of range"
                );
                assert!(
                    (0.0..=1.0).contains(&m.mutation_rate),
                    "mut_rate out of range"
                );
                assert!(m.cognitive_cost >= 0.0, "cog cost negative");
                // Trigger/Target/Effect are checked by the type system (closed enums).
                let _ = m.trigger;
                let _ = m.target;
                let _ = m.effect;
            }
        }
    }
}

#[test]
fn zero_mutation_rate_means_no_mutation_events() {
    let mut cfg = cfg();
    cfg.mutation.strength_jitter_max = 0.0;
    cfg.mutation.enum_swap_probability = 0.0;
    let mut sim = Simulation::new(cfg, Some(0));
    // Set per-meme mutation rate to 0 across the board.
    for a in sim.agents.iter_mut() {
        for m in a.inventory.iter_mut() {
            m.mutation_rate = 0.0;
        }
    }
    let mut mutations_observed = 0u32;
    for _ in 0..300 {
        sim.step();
        for e in sim.events_drain() {
            if let Event::Mutation { .. } = e {
                mutations_observed += 1;
            }
        }
    }
    assert_eq!(
        mutations_observed, 0,
        "saw {} mutations with rate=0",
        mutations_observed
    );
}
