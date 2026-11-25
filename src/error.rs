use thiserror::Error;

/// Stable exit codes as defined by the AetherLattice specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    Success = 0,
    GeneralError = 1,
    InvalidArguments = 2,
    AuthorizationDenied = 3,
    DependencyUnavailable = 4,
    Timeout = 5,
    PartialResults = 6,
    ValidationOrMigrationFailure = 7,
    CancelledByUser = 8,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug, Error)]
pub enum AetherError {
    #[error("authorization or scope denied: {0}")]
    AuthorizationDenied(String),

    #[error("scope validation failed: {0}")]
    Validation(String),

    #[error("dependency or platform feature unavailable: {0}")]
    DependencyUnavailable(String),

    #[error("operation timed out: {0}")]
    Timeout(String),

    #[error("operation cancelled by user")]
    Cancelled,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("{0}")]
    General(String),
}

impl AetherError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AetherError::AuthorizationDenied(_) => ExitCode::AuthorizationDenied,
            AetherError::Validation(_)
            | AetherError::Yaml(_)
            | AetherError::Json(_)
            | AetherError::Sqlite(_) => ExitCode::ValidationOrMigrationFailure,
            AetherError::DependencyUnavailable(_) => ExitCode::DependencyUnavailable,
            AetherError::Timeout(_) => ExitCode::Timeout,
            AetherError::Cancelled => ExitCode::CancelledByUser,
            AetherError::Io(_) | AetherError::General(_) => ExitCode::GeneralError,
        }
    }
}
