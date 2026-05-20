//! Single-starter-meme transmission: confirms each starter can spread beyond
//! its initial seed under default-ish conditions.

use meme_garden_core::config::*;
use meme_garden_core::*;

fn single_starter_cfg(starter: &str) -> SimConfig {
    SimConfig {
        world: WorldConfig {
            width: 30,
            height: 20,
        },
        agents: AgentConfig {
            count: 80,
            starting_energy: 25.0,
            metabolism: 0.3,
            max_energy: 60.0,
            max_age: 1500,
            initial_traits_dist: [0.4, 0.2, 0.2, 0.2],
            trait_mutation_rate: 0.0,
        },
        food: FoodConfig {
            initial_density: 0.25,
            regrowth_rate: 0.008,
            energy_per_food: 10.0,
        },
        scarcity: ScarcityConfig {
            level: "custom".into(),
        },
        cognition: CognitionConfig { inventory_cap: 4 },
        transmission: TransmissionConfig {
            base_rate: 0.8,
            social_copying_bias_mean: 0.8,
            social_copying_bias_std: 0.0,
            prestige_boost: 0.0,
        },
        mutation: MutationConfig {
            strength_jitter_max: 0.0,
            enum_swap_probability: 0.0,
        },
        reproduction: ReproductionConfig {
            energy_threshold: 50.0,
            offspring_energy_cost: 20.0,
            inherit_meme_prob: 0.0,
            min_age: 500,
        },
        attack: AttackConfig {
            energy_cost_attacker: 1.0,
            energy_steal: 2.0,
            retaliation_chance: 0.0,
        },
        sharing: SharingConfig {
            share_threshold: 12.0,
            share_amount: 1.0,
        },
        memes: MemePoolConfig {
            seed: vec![SeedMemeEntry {
                name: starter.into(),
                carrier_fraction: 0.05,
            }],
        },
        run: RunConfig {
            seed: 13,
            horizon: 500,
            stop_on_extinction: false,
            cluster_snapshot_every: 0,
            metrics_emit_every: 1,
            survival_threshold: 0.05,
        },
    }
}

fn carrier_fraction(sim: &Simulation, kind: MemeKind) -> f32 {
    let alive: u32 = sim.agents.iter().filter(|a| a.alive).count() as u32;
    if alive == 0 {
        return 0.0;
    }
    let carriers = sim
        .agents
        .iter()
        .filter(|a| a.alive && a.has_kind(kind))
        .count();
    carriers as f32 / alive as f32
}

#[test]
fn share_with_allies_spreads() {
    let mut sim = Simulation::new(single_starter_cfg("share_with_allies"), Some(7));
    let initial = carrier_fraction(&sim, MemeKind::Cooperative);
    let mut peak = initial;
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
        peak = peak.max(carrier_fraction(&sim, MemeKind::Cooperative));
    }
    assert!(
        peak > initial,
        "share_with_allies did not spread (initial={initial}, peak={peak})"
    );
}

#[test]
fn attack_low_energy_outsiders_spreads() {
    let mut sim = Simulation::new(single_starter_cfg("attack_low_energy_outsiders"), Some(7));
    let initial = carrier_fraction(&sim, MemeKind::Aggressive);
    let mut peak = initial;
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
        peak = peak.max(carrier_fraction(&sim, MemeKind::Aggressive));
    }
    assert!(peak > initial, "attack_low_energy_outsiders did not spread");
}

#[test]
fn avoid_strangers_spreads() {
    let mut sim = Simulation::new(single_starter_cfg("avoid_strangers"), Some(11));
    let initial = carrier_fraction(&sim, MemeKind::Defensive);
    let mut peak = initial;
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
        peak = peak.max(carrier_fraction(&sim, MemeKind::Defensive));
    }
    assert!(peak > initial, "avoid_strangers did not spread");
}

#[test]
fn copy_high_energy_spreads() {
    let mut sim = Simulation::new(single_starter_cfg("copy_high_energy"), Some(13));
    let initial = carrier_fraction(&sim, MemeKind::Imitative);
    let mut peak = initial;
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
        peak = peak.max(carrier_fraction(&sim, MemeKind::Imitative));
    }
    assert!(peak > initial, "copy_high_energy did not spread");
}

#[test]
fn punish_non_sharers_spreads() {
    let mut sim = Simulation::new(single_starter_cfg("punish_non_sharers"), Some(17));
    let initial = carrier_fraction(&sim, MemeKind::Punitive);
    let mut peak = initial;
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
        peak = peak.max(carrier_fraction(&sim, MemeKind::Punitive));
    }
    assert!(peak > initial, "punish_non_sharers did not spread");
}

#[test]
fn prefer_same_meme_spreads() {
    let mut sim = Simulation::new(single_starter_cfg("prefer_same_meme"), Some(19));
    let initial = carrier_fraction(&sim, MemeKind::Conformist);
    let mut peak = initial;
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
        peak = peak.max(carrier_fraction(&sim, MemeKind::Conformist));
    }
    assert!(peak > initial, "prefer_same_meme did not spread");
}

#[test]
fn zero_transmission_means_no_transmission_events() {
    let mut cfg = single_starter_cfg("share_with_allies");
    cfg.transmission.base_rate = 0.0;
    let mut sim = Simulation::new(cfg, Some(3));
    for _ in 0..200 {
        sim.step();
        for e in sim.events_drain() {
            if let Event::Transmission { .. } = e {
                panic!("transmission event observed with base_rate=0");
            }
        }
    }
}
