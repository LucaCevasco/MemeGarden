use std::time::{Duration, Instant};

use meme_garden_core::{Metrics, Simulation};

/// Soft cap on retained metrics for the sparkline pane. History is trimmed back
/// to this length once it grows past 2× (amortizes the `drain` shifts).
const HISTORY_RING_CAP: usize = 2048;

pub struct App {
    pub sim: Simulation,
    pub history: Vec<Metrics>,
    pub paused: bool,
    pub tps: f32,
    pub should_quit: bool,
    last_tick_at: Instant,
}

impl App {
    pub fn new(sim: Simulation, tps: f32) -> Self {
        Self {
            sim,
            history: Vec::new(),
            paused: false,
            tps: tps.max(0.1),
            should_quit: false,
            last_tick_at: Instant::now(),
        }
    }

    /// Advance the simulation as many ticks as the configured rate dictates.
    /// Drained events are discarded — the TUI does not write run artifacts; the
    /// CLI runner does.
    pub fn maybe_tick(&mut self) {
        if self.paused {
            self.last_tick_at = Instant::now();
            return;
        }
        let interval = Duration::from_secs_f32(1.0 / self.tps);
        while self.last_tick_at.elapsed() >= interval {
            self.step_once();
            self.last_tick_at += interval;
        }
    }

    pub fn force_step(&mut self) {
        self.step_once();
    }

    /// Advance one tick, discard its events (the TUI writes no artifacts), and
    /// record the metrics, trimming the history back to the ring cap when it
    /// grows past 2×.
    fn step_once(&mut self) {
        let m = self.sim.step();
        let _ = self.sim.events_drain();
        self.history.push(m);
        if self.history.len() > HISTORY_RING_CAP * 2 {
            self.history.drain(0..self.history.len() - HISTORY_RING_CAP);
        }
    }

    pub fn last_metrics(&self) -> Option<&Metrics> {
        self.history.last()
    }

    pub fn speed_up(&mut self) {
        self.tps = (self.tps * 1.5).min(240.0);
    }

    pub fn slow_down(&mut self) {
        self.tps = (self.tps / 1.5).max(0.5);
    }
}
