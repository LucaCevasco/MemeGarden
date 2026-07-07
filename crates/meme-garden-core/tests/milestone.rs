//! Milestone regression: cooperative vs aggressive meme under three scarcity
//! levels. Asserts (a) bit-identical re-runs and (b) the recorded *ensemble*
//! direction of cooperative-meme survival under each scarcity.
//!
//! Single-seed outcomes are drift-dominated at these population sizes (mid/high
//! retain only ~23/~0 agents), so the milestone is recorded as a win-rate across
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
    mean_alive: f32,
}

/// A cooperative "win" = population survives and cooperative prevalence strictly
/// exceeds aggressive. Collapse = nobody alive at horizon (neither side wins).
fn run_ensemble(level: &str, seeds: std::ops::RangeInclusive<u64>) -> Ensemble {
    let mut e = Ensemble::default();
    let mut alive_sum = 0u64;
    let mut n = 0u32;
    for seed in seeds {
        let o = run_seed(level, seed);
        alive_sum += o.alive_final as u64;
        n += 1;
        if o.alive_final == 0 {
            e.collapses += 1;
        } else if o.coop_final > o.aggr_final {
            e.coop_wins += 1;
        } else {
            e.aggr_wins += 1;
        }
    }
    e.mean_alive = if n == 0 {
        0.0
    } else {
        alive_sum as f32 / n as f32
    };
    e
}

#[test]
fn rerun_is_bit_identical() {
    for level in ["low", "mid", "high"] {
        let cfg = cfg(level);
        let mut a = Simulation::new(cfg.clone(), Some(SEED));
        let mut b = Simulation::new(cfg, Some(SEED));
        for _ in 0..HORIZON {
            a.step();
            b.step();
            let ja = serde_json::to_string(&a.events_drain()).unwrap();
            let jb = serde_json::to_string(&b.events_drain()).unwrap();
            assert_eq!(ja, jb, "re-run diverged at scarcity level {level}");
        }
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
    //   low : coop 11/16, aggr 5/16, collapse  0  — cooperation usually wins
    //   mid : coop  8/16, aggr 8/16, collapse  0  — contested, roughly a coin-flip
    //   high: coop  1/16, aggr 0/16, collapse 15  — the system almost always collapses
    // The story: positive-sum sharing keeps cooperation broadly viable under low
    // scarcity; at mid scarcity predation catches up and the outcome is contested;
    // under high scarcity nearly everyone starves regardless of strategy.
    // These ARE the milestone outcome. Margins below the recorded counts absorb
    // incidental drift; a real directional flip should fail loudly. Changing them
    // is a deliberate act — investigate why behavior shifted before relaxing.
    assert!(
        low.coop_wins >= 9,
        "cooperation should usually win under low scarcity (got {low:?})"
    );
    assert!(
        mid.collapses == 0 && mid.coop_wins >= 5 && mid.aggr_wins >= 5,
        "mid scarcity should stay contested with no collapse (got {mid:?})"
    );
    assert!(
        high.collapses >= 12,
        "the population should almost always collapse under high scarcity (got {high:?})"
    );
}
