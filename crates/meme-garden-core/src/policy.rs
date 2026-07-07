//! Per-tick policy resolution. Translates an agent's perception, traits, and
//! meme inventory into a concrete `Action`. The decision path is deterministic
//! given `(agent_state, perception, rng)`.

use crate::action::{Action, Direction};
use crate::agent::{Agent, AgentId, AgentTrait, Position};
use crate::config::SimConfig;
use crate::meme::{Effect, Trigger};
use crate::rng::SimRng;

/// Snapshot of an agent's neighborhood, computed once per tick in the perception
/// phase and consumed by the policy phase. Read-only.
#[derive(Debug, Clone, Default)]
pub struct Perception {
    pub agent_id: AgentId,
    pub position: Position,
    pub adjacent_food: smallvec::SmallVec<[(i32, i32); 4]>,
    /// All neighbors in 4-cell radius (chebyshev), sorted by `AgentId`.
    pub neighbors: Vec<NeighborInfo>,
    pub hungry: bool,
    pub attacked_recently: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct NeighborInfo {
    pub id: AgentId,
    pub position: Position,
    pub energy: f32,
    /// `trust >= 0.0` is "ally"; the threshold is configurable elsewhere.
    pub trust: f32,
    pub is_kin: bool,
    pub shares_meme: bool,
    pub high_energy: bool,
    pub low_energy: bool,
}

/// Pick an action for `agent` given its current perception. Implements the
/// policy resolution algorithm specified in `data-model.md §State transitions`:
/// start from a uniform-ish default distribution; for each matching meme in
/// inventory order, multiplicatively bias the distribution toward the meme's
/// effect by `strength`.
pub fn compute_action(
    agent: &Agent,
    perception: &Perception,
    cfg: &SimConfig,
    rng: &mut SimRng,
) -> Action {
    // 1) Build a coarse action category distribution. Categories are:
    //    [Move, Eat, Share, Attack, Imitate, Idle]
    // Transmission and reproduction are ambient — they run in their own phases
    // for every eligible agent regardless of the sampled action, so they are not
    // sampled here.
    const N: usize = 6;
    let mut weights = [1.0_f32; N];

    // 2) Trait-based default biases.
    if agent.has_trait(AgentTrait::Generous) {
        weights[2] *= 1.4; // Share
    }
    if agent.has_trait(AgentTrait::Cautious) {
        weights[3] *= 0.6; // Attack
        weights[0] *= 1.2; // Move
    }
    if agent.has_trait(AgentTrait::Aggressive) {
        weights[3] *= 1.6; // Attack
    }
    if agent.has_trait(AgentTrait::Conformist) {
        weights[4] *= 1.3; // Imitate
    }

    // 3) Hunger / food bias.
    if perception.hungry && !perception.adjacent_food.is_empty() {
        weights[1] *= 3.0; // Eat
    } else if !perception.adjacent_food.is_empty() {
        weights[1] *= 1.5;
    }
    if perception.hungry && perception.adjacent_food.is_empty() {
        weights[0] *= 1.5; // Move (forage)
    }

    // 4) Meme-driven biases. Each meme whose trigger matches multiplies the
    //    weight of the action category corresponding to its effect.
    for meme in &agent.inventory {
        if !trigger_matches(meme.trigger, perception) {
            continue;
        }
        let cat = effect_to_category(meme.effect);
        // Multiplicative bias: 1 + strength means a strength=0.7 meme triples
        // its category's weight (1 + 0.7 = 1.7x).
        weights[cat] *= 1.0 + meme.strength.clamp(0.0, 1.0);
    }

    // 5) Disable categories that have no valid target.
    if perception.adjacent_food.is_empty() {
        weights[1] = 0.0; // Eat
    }
    if !any_neighbor_satisfying(&perception.neighbors, perception.position, |n| {
        adjacent(perception.position, n.position) && n.energy < cfg.sharing.share_threshold
    }) {
        weights[2] = 0.0; // Share
    }
    if !any_neighbor_satisfying(&perception.neighbors, perception.position, |n| {
        adjacent(perception.position, n.position) && n.energy < agent.energy
    }) {
        weights[3] = 0.0; // Attack
    }
    if perception.neighbors.is_empty() {
        weights[4] = 0.0; // Imitate
    }

    // 6) Sample a category.
    let total: f32 = weights.iter().sum();
    if total <= 0.0 {
        return Action::Idle;
    }
    let mut r = (rng.gen_u32() as f32 / u32::MAX as f32) * total;
    let mut category = 5; // default Idle
    for (i, w) in weights.iter().enumerate() {
        if r < *w {
            category = i;
            break;
        }
        r -= *w;
    }

    // 7) Translate category to a concrete Action by picking a target where needed.
    match category {
        0 => sample_move(perception, agent, rng),
        1 => Action::Eat,
        2 => match pick_share_target(perception, agent, cfg) {
            Some(t) => Action::Share(t),
            None => Action::Idle,
        },
        3 => match pick_attack_target(perception, agent) {
            Some(t) => Action::Attack(t),
            None => Action::Idle,
        },
        4 => match pick_imitate_target(perception) {
            Some(t) => Action::Imitate(t),
            None => Action::Idle,
        },
        _ => Action::Idle,
    }
}

pub fn trigger_matches(trigger: Trigger, p: &Perception) -> bool {
    match trigger {
        Trigger::Hungry => p.hungry,
        Trigger::NearFood => !p.adjacent_food.is_empty(),
        Trigger::NearAlly => p
            .neighbors
            .iter()
            .any(|n| n.trust >= 0.0 && adjacent(p.position, n.position)),
        Trigger::NearStranger => p
            .neighbors
            .iter()
            .any(|n| n.trust < 0.0 && adjacent(p.position, n.position)),
        Trigger::AttackedRecently => p.attacked_recently,
        Trigger::SawAgentGainEnergy => p.neighbors.iter().any(|n| n.high_energy),
    }
}

fn effect_to_category(effect: Effect) -> usize {
    match effect {
        Effect::MoveToward | Effect::MoveAway => 0,
        Effect::Share | Effect::IncreaseTrust => 2,
        Effect::Attack | Effect::DecreaseTrust => 3,
        Effect::Imitate => 4,
        // TransmitMeme/RefuseInteraction have no sampled action category; they
        // fall through to Idle (category 5).
        Effect::TransmitMeme | Effect::RefuseInteraction => 5,
    }
}

fn sample_move(p: &Perception, agent: &Agent, rng: &mut SimRng) -> Action {
    // Bias toward food if adjacent.
    if !p.adjacent_food.is_empty() {
        let idx = rng.gen_range_usize(0, p.adjacent_food.len());
        let (fx, fy) = p.adjacent_food[idx];
        let dx = fx - p.position.x;
        let dy = fy - p.position.y;
        let dir = match (dx.signum(), dy.signum()) {
            (1, _) => Direction::East,
            (-1, _) => Direction::West,
            (_, 1) => Direction::South,
            (_, -1) => Direction::North,
            _ => Direction::North,
        };
        return Action::Move(dir);
    }
    // Otherwise: if there's a cautious-triggering meme and a stranger nearby,
    // move away; otherwise uniform random direction.
    let stranger_dir = p
        .neighbors
        .iter()
        .find(|n| n.trust < 0.0 && adjacent(p.position, n.position))
        .map(|n| (n.position.x - p.position.x, n.position.y - p.position.y));
    if let Some((sx, sy)) = stranger_dir {
        // Move away.
        let dir = match (sx.signum(), sy.signum()) {
            (1, _) => Direction::West,
            (-1, _) => Direction::East,
            (_, 1) => Direction::North,
            (_, -1) => Direction::South,
            _ => Direction::North,
        };
        return Action::Move(dir);
    }
    let idx = rng.gen_range_usize(0, Direction::ALL.len());
    let _ = agent; // silence unused warning when policy needs no extra agent state
    Action::Move(Direction::ALL[idx])
}

fn pick_share_target(p: &Perception, agent: &Agent, cfg: &SimConfig) -> Option<AgentId> {
    if agent.energy <= cfg.sharing.share_threshold {
        return None;
    }
    p.neighbors
        .iter()
        .find(|n| adjacent(p.position, n.position) && n.energy < cfg.sharing.share_threshold)
        .map(|n| n.id)
}

fn pick_attack_target(p: &Perception, agent: &Agent) -> Option<AgentId> {
    p.neighbors
        .iter()
        .find(|n| adjacent(p.position, n.position) && n.energy < agent.energy)
        .map(|n| n.id)
}

fn pick_imitate_target(p: &Perception) -> Option<AgentId> {
    p.neighbors
        .iter()
        .max_by(|a, b| {
            a.energy
                .partial_cmp(&b.energy)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|n| n.id)
}

fn any_neighbor_satisfying(
    neighbors: &[NeighborInfo],
    _pos: Position,
    pred: impl Fn(&NeighborInfo) -> bool,
) -> bool {
    neighbors.iter().any(pred)
}

/// Manhattan-adjacent (4-neighborhood).
pub fn adjacent(a: Position, b: Position) -> bool {
    let dx = (a.x - b.x).abs();
    let dy = (a.y - b.y).abs();
    dx + dy == 1
}
