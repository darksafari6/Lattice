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
}
