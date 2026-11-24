use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::error::AetherError;

pub const CURRENT_SCHEMA_VERSION: i64 = 1;

pub struct Storage {
    conn: Connection,
}

impl Storage {
    pub fn open(data_dir: &Path) -> Result<Self, AetherError> {
        std::fs::create_dir_all(data_dir)?;
        let db_path = data_dir.join("aetherlattice.db");
        let conn = Connection::open(db_path)?;
        let storage = Self { conn };
        storage.migrate(false)?;
        Ok(storage)
    }

    pub fn open_in_memory() -> Result<Self, AetherError> {
        let conn = Connection::open_in_memory()?;
        let storage = Self { conn };
        storage.migrate(false)?;
        Ok(storage)
    }

    pub fn schema_version(&self) -> Result<i64, AetherError> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
            [],
            |row| row.get(0),
        )?;
        if !exists {
            return Ok(0);
        }
        let version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;
        Ok(version)
    }

    pub fn migrate(&self, dry_run: bool) -> Result<Vec<i64>, AetherError> {
        let current = self.schema_version()?;
        let mut applied = Vec::new();

        if current < 1 {
            applied.push(1);
            if !dry_run {
                self.conn.execute_batch(
                    "
                    CREATE TABLE IF NOT EXISTS schema_migrations (
                        version INTEGER PRIMARY KEY,
                        applied_at TEXT NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS nodes (
                        id TEXT PRIMARY KEY,
                        kind TEXT NOT NULL,
                        label TEXT NOT NULL,
                        trust_level INTEGER NOT NULL DEFAULT 0,
                        metadata_json TEXT NOT NULL DEFAULT '{}',
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS edges (
                        id TEXT PRIMARY KEY,
                        source_id TEXT NOT NULL,
                        target_id TEXT NOT NULL,
                        kind TEXT NOT NULL,
                        confidence REAL NOT NULL DEFAULT 1.0,
                        evidence_id TEXT,
                        metadata_json TEXT NOT NULL DEFAULT '{}',
                        created_at TEXT NOT NULL,
                        FOREIGN KEY(source_id) REFERENCES nodes(id),
                        FOREIGN KEY(target_id) REFERENCES nodes(id)
                    );
                    CREATE TABLE IF NOT EXISTS snapshots (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL UNIQUE,
                        scope_hash TEXT NOT NULL,
                        graph_json TEXT NOT NULL,
                        created_at TEXT NOT NULL
                    );
                    ",
                )?;
                self.conn.execute(
                    "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
                    params![1, Utc::now().to_rfc3339()],
                )?;
            }
        }

        Ok(applied)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_migration() {
        let storage = Storage::open_in_memory().unwrap();
        assert_eq!(storage.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
        let pending = storage.migrate(true).unwrap();
        assert!(pending.is_empty());
    }
}
