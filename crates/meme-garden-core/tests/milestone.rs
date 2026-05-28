//! Milestone regression: cooperative vs aggressive meme under three scarcity
//! levels. Asserts (a) bit-identical re-runs and (b) the recorded *ensemble*
//! direction of cooperative-meme survival under each scarcity.
//!
//! Single-seed outcomes are drift-dominated at these population sizes (mid/high
//! retain only ~17/~0 agents), so the milestone is recorded as a win-rate across
//! a fixed seed sweep, not one cherry-picked seed. When this test fails after
//! intentional simulator changes, update the recorded thresholds below
//! intentionally — they ARE the milestone outcome.

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
    coop_final: f32,
    aggr_final: f32,
    alive_final: u32,
}

fn run_seed(level: &str, seed: u64) -> Outcome {
    let cfg = cfg(level);
    let horizon = cfg.run.horizon;
    let mut sim = Simulation::new(cfg, Some(seed));
    let mut last = None;
    for _ in 0..horizon {
        let m = sim.step();
        let _ = sim.events_drain();
        last = Some(m);
    }
    let last = last.unwrap();
    Outcome {
        coop_final: last.meme_prevalence_by_kind.cooperative,
        aggr_final: last.meme_prevalence_by_kind.aggressive,
        alive_final: last.alive,
    }
}

#[derive(Debug, Default)]
struct Ensemble {
    coop_wins: u32,
    aggr_wins: u32,
    collapses: u32,
}

/// A cooperative "win" = population survives and cooperative prevalence strictly
/// exceeds aggressive. Collapse = nobody alive at horizon (neither side wins).
fn run_ensemble(level: &str, seeds: std::ops::RangeInclusive<u64>) -> Ensemble {
    let mut e = Ensemble::default();
    for seed in seeds {
        let o = run_seed(level, seed);
        if o.alive_final == 0 {
            e.collapses += 1;
        } else if o.coop_final > o.aggr_final {
            e.coop_wins += 1;
        } else {
            e.aggr_wins += 1;
        }
    }
    e
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
fn milestone_ensemble_direction_is_recorded() {
    // Fixed seed sweep — the milestone is the win-rate across these, not any one.
    let seeds = 1..=16;
    let low = run_ensemble("low", seeds.clone());
    let mid = run_ensemble("mid", seeds.clone());
    let high = run_ensemble("high", seeds);

    eprintln!("low:  {low:?}");
    eprintln!("mid:  {mid:?}");
    eprintln!("high: {high:?}");

    // Recorded ensemble outcome (seeds 1..=16, shipped presets, horizon 1000):
    //   low : coop 14/16, aggr 2/16, collapse 0   — cooperation usually wins
    //   mid : coop 11/16, aggr 5/16, collapse 0   — cooperation still usually wins
    //   high: coop  3/16, aggr 3/16, collapse 10  — the system mostly collapses
    // The story: removing the predation free-ride + positive-sum sharing makes
    // cooperation broadly viable, until extreme scarcity collapses everyone.
    // These ARE the milestone outcome. Margins below the recorded counts absorb
    // incidental drift; a real directional flip should fail loudly. Changing them
    // is a deliberate act — investigate why behavior shifted before relaxing.
    assert!(
        low.coop_wins >= 12,
        "cooperation should usually win under low scarcity (got {low:?})"
    );
    assert!(
        mid.coop_wins > mid.aggr_wins && mid.coop_wins >= 8,
        "cooperation should usually win under mid scarcity (got {mid:?})"
    );
    assert!(
        high.collapses >= 8,
        "the population should mostly collapse under high scarcity (got {high:?})"
    );
}
