//! Lineage closure invariants.

use meme_garden_core::*;

fn cfg() -> SimConfig {
    let toml = include_str!("../../../configs/presets/cooperation-vs-selfish-mid.toml");
    SimConfig::from_toml_str(toml).unwrap()
}

#[test]
fn every_live_meme_traces_to_a_starter() {
    let mut sim = Simulation::new(cfg(), Some(99));
    for _ in 0..500 {
        sim.step();
        let _ = sim.events_drain();
    }
    let mut checked = 0;
    for a in &sim.agents {
        if !a.alive {
            continue;
        }
        for m in &a.inventory {
            let starter = sim.lineage.trace_to_starter(m.lineage_id);
            assert!(
                starter.is_some(),
                "meme {} lineage {:?} did not trace to a starter",
                m.id.0,
                m.lineage_id
            );
            let node = sim.lineage.get(starter.unwrap()).unwrap();
            assert_eq!(node.origin, LineageOrigin::Starter);
            checked += 1;
        }
    }
    assert!(checked > 0, "no live memes to check — config too harsh?");
}

#[test]
fn lineage_parents_capped_at_two() {
    let mut sim = Simulation::new(cfg(), Some(3));
    for _ in 0..300 {
        sim.step();
        let _ = sim.events_drain();
    }
    for node in sim.lineage.nodes() {
        assert!(
            node.parents.len() <= 2,
            "lineage node had {} parents",
            node.parents.len()
        );
    }
}
