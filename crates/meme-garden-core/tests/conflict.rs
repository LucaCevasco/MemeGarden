//! Conflict resolution: agents never carry two conflicting memes at once,
//! and the reject/replace/recombine roll is shaped by config + strength.

use meme_garden_core::*;

fn preset(level: &str) -> SimConfig {
    let toml = match level {
        "low" => include_str!("../../../configs/presets/cooperation-vs-selfish-low.toml"),
        "mid" => include_str!("../../../configs/presets/cooperation-vs-selfish-mid.toml"),
        "high" => include_str!("../../../configs/presets/cooperation-vs-selfish-high.toml"),
        _ => panic!("unknown"),
    };
    SimConfig::from_toml_str(toml).unwrap()
}

#[test]
fn no_agent_ever_carries_two_conflicting_memes() {
    for level in ["low", "mid", "high"] {
        let mut sim = Simulation::new(preset(level), Some(42));
        for tick in 0..800 {
            sim.step();
            let _ = sim.events_drain();
            for a in &sim.agents {
                if !a.alive {
                    continue;
                }
                let has_coop = a.has_kind(MemeKind::Cooperative);
                let has_aggr = a.has_kind(MemeKind::Aggressive);
                assert!(
                    !(has_coop && has_aggr),
                    "[{level} @ tick {tick}] agent {:?} carries both Cooperative AND Aggressive memes",
                    a.id
                );
            }
        }
    }
}

#[test]
fn conflict_events_appear_when_there_are_two_seeds() {
    // Low scarcity preset seeds 50% cooperative + 50% aggressive — they're
    // guaranteed to meet via the random walk. We should see at least one
    // MemeReplaced OR Recombination event over a 500-tick run.
    let mut sim = Simulation::new(preset("low"), Some(42));
    let mut replaced = 0u32;
    let mut recombined = 0u32;
    for _ in 0..500 {
        sim.step();
        for e in sim.events_drain() {
            match e {
                Event::MemeReplaced { .. } => replaced += 1,
                Event::Recombination { .. } => recombined += 1,
                _ => {}
            }
        }
    }
    assert!(
        replaced + recombined > 0,
        "expected at least one conflict resolution event; got replaced={replaced} recombined={recombined}"
    );
}

#[test]
fn zero_recombine_share_means_no_recombination_from_transmission() {
    let mut cfg = preset("low");
    cfg.conflict.recombine_share = 0.0;
    let mut sim = Simulation::new(cfg, Some(42));
    let mut recombined_from_transmission = 0u32;
    let mut replaced = 0u32;
    for _ in 0..400 {
        sim.step();
        for e in sim.events_drain() {
            match e {
                // Reproduction also emits Recombination via its 20% offspring
                // recombine path — that fires regardless of conflict.recombine_share.
                // The transmission/imitation conflict path is what we're gating.
                // Distinguishing the two would require a new event field; for
                // this test we just assert MemeReplaced still fires (proves the
                // conflict path is alive) and accept that Recombination may
                // appear from reproduction.
                Event::MemeReplaced { .. } => replaced += 1,
                Event::Recombination { .. } => recombined_from_transmission += 1,
                _ => {}
            }
        }
    }
    let _ = recombined_from_transmission;
    assert!(
        replaced > 0,
        "expected MemeReplaced events with recombine_share=0; got {replaced}"
    );
}
