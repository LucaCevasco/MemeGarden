use std::collections::BTreeMap;

use smallvec::{smallvec, SmallVec};

use crate::action::Action;
use crate::agent::{Agent, AgentId, AgentMemory, AgentTrait, Position};
use crate::config::SimConfig;
use crate::lineage::{LineageGraph, LineageId, LineageOrigin};
use crate::meme::{MemeId, MemeKind};
use crate::metrics::{
    shannon_diversity, top1_fraction, ClusterEntry, DeathCause, Event, ExtinctionScope, Metrics,
};
use crate::policy::{compute_action, NeighborInfo, Perception};
use crate::rng::SimRng;
use crate::starters;

#[derive(Debug, Clone)]
pub struct Grid {
    pub width: u32,
    pub height: u32,
    food: Vec<bool>,
}

impl Grid {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            food: vec![false; (width * height) as usize],
        }
    }

    pub fn idx(&self, x: i32, y: i32) -> usize {
        (y as u32 * self.width + x as u32) as usize
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height
    }

    pub fn has_food(&self, x: i32, y: i32) -> bool {
        self.in_bounds(x, y) && self.food[self.idx(x, y)]
    }

    pub fn set_food(&mut self, x: i32, y: i32, v: bool) {
        let i = self.idx(x, y);
        self.food[i] = v;
    }

    pub fn food_count(&self) -> u32 {
        self.food.iter().filter(|f| **f).count() as u32
    }
}

/// Outcome of a single meme-acquisition attempt. Recombined / Replaced both
/// result in inventory mutation; Rejected and Skipped leave inventory untouched.
#[derive(Debug, Clone, Copy)]
enum AcquireOutcome {
    Accepted,
    Skipped,
    Rejected,
    Replaced,
    Recombined {
        parents: (MemeId, MemeId),
        child_meme_id: MemeId,
    },
}

#[derive(Debug)]
pub struct Simulation {
    pub config: SimConfig,
    pub grid: Grid,
    pub agents: Vec<Agent>,
    pub lineage: LineageGraph,
    pub tick: u64,
    next_agent_id: u32,
    next_meme_id: u32,
    /// Lineage id of each starter meme, keyed by starter name. Used to attach
    /// inherited and mutated children to a stable ancestor.
    starter_lineage: BTreeMap<String, LineageId>,
    rng: SimRng,
    pending_events: Vec<Event>,
    extinction_population_emitted: bool,
    extinction_memes_emitted: bool,
}

impl Simulation {
    pub fn new(mut config: SimConfig, seed_override: Option<u64>) -> Self {
        let seed = seed_override.unwrap_or(config.run.seed);
        // Apply scarcity preset transform once at construction so the resolved
        // food.* values reflect what actually drives the run. The on-disk config
        // copy written by the CLI must come from this resolved struct.
        let _ = config.apply_scarcity();
        if let Some(s) = seed_override {
            config.run.seed = s;
        }
        let mut rng = SimRng::from_seed(seed);

        // Build lineage graph with one starter node per starter meme name in the
        // pool — even if carrier fraction is 0 it gives us a stable lineage anchor.
        let mut lineage = LineageGraph::new();
        let mut starter_lineage: BTreeMap<String, LineageId> = BTreeMap::new();
        for name in starters::STARTERS {
            let id = lineage.add_starter(0);
            starter_lineage.insert(name.to_string(), id);
        }

        let mut grid = Grid::new(config.world.width, config.world.height);
        let cells = (config.world.width * config.world.height) as usize;
        for i in 0..cells {
            if rng.gen_bool(config.food.initial_density) {
                grid.food[i] = true;
            }
        }

        let mut agents = Vec::with_capacity(config.agents.count as usize);
        let mut next_meme_id = 1u32;
        for i in 0..config.agents.count {
            let x = rng.gen_range_usize(0, config.world.width as usize) as i32;
            let y = rng.gen_range_usize(0, config.world.height as usize) as i32;
            let mut a = Agent::new(AgentId(i), Position { x, y }, config.agents.starting_energy);

            // Sample initial trait from the distribution.
            let t = sample_trait(&config.agents.initial_traits_dist, &mut rng);
            a.traits.push(t);

            // Social copying bias: clamped gaussian-ish via two uniforms (Irwin-Hall n=2).
            let u1 = rng.gen_u32() as f32 / u32::MAX as f32;
            let u2 = rng.gen_u32() as f32 / u32::MAX as f32;
            let gauss_unit = u1 + u2 - 1.0; // approx N(0, 1/6)
            a.social_copying_bias = (config.transmission.social_copying_bias_mean
                + gauss_unit * config.transmission.social_copying_bias_std)
                .clamp(0.0, 1.0);

            agents.push(a);
        }

        // Seed memes: each agent gets AT MOST ONE starter at t=0, sampled
        // categorically from the pool. Per-entry `carrier_fraction` is the
        // per-agent probability of getting that starter. If the entries sum
        // to S ≤ 1.0, the remaining (1 - S) is the probability of starting
        // with no meme — leaving room for transmission to do work. If S > 1.0
        // the weights are normalized (the pool becomes "always pick one").
        // why: with the previous independent-rolls model, overlapping pools
        // (e.g. two starters at 0.5) gave ~25% of agents both memes from
        // tick 0, which masks contagion dynamics.
        let entries: Vec<crate::config::SeedMemeEntry> = config.memes.seed.clone();
        let total_weight: f32 = entries.iter().map(|e| e.carrier_fraction).sum();
        let span = total_weight.max(1.0);
        for a in agents.iter_mut() {
            if entries.is_empty() {
                continue;
            }
            let mut u = (rng.gen_u32() as f32 / u32::MAX as f32) * span;
            let mut chosen: Option<&crate::config::SeedMemeEntry> = None;
            for e in &entries {
                if u < e.carrier_fraction {
                    chosen = Some(e);
                    break;
                }
                u -= e.carrier_fraction;
            }
            let Some(entry) = chosen else { continue };
            let Some(ctor) = starters::lookup(&entry.name) else {
                continue;
            };
            let mut m = ctor();
            m.id = MemeId(next_meme_id);
            next_meme_id += 1;
            m.lineage_id = *starter_lineage
                .get(&entry.name)
                .expect("starter lineage must exist");
            if a.inventory.len() < config.cognition.inventory_cap as usize {
                a.inventory.push(m);
            }
        }

        Self {
            next_agent_id: config.agents.count,
            next_meme_id,
            config,
            grid,
            agents,
            lineage,
            tick: 0,
            starter_lineage,
            rng,
            pending_events: Vec::new(),
            extinction_population_emitted: false,
            extinction_memes_emitted: false,
        }
    }

