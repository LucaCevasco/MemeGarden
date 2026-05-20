//! Cognitive cost drains carriers' energy faster than non-carriers.

use meme_garden_core::config::*;
use meme_garden_core::*;

fn no_food_cfg() -> SimConfig {
    SimConfig {
        world: WorldConfig {
            width: 12,
            height: 12,
        },
        agents: AgentConfig {
            count: 40,
            starting_energy: 50.0,
            metabolism: 0.2,
            max_energy: 100.0,
            max_age: 1000,
            initial_traits_dist: [0.4, 0.2, 0.2, 0.2],
            trait_mutation_rate: 0.0,
        },
        food: FoodConfig {
            initial_density: 0.0,
            regrowth_rate: 0.0,
            energy_per_food: 0.0,
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
        reproduction: ReproductionConfig {
            energy_threshold: 1000.0,
            offspring_energy_cost: 0.0,
            inherit_meme_prob: 0.0,
            min_age: 1000,
        },
        attack: AttackConfig {
            energy_cost_attacker: 0.0,
            energy_steal: 0.0,
            retaliation_chance: 0.0,
        },
        sharing: SharingConfig {
            share_threshold: 1000.0,
            share_amount: 0.0,
        },
        memes: MemePoolConfig {
            seed: vec![SeedMemeEntry {
                name: "share_with_allies".into(),
                carrier_fraction: 0.5,
            }],
        },
        run: RunConfig {
            seed: 1,
            horizon: 100,
            stop_on_extinction: false,
            cluster_snapshot_every: 0,
            metrics_emit_every: 1,
            survival_threshold: 0.05,
        },
    }
}

#[test]
fn carriers_lose_energy_faster_than_non_carriers() {
    let mut sim = Simulation::new(no_food_cfg(), Some(42));
    let cog_cost = sim
        .agents
        .iter()
        .find(|a| !a.inventory.is_empty())
        .map(|a| a.inventory[0].cognitive_cost)
        .unwrap_or(0.0);
    assert!(cog_cost > 0.0, "starter cognitive cost should be > 0");

    let start_energy_carrier: f32 = sim
        .agents
        .iter()
        .filter(|a| !a.inventory.is_empty())
        .map(|a| a.energy)
        .sum();
    let start_energy_other: f32 = sim
        .agents
        .iter()
        .filter(|a| a.inventory.is_empty())
        .map(|a| a.energy)
        .sum();
    let n_carriers = sim
        .agents
        .iter()
        .filter(|a| !a.inventory.is_empty())
        .count() as f32;
    let n_other = sim.agents.iter().filter(|a| a.inventory.is_empty()).count() as f32;
    assert!(n_carriers > 0.0 && n_other > 0.0);

    let start_mean_carrier = start_energy_carrier / n_carriers;
    let start_mean_other = start_energy_other / n_other;

    for _ in 0..30 {
        sim.step();
        let _ = sim.events_drain();
    }

    let end_energy_carrier: f32 = sim
        .agents
        .iter()
        .filter(|a| a.alive && !a.inventory.is_empty())
        .map(|a| a.energy)
        .sum();
    let end_energy_other: f32 = sim
        .agents
        .iter()
        .filter(|a| a.alive && a.inventory.is_empty())
        .map(|a| a.energy)
        .sum();
    let n_c = sim
        .agents
        .iter()
        .filter(|a| a.alive && !a.inventory.is_empty())
        .count() as f32;
    let n_o = sim
        .agents
        .iter()
        .filter(|a| a.alive && a.inventory.is_empty())
        .count() as f32;
    assert!(
        n_c > 0.0 && n_o > 0.0,
        "everyone died — pick a milder window"
    );

    let drop_carrier = start_mean_carrier - end_energy_carrier / n_c;
    let drop_other = start_mean_other - end_energy_other / n_o;
    assert!(
        drop_carrier > drop_other,
        "carriers should have dropped energy more (carrier: {drop_carrier}, other: {drop_other})"
    );
}
