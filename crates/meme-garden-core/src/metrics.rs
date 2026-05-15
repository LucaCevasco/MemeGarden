use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Metrics {
    pub tick: u64,
    pub alive: u32,
    pub food_count: u32,
    pub meme_carriers: u32,
    /// Carriers as a fraction of the living population, 0.0..=1.0.
    pub meme_prevalence: f32,
    /// Mean energy across living agents.
    pub mean_energy: f32,
}

impl Metrics {
    pub fn csv_header() -> &'static str {
        "tick,alive,food_count,meme_carriers,meme_prevalence,mean_energy"
    }

    pub fn to_csv_row(&self) -> String {
        format!(
            "{},{},{},{},{:.4},{:.3}",
            self.tick,
            self.alive,
            self.food_count,
            self.meme_carriers,
            self.meme_prevalence,
            self.mean_energy
        )
    }
}