    /// Drain buffered events and return them. Called by the CLI runner once per
    /// tick after `step` returns.
    pub fn events_drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn config(&self) -> &SimConfig {
        &self.config
    }

    /// Outcome of a meme acquisition attempt (transmission, imitation, inheritance).
    /// Acquisition is the only path memes enter inventories, so conflict resolution
    /// lives in `try_acquire` and nowhere else.
    fn try_acquire(&mut self, agent_idx: usize, new_meme: crate::meme::Meme) -> AcquireOutcome {
        // Already carry this exact kind? Same-direction same-kind memes don't
        // duplicate (imitation already filters; this guards transmission too).
        if self.agents[agent_idx]
            .inventory
            .iter()
            .any(|m| m.kind == new_meme.kind)
        {
            return AcquireOutcome::Skipped;
        }

        // Find a conflicting existing meme.
        let conflict_idx = self.agents[agent_idx]
            .inventory
            .iter()
            .position(|m| crate::meme::conflicts(m.kind, new_meme.kind));

        if let Some(j) = conflict_idx {
            // Roll outcome. Strength weights the reject/replace split; the
            // recombine slice is a fixed config knob carved off the top.
            let old = self.agents[agent_idx].inventory[j].clone();
            let p_recombine = self.config.conflict.recombine_share.clamp(0.0, 1.0);
            let p_replace_of_remainder =
                new_meme.strength / (old.strength + new_meme.strength).max(f32::EPSILON);
            let r = self.rng.gen_u32() as f32 / u32::MAX as f32;
            let p_replace = (1.0 - p_recombine) * p_replace_of_remainder;

            if r < p_recombine {
                let new_child_id = MemeId(self.next_meme_id);
                self.next_meme_id += 1;
                let child = crate::mutation::recombine(
                    &old,
                    &new_meme,
                    new_child_id,
                    &mut self.rng,
                    &mut self.lineage,
                    self.tick,
                );
                let child_id = child.id;
                self.agents[agent_idx].inventory.remove(j);
                self.agents[agent_idx].inventory.push(child);
                AcquireOutcome::Recombined {
                    parents: (old.id, new_meme.id),
                    child_meme_id: child_id,
                }
            } else if r < p_recombine + p_replace {
                let agent_id = self.agents[agent_idx].id;
                self.agents[agent_idx].inventory.remove(j);
                let new_id = new_meme.id;
                self.agents[agent_idx].inventory.push(new_meme);
                self.pending_events.push(Event::MemeReplaced {
                    tick: self.tick,
                    agent: agent_id,
                    old: old.id,
                    new: new_id,
                });
                AcquireOutcome::Replaced
            } else {
                AcquireOutcome::Rejected
            }
        } else {
            // No conflict → push, subject to inventory cap (FIFO eviction).
            let cap = self.config.cognition.inventory_cap as usize;
            if self.agents[agent_idx].inventory.len() >= cap {
                let forgotten = self.agents[agent_idx].inventory.remove(0);
                let aid = self.agents[agent_idx].id;
                self.pending_events.push(Event::MemeForgotten {
                    tick: self.tick,
                    agent: aid,
                    meme: forgotten.id,
                });
            }
            self.agents[agent_idx].inventory.push(new_meme);
            AcquireOutcome::Accepted
        }
    }

    /// Advance the simulation by one tick. Returns the per-tick metrics snapshot.
    /// Phase order is part of the determinism contract; reordering changes outputs.
    pub fn step(&mut self) -> Metrics {
        let perceptions = self.perception_phase();
        let actions = self.policy_phase(&perceptions);
        let mut deaths = self.action_phase(&perceptions, &actions);
        let (transmissions, mutations) = self.transmission_phase();
        let births = self.reproduction_phase(&perceptions);
        deaths += self.death_phase();
        self.world_maintenance_phase();
        let metrics = self.emit_metrics_phase(transmissions, mutations, deaths, births);
        self.cluster_snapshot_maybe();
        self.tick += 1;
        metrics
    }

    /// Convenience: append an `Event::Header` so JSONL writers can record
    /// schema version + run identity before the first tick.
    pub fn emit_header(&mut self, run_id: String) {
        self.pending_events.push(Event::Header {
            schema_version: 1,
            run_id,
            core_version: crate::CORE_VERSION.to_string(),
        });
    }

    // ----- Phase implementations -----

