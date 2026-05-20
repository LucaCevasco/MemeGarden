//! TUI and headless modes must produce identical event streams for the same
//! `(config, seed)`. Since both modes use the same `Simulation` instance and
//! the TUI doesn't drive the simulator independently, we verify this at the
//! library level: two `Simulation`s with the same seed and config produce
//! byte-identical JSONL streams whether we drain events between ticks or
//! all at once at the end.

use meme_garden_core::*;

fn cfg() -> SimConfig {
    let toml = include_str!("../../../configs/presets/cooperation-vs-selfish-mid.toml");
    SimConfig::from_toml_str(toml).unwrap()
}

#[test]
fn drain_cadence_does_not_affect_outputs() {
    // Run A drains events every tick (headless cadence).
    let mut sim_a = Simulation::new(cfg(), Some(42));
    let mut a_records: Vec<String> = Vec::new();
    for _ in 0..200 {
        sim_a.step();
        for e in sim_a.events_drain() {
            a_records.push(serde_json::to_string(&e).unwrap());
        }
    }

    // Run B drains events every 20 ticks (TUI-ish cadence — but since we still
    // drain into pending_events, the contents must be identical).
    let mut sim_b = Simulation::new(cfg(), Some(42));
    let mut b_records: Vec<String> = Vec::new();
    for tick in 0..200u32 {
        sim_b.step();
        if tick % 20 == 19 {
            for e in sim_b.events_drain() {
                b_records.push(serde_json::to_string(&e).unwrap());
            }
        }
    }
    // Final drain in case any leftovers.
    for e in sim_b.events_drain() {
        b_records.push(serde_json::to_string(&e).unwrap());
    }

    assert_eq!(
        a_records, b_records,
        "different drain cadences produced different event streams"
    );
}
