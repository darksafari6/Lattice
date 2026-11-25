use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AetherError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseStatus {
    Open,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationCase {
    pub id: String,
    pub finding_id: String,
    pub status: CaseStatus,
    pub timeline: Vec<TimelineEvent>,
    pub close_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl InvestigationCase {
    pub fn create(data_dir: &Path, finding_id: &str) -> Result<Self, AetherError> {
        let now = Utc::now();
        let case = Self {
            id: format!("case-{}", Uuid::new_v4()),
            finding_id: finding_id.to_string(),
            status: CaseStatus::Open,
            timeline: vec![TimelineEvent {
                timestamp: now,
                description: format!("Investigation opened for finding '{finding_id}'"),
            }],
            close_reason: None,
            created_at: now,
        };
        case.save(data_dir)?;
        Ok(case)
    }

    pub fn save(&self, data_dir: &Path) -> Result<(), AetherError> {
        let dir = data_dir.join("cases");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", self.id));
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn load(data_dir: &Path, id: &str) -> Result<Self, AetherError> {
        let path = data_dir.join("cases").join(format!("{id}.json"));
        if !path.exists() {
            return Err(AetherError::Validation(format!("case '{id}' not found")));
        }
        let content = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&content)?)
    }

    pub fn list(data_dir: &Path) -> Result<Vec<Self>, AetherError> {
        let dir = data_dir.join("cases");
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                let content = std::fs::read_to_string(entry.path())?;
                let c = serde_json::from_str::<Self>(&content)?;
                out.push(c);
            }
        }
        Ok(out)
    }

    pub fn close(&mut self, data_dir: &Path, reason: &str) -> Result<(), AetherError> {
        self.status = CaseStatus::Closed;
        self.close_reason = Some(reason.to_string());
        self.timeline.push(TimelineEvent {
            timestamp: Utc::now(),
            description: format!("Case closed: {reason}"),
        });
        self.save(data_dir)
    }
}

pub fn export_case_report(
    case: &InvestigationCase,
    format: &str,
    output_dir: &Path,
) -> Result<std::path::PathBuf, AetherError> {
    std::fs::create_dir_all(output_dir)?;
    match format {
        "json" => {
            let path = output_dir.join(format!("{}.json", case.id));
            std::fs::write(&path, serde_json::to_string_pretty(case)?)?;
            Ok(path)
        }
        "html" => {
            fn html_escape(input: &str) -> String {
                input
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\'', "&#x27;")
            }
            let path = output_dir.join(format!("{}.html", case.id));
            let html = format!(
                "<!doctype html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>Investigation {}</h1><p>Finding: {}</p><p>Status: {:?}</p></body></html>",
                html_escape(&case.id),
                html_escape(&case.id),
                html_escape(&case.finding_id),
                case.status
            );
            std::fs::write(&path, html)?;
            Ok(path)
        }
        other => Err(AetherError::Validation(format!(
            "unsupported report format '{other}' (supported: json, html)"
        ))),
    }
}