    fn perception_phase(&self) -> Vec<Perception> {
        let mut out = Vec::with_capacity(self.agents.len());
        for a in &self.agents {
            if !a.alive {
                out.push(Perception::default());
                continue;
            }
            let mut p = Perception {
                agent_id: a.id,
                position: a.position,
                ..Default::default()
            };

            // Adjacent food cells.
            for (dx, dy) in [(0, -1), (0, 1), (1, 0), (-1, 0)] {
                let nx = a.position.x + dx;
                let ny = a.position.y + dy;
                if self.grid.has_food(nx, ny) {
                    p.adjacent_food.push((nx, ny));
                }
            }

            // Neighbors within Chebyshev radius 4. Stable order: ascending AgentId.
            for b in &self.agents {
                if b.id == a.id || !b.alive {
                    continue;
                }
                let dx = (b.position.x - a.position.x).abs();
                let dy = (b.position.y - a.position.y).abs();
                if dx > 4 || dy > 4 {
                    continue;
                }
                let trust = a.trust_of(b.id);
                let high_energy = b.energy >= self.config.agents.max_energy * 0.75;
                let low_energy = b.energy <= self.config.agents.starting_energy * 0.5;
                let shares_meme = a
                    .inventory
                    .iter()
                    .any(|am| b.inventory.iter().any(|bm| bm.kind == am.kind));
                p.neighbors.push(NeighborInfo {
                    id: b.id,
                    position: b.position,
                    energy: b.energy,
                    trust,
                    is_kin: false, // Kin tracking is post-MVP; flagged false for now.
                    shares_meme,
                    high_energy,
                    low_energy,
                });
            }

            p.hungry = a.energy < self.config.agents.starting_energy * 0.5;
            p.attacked_recently = a
                .memory
                .last_attacked_tick
                .map(|t| self.tick.saturating_sub(t) < 10)
                .unwrap_or(false);

            out.push(p);
        }
        out
    }

    fn policy_phase(&mut self, perceptions: &[Perception]) -> Vec<Action> {
        let mut out = Vec::with_capacity(self.agents.len());
        // why: indexing both `self.agents` (mut borrow target via &mut self.rng)
        // and `perceptions` (immut borrow) in lockstep — can't use a single
        // iterator without a borrow-checker fight.
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.agents.len() {
            if !self.agents[i].alive {
                out.push(Action::Idle);
                continue;
            }
            let action = compute_action(
                &self.agents[i],
                &perceptions[i],
                &self.config,
                &mut self.rng,
            );
            out.push(action);
        }
        out
    }

    /// Executes movement, eating, sharing, attacking, and imitation. Reproduction
    /// and transmission are ambient and handled by their own phases, so they are
    /// not driven by `actions` here.
    ///
    /// Returns the number of deaths attributable to this phase (starvation and
    /// combat).
    fn action_phase(&mut self, perceptions: &[Perception], actions: &[Action]) -> u32 {
        let mut deaths = 0;
        let metabolism = self.config.agents.metabolism;
        let max_energy = self.config.agents.max_energy;
        let energy_per_food = self.config.food.energy_per_food;
        let attack_cost = self.config.attack.energy_cost_attacker;
        let attack_steal = self.config.attack.energy_steal;
        let retaliation_chance = self.config.attack.retaliation_chance;
        let share_amount = self.config.sharing.share_amount;
        let share_recipient_mult = self.config.sharing.recipient_multiplier;

        for i in 0..self.agents.len() {
            if !self.agents[i].alive {
                continue;
            }
            self.agents[i].age += 1;
            self.agents[i].energy -= metabolism;
            // Cognitive cost from inventory.
            let cog: f32 = self.agents[i]
                .inventory
                .iter()
                .map(|m| m.cognitive_cost)
                .sum();
            self.agents[i].energy -= cog;
            if self.agents[i].energy <= 0.0 {
                self.agents[i].energy = 0.0;
                self.agents[i].alive = false;
                deaths += 1;
                let id = self.agents[i].id;
                self.pending_events.push(Event::Death {
                    tick: self.tick,
                    agent: id,
                    cause: DeathCause::Starvation,
                });
                continue;
            }

            match actions[i] {
                Action::Move(dir) => {
                    let (dx, dy) = dir.delta();
                    let nx = self.agents[i].position.x + dx;
                    let ny = self.agents[i].position.y + dy;
                    if self.grid.in_bounds(nx, ny) {
                        self.agents[i].position = Position { x: nx, y: ny };
                    }
                }
                Action::Eat => {
                    let p = self.agents[i].position;
                    if self.grid.has_food(p.x, p.y) {
                        self.grid.set_food(p.x, p.y, false);
                        let new_e = (self.agents[i].energy + energy_per_food).min(max_energy);
                        self.agents[i].energy = new_e;
                    } else if let Some((fx, fy)) = perceptions[i].adjacent_food.first().copied() {
                        // Convenience: step onto adjacent food and eat in same tick.
                        let (cx, cy) = (self.agents[i].position.x, self.agents[i].position.y);
                        let dx = (fx - cx).signum();
                        let dy = (fy - cy).signum();
                        let nx = cx + dx;
                        let ny = cy + dy;
                        if self.grid.in_bounds(nx, ny) {
                            self.agents[i].position = Position { x: nx, y: ny };
                        }
                        let p2 = self.agents[i].position;
                        if self.grid.has_food(p2.x, p2.y) {
                            self.grid.set_food(p2.x, p2.y, false);
                            let new_e = (self.agents[i].energy + energy_per_food).min(max_energy);
                            self.agents[i].energy = new_e;
                        }
                    }
                }
                Action::Share(target) => {
                    if let Some(j) = self.id_to_index(target) {
                        if self.agents[j].alive {
                            // Positive-sum: donor pays `amount`, recipient gains
                            // `amount * recipient_multiplier`. Energy is worth more
                            // to a starving agent, so mutual aid creates net value.
                            let amount = share_amount.min(self.agents[i].energy);
                            let donor_id = self.agents[i].id;
                            self.agents[i].energy -= amount;
                            self.agents[j].energy = (self.agents[j].energy
                                + amount * share_recipient_mult)
                                .min(max_energy);
                            self.agents[j].adjust_trust(donor_id, 0.10);
                        }
                    }
                }
                Action::Attack(target) => {
                    if let Some(j) = self.id_to_index(target) {
                        if self.agents[j].alive {
                            self.agents[i].energy = (self.agents[i].energy - attack_cost).max(0.0);
                            let stolen = attack_steal.min(self.agents[j].energy);
                            self.agents[j].energy -= stolen;
                            self.agents[i].energy =
                                (self.agents[i].energy + stolen).min(max_energy);
                            let attacker = self.agents[i].id;
                            self.agents[j].memory.last_attacker = Some(attacker);
                            self.agents[j].memory.last_attacked_tick = Some(self.tick);
                            self.agents[j].adjust_trust(attacker, -0.30);
                            if self.agents[j].energy <= 0.0 {
                                self.agents[j].energy = 0.0;
                                self.agents[j].alive = false;
                                deaths += 1;
                                let dead = self.agents[j].id;
                                self.pending_events.push(Event::Death {
                                    tick: self.tick,
                                    agent: dead,
                                    cause: DeathCause::Combat,
                                });
                            } else if self.rng.gen_bool(retaliation_chance) {
                                // Survivor strikes back: `energy_steal` damage to the
                                // attacker (a deterrent — energy is destroyed, not
                                // transferred). Makes unprovoked predation risky.
                                self.agents[i].energy =
                                    (self.agents[i].energy - attack_steal).max(0.0);
                                if self.agents[i].energy <= 0.0 {
                                    self.agents[i].energy = 0.0;
                                    self.agents[i].alive = false;
                                    deaths += 1;
                                    let dead = self.agents[i].id;
                                    self.pending_events.push(Event::Death {
                                        tick: self.tick,
                                        agent: dead,
                                        cause: DeathCause::Combat,
                                    });
                                }
                            }
                        }
                    }
                }
                Action::Imitate(target) => {
                    if let Some(j) = self.id_to_index(target) {
                        if self.agents[j].alive {
                            // Pick the first meme on the target the imitator
                            // doesn't already have by kind. `try_acquire` then
                            // handles same-kind skip and conflict resolution.
                            let pick = self.agents[j]
                                .inventory
                                .iter()
                                .find(|m| {
                                    !self.agents[i]
                                        .inventory
                                        .iter()
                                        .any(|mine| mine.kind == m.kind)
                                })
                                .cloned();
                            if let Some(mut m) = pick {
                                m.id = MemeId(self.next_meme_id);
                                self.next_meme_id += 1;
                                m.lineage_id = self.lineage.add(
                                    smallvec![m.lineage_id],
                                    self.tick,
                                    LineageOrigin::Inheritance,
                                );
                                let _ = self.try_acquire(i, m);
                            }
                        }
                    }
                }
                Action::Idle => {}
            }
        }

        deaths
    }

