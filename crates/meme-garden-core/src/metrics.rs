use serde::{Deserialize, Serialize};

use crate::agent::AgentId;
use crate::meme::{MemeId, MemeKind};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrevalenceByKind {
    pub cooperative: f32,
    pub defensive: f32,
    pub imitative: f32,
    pub aggressive: f32,
    pub punitive: f32,
    pub conformist: f32,
    pub mutant: f32,
}

impl PrevalenceByKind {
    pub fn from_array(a: [f32; 7]) -> Self {
        Self {
            cooperative: a[0],
            defensive: a[1],
            imitative: a[2],
            aggressive: a[3],
            punitive: a[4],
            conformist: a[5],
            mutant: a[6],
        }
    }

    pub fn as_array(&self) -> [f32; 7] {
        [
            self.cooperative,
            self.defensive,
            self.imitative,
            self.aggressive,
            self.punitive,
            self.conformist,
            self.mutant,
        ]
    }

    pub fn get(&self, kind: MemeKind) -> f32 {
        self.as_array()[kind.idx()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopulationByTrait {
    pub generous: u32,
    pub cautious: u32,
    pub aggressive: u32,
    pub conformist: u32,
}

impl PopulationByTrait {
    pub fn from_array(a: [u32; 4]) -> Self {
        Self {
            generous: a[0],
            cautious: a[1],
            aggressive: a[2],
            conformist: a[3],
        }
    }

    pub fn as_array(&self) -> [u32; 4] {
        [self.generous, self.cautious, self.aggressive, self.conformist]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub tick: u64,
    pub alive: u32,
    pub food_count: u32,
    pub population_by_trait: PopulationByTrait,
    pub meme_count: u32,
    pub meme_prevalence_by_kind: PrevalenceByKind,
    pub diversity_shannon: f32,
    pub dominance_top1_fraction: f32,
    pub mean_energy: f32,
    pub mean_age: f32,
    pub transmissions_this_tick: u32,
    pub mutations_this_tick: u32,
    pub deaths_this_tick: u32,
    pub births_this_tick: u32,
}

impl Metrics {
    pub fn csv_header() -> &'static str {
        "tick,alive,food_count,meme_count,prevalence_cooperative,prevalence_defensive,prevalence_imitative,prevalence_aggressive,prevalence_punitive,prevalence_conformist,prevalence_mutant,diversity_shannon,dominance_top1,mean_energy,mean_age,transmissions,mutations,births,deaths"
    }

    pub fn to_csv_row(&self) -> String {
        let p = self.meme_prevalence_by_kind.as_array();
        format!(
            "{},{},{},{},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.3},{:.3},{},{},{},{}",
            self.tick,
            self.alive,
            self.food_count,
            self.meme_count,
            p[0],
            p[1],
            p[2],
            p[3],
            p[4],
            p[5],
            p[6],
            self.diversity_shannon,
            self.dominance_top1_fraction,
            self.mean_energy,
            self.mean_age,
            self.transmissions_this_tick,
            self.mutations_this_tick,
            self.births_this_tick,
            self.deaths_this_tick,
        )
    }
}

/// Shannon diversity index H = -Σ p_i * ln(p_i), with p_i renormalized so they sum to 1.
pub fn shannon_diversity(prevalence_by_kind: &[f32]) -> f32 {
    let total: f32 = prevalence_by_kind.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let mut h = 0.0;
    for &p in prevalence_by_kind {
        if p > 0.0 {
            let q = p / total;
            h -= q * q.ln();
        }
    }
    h
}

/// Largest single bucket as a fraction of the total. 0.0 if total is 0.
pub fn top1_fraction(prevalence_by_kind: &[f32]) -> f32 {
    let total: f32 = prevalence_by_kind.iter().sum();
    if total <= 0.0 {
        return 0.0;
    }
    let max = prevalence_by_kind.iter().copied().fold(0.0_f32, f32::max);
    max / total
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeathCause {
    Starvation,
    Aging,
    Combat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutatedField {
    Trigger,
    Target,
    Effect,
    Strength,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum ExtinctionScope {
    Population,
    AllMemes,
    SingleMeme { meme: MemeId },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterEntry {
    pub id: u32,
    pub members: Vec<AgentId>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    Header {
        schema_version: u32,
        run_id: String,
        core_version: String,
    },
    Tick(Box<Metrics>),
    Birth {
        tick: u64,
        child: AgentId,
        parent: AgentId,
        inherited: Vec<MemeId>,
    },
    Death {
        tick: u64,
        agent: AgentId,
        cause: DeathCause,
    },
    Transmission {
        tick: u64,
        from: AgentId,
        to: AgentId,
        meme: MemeId,
    },
    Mutation {
        tick: u64,
        parent_meme: MemeId,
        child_meme: MemeId,
        field: MutatedField,
    },
    Recombination {
        tick: u64,
        parents: (MemeId, MemeId),
        child_meme: MemeId,
    },
    MemeForgotten {
        tick: u64,
        agent: AgentId,
        meme: MemeId,
    },
    MemeReplaced {
        tick: u64,
        agent: AgentId,
        old: MemeId,
        new: MemeId,
    },
    Extinction {
        tick: u64,
        #[serde(flatten)]
        scope: ExtinctionScope,
    },
    ClusterSnapshot {
        tick: u64,
        clusters: Vec<ClusterEntry>,
    },
}

impl Event {
    pub fn kind_word(&self) -> &'static str {
        match self {
            Event::Header { .. } => "header",
            Event::Tick(_) => "tick",
            Event::Birth { .. } => "birth",
            Event::Death { .. } => "death",
            Event::Transmission { .. } => "transmission",
            Event::Mutation { .. } => "mutation",
            Event::Recombination { .. } => "recombination",
            Event::MemeForgotten { .. } => "meme_forgotten",
            Event::MemeReplaced { .. } => "meme_replaced",
            Event::Extinction { .. } => "extinction",
            Event::ClusterSnapshot { .. } => "cluster_snapshot",
        }
    }
}

/// Index in the prevalence array for `kind` (so callers don't need to import the enum).
pub fn prevalence_index(kind: MemeKind) -> usize {
    kind.idx()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diversity_zero_total() {
        assert_eq!(shannon_diversity(&[0.0, 0.0, 0.0]), 0.0);
        assert_eq!(top1_fraction(&[0.0, 0.0, 0.0]), 0.0);
    }

    #[test]
    fn diversity_uniform_two() {
        // Two equal buckets → H = ln(2)
        let h = shannon_diversity(&[0.5, 0.5]);
        assert!((h - std::f32::consts::LN_2).abs() < 1e-5);
    }

    #[test]
    fn top1_fraction_basic() {
        assert!((top1_fraction(&[0.2, 0.6, 0.2]) - 0.6).abs() < 1e-5);
    }

    #[test]
    fn csv_header_column_count_matches_row() {
        let m = Metrics {
            tick: 0,
            alive: 0,
            food_count: 0,
            population_by_trait: PopulationByTrait::from_array([0; 4]),
            meme_count: 0,
            meme_prevalence_by_kind: PrevalenceByKind::from_array([0.0; 7]),
            diversity_shannon: 0.0,
            dominance_top1_fraction: 0.0,
            mean_energy: 0.0,
            mean_age: 0.0,
            transmissions_this_tick: 0,
            mutations_this_tick: 0,
            deaths_this_tick: 0,
            births_this_tick: 0,
        };
        let header_cols = Metrics::csv_header().split(',').count();
        let row_cols = m.to_csv_row().split(',').count();
        assert_eq!(header_cols, row_cols);
    }
}
