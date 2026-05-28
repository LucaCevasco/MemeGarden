//! Extinction events fire exactly once and the simulation continues to horizon.

use meme_garden_core::config::*;
use meme_garden_core::*;

fn high_metabolism_cfg() -> SimConfig {
    SimConfig {
        world: WorldConfig {
            width: 10,
            height: 10,
        },
        agents: AgentConfig {
            count: 10,
            starting_energy: 5.0,
            metabolism: 2.0, // dies in ~3 ticks without food
            max_energy: 10.0,
            max_age: 1000,
            initial_traits_dist: [0.4, 0.2, 0.2, 0.2],
            trait_mutation_rate: 0.0,
        },
        food: FoodConfig {
            initial_density: 0.0,
            regrowth_rate: 0.0,
            energy_per_food: 1.0,
        },
        scarcity: ScarcityConfig {
            level: "custom".into(),
        },
        cognition: CognitionConfig { inventory_cap: 4 },
        transmission: TransmissionConfig {
            base_rate: 0.0,
            social_copying_bias_mean: 0.0,
            social_copying_bias_std: 0.0,
            prestige_boost: 0.0,
        },
        mutation: MutationConfig {
            strength_jitter_max: 0.0,
            enum_swap_probability: 0.0,
        },
        conflict: ConflictConfig { recombine_share: 0.20 },
        reproduction: ReproductionConfig {
            energy_threshold: 100.0,
            offspring_energy_cost: 1.0,
            inherit_meme_prob: 0.0,
            min_age: 1000,
        },
        attack: AttackConfig {
            energy_cost_attacker: 0.0,
            energy_steal: 0.0,
            retaliation_chance: 0.0,
        },
        sharing: SharingConfig {
            share_threshold: 100.0,
            share_amount: 0.0,
            recipient_multiplier: 1.0,
        },
        memes: MemePoolConfig {
            seed: vec![SeedMemeEntry {
                name: "share_with_allies".into(),
                carrier_fraction: 1.0,
            }],
        },
        run: RunConfig {
            seed: 1,
            horizon: 200,
            stop_on_extinction: false,
            cluster_snapshot_every: 0,
            metrics_emit_every: 1,
            survival_threshold: 0.05,
        },
    }
}

#[test]
fn population_extinction_fires_once_and_run_continues() {
    let mut sim = Simulation::new(high_metabolism_cfg(), Some(0));
    let mut extinction_count = 0u32;
    let mut last_metric = None;
    for _ in 0..200 {
        let m = sim.step();
        for e in sim.events_drain() {
            if let Event::Extinction {
                scope: ExtinctionScope::Population,
                ..
            } = e
            {
                extinction_count += 1;
            }
        }
        last_metric = Some(m);
    }
    assert_eq!(
        extinction_count, 1,
        "expected exactly one Population extinction event, saw {extinction_count}"
    );
    let last = last_metric.unwrap();
    assert_eq!(last.alive, 0, "population should be 0 at horizon");
    assert_eq!(last.tick, 199, "should run to horizon despite extinction");
}
