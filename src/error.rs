//! Error types for the crate checker application

use thiserror::Error;

/// Result type alias for crate checker operations
pub type Result<T> = std::result::Result<T, CrateCheckerError>;

/// Main error type for the crate checker application
#[derive(Error, Debug)]
pub enum CrateCheckerError {
    /// HTTP request failed
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    /// JSON serialization/deserialization failed
    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    /// YAML serialization/deserialization failed
    #[error("YAML parsing failed: {0}")]
    YamlError(#[from] serde_yaml::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(#[from] config::ConfigError),

    /// IO operation failed
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Crate not found
    #[error("Crate '{0}' not found")]
    CrateNotFound(String),

    /// Version not found
    #[error("Version '{version}' not found for crate '{crate_name}'")]
    VersionNotFound { crate_name: String, version: String },

    /// Invalid crate name
    #[error("Invalid crate name: '{0}'. Crate names must match the pattern: {1}")]
    InvalidCrateName(String, &'static str),

    /// API rate limit exceeded
    #[error("API rate limit exceeded. Please try again later")]
    RateLimitExceeded,

    /// Server error from crates.io API
    #[error("Server error: {status} - {message}")]
    ServerError { status: u16, message: String },

    /// Timeout error
    #[error("Request timeout after {0} seconds")]
    Timeout(u64),

    /// Batch processing error
    #[error("Batch processing failed: {0}")]
    BatchError(String),

    /// Invalid batch input
    #[error("Invalid batch input: {0}")]
    InvalidBatchInput(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Generic application error
    #[error("Application error: {0}")]
    ApplicationError(String),

    /// Network connectivity error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// Service unavailable
    #[error("Service temporarily unavailable: {0}")]
    ServiceUnavailable(String),
}

impl CrateCheckerError {
    /// Create a new application error
    pub fn application<S: Into<String>>(message: S) -> Self {
        Self::ApplicationError(message.into())
    }

    /// Create a new validation error
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Self::ValidationError(message.into())
    }

    /// Create a new batch error
    pub fn batch<S: Into<String>>(message: S) -> Self {
        Self::BatchError(message.into())
    }

    /// Create a new network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::NetworkError(message.into())
    }

    /// Check if this error is recoverable (i.e., worth retrying)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::HttpError(_)
                | Self::NetworkError(_)
                | Self::Timeout(_)
                | Self::ServiceUnavailable(_)
                | Self::RateLimitExceeded
        )
    }

    /// Get the HTTP status code if this error represents an HTTP error
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::ServerError { status, .. } => Some(*status),
            Self::CrateNotFound(_) | Self::VersionNotFound { .. } => Some(404),
            Self::RateLimitExceeded => Some(429),
            Self::AuthError(_) => Some(401),
            Self::ValidationError(_) | Self::InvalidCrateName(_, _) => Some(400),
            Self::ServiceUnavailable(_) => Some(503),
            _ => None,
        }
    }

    /// Convert to a user-friendly message
    pub fn user_message(&self) -> String {
        match self {
            Self::CrateNotFound(name) => {
                format!("The crate '{}' does not exist on crates.io", name)
            }
            Self::VersionNotFound {
                crate_name,
                version,
            } => {
                format!(
                    "Version '{}' of crate '{}' was not found",
                    version, crate_name
                )
            }
            Self::InvalidCrateName(name, pattern) => {
                format!(
                    "'{}' is not a valid crate name. Names must match: {}",
                    name, pattern
                )
            }
            Self::RateLimitExceeded => {
                "You've exceeded the API rate limit. Please wait a moment before trying again."
                    .to_string()
            }
            Self::NetworkError(_) => {
                "Network connection failed. Please check your internet connection.".to_string()
            }
            Self::ServiceUnavailable(_) => {
                "The crates.io service is temporarily unavailable. Please try again later."
                    .to_string()
            }
            _ => self.to_string(),
        }
    }
}

/// Convert reqwest status codes to appropriate errors
impl From<reqwest::StatusCode> for CrateCheckerError {
    fn from(status: reqwest::StatusCode) -> Self {
        match status.as_u16() {
            404 => Self::ValidationError("Resource not found".to_string()),
            429 => Self::RateLimitExceeded,
            500..=599 => Self::ServiceUnavailable(format!("Server error: {}", status)),
            _ => Self::ServerError {
                status: status.as_u16(),
                message: status
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string(),
            },
        }
    }
}
