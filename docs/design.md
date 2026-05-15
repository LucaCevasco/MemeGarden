# Meme Garden — design notes

> Status: kickoff doc. Reproduces the project brief verbatim as a north star. The code under `crates/` is **not** a literal implementation of this — it's a deliberately tiny POC that proves the meme-transmission pipeline. Use this doc to decide what to add next, not as a spec.

## Frame

A controlled "memetic petri dish," not a full artificial society at first: tiny agents, local interactions, bounded meme formats, and strong metrics around spread, mutation, and fitness.

A controlled artificial society where ideas behave like replicators: they spread, mutate, compete, combine, and affect host behavior. The key is to make memes real simulation objects, not just flavor text.

## Core concept

Each agent lives in a grid/world and has:

```
Agent {
    energy, age, position, traits, memory,
    trust_map, meme_inventory, current_policy,
}
```

Each meme is something like:

```
Meme {
    id, lineage_id, kind, trigger, behavioral_effect,
    transmissibility, mutation_rate, cognitive_cost, prestige_bonus,
}
```

So a meme is not merely "share food" — it is:

> *When near an allied agent with low energy: increase probability of sharing food by 35%.*

or:

> *When seeing an unknown agent: decrease trust by 20%, increase probability of avoiding them.*

This lets memes have actual consequences. The simulation tracks whether memes survive because they help the agent, help the group, spread aggressively, exploit biases, or simply hitchhike with successful agents.

## Three layers

1. **Biological layer** — agents need to survive: consume food, move, reproduce, die, fight, cooperate.
2. **Social layer** — agents interact locally: observe, copy, trust, reject, punish, imitate high-status agents.
3. **Memetic layer** — ideas spread between agents. Memes affect behavior but also have their own replication logic. Some help the host. Some help the group. Some hurt the host but spread well — that last category is the most interesting.

## Memetic evolution

Memes evolve through **replication, mutation, recombination, and selection**. Examples:

- "Share with allies" → "Share only with kin" (mutation).
- "Follow high-energy agents" + "Never trust outsiders" → "Follow high-energy agents from your own group" (recombination).

Selection pressure comes from host survival, host reproduction, group survival, transmission frequency, prestige of carrier, conformity bias, mutation rate, and meme-meme compatibility. The most successful meme is not necessarily the "true" or "useful" one — it may simply be the one that spreads best.

## Avoid natural language at first

Free-form LLM beliefs would make the sim hard to measure, expensive, nondeterministic, and impossible to debug. Instead, a small **symbolic meme grammar**:

```
Trigger:  Hungry | NearFood | NearAlly | NearStranger | AttackedRecently | SawAgentGainEnergy
Target:   Self | Kin | Ally | Stranger | HighEnergyAgent | LowEnergyAgent
Effect:   MoveToward | MoveAway | Share | Attack | Imitate | RefuseInteraction
          | TransmitMeme | IncreaseTrust | DecreaseTrust
```

A meme is a tiny behavioral rule: `IF near_low_energy_ally THEN share, strength=0.7`.

Later an LLM can generate human-readable names: internally `{ trigger: NearStranger, effect: Avoid, strength: 0.4 }`, displayed as "Outsider Caution."

## Experiment ideas (ranked by feasibility)

1. **Cooperation vs selfishness** — best first experiment. Memes: `share_with_allies`, `hoard_food`, `punish_non_sharers`, `copy_generous_agents`, `copy_successful_agents`. Questions: does cooperation survive without punishment? do selfish memes dominate in scarcity?
2. **Scarcity and violence** — only interesting if violence has real tradeoffs (cost, injury, retaliation, reputation, group punishment).
3. **Communication** — start with fixed signal types (`danger`, `food_here`, `enemy_near`, etc.), not emergent language. Ask whether truthful signaling can emerge and whether deception spreads.
4. **Specialization** — needs role-specific tradeoffs (foraging vs combat vs teaching) before memes can push agents into niches.
5. **Territoriality** — feasible and visually striking. Memes around marking, defending, avoiding zones; pairs well with cultural clustering.
6. **Civilization collapse** — too broad for v1. Should emerge from simpler mechanics, never be hardcoded.

## MVP scope (what the codebase is growing toward)

A grid world where agents survive, reproduce, interact locally, and transmit symbolic memes that modify behavior. **No LLM agents in the hot loop. No complex language. No civilization model.**

Just: agents, food, energy, movement, reproduction, death, local interaction, meme transmission, meme mutation, metrics, visualization.

Core actions: `move | eat | share | attack | imitate | transmit | reproduce`.
Core meme types: survival, social, trust, aggression, imitation.

Six starter memes that already produce surprising dynamics:

1. Share with allies
2. Avoid strangers
3. Copy high-energy agents
4. Attack low-energy outsiders
5. Punish non-sharers
6. Prefer agents with same meme

## Constraint: memes are not arbitrary code

A meme is a small policy modifier, not a script:

```
struct Meme {
    trigger: Trigger,
    effect: Effect,
    target: TargetSelector,
    strength: f32,
    transmissibility: f32,
    mutation_rate: f32,
}
```

Expressive but bounded — keeps the sim reasonable to debug and to measure.

## Memetic analytics

Track:

- meme prevalence over time
- meme lineage tree
- mutation events
- host fitness per meme
- group fitness per meme cluster
- mean lifespan of meme carriers
- transmission rate
- extinction events
- cultural clusters
- meme diversity / dominance
- recombination events

Visualization endgame: split screen — world grid on the left, live meme phylogenetic tree / prevalence chart on the right. Watch an idea mutate, spread, dominate, fragment, disappear.

## AI

- **Experiment designer** — natural-language → config file.
- **Meme namer** — internal `{trigger, effect, strength}` → human-readable label.
- **Post-run analyst** — metrics history → prose summary ("the punishment meme initially reduced selfish behavior, but later mutated into an exclusionary norm…").

These are the seams stubbed by `core::ai::{MemeNamer, RunAnalyst}` today.

## The first real milestone

> Can a cooperative meme survive against a selfish meme under different levels of scarcity, mutation, and social copying?

That single question is enough to drive the next several iterations.
