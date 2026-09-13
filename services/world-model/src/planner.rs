//! Dijkstra path planning over a [`WorldModel`].

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::model::WorldModel;

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub from: i32,
    pub to: i32,
    pub cost: f64,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PlanError {
    #[error("unknown node: {0}")]
    UnknownNode(i32),
    #[error("unknown route: {0}")]
    UnknownRoute(String),
    #[error("no route from {from} to {to}")]
    NoRoute { from: i32, to: i32 },
}

pub struct Planner<'a> {
    model: &'a WorldModel,
}

struct HeapItem {
    cost: f64,
    node: i32,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.node == other.node
    }
}
impl Eq for HeapItem {}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed so the `BinaryHeap` (a max-heap) acts as a min-heap.
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.node.cmp(&self.node))
    }
}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Planner<'a> {
    pub fn new(model: &'a WorldModel) -> Self {
        Self { model }
    }

    /// Shortest path `from -> to`, skipping nodes in `avoid`.
    pub fn shortest_path(
        &self,
        from: i32,
        to: i32,
        avoid: &[i32],
    ) -> Result<Vec<Segment>, PlanError> {
        if !self.model.nodes.contains_key(&from) {
            return Err(PlanError::UnknownNode(from));
        }
        if !self.model.nodes.contains_key(&to) {
            return Err(PlanError::UnknownNode(to));
        }
        if from == to {
            return Ok(Vec::new());
        }

        let avoid: HashSet<i32> = avoid.iter().copied().collect();
        let mut dist: HashMap<i32, f64> = HashMap::new();
        let mut prev: HashMap<i32, (i32, f64)> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(from, 0.0);
        heap.push(HeapItem {
            cost: 0.0,
            node: from,
        });

        while let Some(HeapItem { cost, node }) = heap.pop() {
            if cost > *dist.get(&node).unwrap_or(&f64::INFINITY) {
                continue;
            }
            if node == to {
                break;
            }
            for edge in self.model.neighbors(node) {
                if edge.to != to && avoid.contains(&edge.to) {
                    continue;
                }
                let next_cost = cost + edge.cost;
                if next_cost < *dist.get(&edge.to).unwrap_or(&f64::INFINITY) {
                    dist.insert(edge.to, next_cost);
                    prev.insert(edge.to, (node, edge.cost));
                    heap.push(HeapItem {
                        cost: next_cost,
                        node: edge.to,
                    });
                }
            }
        }

        if !dist.contains_key(&to) {
            return Err(PlanError::NoRoute { from, to });
        }

        // Reconstruct.
        let mut path = vec![to];
        let mut cursor = to;
        while cursor != from {
            let (parent, _) = prev
                .get(&cursor)
                .copied()
                .ok_or(PlanError::NoRoute { from, to })?;
            path.push(parent);
            cursor = parent;
        }
        path.reverse();

        let segments = path
            .windows(2)
            .map(|pair| {
                let (a, b) = (pair[0], pair[1]);
                let cost = self.model.edge_cost(a, b).unwrap_or(0.0);
                Segment {
                    from: a,
                    to: b,
                    cost,
                }
            })
            .collect();
        Ok(segments)
    }

    /// Expand a predefined route into segments, planning between consecutive
    /// route nodes.
    pub fn route(&self, route_id: &str, avoid: &[i32]) -> Result<Vec<Segment>, PlanError> {
        let nodes = self
            .model
            .routes
            .get(route_id)
            .ok_or_else(|| PlanError::UnknownRoute(route_id.to_string()))?;
        let mut segments = Vec::new();
        for pair in nodes.windows(2) {
            segments.extend(self.shortest_path(pair[0], pair[1], avoid)?);
        }
        Ok(segments)
    }
}
