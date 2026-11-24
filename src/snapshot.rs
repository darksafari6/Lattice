use std::collections::HashSet;
use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AetherError;
use crate::graph::{GraphEdge, GraphNode, NetworkGraph};
use crate::scope::ScopeFile;
use crate::storage::Storage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotPayload {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub id: String,
    pub name: String,
    pub scope_hash: String,
    pub payload: SnapshotPayload,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotSummary {
    pub id: String,
    pub name: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SnapshotDiff {
    pub first: String,
    pub second: String,
    pub added_nodes: Vec<String>,
    pub removed_nodes: Vec<String>,
    pub added_edges: Vec<String>,
    pub removed_edges: Vec<String>,
}

pub fn create_snapshot(
    storage: &Storage,
    name: &str,
    scope_path: &Path,
) -> Result<SnapshotRecord, AetherError> {
    let scope = ScopeFile::load(scope_path)?;
    scope.validate()?;

    let g = NetworkGraph::load(storage)?;
    let payload = SnapshotPayload {
        nodes: g.nodes,
        edges: g.edges,
    };
    let graph_json = serde_json::to_string(&payload)?;
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let scope_hash = format!("scope:{}", scope.owner);

    storage.conn().execute(
        "INSERT INTO snapshots (id, name, scope_hash, graph_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, name, scope_hash, graph_json, now.to_rfc3339()],
    )?;

    Ok(SnapshotRecord {
        id,
        name: name.to_string(),
        scope_hash,
        payload,
        created_at: now,
    })
}

pub fn list_snapshots(storage: &Storage) -> Result<Vec<SnapshotSummary>, AetherError> {
    let mut stmt = storage
        .conn()
        .prepare("SELECT id, name, graph_json, created_at FROM snapshots ORDER BY created_at DESC")?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let graph_json: String = row.get(2)?;
        let created_str: String = row.get(3)?;
        let payload: SnapshotPayload =
            serde_json::from_str(&graph_json).unwrap_or(SnapshotPayload {
                nodes: vec![],
                edges: vec![],
            });
        let created_at = DateTime::parse_from_rfc3339(&created_str)
            .map(|d| d.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        Ok(SnapshotSummary {
            id,
            name,
            node_count: payload.nodes.len(),
            edge_count: payload.edges.len(),
            created_at,
        })
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn load_snapshot(storage: &Storage, id_or_name: &str) -> Result<SnapshotRecord, AetherError> {
    let mut stmt = storage.conn().prepare(
        "SELECT id, name, scope_hash, graph_json, created_at FROM snapshots WHERE id = ?1 OR name = ?1 LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![id_or_name], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let scope_hash: String = row.get(2)?;
        let graph_json: String = row.get(3)?;
        let created_str: String = row.get(4)?;
        Ok((id, name, scope_hash, graph_json, created_str))
    })?;

    let row = rows
        .next()
        .ok_or_else(|| AetherError::Validation(format!("snapshot '{id_or_name}' not found")))??;

    let payload: SnapshotPayload = serde_json::from_str(&row.3)?;
    let created_at = DateTime::parse_from_rfc3339(&row.4)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());

    Ok(SnapshotRecord {
        id: row.0,
        name: row.1,
        scope_hash: row.2,
        payload,
        created_at,
    })
}

pub fn diff_snapshots(
    storage: &Storage,
    first: &str,
    second: &str,
) -> Result<SnapshotDiff, AetherError> {
    let s1 = load_snapshot(storage, first)?;
    let s2 = load_snapshot(storage, second)?;

    let n1: HashSet<_> = s1.payload.nodes.iter().map(|n| n.id.clone()).collect();
    let n2: HashSet<_> = s2.payload.nodes.iter().map(|n| n.id.clone()).collect();

    let e1: HashSet<_> = s1.payload.edges.iter().map(|e| e.id.clone()).collect();
    let e2: HashSet<_> = s2.payload.edges.iter().map(|e| e.id.clone()).collect();

    let mut added_nodes: Vec<_> = n2.difference(&n1).cloned().collect();
    let mut removed_nodes: Vec<_> = n1.difference(&n2).cloned().collect();
    let mut added_edges: Vec<_> = e2.difference(&e1).cloned().collect();
    let mut removed_edges: Vec<_> = e1.difference(&e2).cloned().collect();

    added_nodes.sort();
    removed_nodes.sort();
    added_edges.sort();
    removed_edges.sort();

    Ok(SnapshotDiff {
        first: s1.name,
        second: s2.name,
        added_nodes,
        removed_nodes,
        added_edges,
        removed_edges,
    })
}

pub fn prune_snapshots(
    storage: &Storage,
    older_than_days: i64,
    dry_run: bool,
) -> Result<usize, AetherError> {
    let cutoff = (Utc::now() - Duration::days(older_than_days)).to_rfc3339();
    if dry_run {
        let count: i64 = storage.conn().query_row(
            "SELECT COUNT(*) FROM snapshots WHERE created_at < ?1",
            params![cutoff],
            |r| r.get(0),
        )?;
        return Ok(count as usize);
    }
    let deleted = storage.conn().execute(
        "DELETE FROM snapshots WHERE created_at < ?1",
        params![cutoff],
    )?;
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{GraphNode, NodeKind};

    #[test]
    fn test_snapshot_and_diff() {
        let storage = Storage::open_in_memory().unwrap();
        let scope_path = std::env::temp_dir().join("aether_test_scope.yaml");
        ScopeFile::default_template().save(&scope_path).unwrap();

        let now = Utc::now();
        NetworkGraph::upsert_node(
            &storage,
            &GraphNode {
                id: "dev-1".to_string(),
                kind: NodeKind::Device,
                label: "dev-1".to_string(),
                trust_level: 1,
                metadata: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            },
        )
        .unwrap();
        create_snapshot(&storage, "s1", &scope_path).unwrap();

        NetworkGraph::upsert_node(
            &storage,
            &GraphNode {
                id: "dev-2".to_string(),
                kind: NodeKind::Device,
                label: "dev-2".to_string(),
                trust_level: 2,
                metadata: serde_json::json!({}),
                created_at: now,
                updated_at: now,
            },
        )
        .unwrap();
        create_snapshot(&storage, "s2", &scope_path).unwrap();

        let diff = diff_snapshots(&storage, "s1", "s2").unwrap();
        assert_eq!(diff.added_nodes, vec!["dev-2"]);
        assert!(diff.removed_nodes.is_empty());

        let _ = std::fs::remove_file(&scope_path);
    }
}