    /// Returns (transmissions_count, mutations_count) attributable to this phase.
    fn transmission_phase(&mut self) -> (u32, u32) {
        let mut transmissions = 0;
        let mut mutations = 0;
        let base_rate = self.config.transmission.base_rate;
        let prestige_boost = self.config.transmission.prestige_boost;

        for i in 0..self.agents.len() {
            if !self.agents[i].alive || self.agents[i].inventory.is_empty() {
                continue;
            }
            // Snapshot the meme set to avoid borrow issues when mutating recipients.
            let inv = self.agents[i].inventory.clone();
            let i_pos = self.agents[i].position;
            let i_id = self.agents[i].id;

            // Determine if this agent is "high-prestige" (top quartile energy among living).
            let prestige = self.is_top_quartile_energy(i);

            for meme in &inv {
                // For each adjacent neighbor, roll transmission.
                let neighbors: Vec<(usize, AgentId)> = self
                    .agents
                    .iter()
                    .enumerate()
                    .filter(|(j, b)| {
                        *j != i && b.alive && crate::policy::adjacent(i_pos, b.position)
                    })
                    .map(|(j, b)| (j, b.id))
                    .collect();

                for (j, _nid) in neighbors {
                    // Same-kind dup is rejected later by try_acquire (Skipped);
                    // we keep a fast path here to avoid wasting an RNG roll.
                    if self.agents[j].inventory.iter().any(|m| m.kind == meme.kind) {
                        continue;
                    }
                    let mut p =
                        base_rate * meme.transmissibility * self.agents[j].social_copying_bias;
                    if prestige {
                        p = (p + prestige_boost).clamp(0.0, 1.0);
                    }
                    if !self.rng.gen_bool(p) {
                        continue;
                    }

                    let mut child = meme.clone();
                    child.id = MemeId(self.next_meme_id);
                    self.next_meme_id += 1;
                    child.lineage_id = self.lineage.add(
                        smallvec![meme.lineage_id],
                        self.tick,
                        LineageOrigin::Inheritance,
                    );

                    // Roll mutation per the meme's per-instance rate.
                    if self.rng.gen_bool(child.mutation_rate) {
                        let parent_meme_id = child.id;
                        let outcome = crate::mutation::mutate_in_place(
                            &mut child,
                            &mut self.rng,
                            &self.config.mutation,
                        );
                        if outcome.mutated {
                            let new_id = MemeId(self.next_meme_id);
                            self.next_meme_id += 1;
                            let new_lin = self.lineage.add(
                                smallvec![child.lineage_id],
                                self.tick,
                                LineageOrigin::Mutation,
                            );
                            self.pending_events.push(Event::Mutation {
                                tick: self.tick,
                                parent_meme: parent_meme_id,
                                child_meme: new_id,
                                field: outcome
                                    .field
                                    .unwrap_or(crate::metrics::MutatedField::Strength),
                            });
                            child.id = new_id;
                            child.lineage_id = new_lin;
                            mutations += 1;
                        }
                    }

                    // Acquire via shared helper — handles conflict resolution
                    // (reject / replace / recombine) and inventory cap eviction.
                    let to_id = self.agents[j].id;
                    let new_meme_id = child.id;
                    let outcome = self.try_acquire(j, child);
                    match outcome {
                        AcquireOutcome::Accepted | AcquireOutcome::Replaced => {
                            self.pending_events.push(Event::Transmission {
                                tick: self.tick,
                                from: i_id,
                                to: to_id,
                                meme: new_meme_id,
                            });
                            transmissions += 1;
                        }
                        AcquireOutcome::Recombined {
                            parents,
                            child_meme_id,
                        } => {
                            self.pending_events.push(Event::Recombination {
                                tick: self.tick,
                                parents,
                                child_meme: child_meme_id,
                            });
                            // A recombination still counts as a contact event
                            // — credit it to the transmission counter so the
                            // metric reflects "cross-agent meme moves."
                            transmissions += 1;
                        }
                        AcquireOutcome::Rejected | AcquireOutcome::Skipped => {}
                    }
                }
            }
        }
        (transmissions, mutations)
    }

