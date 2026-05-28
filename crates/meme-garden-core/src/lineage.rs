use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LineageId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageOrigin {
    Starter,
    Mutation,
    Recombination,
    Inheritance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: LineageId,
    pub parents: SmallVec<[LineageId; 2]>,
    pub birth_tick: u64,
    pub origin: LineageOrigin,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LineageGraph {
    nodes: Vec<LineageNode>,
}

impl LineageGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add(
        &mut self,
        parents: SmallVec<[LineageId; 2]>,
        birth_tick: u64,
        origin: LineageOrigin,
    ) -> LineageId {
        let id = LineageId(self.nodes.len() as u32);
        self.nodes.push(LineageNode {
            id,
            parents,
            birth_tick,
            origin,
        });
        id
    }

    pub fn add_starter(&mut self, tick: u64) -> LineageId {
        self.add(SmallVec::new(), tick, LineageOrigin::Starter)
    }

    pub fn get(&self, id: LineageId) -> Option<&LineageNode> {
        self.nodes.get(id.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn nodes(&self) -> &[LineageNode] {
        &self.nodes
    }

    /// True if any node in this lineage's ancestry was born from a recombination.
    /// Walks all parents (recombination nodes have two), so a hybrid that later
    /// mutates still counts as a hybrid.
    pub fn has_recombination_ancestor(&self, id: LineageId) -> bool {
        // Iterative DFS with a visited guard; lineage is a DAG, not a tree.
        let mut stack = vec![id];
        let mut seen = std::collections::HashSet::new();
        while let Some(current) = stack.pop() {
            if !seen.insert(current) {
                continue;
            }
            let Some(node) = self.get(current) else {
                continue;
            };
            if node.origin == LineageOrigin::Recombination {
                return true;
            }
            stack.extend(node.parents.iter().copied());
        }
        false
    }

    /// Walk the parent chain (first parent only) until a Starter is reached.
    /// Returns the founding Starter's id, or None if the chain is malformed.
    pub fn trace_to_starter(&self, id: LineageId) -> Option<LineageId> {
        let mut current = id;
        for _ in 0..1024 {
            let node = self.get(current)?;
            if node.origin == LineageOrigin::Starter {
                return Some(current);
            }
            current = *node.parents.first()?;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_and_mutation_chain() {
        let mut g = LineageGraph::new();
        let starter = g.add_starter(0);
        let mut p = SmallVec::new();
        p.push(starter);
        let mutant = g.add(p.clone(), 12, LineageOrigin::Mutation);
        let mut p2 = SmallVec::new();
        p2.push(mutant);
        let mutant2 = g.add(p2, 30, LineageOrigin::Mutation);

        assert_eq!(g.trace_to_starter(mutant2), Some(starter));
        assert_eq!(g.trace_to_starter(starter), Some(starter));
        assert_eq!(g.get(mutant).unwrap().parents.len(), 1);
    }
}
