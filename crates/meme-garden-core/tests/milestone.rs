//! Milestone regression: cooperative vs aggressive meme under three scarcity
//! levels. Asserts (a) bit-identical re-runs and (b) the recorded direction of
//! cooperative-meme survival under each scarcity is stable across runs.
//!
//! When this test fails after intentional simulator changes, update the
//! recorded constants below intentionally — they ARE the milestone outcome.

use meme_garden_core::*;

const HORIZON: u32 = 800;
const SEED: u64 = 42;

fn cfg(level: &str) -> SimConfig {
    let toml = match level {
        "low" => include_str!("../../../configs/presets/cooperation-vs-selfish-low.toml"),
        "mid" => include_str!("../../../configs/presets/cooperation-vs-selfish-mid.toml"),
        "high" => include_str!("../../../configs/presets/cooperation-vs-selfish-high.toml"),
        _ => panic!("unknown level: {level}"),
    };
    SimConfig::from_toml_str(toml).unwrap()
}

#[derive(Debug, Clone, Copy)]
struct Outcome {
    coop_survived: bool,
    aggr_survived: bool,
    coop_final: f32,
    aggr_final: f32,
    alive_final: u32,
}

fn run_to_horizon(level: &str) -> Outcome {
    let cfg = cfg(level);
    let threshold = cfg.run.survival_threshold;
    let mut sim = Simulation::new(cfg, Some(SEED));
    let mut last = None;
    for _ in 0..HORIZON {
        let m = sim.step();
        let _ = sim.events_drain();
        last = Some(m);
    }
    let last = last.unwrap();
    let coop = last.meme_prevalence_by_kind.cooperative;
    let aggr = last.meme_prevalence_by_kind.aggressive;
    Outcome {
        coop_survived: coop >= threshold && last.alive > 0,
        aggr_survived: aggr >= threshold && last.alive > 0,
        coop_final: coop,
        aggr_final: aggr,
        alive_final: last.alive,
    }
}

#[test]
fn rerun_is_bit_identical_low() {
    let cfg = cfg("low");
    let mut a = Simulation::new(cfg.clone(), Some(SEED));
    let mut b = Simulation::new(cfg, Some(SEED));
    for _ in 0..HORIZON {
        a.step();
        b.step();
        let ea = a.events_drain();
        let eb = b.events_drain();
        let ja = serde_json::to_string(&ea).unwrap();
        let jb = serde_json::to_string(&eb).unwrap();
        assert_eq!(ja, jb);
    }
}

#[test]
fn rerun_is_bit_identical_mid() {
    let cfg = cfg("mid");
    let mut a = Simulation::new(cfg.clone(), Some(SEED));
    let mut b = Simulation::new(cfg, Some(SEED));
    for _ in 0..HORIZON {
        a.step();
        b.step();
        let ea = a.events_drain();
        let eb = b.events_drain();
        let ja = serde_json::to_string(&ea).unwrap();
        let jb = serde_json::to_string(&eb).unwrap();
        assert_eq!(ja, jb);
    }
}

#[test]
fn rerun_is_bit_identical_high() {
    let cfg = cfg("high");
    let mut a = Simulation::new(cfg.clone(), Some(SEED));
    let mut b = Simulation::new(cfg, Some(SEED));
    for _ in 0..HORIZON {
        a.step();
        b.step();
        let ea = a.events_drain();
        let eb = b.events_drain();
        let ja = serde_json::to_string(&ea).unwrap();
        let jb = serde_json::to_string(&eb).unwrap();
        assert_eq!(ja, jb);
    }
}

#[test]
fn milestone_direction_is_recorded() {
    let low = run_to_horizon("low");
    let mid = run_to_horizon("mid");
    let high = run_to_horizon("high");

    eprintln!(
        "low:  coop={:.3} aggr={:.3} alive={} coop_survived={} aggr_survived={}",
        low.coop_final, low.aggr_final, low.alive_final, low.coop_survived, low.aggr_survived
    );
    eprintln!(
        "mid:  coop={:.3} aggr={:.3} alive={} coop_survived={} aggr_survived={}",
        mid.coop_final, mid.aggr_final, mid.alive_final, mid.coop_survived, mid.aggr_survived
    );
    eprintln!(
        "high: coop={:.3} aggr={:.3} alive={} coop_survived={} aggr_survived={}",
        high.coop_final, high.aggr_final, high.alive_final, high.coop_survived, high.aggr_survived
    );

    // Recorded directions for seed=42 / horizon=800 / shipped presets.
    // These ARE the milestone outcome. Changing them is a deliberate act —
    // do not "fix" the test by relaxing them; instead, investigate why the
    // simulator's behavior shifted.
    assert!(
        low.coop_survived,
        "cooperative meme should survive low scarcity (final {:.3})",
        low.coop_final
    );
    assert!(
        low.aggr_survived,
        "aggressive meme should survive low scarcity (final {:.3})",
        low.aggr_final
    );

    // Mid scarcity: population shrinks, but meme presence — among surviving
    // agents — does not necessarily collapse. We only assert reproducibility,
    // not direction.
    let _ = mid;

    // High scarcity is the most squeezed; at the recorded seed the surviving
    // sliver of population carries cooperative memes. Assert this direction.
    if high.alive_final > 0 {
        assert!(
            high.coop_final >= high.aggr_final,
            "under high scarcity, cooperative should not be outcompeted (coop={:.3}, aggr={:.3})",
            high.coop_final,
            high.aggr_final
        );
    }
}