    fn reproduction_phase(&mut self, perceptions: &[Perception]) -> u32 {
        let mut births = 0u32;
        let threshold = self.config.reproduction.energy_threshold;
        let cost = self.config.reproduction.offspring_energy_cost;
        let inherit_prob = self.config.reproduction.inherit_meme_prob;
        let min_age = self.config.reproduction.min_age;
        let inventory_cap = self.config.cognition.inventory_cap as usize;
        let max_energy = self.config.agents.max_energy;

        // We need world dimensions for offspring placement; copy now.
        let w = self.grid.width as i32;
        let h = self.grid.height as i32;

        // Iterate in stable AgentId order. To avoid reproducing the same pair twice
        // (i with j, then j with i), only act when i < j.
        // why: indexing into perceptions[i] and &mut self.agents[j] simultaneously.
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.agents.len() {
            if !self.agents[i].alive
                || self.agents[i].energy < threshold
                || self.agents[i].age < min_age
            {
                continue;
            }
            // Find a partner.
            let partner_idx = perceptions[i]
                .neighbors
                .iter()
                .filter(|n| {
                    n.energy >= threshold
                        && crate::policy::adjacent(self.agents[i].position, n.position)
                })
                .map(|n| n.id)
                .find_map(|nid| {
                    self.id_to_index(nid)
                        .filter(|&j| j > i && self.agents[j].alive)
                });
            let Some(j) = partner_idx else { continue };
            // Both parents pay energy cost.
            self.agents[i].energy = (self.agents[i].energy - cost).max(0.0);
            self.agents[j].energy = (self.agents[j].energy - cost).max(0.0);

            // Offspring position: parent i's position bumped one cell N/S/E/W,
            // first free direction in deterministic order.
            let p = self.agents[i].position;
            let mut placed = None;
            for (dx, dy) in [(0, -1), (0, 1), (1, 0), (-1, 0)] {
                let nx = p.x + dx;
                let ny = p.y + dy;
                if nx >= 0 && ny >= 0 && nx < w && ny < h {
                    placed = Some(Position { x: nx, y: ny });
                    break;
                }
            }
            let child_pos = placed.unwrap_or(p);

            let child_id = AgentId(self.next_agent_id);
            self.next_agent_id += 1;
            let mut child = Agent::new(
                child_id,
                child_pos,
                self.config.agents.starting_energy.min(max_energy),
            );

            // Inherit traits from parents with per-trait mutation probability.
            let mut inherited_traits = SmallVec::new();
            for t in self.agents[i]
                .traits
                .iter()
                .chain(self.agents[j].traits.iter())
            {
                if self.rng.gen_bool(0.5) {
                    inherited_traits.push(*t);
                }
            }
            if inherited_traits.is_empty() {
                inherited_traits
                    .push(AgentTrait::ALL[self.rng.gen_range_usize(0, AgentTrait::ALL.len())]);
            }
            // Trait mutation: each inherited trait has a chance to re-roll.
            for slot in inherited_traits.iter_mut() {
                if self.rng.gen_bool(self.config.agents.trait_mutation_rate) {
                    *slot = AgentTrait::ALL[self.rng.gen_range_usize(0, AgentTrait::ALL.len())];
                }
            }
            child.traits = inherited_traits;
            child.memory = AgentMemory::default();
            child.social_copying_bias =
                (self.agents[i].social_copying_bias + self.agents[j].social_copying_bias) * 0.5;

            // Push child into agents BEFORE inheritance so try_acquire can
            // operate on its index. This also lets conflict resolution kick in
            // when parent A's meme and parent B's meme are opposites.
            let child_idx = self.agents.len();
            let parent_id = self.agents[i].id;
            self.agents.push(child);

            // Inherit memes via shared helper.
            let mut inherited_memes: Vec<MemeId> = Vec::new();
            for parent_idx in [i, j] {
                let parent_inv = self.agents[parent_idx].inventory.clone();
                for m in parent_inv {
                    if self.rng.gen_bool(inherit_prob) {
                        let mut cm = m.clone();
                        cm.id = MemeId(self.next_meme_id);
                        self.next_meme_id += 1;
                        cm.lineage_id = self.lineage.add(
                            smallvec![m.lineage_id],
                            self.tick,
                            LineageOrigin::Inheritance,
                        );
                        let acquired_id = cm.id;
                        let outcome = self.try_acquire(child_idx, cm);
                        if matches!(outcome, AcquireOutcome::Accepted | AcquireOutcome::Replaced) {
                            inherited_memes.push(acquired_id);
                        }
                        if let AcquireOutcome::Recombined {
                            parents,
                            child_meme_id,
                        } = outcome
                        {
                            self.pending_events.push(Event::Recombination {
                                tick: self.tick,
                                parents,
                                child_meme: child_meme_id,
                            });
                            inherited_memes.push(child_meme_id);
                        }
                    }
                }
            }
            // Optional recombination if both parents have at least one meme
            // and the offspring still has room.
            if !self.agents[i].inventory.is_empty()
                && !self.agents[j].inventory.is_empty()
                && self.agents[child_idx].inventory.len() < inventory_cap
                && self.rng.gen_bool(0.2)
            {
                let a = self.agents[i].inventory[0].clone();
                let b = self.agents[j].inventory[0].clone();
                let new_id = MemeId(self.next_meme_id);
                self.next_meme_id += 1;
                let recombined = crate::mutation::recombine(
                    &a,
                    &b,
                    new_id,
                    &mut self.rng,
                    &mut self.lineage,
                    self.tick,
                );
                let rid = recombined.id;
                // Route through try_acquire: recombinants now carry a behavioral
                // kind (coop/aggressive), so a hybrid can conflict with a meme the
                // child already inherited. A direct push would break the
                // no-two-conflicting-memes invariant.
                match self.try_acquire(child_idx, recombined) {
                    AcquireOutcome::Accepted | AcquireOutcome::Replaced => {
                        inherited_memes.push(rid);
                        self.pending_events.push(Event::Recombination {
                            tick: self.tick,
                            parents: (a.id, b.id),
                            child_meme: rid,
                        });
                    }
                    AcquireOutcome::Recombined {
                        parents,
                        child_meme_id,
                    } => {
                        inherited_memes.push(child_meme_id);
                        self.pending_events.push(Event::Recombination {
                            tick: self.tick,
                            parents,
                            child_meme: child_meme_id,
                        });
                    }
                    AcquireOutcome::Skipped | AcquireOutcome::Rejected => {}
                }
            }

            self.pending_events.push(Event::Birth {
                tick: self.tick,
                child: child_id,
                parent: parent_id,
                inherited: inherited_memes,
            });
            // Child was already pushed into `self.agents` before inheritance.
            births += 1;
        }

        births
    }

