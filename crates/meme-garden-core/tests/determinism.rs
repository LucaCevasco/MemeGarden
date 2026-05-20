//! Full-simulation determinism: byte-identical event streams for fixed seeds.

use meme_garden_core::*;

fn cfg() -> SimConfig {
    let toml = include_str!("../../../configs/presets/cooperation-vs-selfish-low.toml");
    SimConfig::from_toml_str(toml).unwrap()
}

#[test]
fn paired_runs_are_bit_identical() {
    let c1 = cfg();
    let c2 = cfg();
    let mut a = Simulation::new(c1, Some(42));
    let mut b = Simulation::new(c2, Some(42));
    for _ in 0..500 {
        let ma = a.step();
        let mb = b.step();
        let ea = a.events_drain();
        let eb = b.events_drain();
        let ja = serde_json::to_string(&ea).unwrap();
        let jb = serde_json::to_string(&eb).unwrap();
        assert_eq!(ja, jb, "events diverged at tick {}", ma.tick);
        assert_eq!(ma.meme_prevalence_by_kind, mb.meme_prevalence_by_kind);
        assert_eq!(ma.diversity_shannon, mb.diversity_shannon);
        assert_eq!(ma.transmissions_this_tick, mb.transmissions_this_tick);
    }
}

#[test]
fn different_seeds_diverge() {
    let mut a = Simulation::new(cfg(), Some(1));
    let mut b = Simulation::new(cfg(), Some(2));
    let mut diverged = false;
    for _ in 0..200 {
        let ma = a.step();
        let mb = b.step();
        let _ = a.events_drain();
        let _ = b.events_drain();
        if ma.meme_prevalence_by_kind != mb.meme_prevalence_by_kind {
            diverged = true;
            break;
        }
    }
    assert!(
        diverged,
        "different seeds produced identical metric streams — RNG plumbing is suspicious"
    );
}
