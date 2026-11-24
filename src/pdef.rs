use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AetherError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRule {
    pub from: String,
    pub input: String,
    pub expected_response_prefix: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolDefinition {
    pub name: String,
    pub version: String,
    pub transport: String,
    pub default_port: u16,
    pub initial_state: String,
    pub states: Vec<String>,
    pub transitions: Vec<TransitionRule>,
}

impl ProtocolDefinition {
    pub fn default_smtp() -> Self {
        Self {
            name: "smtp".to_string(),
            version: "1.0".to_string(),
            transport: "tcp".to_string(),
            default_port: 25,
            initial_state: "INIT".to_string(),
            states: vec![
                "INIT".to_string(),
                "GREETED".to_string(),
                "QUIT_SENT".to_string(),
            ],
            transitions: vec![
                TransitionRule {
                    from: "INIT".to_string(),
                    input: "EHLO aetherlattice.local".to_string(),
                    expected_response_prefix: "250".to_string(),
                    to: "GREETED".to_string(),
                },
                TransitionRule {
                    from: "GREETED".to_string(),
                    input: "NOOP".to_string(),
                    expected_response_prefix: "250".to_string(),
                    to: "GREETED".to_string(),
                },
                TransitionRule {
                    from: "GREETED".to_string(),
                    input: "QUIT".to_string(),
                    expected_response_prefix: "221".to_string(),
                    to: "QUIT_SENT".to_string(),
                },
            ],
        }
    }

    pub fn load(path: &Path) -> Result<Self, AetherError> {
        let content = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), AetherError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, serde_yaml::to_string(self)?)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), AetherError> {
        if self.name.trim().is_empty() {
            return Err(AetherError::Validation("protocol name must not be empty".into()));
        }
        let state_set: HashSet<&str> = self.states.iter().map(|s| s.as_str()).collect();
        if !state_set.contains(self.initial_state.as_str()) {
            return Err(AetherError::Validation(format!(
                "initial_state '{}' is not in states list",
                self.initial_state
            )));
        }
        for t in &self.transitions {
            if !state_set.contains(t.from.as_str()) {
                return Err(AetherError::Validation(format!(
                    "transition 'from' state '{}' is not declared",
                    t.from
                )));
            }
            if !state_set.contains(t.to.as_str()) {
                return Err(AetherError::Validation(format!(
                    "transition 'to' state '{}' is not declared",
                    t.to
                )));
            }
        }
        Ok(())
    }

    pub fn lint(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.transitions.is_empty() {
            warnings.push("no transitions defined".to_string());
        }
        let used_from: HashSet<&str> = self.transitions.iter().map(|t| t.from.as_str()).collect();
        for s in &self.states {
            if s != &self.initial_state && !used_from.contains(s.as_str()) {
                warnings.push(format!("state '{s}' has no outgoing transitions (terminal state)"));
            }
        }
        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_smtp_pdef_validates() {
        let pdef = ProtocolDefinition::default_smtp();
        assert!(pdef.validate().is_ok());
        let warnings = pdef.lint();
        assert_eq!(warnings.len(), 1);
    }
}