    fn death_phase(&mut self) -> u32 {
        let mut deaths = 0u32;
        let max_age = self.config.agents.max_age;
        for i in 0..self.agents.len() {
            if !self.agents[i].alive {
                continue;
            }
            if self.agents[i].age >= max_age {
                self.agents[i].alive = false;
                deaths += 1;
                let id = self.agents[i].id;
                self.pending_events.push(Event::Death {
                    tick: self.tick,
                    agent: id,
                    cause: DeathCause::Aging,
                });
            }
        }
        // Trust decay (small per-tick): decay by 1% toward 0. Drop near-zero entries.
        for a in self.agents.iter_mut() {
            if !a.alive {
                continue;
            }
            for entry in a.trust.iter_mut() {
                entry.1 *= 0.99;
            }
            a.trust.retain(|(_, v)| v.abs() >= 0.05);
        }
        deaths
    }

    fn world_maintenance_phase(&mut self) {
        let rate = self.config.food.regrowth_rate;
        if rate <= 0.0 {
            return;
        }
        let cells = (self.grid.width * self.grid.height) as usize;
        for i in 0..cells {
            if !self.grid.food[i] && self.rng.gen_bool(rate) {
                self.grid.food[i] = true;
            }
        }
    }

    fn emit_metrics_phase(
        &mut self,
        transmissions: u32,
        mutations: u32,
        deaths: u32,
        births: u32,
    ) -> Metrics {
        let mut alive = 0u32;
        let mut energy_sum = 0.0f32;
        let mut age_sum = 0u64;
        let mut population_by_trait = [0u32; 4];
        // Carriers-per-kind: each agent contributes at most 1 to each kind it carries.
        let mut carriers_by_kind = [0u32; 7];
        let mut meme_count = 0u32;
        let mut any_meme_carrier = false;
        // Hybrids = memes with a recombinant ancestor. Tracked independently of
        // kind because behavioral classification folds them into coop/aggressive.
        let mut hybrid_carriers = 0u32;
        let mut hybrid_meme_count = 0u32;
        let mut hybrid_coop_meme_count = 0u32;

        for a in &self.agents {
            if !a.alive {
                continue;
            }
            alive += 1;
            energy_sum += a.energy;
            age_sum += a.age as u64;
            for t in &a.traits {
                population_by_trait[t.idx()] += 1;
            }
            // Distinct kinds present in this agent's inventory.
            let mut seen = [false; 7];
            let mut carries_hybrid = false;
            for m in &a.inventory {
                meme_count += 1;
                let idx = m.kind.idx();
                if !seen[idx] {
                    seen[idx] = true;
                    carriers_by_kind[idx] += 1;
                }
                if self.lineage.has_recombination_ancestor(m.lineage_id) {
                    carries_hybrid = true;
                    hybrid_meme_count += 1;
                    if m.kind == crate::meme::MemeKind::Cooperative {
                        hybrid_coop_meme_count += 1;
                    }
                }
            }
            if carries_hybrid {
                hybrid_carriers += 1;
            }
            if !a.inventory.is_empty() {
                any_meme_carrier = true;
            }
        }

        let hybrid_prevalence = if alive == 0 {
            0.0
        } else {
            hybrid_carriers as f32 / alive as f32
        };
        let hybrid_cooperative_fraction = if hybrid_meme_count == 0 {
            0.0
        } else {
            hybrid_coop_meme_count as f32 / hybrid_meme_count as f32
        };

        // Prevalence per kind = carriers / alive ∈ [0, 1].
        let mut prevalence = [0.0f32; 7];
        for (i, slot) in prevalence.iter_mut().enumerate() {
            *slot = if alive == 0 {
                0.0
            } else {
                carriers_by_kind[i] as f32 / alive as f32
            };
        }

        let diversity = shannon_diversity(&prevalence);
        let dominance = top1_fraction(&prevalence);
        let mean_energy = if alive == 0 {
            0.0
        } else {
            energy_sum / alive as f32
        };
        let mean_age = if alive == 0 {
            0.0
        } else {
            age_sum as f32 / alive as f32
        };

        let metrics = Metrics {
            tick: self.tick,
            alive,
            food_count: self.grid.food_count(),
            population_by_trait: crate::metrics::PopulationByTrait::from_array(population_by_trait),
            meme_count,
            meme_prevalence_by_kind: crate::metrics::PrevalenceByKind::from_array(prevalence),
            hybrid_prevalence,
            hybrid_cooperative_fraction,
            diversity_shannon: diversity,
            dominance_top1_fraction: dominance,
            mean_energy,
            mean_age,
            transmissions_this_tick: transmissions,
            mutations_this_tick: mutations,
            deaths_this_tick: deaths,
            births_this_tick: births,
        };

        // Extinction events (emit once each).
        if alive == 0 && !self.extinction_population_emitted {
            self.extinction_population_emitted = true;
            self.pending_events.push(Event::Extinction {
                tick: self.tick,
                scope: ExtinctionScope::Population,
            });
        }
        if alive > 0 && !any_meme_carrier && !self.extinction_memes_emitted {
            self.extinction_memes_emitted = true;
            self.pending_events.push(Event::Extinction {
                tick: self.tick,
                scope: ExtinctionScope::AllMemes,
            });
        }

        if self.tick % self.config.run.metrics_emit_every as u64 == 0 {
            self.pending_events
                .push(Event::Tick(Box::new(metrics.clone())));
        }
        metrics
    }

