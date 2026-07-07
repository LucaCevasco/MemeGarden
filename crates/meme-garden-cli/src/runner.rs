use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use meme_garden_core::Simulation;

use crate::export::RunWriter;

/// Generate a default run-id of the form `YYYYMMDD-HHMMSS-<short-name>` using
/// UTC. This is the ONE place a wall-clock dependency lives — the simulation
/// core never sees it (Principle I).
pub fn default_run_id(short_name: &str) -> String {
    // Use a coarse formatted UTC stamp without pulling chrono. We accept that
    // two runs in the same wall-second collide on the run-id; the CLI errors
    // on that collision explicitly.
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (y, mo, d, h, mi, se) = ymdhms(secs);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{se:02}-{short_name}")
}

/// Very small UTC date math: epoch seconds → (Y, M, D, h, m, s). Sufficient
/// for the next ~80 years; not a calendar implementation.
fn ymdhms(t: u64) -> (u32, u32, u32, u32, u32, u32) {
    let se = (t % 60) as u32;
    let m = ((t / 60) % 60) as u32;
    let h = ((t / 3600) % 24) as u32;
    let mut days = (t / 86400) as i64;

    let mut year: i64 = 1970;
    loop {
        let leap = is_leap(year as u32);
        let yd = if leap { 366 } else { 365 };
        if days < yd {
            break;
        }
        days -= yd;
        year += 1;
    }
    let leap = is_leap(year as u32);
    let mdays = [
        31u32,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u32;
    for (i, &d_in_m) in mdays.iter().enumerate() {
        if (days as u32) < d_in_m {
            month = (i + 1) as u32;
            break;
        }
        days -= d_in_m as i64;
    }
    let day = days as u32 + 1;
    (year as u32, month, day, h, m, se)
}

fn is_leap(y: u32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Run the simulation through `horizon` ticks, emitting a Header event first,
/// then writing every emitted event and per-tick summary row to `writer`.
pub fn run_to_horizon(
    sim: &mut Simulation,
    writer: &mut RunWriter,
    horizon: u32,
    run_id: &str,
) -> Result<()> {
    sim.emit_header(run_id.to_string());
    for e in &sim.events_drain() {
        writer.write_event(e)?;
    }
    let mut ticks_done = 0u32;
    while ticks_done < horizon {
        let m = sim.step();
        let events = sim.events_drain();
        // Order: events emitted during the tick, then the tick aggregate is the
        // last record in that batch. Our emit_metrics_phase pushes Event::Tick
        // already, so we just write events as-is.
        for e in &events {
            writer.write_event(e)?;
        }
        writer.write_summary_row(&m)?;
        // Stop-on-extinction short circuit.
        if sim.config().run.stop_on_extinction && m.alive == 0 {
            break;
        }
        ticks_done += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ymdhms_epoch() {
        let (y, mo, d, h, mi, s) = ymdhms(0);
        assert_eq!((y, mo, d, h, mi, s), (1970, 1, 1, 0, 0, 0));
    }

    #[test]
    fn ymdhms_known() {
        // 2020-01-01 00:00:00 UTC = 1577836800
        let (y, mo, d, _, _, _) = ymdhms(1577836800);
        assert_eq!((y, mo, d), (2020, 1, 1));
    }
}
