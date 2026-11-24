use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::AetherError;
use crate::graph::{GraphEdge, GraphNode, NetworkGraph, NodeKind};
use crate::scope::ScopeFile;
use crate::storage::Storage;

#[derive(Debug, Deserialize)]
struct JsonInventory {
    #[serde(default)]
    nodes: Vec<GraphNode>,
    #[serde(default)]
    edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize)]
pub struct ImportSummary {
    pub nodes_imported: usize,
    pub edges_imported: usize,
}

pub fn import_inventory(
    storage: &Storage,
    input: &Path,
    format: &str,
) -> Result<ImportSummary, AetherError> {
    let content = std::fs::read_to_string(input)?;
    match format {
        "json" => {
            let inv: JsonInventory = serde_json::from_str(&content)?;
            for n in &inv.nodes {
                NetworkGraph::upsert_node(storage, n)?;
            }
            for e in &inv.edges {
                NetworkGraph::upsert_edge(storage, e)?;
            }
            Ok(ImportSummary {
                nodes_imported: inv.nodes.len(),
                edges_imported: inv.edges.len(),
            })
        }
        "csv" => {
            let mut count = 0usize;
            let now = Utc::now();
            for (idx, line) in content.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || (idx == 0 && line.starts_with("id,")) {
                    continue;
                }
                let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                if parts.len() < 4 {
                    return Err(AetherError::Validation(format!(
                        "invalid CSV row at line {}: expected id,kind,label,trust_level",
                        idx + 1
                    )));
                }
                let trust_level: i64 = parts[3].parse().map_err(|_| {
                    AetherError::Validation(format!("invalid trust_level at line {}", idx + 1))
                })?;
                let node = GraphNode {
                    id: parts[0].to_string(),
                    kind: NodeKind::parse(parts[1]),
                    label: parts[2].to_string(),
                    trust_level,
                    metadata: serde_json::json!({}),
                    created_at: now,
                    updated_at: now,
                };
                NetworkGraph::upsert_node(storage, &node)?;
                count += 1;
            }
            Ok(ImportSummary {
                nodes_imported: count,
                edges_imported: 0,
            })
        }
        other => Err(AetherError::Validation(format!(
            "unsupported import format '{other}' (supported: json, csv)"
        ))),
    }
}

pub fn validate_discovery_request(
    scope_path: &Path,
    mode: &str,
    i_have_permission: bool,
) -> Result<ScopeFile, AetherError> {
    let scope = ScopeFile::load(scope_path)?;
    scope.validate()?;
    if mode == "authorized-active" && !i_have_permission {
        return Err(AetherError::AuthorizationDenied(
            "authorized-active discovery requires --i-have-permission".into(),
        ));
    }
    Ok(scope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_import() {
        let storage = Storage::open_in_memory().unwrap();
        let csv_path = std::env::temp_dir().join("aether_test_inv.csv");
        std::fs::write(&csv_path, "id,kind,label,trust_level\ndev-10,device,gw,3\n").unwrap();
        let summary = import_inventory(&storage, &csv_path, "csv").unwrap();
        assert_eq!(summary.nodes_imported, 1);
        let _ = std::fs::remove_file(&csv_path);
    }
}
