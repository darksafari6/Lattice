use std::net::IpAddr;
use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::error::AetherError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TestCategory {
    PassiveObservation,
    LocalFixture,
    SafeTransitionChecks,
    AuthorizedActiveDiscovery,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_second: u32,
    pub max_concurrency: u32,
    pub timeout_secs: u64,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_second: 10,
            max_concurrency: 4,
            timeout_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeFile {
    pub version: String,
    pub owner: String,
    pub purpose: String,
    pub permitted_targets: Vec<String>,
    #[serde(default)]
    pub excluded_targets: Vec<String>,
    pub allowed_test_categories: Vec<TestCategory>,
    #[serde(default)]
    pub rate_limits: RateLimits,
    pub expires_at: DateTime<Utc>,
}

impl ScopeFile {
    pub fn default_template() -> Self {
        Self {
            version: "1.0".to_string(),
            owner: "security-researcher".to_string(),
            purpose: "Authorized local lab protocol research".to_string(),
            permitted_targets: vec!["local://smtp-fixture".to_string(), "127.0.0.1/32".to_string()],
            excluded_targets: vec![],
            allowed_test_categories: vec![
                TestCategory::PassiveObservation,
                TestCategory::LocalFixture,
                TestCategory::SafeTransitionChecks,
            ],
            rate_limits: RateLimits::default(),
            expires_at: Utc::now() + Duration::days(30),
        }
    }

    pub fn load(path: &Path) -> Result<Self, AetherError> {
        let content = std::fs::read_to_string(path)?;
        let scope: Self = serde_yaml::from_str(&content)?;
        Ok(scope)
    }

    pub fn save(&self, path: &Path) -> Result<(), AetherError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), AetherError> {
        if self.owner.trim().is_empty() {
            return Err(AetherError::Validation("scope owner must not be empty".into()));
        }
        if self.purpose.trim().is_empty() {
            return Err(AetherError::Validation("scope purpose must not be empty".into()));
        }
        if self.permitted_targets.is_empty() {
            return Err(AetherError::Validation(
                "scope must define at least one permitted target".into(),
            ));
        }
        if self.expires_at <= Utc::now() {
            return Err(AetherError::AuthorizationDenied(format!(
                "scope expired at {}",
                self.expires_at.to_rfc3339()
            )));
        }
        if self.rate_limits.requests_per_second == 0 || self.rate_limits.max_concurrency == 0 {
            return Err(AetherError::Validation(
                "rate_limits.requests_per_second and max_concurrency must be > 0".into(),
            ));
        }
        Ok(())
    }

    pub fn is_target_permitted(&self, target: &str) -> Result<bool, AetherError> {
        self.validate()?;
        let normalized = strip_port(target);

        for excluded in &self.excluded_targets {
            if target_matches(excluded, target, normalized) {
                return Ok(false);
            }
        }

        for permitted in &self.permitted_targets {
            if target_matches(permitted, target, normalized) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

fn strip_port(target: &str) -> &str {
    if target.starts_with("local://") {
        return target;
    }
    if let Some((host, port)) = target.rsplit_once(':') {
        if !host.contains(':') && port.parse::<u16>().is_ok() {
            return host;
        }
    }
    target
}

fn target_matches(pattern: &str, full_target: &str, host_only: &str) -> bool {
    if pattern == full_target || pattern == host_only {
        return true;
    }
    if let (Some((net_str, prefix_str)), Ok(ip)) = (pattern.split_once('/'), host_only.parse::<IpAddr>()) {
        if let (Ok(net_ip), Ok(prefix)) = (net_str.parse::<IpAddr>(), prefix_str.parse::<u32>()) {
            return ip_in_cidr(ip, net_ip, prefix);
        }
    }
    false
}

fn ip_in_cidr(ip: IpAddr, net: IpAddr, prefix: u32) -> bool {
    match (ip, net) {
        (IpAddr::V4(ip4), IpAddr::V4(net4)) if prefix <= 32 => {
            let mask = if prefix == 0 { 0 } else { !0u32 << (32 - prefix) };
            (u32::from(ip4) & mask) == (u32::from(net4) & mask)
        }
        (IpAddr::V6(ip6), IpAddr::V6(net6)) if prefix <= 128 => {
            let mask = if prefix == 0 { 0 } else { !0u128 << (128 - prefix) };
            (u128::from(ip6) & mask) == (u128::from(net6) & mask)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scope_is_valid() {
        let scope = ScopeFile::default_template();
        assert!(scope.validate().is_ok());
        assert!(scope.is_target_permitted("local://smtp-fixture").unwrap());
        assert!(scope.is_target_permitted("127.0.0.1:25").unwrap());
        assert!(!scope.is_target_permitted("192.0.2.10").unwrap());
    }

    #[test]
    fn test_excluded_target_takes_precedence() {
        let mut scope = ScopeFile::default_template();
        scope.permitted_targets = vec!["192.0.2.0/24".to_string()];
        scope.excluded_targets = vec!["192.0.2.50".to_string()];
        assert!(scope.is_target_permitted("192.0.2.10").unwrap());
        assert!(!scope.is_target_permitted("192.0.2.50").unwrap());
    }

    #[test]
    fn test_expired_scope_is_denied() {
        let mut scope = ScopeFile::default_template();
        scope.expires_at = Utc::now() - Duration::hours(1);
        let err = scope.validate().unwrap_err();
        assert!(matches!(err, AetherError::AuthorizationDenied(_)));
    }
}
