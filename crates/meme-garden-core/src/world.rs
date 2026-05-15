use crate::agent::{Agent, AgentId, Position};
use crate::config::SimConfig;
use crate::meme::Meme;
use crate::metrics::Metrics;
use crate::rng::SimRng;

const DIRECTIONS: [(i32, i32); 4] = [(0, -1), (0, 1), (-1, 0), (1, 0)];

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

#[derive(Debug)]
pub struct Simulation {
    pub config: SimConfig,
    pub grid: Grid,
    pub agents: Vec<Agent>,
    pub tick: u64,
    rng: SimRng,
}

impl Simulation {
    pub fn new(config: SimConfig, seed_override: Option<u64>) -> Self {
        let seed = seed_override.unwrap_or(config.run.seed);
        let mut rng = SimRng::from_seed(seed);
        let mut grid = Grid::new(config.world.width, config.world.height);

        let cells = (config.world.width * config.world.height) as usize;
        for i in 0..cells {
            if rng.gen_bool(config.food.initial_density) {
                grid.food[i] = true;
            }
        }

        let mut agents = Vec::with_capacity(config.agents.count as usize);
        for i in 0..config.agents.count {
            let x = rng.gen_range_usize(0, config.world.width as usize) as i32;
            let y = rng.gen_range_usize(0, config.world.height as usize) as i32;
            let mut agent = Agent::new(AgentId(i), Position { x, y }, config.agents.starting_energy);
            if rng.gen_bool(config.meme.initial_carrier_fraction) {
                agent.meme = Some(Meme::sharer_norm(config.meme.transmissibility));
            }
            agents.push(agent);
        }

        Self { config, grid, agents, tick: 0, rng }
    }

    /// Advance the simulation by one tick and return a metrics snapshot.
    pub fn step(&mut self) -> Metrics {
        self.move_and_feed_phase();
        self.meme_phase();
        self.regrowth_phase();
        let metrics = self.snapshot();
        self.tick += 1;
        metrics
    }

    fn move_and_feed_phase(&mut self) {
        let metabolism = self.config.agents.metabolism;
        let max_energy = self.config.agents.max_energy;
        let energy_per_food = self.config.food.energy_per_food;

        for i in 0..self.agents.len() {
            if !self.agents[i].alive {
                continue;
            }
            self.agents[i].age += 1;
            self.agents[i].energy -= metabolism;
            if self.agents[i].energy <= 0.0 {
                self.agents[i].alive = false;
                self.agents[i].energy = 0.0;
                continue;
            }

            let pos = self.agents[i].position;

            // Food-seeking bias: if an adjacent cell has food, pick one at random
            // among the food-bearing neighbors. Otherwise pick any direction.
            let mut food_neighbors: [Option<(i32, i32)>; 4] = [None; 4];
            let mut food_n = 0usize;
            for (dx, dy) in DIRECTIONS {
                let nx = pos.x + dx;
                let ny = pos.y + dy;
                if self.grid.in_bounds(nx, ny) && self.grid.has_food(nx, ny) {
                    food_neighbors[food_n] = Some((nx, ny));
                    food_n += 1;
                }
            }

            let (target_x, target_y) = if food_n > 0 {
                let pick = self.rng.gen_range_usize(0, food_n);
                food_neighbors[pick].unwrap()
            } else {
                let pick = self.rng.gen_range_usize(0, DIRECTIONS.len());
                let (dx, dy) = DIRECTIONS[pick];
                (pos.x + dx, pos.y + dy)
            };

            if self.grid.in_bounds(target_x, target_y) {
                self.agents[i].position = Position { x: target_x, y: target_y };
            }

            let p = self.agents[i].position;
            if self.grid.has_food(p.x, p.y) {
                self.grid.set_food(p.x, p.y, false);
                let new_e = (self.agents[i].energy + energy_per_food).min(max_energy);
                self.agents[i].energy = new_e;
            }
        }
    }

    fn meme_phase(&mut self) {
        let share_threshold = self.config.meme.share_threshold;
        let share_amount = self.config.meme.share_amount;
        let max_energy = self.config.agents.max_energy;

        for i in 0..self.agents.len() {
            if !self.agents[i].alive {
                continue;
            }
            let Some(meme) = self.agents[i].meme.clone() else {
                continue;
            };
            if self.agents[i].energy <= share_threshold {
                continue;
            }
            let pos = self.agents[i].position;

            // First adjacent low-energy ally (stable id order) is the recipient.
            let mut chosen: Option<usize> = None;
            for j in 0..self.agents.len() {
                if i == j || !self.agents[j].alive {
                    continue;
                }
                let other_pos = self.agents[j].position;
                if !is_adjacent(pos, other_pos) {
                    continue;
                }
                if self.agents[j].energy >= share_threshold {
                    continue;
                }
                chosen = Some(j);
                break;
            }

            if let Some(j) = chosen {
                let amount = share_amount.min(self.agents[i].energy);
                self.agents[i].energy -= amount;
                self.agents[j].energy = (self.agents[j].energy + amount).min(max_energy);

                if self.agents[j].meme.is_none() && self.rng.gen_bool(meme.transmissibility) {
                    self.agents[j].meme = Some(meme);
                }
            }
        }
    }

    fn regrowth_phase(&mut self) {
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

    fn snapshot(&self) -> Metrics {
        let mut alive = 0u32;
        let mut carriers = 0u32;
        let mut energy_sum = 0.0f32;
        for a in &self.agents {
            if !a.alive {
                continue;
            }
            alive += 1;
            energy_sum += a.energy;
            if a.meme.is_some() {
                carriers += 1;
            }
        }
        let prevalence = if alive == 0 { 0.0 } else { carriers as f32 / alive as f32 };
        let mean_energy = if alive == 0 { 0.0 } else { energy_sum / alive as f32 };
        Metrics {
            tick: self.tick,
            alive,
            food_count: self.grid.food_count(),
            meme_carriers: carriers,
            meme_prevalence: prevalence,
            mean_energy,
        }
    }
}

fn is_adjacent(a: Position, b: Position) -> bool {
    let dx = (a.x - b.x).abs();
    let dy = (a.y - b.y).abs();
    dx + dy == 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::*;

    fn test_config() -> SimConfig {
        SimConfig {
            world: WorldConfig { width: 20, height: 20 },
            agents: AgentConfig {
                count: 30,
                starting_energy: 20.0,
                metabolism: 0.5,
                max_energy: 50.0,
            },
            food: FoodConfig {
                initial_density: 0.15,
                regrowth_rate: 0.005,
                energy_per_food: 8.0,
            },
            meme: MemeConfig {
                initial_carrier_fraction: 0.2,
                transmissibility: 0.5,
                share_threshold: 15.0,
                share_amount: 4.0,
            },
            run: RunConfig { seed: 1 },
        }
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
            assert_eq!(ma.meme_carriers, mb.meme_carriers);
        }
    }

    #[test]
    fn meme_can_transmit() {
        let cfg = test_config();
        let mut sim = Simulation::new(cfg, Some(7));
        let initial_carriers = sim.agents.iter().filter(|a| a.meme.is_some()).count();
        let mut max_carriers = initial_carriers;
        for _ in 0..400 {
            let m = sim.step();
            max_carriers = max_carriers.max(m.meme_carriers as usize);
        }
        assert!(
            max_carriers > initial_carriers,
            "meme never spread beyond initial seed (start={}, max={})",
            initial_carriers,
            max_carriers
        );
    }
}
