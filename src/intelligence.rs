use serde::{Deserialize, Serialize};

use crate::graph::NetworkGraph;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    Confirmed,
    Suspected,
    Inferred,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub confidence: ConfidenceLevel,
    pub score: u32,
    pub explanation: String,
    pub recommendation: String,
}

pub fn analyze_graph(graph: &NetworkGraph) -> Vec<Finding> {
    let mut findings = Vec::new();

    for edge in &graph.edges {
        if edge.confidence < 0.6 {
            findings.push(Finding {
                id: format!("finding-lowconf-{}", edge.id),
                title: format!(
                    "Low-confidence relationship {} -> {}",
                    edge.source_id, edge.target_id
                ),
                confidence: ConfidenceLevel::Inferred,
                score: 35,
                explanation: format!(
                    "Edge '{}' ({}) has confidence {:.2} < 0.60 and lacks strong corroborating evidence.",
                    edge.id,
                    edge.kind.as_str(),
                    edge.confidence
                ),
                recommendation: "Verify relationship via passive observation or authorized active discovery."
                    .to_string(),
            });
        }
    }

    for node in &graph.nodes {
        if node.trust_level >= 3 {
            findings.push(Finding {
                id: format!("finding-hightrust-{}", node.id),
                title: format!("High-trust node '{}'", node.label),
                confidence: ConfidenceLevel::Confirmed,
                score: 60,
                explanation: format!(
                    "Node '{}' has trust_level={} (>= 3), making it a critical asset in path analysis.",
                    node.id, node.trust_level
                ),
                recommendation: "Ensure strict scope boundaries and regular snapshot diffing."
                    .to_string(),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EdgeKind, GraphEdge, GraphNode, NodeKind};
    use crate::storage::Storage;
    use chrono::Utc;

    #[test]
    fn test_analyze_graph_produces_explainable_findings() {
        let storage = Storage::open_in_memory().unwrap();
        let now = Utc::now();
        NetworkGraph::upsert_node(
            &storage,
            &GraphNode {
                id: "core-gw".to_string(),
                kind: NodeKind::Device,
                label: "core-gw".to_string(),
                trust_level: 3,
                metadata: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            },
        )
        .unwrap();
        NetworkGraph::upsert_edge(
            &storage,
            &GraphEdge {
                id: "e-weak".to_string(),
                source_id: "core-gw".to_string(),
                target_id: "core-gw".to_string(),
                kind: EdgeKind::ObservedWith,
                confidence: 0.4,
                evidence_id: None,
                metadata: serde_json::json!({}),
                created_at: now,
            },
        )
        .unwrap();

        let g = NetworkGraph::load(&storage).unwrap();
        let findings = analyze_graph(&g);
        assert_eq!(findings.len(), 2);
    }
}
