use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AetherError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveEvent {
    pub timestamp: String,
    pub protocol: String,
    pub src: String,
    pub dst: String,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct ReplaySummary {
    pub input: String,
    pub speed: f64,
    pub events_replayed: usize,
    pub events: Vec<ObserveEvent>,
}

pub fn replay_capture(input: &Path, speed: f64) -> Result<ReplaySummary, AetherError> {
    if speed <= 0.0 {
        return Err(AetherError::Validation("replay speed must be > 0.0".into()));
    }
    let content = std::fs::read_to_string(input)?;
    let events: Vec<ObserveEvent> = serde_json::from_str(&content)?;
    Ok(ReplaySummary {
        input: input.display().to_string(),
        speed,
        events_replayed: events.len(),
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_capture() {
        let path = std::env::temp_dir().join("aether_test_capture.json");
        let sample = serde_json::json!([
            {
                "timestamp": "2026-11-24T00:00:00Z",
                "protocol": "smtp",
                "src": "127.0.0.1:41000",
                "dst": "127.0.0.1:25",
                "summary": "EHLO aetherlattice.local"
            }
        ]);
        std::fs::write(&path, serde_json::to_string(&sample).unwrap()).unwrap();
        let res = replay_capture(&path, 1.0).unwrap();
        assert_eq!(res.events_replayed, 1);
        let _ = std::fs::remove_file(&path);
    }
}
