use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::AetherError;
use crate::pdef::ProtocolDefinition;

#[derive(Debug, Serialize)]
pub struct TransitionTestCaseResult {
    pub from_state: String,
    pub input: String,
    pub expected_prefix: String,
    pub observed_response: String,
    pub to_state: String,
    pub passed: bool,
}

#[derive(Debug, Serialize)]
pub struct ProtocolTestReport {
    pub target: String,
    pub protocol: String,
    pub suite: String,
    pub dry_run: bool,
    pub cases_run: usize,
    pub cases_passed: usize,
    pub results: Vec<TransitionTestCaseResult>,
}

pub fn run_protocol_test(
    target: &str,
    pdef: &ProtocolDefinition,
    suite: &str,
    max_cases: usize,
    dry_run: bool,
    i_have_permission: bool,
) -> Result<ProtocolTestReport, AetherError> {
    pdef.validate()?;
    let is_local = target.starts_with("local://");

    if !is_local && !i_have_permission {
        return Err(AetherError::AuthorizationDenied(format!(
            "non-local target '{target}' requires --i-have-permission and a valid scope"
        )));
    }

    if dry_run {
        return Ok(ProtocolTestReport {
            target: target.to_string(),
            protocol: pdef.name.clone(),
            suite: suite.to_string(),
            dry_run: true,
            cases_run: 0,
            cases_passed: 0,
            results: Vec::new(),
        });
    }

    if !is_local {
        return Err(AetherError::DependencyUnavailable(
            "live network protocol testing is not enabled in this build; use local:// fixtures"
                .to_string(),
        ));
    }

    let mut current_state = pdef.initial_state.clone();
    let mut results = Vec::new();

    for rule in pdef.transitions.iter().take(max_cases) {
        if rule.from != current_state {
            continue;
        }
        let observed = format!("{} OK (local fixture)", rule.expected_response_prefix);
        let passed = observed.starts_with(&rule.expected_response_prefix);
        if passed {
            current_state = rule.to.clone();
        }
        results.push(TransitionTestCaseResult {
            from_state: rule.from.clone(),
            input: rule.input.clone(),
            expected_prefix: rule.expected_response_prefix.clone(),
            observed_response: observed,
            to_state: rule.to.clone(),
            passed,
        });
    }

    let cases_passed = results.iter().filter(|r| r.passed).count();
    Ok(ProtocolTestReport {
        target: target.to_string(),
        protocol: pdef.name.clone(),
        suite: suite.to_string(),
        dry_run: false,
        cases_run: results.len(),
        cases_passed,
        results,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolDna {
    pub version: String,
    pub target: String,
    pub profile: String,
    pub analysis_mode: String,
    pub banner_prefix: String,
    pub transition_signatures: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct DnaComparison {
    pub first_target: String,
    pub second_target: String,
    pub identical: bool,
    pub added_signatures: Vec<String>,
    pub removed_signatures: Vec<String>,
}

/// Generates a definition-based Protocol DNA fingerprint for a `local://` fixture target.
pub fn fingerprint_protocol(target: &str, profile: &str) -> Result<ProtocolDna, AetherError> {
    if !target.starts_with("local://") {
        return Err(AetherError::AuthorizationDenied(format!(
            "live fingerprinting of '{target}' requires authorized scope; use local:// fixtures"
        )));
    }
    let pdef = match profile {
        "smtp" => ProtocolDefinition::default_smtp(),
        other => {
            return Err(AetherError::Validation(format!(
                "unsupported profile '{other}' (supported: smtp)"
            )))
        }
    };

    let transition_signatures = pdef
        .transitions
        .iter()
        .map(|t| format!("{}:{}->{}", t.from, t.expected_response_prefix, t.to))
        .collect();

    Ok(ProtocolDna {
        version: "1.0".to_string(),
        target: target.to_string(),
        profile: pdef.name,
        analysis_mode: "definition-based-fixture".to_string(),
        banner_prefix: "220".to_string(),
        transition_signatures,
        captured_at: Utc::now(),
    })
}

/// Compares two serialized `ProtocolDna` JSON files and reports added/removed signatures.
pub fn compare_dna(first: &Path, second: &Path) -> Result<DnaComparison, AetherError> {
    let d1: ProtocolDna = serde_json::from_str(&std::fs::read_to_string(first)?)?;
    let d2: ProtocolDna = serde_json::from_str(&std::fs::read_to_string(second)?)?;

    let added_signatures: Vec<String> = d2
        .transition_signatures
        .iter()
        .filter(|s| !d1.transition_signatures.contains(s))
        .cloned()
        .collect();
    let removed_signatures: Vec<String> = d1
        .transition_signatures
        .iter()
        .filter(|s| !d2.transition_signatures.contains(s))
        .cloned()
        .collect();

    Ok(DnaComparison {
        first_target: d1.target,
        second_target: d2.target,
        identical: added_signatures.is_empty()
            && removed_signatures.is_empty()
            && d1.banner_prefix == d2.banner_prefix,
        added_signatures,
        removed_signatures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_smtp_fixture_fsm() {
        let pdef = ProtocolDefinition::default_smtp();
        let report = run_protocol_test(
            "local://smtp-fixture",
            &pdef,
            "safe-transition-checks",
            10,
            false,
            false,
        )
        .unwrap();
        assert_eq!(report.cases_run, 3);
        assert_eq!(report.cases_passed, 3);
    }

    #[test]
    fn test_non_local_requires_permission() {
        let pdef = ProtocolDefinition::default_smtp();
        let err = run_protocol_test(
            "192.0.2.10:25",
            &pdef,
            "safe-transition-checks",
            10,
            false,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, AetherError::AuthorizationDenied(_)));
    }

    #[test]
    fn test_fingerprint_and_compare_dna() {
        let dna1 = fingerprint_protocol("local://smtp-fixture", "smtp").unwrap();
        assert_eq!(dna1.profile, "smtp");
        assert!(!dna1.transition_signatures.is_empty());

        let mut dna2 = dna1.clone();
        dna2.transition_signatures
            .push("GREETED:250->STARTTLS".to_string());

        let p1 = std::env::temp_dir().join("aether_dna1.json");
        let p2 = std::env::temp_dir().join("aether_dna2.json");
        std::fs::write(&p1, serde_json::to_string(&dna1).unwrap()).unwrap();
        std::fs::write(&p2, serde_json::to_string(&dna2).unwrap()).unwrap();

        let cmp = compare_dna(&p1, &p2).unwrap();
        assert!(!cmp.identical);
        assert_eq!(cmp.added_signatures, vec!["GREETED:250->STARTTLS"]);
        assert!(cmp.removed_signatures.is_empty());

        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
    }
}