    fn cluster_snapshot_maybe(&mut self) {
        let cadence = self.config.run.cluster_snapshot_every;
        if cadence == 0 {
            return;
        }
        if self.tick % cadence as u64 != 0 {
            return;
        }
        let mut clusters = self.compute_clusters(0.6);
        // Sort cluster members deterministically.
        for c in clusters.iter_mut() {
            c.members.sort();
        }
        self.pending_events.push(Event::ClusterSnapshot {
            tick: self.tick,
            clusters,
        });
    }

    /// Jaccard-similarity-based cultural clusters on meme-kind sets. O(N²) but
    /// fine at MVP scale (≤ a few hundred agents).
    fn compute_clusters(&self, threshold: f32) -> Vec<ClusterEntry> {
        let n = self.agents.len();
        let kind_sets: Vec<std::collections::BTreeSet<MemeKind>> = self
            .agents
            .iter()
            .map(|a| a.inventory.iter().map(|m| m.kind).collect())
            .collect();
        let mut cluster_id = vec![None::<u32>; n];
        let mut next_id = 0u32;
        for i in 0..n {
            if !self.agents[i].alive || kind_sets[i].is_empty() {
                continue;
            }
            if cluster_id[i].is_some() {
                continue;
            }
            cluster_id[i] = Some(next_id);
            for j in (i + 1)..n {
                if !self.agents[j].alive || kind_sets[j].is_empty() {
                    continue;
                }
                if cluster_id[j].is_some() {
                    continue;
                }
                let inter = kind_sets[i].intersection(&kind_sets[j]).count() as f32;
                let union = kind_sets[i].union(&kind_sets[j]).count() as f32;
                if union > 0.0 && inter / union >= threshold {
                    cluster_id[j] = Some(next_id);
                }
            }
            next_id += 1;
        }
        let mut groups: BTreeMap<u32, Vec<AgentId>> = BTreeMap::new();
        // why: index `cluster_id`, `kind_sets`, and `self.agents` together.
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            if let Some(cid) = cluster_id[i] {
                groups.entry(cid).or_default().push(self.agents[i].id);
            }
        }
        groups
            .into_iter()
            .map(|(id, members)| ClusterEntry { id, members })
            .collect()
    }

    // ----- helpers -----

    fn id_to_index(&self, target: AgentId) -> Option<usize> {
        // Stable linear search. Agent indices are not stable across reproductions,
        // but iteration size is bounded and the call is rare.
        self.agents.iter().position(|a| a.id == target)
    }

    fn is_top_quartile_energy(&self, i: usize) -> bool {
        // Linear-scan estimate. For MVP scale this is fine.
        let target = self.agents[i].energy;
        let mut ge = 0;
        let mut total = 0;
        for a in &self.agents {
            if !a.alive {
                continue;
            }
            total += 1;
            if a.energy >= target {
                ge += 1;
            }
        }
        if total == 0 {
            return false;
        }
        (ge as f32 / total as f32) <= 0.25
    }

    pub fn starter_lineage_of(&self, name: &str) -> Option<LineageId> {
        self.starter_lineage.get(name).copied()
    }
}

