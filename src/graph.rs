use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use chrono::{DateTime, Utc};
use petgraph::graph::{DiGraph, NodeIndex};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::error::AetherError;
use crate::storage::Storage;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Device,
    Service,
    Network,
    Unknown,
}

impl NodeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeKind::Device => "device",
            NodeKind::Service => "service",
            NodeKind::Network => "network",
            NodeKind::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "device" => NodeKind::Device,
            "service" => NodeKind::Service,
            "network" => NodeKind::Network,
            _ => NodeKind::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Hosts,
    ConnectsTo,
    Trusts,
    RoutesTo,
    ObservedWith,
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeKind::Hosts => "hosts",
            EdgeKind::ConnectsTo => "connects_to",
            EdgeKind::Trusts => "trusts",
            EdgeKind::RoutesTo => "routes_to",
            EdgeKind::ObservedWith => "observed_with",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "hosts" => EdgeKind::Hosts,
            "connects_to" => EdgeKind::ConnectsTo,
            "trusts" => EdgeKind::Trusts,
            "routes_to" => EdgeKind::RoutesTo,
            _ => EdgeKind::ObservedWith,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub trust_level: i64,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub kind: EdgeKind,
    pub confidence: f64,
    pub evidence_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub devices: usize,
    pub services: usize,
}

pub struct NetworkGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    graph: DiGraph<String, String>,
    index_map: HashMap<String, NodeIndex>,
}

