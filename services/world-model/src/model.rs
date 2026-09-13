//! World model graph and JSON loading.

use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: i32,
    pub name: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub from: i32,
    pub to: i32,
    pub cost: f64,
}

#[derive(Debug, Clone, Default)]
pub struct WorldModel {
    pub nodes: HashMap<i32, Node>,
    pub edges: Vec<Edge>,
    pub routes: HashMap<String, Vec<i32>>,
}

impl WorldModel {
    pub fn from_json_str(json: &str) -> Result<Self, WorldModelError> {
        let raw: RawMap = serde_json::from_str(json)?;
        let mut model = WorldModel::default();
        for (id, tag) in raw.tags {
            let id = id
                .parse::<i32>()
                .map_err(|_| WorldModelError::BadNodeId(id))?;
            model.nodes.insert(
                id,
                Node {
                    id,
                    name: tag.name,
                    x: tag.x,
                    y: tag.y,
                },
            );
        }
        model.edges = raw
            .edges
            .into_iter()
            .map(|e| Edge {
                from: e.from,
                to: e.to,
                cost: e.cost,
            })
            .collect();
        model.routes = raw.routes;
        Ok(model)
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id, node);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    pub fn neighbors(&self, id: i32) -> impl Iterator<Item = &Edge> {
        self.edges.iter().filter(move |e| e.from == id)
    }

    pub fn edge_cost(&self, from: i32, to: i32) -> Option<f64> {
        self.edges
            .iter()
            .filter(|e| e.from == from && e.to == to)
            .map(|e| e.cost)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum WorldModelError {
    #[error("invalid JSON: {0}")]
    Json(String),
    #[error("invalid node id: {0}")]
    BadNodeId(String),
}

impl From<serde_json::Error> for WorldModelError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e.to_string())
    }
}

#[derive(Deserialize)]
struct RawMap {
    #[serde(default)]
    tags: HashMap<String, RawTag>,
    #[serde(default)]
    edges: Vec<RawEdge>,
    #[serde(default)]
    routes: HashMap<String, Vec<i32>>,
}

#[derive(Deserialize)]
struct RawTag {
    #[serde(default)]
    name: String,
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
}

#[derive(Deserialize)]
struct RawEdge {
    from: i32,
    to: i32,
    #[serde(default)]
    cost: f64,
}
