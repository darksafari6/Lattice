use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AetherError;
use crate::graph::{GraphEdge, GraphNode};
use crate::snapshot::SnapshotPayload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinModel {
    pub source_snapshot: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ScenarioAction {
    RemoveNode { node_id: String },
    RemoveEdge { edge_id: String },
    SetTrustLevel { node_id: String, trust_level: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinScenario {
    pub name: String,
    pub description: String,
    pub actions: Vec<ScenarioAction>,
}

#[derive(Debug, Serialize)]
pub struct SimulationResult {
    pub scenario: String,
    pub baseline_nodes: usize,
    pub baseline_edges: usize,
    pub simulated_nodes: usize,
    pub simulated_edges: usize,
    pub notes: Vec<String>,
}

impl TwinModel {
    pub fn from_snapshot(name: &str, payload: SnapshotPayload) -> Self {
        Self {
            source_snapshot: name.to_string(),
            nodes: payload.nodes,
            edges: payload.edges,
        }
    }

    pub fn load(path: &Path) -> Result<Self, AetherError> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), AetherError> {
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

impl TwinScenario {
    pub fn load(path: &Path) -> Result<Self, AetherError> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn validate(&self) -> Result<(), AetherError> {
        if self.name.trim().is_empty() {
            return Err(AetherError::Validation("scenario name must not be empty".into()));
        }
        if self.actions.is_empty() {
            return Err(AetherError::Validation(
                "scenario must contain at least one action".into(),
            ));
        }
        Ok(())
    }
}

pub fn simulate(model: &TwinModel, scenario: &TwinScenario) -> Result<SimulationResult, AetherError> {
    scenario.validate()?;
    let mut sim_nodes = model.nodes.clone();
    let mut sim_edges = model.edges.clone();
    let mut notes = Vec::new();

    for action in &scenario.actions {
        match action {
            ScenarioAction::RemoveNode { node_id } => {
                let before_n = sim_nodes.len();
                sim_nodes.retain(|n| &n.id != node_id);
                let before_e = sim_edges.len();
                sim_edges.retain(|e| &e.source_id != node_id && &e.target_id != node_id);
                notes.push(format!(
                    "Removed node '{node_id}' (nodes: {before_n}->{}, edges: {before_e}->{})",
                    sim_nodes.len(),
                    sim_edges.len()
                ));
            }
            ScenarioAction::RemoveEdge { edge_id } => {
                let before_e = sim_edges.len();
                sim_edges.retain(|e| &e.id != edge_id);
                notes.push(format!(
                    "Removed edge '{edge_id}' (edges: {before_e}->{})",
                    sim_edges.len()
                ));
            }
            ScenarioAction::SetTrustLevel {
                node_id,
                trust_level,
            } => {
                if let Some(node) = sim_nodes.iter_mut().find(|n| &n.id == node_id) {
                    let old = node.trust_level;
                    node.trust_level = *trust_level;
                    notes.push(format!(
                        "Updated trust_level for '{node_id}': {old} -> {trust_level}"
                    ));
                } else {
                    notes.push(format!("Node '{node_id}' not found in twin model"));
                }
            }
        }
    }

    Ok(SimulationResult {
        scenario: scenario.name.clone(),
        baseline_nodes: model.nodes.len(),
        baseline_edges: model.edges.len(),
        simulated_nodes: sim_nodes.len(),
        simulated_edges: sim_edges.len(),
        notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EdgeKind, NodeKind};
    use chrono::Utc;

    #[test]
    fn test_twin_simulation_remove_node() {
        let now = Utc::now();
        let model = TwinModel {
            source_snapshot: "s1".to_string(),
            nodes: vec![
                GraphNode {
                    id: "n1".to_string(),
                    kind: NodeKind::Device,
                    label: "n1".to_string(),
                    trust_level: 1,
                    metadata: serde_json::json!({}),
                    created_at: now,
                    updated_at: now,
                },
                GraphNode {
                    id: "n2".to_string(),
                    kind: NodeKind::Device,
                    label: "n2".to_string(),
                    trust_level: 1,
                    metadata: serde_json::json!({}),
                    created_at: now,
                    updated_at: now,
                },
            ],
            edges: vec![GraphEdge {
                id: "e1".to_string(),
                source_id: "n1".to_string(),
                target_id: "n2".to_string(),
                kind: EdgeKind::ConnectsTo,
                confidence: 1.0,
                evidence_id: None,
                metadata: serde_json::json!({}),
                created_at: now,
            }],
        };

        let scenario = TwinScenario {
            name: "link-failure".to_string(),
            description: "Remove n1".to_string(),
            actions: vec![ScenarioAction::RemoveNode {
                node_id: "n1".to_string(),
            }],
        };

        let res = simulate(&model, &scenario).unwrap();
        assert_eq!(res.simulated_nodes, 1);
        assert_eq!(res.simulated_edges, 0);
    }
}