impl NetworkGraph {
    pub fn load(storage: &Storage) -> Result<Self, AetherError> {
        let mut node_stmt = storage.conn().prepare(
            "SELECT id, kind, label, trust_level, metadata_json, created_at, updated_at FROM nodes",
        )?;
        let nodes_iter = node_stmt.query_map([], |row| {
            let kind_str: String = row.get(1)?;
            let meta_str: String = row.get(4)?;
            let created_str: String = row.get(5)?;
            let updated_str: String = row.get(6)?;
            Ok(GraphNode {
                id: row.get(0)?,
                kind: NodeKind::parse(&kind_str),
                label: row.get(2)?,
                trust_level: row.get(3)?,
                metadata: serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({})),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                updated_at: DateTime::parse_from_rfc3339(&updated_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut nodes = Vec::new();
        for n in nodes_iter {
            nodes.push(n?);
        }

        let mut edge_stmt = storage.conn().prepare(
            "SELECT id, source_id, target_id, kind, confidence, evidence_id, metadata_json, created_at FROM edges",
        )?;
        let edges_iter = edge_stmt.query_map([], |row| {
            let kind_str: String = row.get(3)?;
            let meta_str: String = row.get(6)?;
            let created_str: String = row.get(7)?;
            Ok(GraphEdge {
                id: row.get(0)?,
                source_id: row.get(1)?,
                target_id: row.get(2)?,
                kind: EdgeKind::parse(&kind_str),
                confidence: row.get(4)?,
                evidence_id: row.get(5)?,
                metadata: serde_json::from_str(&meta_str).unwrap_or(serde_json::json!({})),
                created_at: DateTime::parse_from_rfc3339(&created_str)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut edges = Vec::new();
        for e in edges_iter {
            edges.push(e?);
        }

        let mut graph = DiGraph::new();
        let mut index_map = HashMap::new();
        for n in &nodes {
            let idx = graph.add_node(n.id.clone());
            index_map.insert(n.id.clone(), idx);
        }
        for e in &edges {
            if let (Some(&src), Some(&dst)) = (index_map.get(&e.source_id), index_map.get(&e.target_id)) {
                graph.add_edge(src, dst, e.kind.as_str().to_string());
            }
        }

        Ok(Self {
            nodes,
            edges,
            graph,
            index_map,
        })
    }

    pub fn upsert_node(storage: &Storage, node: &GraphNode) -> Result<(), AetherError> {
        storage.conn().execute(
            "INSERT INTO nodes (id, kind, label, trust_level, metadata_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               kind = excluded.kind,
               label = excluded.label,
               trust_level = excluded.trust_level,
               metadata_json = excluded.metadata_json,
               updated_at = excluded.updated_at",
            params![
                node.id,
                node.kind.as_str(),
                node.label,
                node.trust_level,
                serde_json::to_string(&node.metadata)?,
                node.created_at.to_rfc3339(),
                node.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn upsert_edge(storage: &Storage, edge: &GraphEdge) -> Result<(), AetherError> {
        storage.conn().execute(
            "INSERT INTO edges (id, source_id, target_id, kind, confidence, evidence_id, metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
               confidence = excluded.confidence,
               evidence_id = excluded.evidence_id,
               metadata_json = excluded.metadata_json",
            params![
                edge.id,
                edge.source_id,
                edge.target_id,
                edge.kind.as_str(),
                edge.confidence,
                edge.evidence_id,
                serde_json::to_string(&edge.metadata)?,
                edge.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn stats(&self) -> GraphStats {
        let devices = self.nodes.iter().filter(|n| n.kind == NodeKind::Device).count();
        let services = self.nodes.iter().filter(|n| n.kind == NodeKind::Service).count();
        GraphStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            devices,
            services,
        }
    }

    pub fn shortest_path(&self, from: &str, to: &str, max_hops: usize) -> Option<Vec<String>> {
        let &start = self.index_map.get(from)?;
        let &goal = self.index_map.get(to)?;
        if start == goal {
            return Some(vec![from.to_string()]);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<NodeIndex, NodeIndex> = HashMap::new();

        queue.push_back((start, 0usize));
        visited.insert(start);

        while let Some((curr, depth)) = queue.pop_front() {
            if depth >= max_hops {
                continue;
            }
            for neighbor in self.graph.neighbors(curr) {
                if visited.insert(neighbor) {
                    parent.insert(neighbor, curr);
                    if neighbor == goal {
                        let mut path = vec![self.graph[goal].clone()];
                        let mut p = goal;
                        while let Some(&prev) = parent.get(&p) {
                            path.push(self.graph[prev].clone());
                            p = prev;
                        }
                        path.reverse();
                        return Some(path);
                    }
                    queue.push_back((neighbor, depth + 1));
                }
            }
        }
        None
    }

    pub fn neighborhood(&self, node_id: &str, depth: usize) -> Vec<String> {
        let Some(&start) = self.index_map.get(node_id) else {
            return Vec::new();
        };
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        visited.insert(start);
        queue.push_back((start, 0usize));

        while let Some((curr, d)) = queue.pop_front() {
            result.push(self.graph[curr].clone());
            if d >= depth {
                continue;
            }
            for neighbor in self.graph.neighbors(curr) {
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, d + 1));
                }
            }
        }
        result
    }

    pub fn export_json(&self, output: &Path) -> Result<(), AetherError> {
        let payload = serde_json::json!({
            "nodes": self.nodes,
            "edges": self.edges,
        });
        std::fs::write(output, serde_json::to_string_pretty(&payload)?)?;
        Ok(())
    }

    pub fn export_csv(&self, output: &Path) -> Result<(), AetherError> {
        let mut out = String::from("source_id,target_id,kind,confidence\n");
        for e in &self.edges {
            out.push_str(&format!(
                "{},{},{},{}\n",
                e.source_id,
                e.target_id,
                e.kind.as_str(),
                e.confidence
            ));
        }
        std::fs::write(output, out)?;
        Ok(())
    }

    /// Safe, limited query grammar:
    /// `<devices|services|nodes> [where trust_level <>=|<=|==|>|< <int>]`
    pub fn query_nodes(&self, query: &str) -> Result<Vec<&GraphNode>, AetherError> {
        let tokens: Vec<&str> = query.split_whitespace().collect();
        if tokens.is_empty() {
            return Err(AetherError::Validation("empty graph query".into()));
        }

        let kind_filter = match tokens[0] {
            "devices" => Some(NodeKind::Device),
            "services" => Some(NodeKind::Service),
            "nodes" => None,
            other => {
                return Err(AetherError::Validation(format!(
                    "unsupported query target '{other}' (expected devices, services, or nodes)"
                )))
            }
        };

        let mut op_filter: Option<(&str, i64)> = None;
        if tokens.len() > 1 {
            if tokens.len() != 5 || tokens[1] != "where" || tokens[2] != "trust_level" {
                return Err(AetherError::Validation(
                    "invalid query syntax; expected '<devices|services|nodes> where trust_level <op> <int>'"
                        .into(),
                ));
            }
            let val: i64 = tokens[4].parse().map_err(|_| {
                AetherError::Validation(format!("invalid integer '{}' in query", tokens[4]))
            })?;
            op_filter = Some((tokens[3], val));
        }

        let mut results = Vec::new();
        for node in &self.nodes {
            if let Some(ref kf) = kind_filter {
                if &node.kind != kf {
                    continue;
                }
            }
            if let Some((op, val)) = op_filter {
                let matched = match op {
                    ">=" => node.trust_level >= val,
                    "<=" => node.trust_level <= val,
                    "==" => node.trust_level == val,
                    ">" => node.trust_level > val,
                    "<" => node.trust_level < val,
                    other => {
                        return Err(AetherError::Validation(format!(
                            "unsupported operator '{other}' (expected >=, <=, ==, >, <)"
                        )))
                    }
                };
                if !matched {
                    continue;
                }
            }
            results.push(node);
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_path_and_neighborhood() {
        let storage = Storage::open_in_memory().unwrap();
        let now = Utc::now();
        for id in ["dev-1", "dev-2", "svc-1"] {
            NetworkGraph::upsert_node(
                &storage,
                &GraphNode {
                    id: id.to_string(),
                    kind: if id.starts_with("dev") {
                        NodeKind::Device
                    } else {
                        NodeKind::Service
                    },
                    label: id.to_string(),
                    trust_level: 1,
                    metadata: serde_json::json!({}),
                    created_at: now,
                    updated_at: now,
                },
            )
            .unwrap();
        }
        NetworkGraph::upsert_edge(
            &storage,
            &GraphEdge {
                id: "e1".to_string(),
                source_id: "dev-1".to_string(),
                target_id: "dev-2".to_string(),
                kind: EdgeKind::ConnectsTo,
                confidence: 0.9,
                evidence_id: None,
                metadata: serde_json::json!({}),
                created_at: now,
            },
        )
        .unwrap();
        NetworkGraph::upsert_edge(
            &storage,
            &GraphEdge {
                id: "e2".to_string(),
                source_id: "dev-2".to_string(),
                target_id: "svc-1".to_string(),
                kind: EdgeKind::Hosts,
                confidence: 1.0,
                evidence_id: None,
                metadata: serde_json::json!({}),
                created_at: now,
            },
        )
        .unwrap();

        let g = NetworkGraph::load(&storage).unwrap();
        assert_eq!(g.stats().total_nodes, 3);
        assert_eq!(
            g.shortest_path("dev-1", "svc-1", 4).unwrap(),
            vec!["dev-1", "dev-2", "svc-1"]
        );
        assert_eq!(g.neighborhood("dev-1", 1), vec!["dev-1", "dev-2"]);
    }
}
