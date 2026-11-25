use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::AetherError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: String,
    pub case_id: Option<String>,
    pub kind: String,
    pub summary: String,
    pub payload: serde_json::Value,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
}

fn compute_sha256(val: &serde_json::Value) -> Result<String, AetherError> {
    let bytes = serde_json::to_vec(val)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

impl EvidenceRecord {
    pub fn new(
        case_id: Option<String>,
        kind: &str,
        summary: &str,
        payload: serde_json::Value,
    ) -> Result<Self, AetherError> {
        let sha256 = compute_sha256(&payload)?;
        Ok(Self {
            id: format!("ev-{}", Uuid::new_v4()),
            case_id,
            kind: kind.to_string(),
            summary: summary.to_string(),
            payload,
            sha256,
            created_at: Utc::now(),
        })
    }

    pub fn verify(&self) -> Result<bool, AetherError> {
        Ok(compute_sha256(&self.payload)? == self.sha256)
    }

    pub fn save(&self, data_dir: &Path) -> Result<(), AetherError> {
        let dir = data_dir.join("evidence");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", self.id));
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn load(data_dir: &Path, id: &str) -> Result<Self, AetherError> {
        let path = data_dir.join("evidence").join(format!("{id}.json"));
        if !path.exists() {
            return Err(AetherError::Validation(format!("evidence '{id}' not found")));
        }
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn list(data_dir: &Path, case_filter: Option<&str>) -> Result<Vec<Self>, AetherError> {
        let dir = data_dir.join("evidence");
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let content = std::fs::read_to_string(entry.path())?;
                if let Ok(rec) = serde_json::from_str::<Self>(&content) {
                    if let Some(cf) = case_filter {
                        if rec.case_id.as_deref() != Some(cf) {
                            continue;
                        }
                    }
                    out.push(rec);
                }
            }
        }
        Ok(out)
    }
}

pub fn redact_text(input: &str) -> String {
    let mut lines = Vec::new();
    for line in input.lines() {
        let mut words = Vec::new();
        for word in line.split_whitespace() {
            let lower = word.to_ascii_lowercase();
            let mut redacted = word.to_string();
            for key in ["password=", "token=", "secret=", "api_key="] {
                if let Some(idx) = lower.find(key) {
                    let prefix = &word[..idx + key.len()];
                    redacted = format!("{prefix}[REDACTED]");
                    break;
                }
            }
            words.push(redacted);
        }
        lines.push(words.join(" "));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_text() {
        assert_eq!(redact_text("auth token=supersecret123"), "auth token=[REDACTED]");
        assert_eq!(redact_text("normal log line"), "normal log line");
    }

    #[test]
    fn test_evidence_sha256_verify() {
        let mut rec = EvidenceRecord::new(
            None,
            "observation",
            "test",
            serde_json::json!({"host": "127.0.0.1"}),
        )
        .unwrap();
        assert!(rec.verify().unwrap());
        rec.payload = serde_json::json!({"host": "192.0.2.1"});
        assert!(!rec.verify().unwrap());
    }
}