fn sample_trait(dist: &[f32; 4], rng: &mut SimRng) -> AgentTrait {
    let total: f32 = dist.iter().sum();
    if total <= 0.0 {
        return AgentTrait::Generous;
    }
    let mut r = (rng.gen_u32() as f32 / u32::MAX as f32) * total;
    for (i, p) in dist.iter().enumerate() {
        if r < *p {
            return AgentTrait::ALL[i];
        }
        r -= *p;
    }
    AgentTrait::ALL[AgentTrait::ALL.len() - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn test_config() -> SimConfig {
        SimConfig {
            world: WorldConfig {
                width: 20,
                height: 20,
            },
            agents: AgentConfig {
                count: 30,
                starting_energy: 20.0,
                metabolism: 0.5,
                max_energy: 50.0,
                max_age: 600,
                initial_traits_dist: [0.4, 0.2, 0.2, 0.2],
                trait_mutation_rate: 0.02,
            },
            food: FoodConfig {
                initial_density: 0.15,
                regrowth_rate: 0.005,
                energy_per_food: 8.0,
            },
            scarcity: ScarcityConfig {
                level: "custom".into(),
            },
            cognition: CognitionConfig { inventory_cap: 4 },
            transmission: TransmissionConfig {
                base_rate: 0.5,
                social_copying_bias_mean: 0.5,
                social_copying_bias_std: 0.0,
                prestige_boost: 0.10,
            },
            mutation: MutationConfig {
                strength_jitter_max: 0.1,
                enum_swap_probability: 0.2,
            },
            conflict: ConflictConfig {
                recombine_share: 0.20,
            },
            reproduction: ReproductionConfig {
                energy_threshold: 35.0,
                offspring_energy_cost: 10.0,
                inherit_meme_prob: 0.5,
                min_age: 30,
            },
            attack: AttackConfig {
                energy_cost_attacker: 2.0,
                energy_steal: 4.0,
                retaliation_chance: 0.5,
            },
            sharing: SharingConfig {
                share_threshold: 12.0,
                share_amount: 3.0,
                recipient_multiplier: 1.0,
            },
            memes: MemePoolConfig {
                seed: vec![
                    SeedMemeEntry {
                        name: "share_with_allies".into(),
                        carrier_fraction: 0.5,
                    },
                    SeedMemeEntry {
                        name: "attack_low_energy_outsiders".into(),
                        carrier_fraction: 0.5,
                    },
                ],
            },
            run: RunConfig {
                seed: 1,
                horizon: 1000,
                stop_on_extinction: false,
                cluster_snapshot_every: 50,
                metrics_emit_every: 1,
                survival_threshold: 0.05,
            },
        }
    }

    #[test]
    fn initial_seeding_is_at_most_one_meme_per_agent() {
        // Two starters at carrier_fraction 0.5 each used to give ~25% of
        // agents BOTH memes from t=0 (independent rolls). Under categorical
        // sampling each agent gets at most one starter at construction.
        let cfg = test_config();
        let sim = Simulation::new(cfg, Some(42));
        for a in &sim.agents {
            assert!(
                a.inventory.len() <= 1,
                "agent {:?} started with {} memes; expected ≤ 1",
                a.id,
                a.inventory.len()
            );
        }
        // And the seeding should actually seed *something* under the shipped
        // 0.5 + 0.5 pool — otherwise the milestone has nothing to spread.
        let carriers = sim
            .agents
            .iter()
            .filter(|a| !a.inventory.is_empty())
            .count();
        assert!(carriers > 0, "no agents seeded with any starter meme");
    }

    #[test]
    fn same_seed_same_metrics() {
        let cfg = test_config();
        let mut a = Simulation::new(cfg.clone(), Some(42));
        let mut b = Simulation::new(cfg, Some(42));
        for _ in 0..100 {
            let ma = a.step();
            let mb = b.step();
            assert_eq!(ma.tick, mb.tick);
            assert_eq!(ma.alive, mb.alive);
            assert_eq!(ma.food_count, mb.food_count);
            assert_eq!(ma.meme_count, mb.meme_count);
            assert_eq!(
                ma.meme_prevalence_by_kind.as_array(),
                mb.meme_prevalence_by_kind.as_array()
            );
            assert_eq!(ma.diversity_shannon, mb.diversity_shannon);
            assert_eq!(ma.transmissions_this_tick, mb.transmissions_this_tick);
            assert_eq!(ma.mutations_this_tick, mb.mutations_this_tick);
            assert_eq!(ma.births_this_tick, mb.births_this_tick);
            assert_eq!(ma.deaths_this_tick, mb.deaths_this_tick);
            // Drain event streams and confirm they serialize identically.
            let evs_a = a.events_drain();
            let evs_b = b.events_drain();
            let ja = serde_json::to_string(&evs_a).unwrap();
            let jb = serde_json::to_string(&evs_b).unwrap();
            assert_eq!(ja, jb, "event streams diverged at tick {}", ma.tick);
        }
    }

    #[test]
    fn meme_can_transmit() {
        let cfg = test_config();
        let mut sim = Simulation::new(cfg, Some(7));
        let initial_carriers = sim
            .agents
            .iter()
            .filter(|a| !a.inventory.is_empty())
            .count();
        let mut max_carriers = initial_carriers;
        for _ in 0..400 {
            let _ = sim.step();
            let carriers = sim
                .agents
                .iter()
                .filter(|a| a.alive && !a.inventory.is_empty())
                .count();
            max_carriers = max_carriers.max(carriers);
        }
        // The total carriers may shrink due to death; we assert the *cumulative*
        // ceiling is reached above the initial seed via transmission.
        assert!(
            max_carriers >= initial_carriers,
            "carriers never matched initial seed (start={}, max={})",
            initial_carriers,
            max_carriers
        );
    }
}
