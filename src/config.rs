//! Configuration management for the crate checker application

use crate::{DEFAULT_API_URL, DEFAULT_SERVER_PORT, DEFAULT_TIMEOUT_SECS, DEFAULT_USER_AGENT};
use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

/// Main application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Logging configuration
    pub logging: LoggingConfig,

    /// Rate limiting configuration
    pub rate_limiting: RateLimitConfig,

    /// Crates.io API configuration
    pub crates_io: CratesIoConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Port to bind to
    #[serde(default = "default_port")]
    pub port: u16,

    /// Host to bind to
    #[serde(default = "default_host")]
    pub host: String,

    /// Number of worker threads
    #[serde(default = "default_workers")]
    pub workers: usize,

    /// Request timeout in seconds
    #[serde(default = "default_request_timeout")]
    pub request_timeout: u64,

    /// Enable CORS
    #[serde(default = "default_enable_cors")]
    pub enable_cors: bool,

    /// Enable request tracing
    #[serde(default = "default_enable_tracing")]
    pub enable_tracing: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    /// TTL for cache entries in seconds
    #[serde(default = "default_cache_ttl")]
    pub ttl_seconds: u64,

    /// Maximum number of cache entries
    #[serde(default = "default_cache_max_entries")]
    pub max_entries: usize,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format (json, pretty, compact)
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Optional log file path
    pub file: Option<String>,

    /// Enable structured logging
    #[serde(default = "default_structured_logging")]
    pub structured: bool,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Requests per minute limit
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,

    /// Burst size for rate limiting
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,

    /// Enable rate limiting
    #[serde(default = "default_rate_limiting_enabled")]
    pub enabled: bool,
}

/// Crates.io API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CratesIoConfig {
    /// API base URL
    #[serde(default = "default_api_url")]
    pub api_url: String,

    /// User agent for requests
    #[serde(default = "default_user_agent")]
    pub user_agent: String,

    /// Request timeout in seconds
    #[serde(default = "default_api_timeout")]
    pub timeout_seconds: u64,

    /// Maximum concurrent requests
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,

    /// Retry attempts for failed requests
    #[serde(default = "default_retry_attempts")]
    pub retry_attempts: u32,
}

// Default value functions
fn default_port() -> u16 {
    DEFAULT_SERVER_PORT
}
fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_workers() -> usize {
    num_cpus::get()
}
fn default_request_timeout() -> u64 {
    30
}
fn default_enable_cors() -> bool {
    true
}
fn default_enable_tracing() -> bool {
    true
}

fn default_cache_enabled() -> bool {
    true
}
fn default_cache_ttl() -> u64 {
    300
}
fn default_cache_max_entries() -> usize {
    1000
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "pretty".to_string()
}
fn default_structured_logging() -> bool {
    false
}

fn default_requests_per_minute() -> u32 {
    100
}
fn default_burst_size() -> u32 {
    20
}
fn default_rate_limiting_enabled() -> bool {
    false
}

fn default_api_url() -> String {
    DEFAULT_API_URL.to_string()
}
fn default_user_agent() -> String {
    DEFAULT_USER_AGENT.to_string()
}
fn default_api_timeout() -> u64 {
    DEFAULT_TIMEOUT_SECS
}
fn default_max_concurrent() -> usize {
    10
}
fn default_retry_attempts() -> u32 {
    3
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            cache: CacheConfig::default(),
            logging: LoggingConfig::default(),
            rate_limiting: RateLimitConfig::default(),
            crates_io: CratesIoConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            host: default_host(),
            workers: default_workers(),
            request_timeout: default_request_timeout(),
            enable_cors: default_enable_cors(),
            enable_tracing: default_enable_tracing(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: default_cache_enabled(),
            ttl_seconds: default_cache_ttl(),
            max_entries: default_cache_max_entries(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            file: None,
            structured: default_structured_logging(),
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: default_requests_per_minute(),
            burst_size: default_burst_size(),
            enabled: default_rate_limiting_enabled(),
        }
    }
}

impl Default for CratesIoConfig {
    fn default() -> Self {
        Self {
            api_url: default_api_url(),
            user_agent: default_user_agent(),
            timeout_seconds: default_api_timeout(),
            max_concurrent: default_max_concurrent(),
            retry_attempts: default_retry_attempts(),
        }
    }
}

impl AppConfig {
    /// Load configuration from file, environment, and defaults
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from_file(None::<std::path::PathBuf>)
    }

    /// Load configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(config_file: Option<P>) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        // Start with defaults
        builder = builder.add_source(Config::try_from(&AppConfig::default())?);

        // Add config file if provided
        if let Some(path) = config_file {
            let path = path.as_ref();
            if path.exists() {
                info!("Loading configuration from: {}", path.display());
                builder = builder.add_source(File::from(path));
            }
        }

        // Add environment variables with CRATE_CHECKER prefix
        builder = builder.add_source(
            Environment::with_prefix("CRATE_CHECKER")
                .separator("__")
                .try_parsing(true),
        );

        builder.build()?.try_deserialize()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.server.port == 0 {
            return Err("Server port cannot be 0".to_string());
        }

        if self.server.workers == 0 {
            return Err("Server workers cannot be 0".to_string());
        }

        if self.server.request_timeout == 0 {
            return Err("Request timeout cannot be 0".to_string());
        }

        if self.cache.enabled && self.cache.max_entries == 0 {
            return Err("Cache max entries cannot be 0 when caching is enabled".to_string());
        }

        if !["trace", "debug", "info", "warn", "error"].contains(&self.logging.level.as_str()) {
            return Err(format!("Invalid log level: {}", self.logging.level));
        }

        if !["json", "pretty", "compact"].contains(&self.logging.format.as_str()) {
            return Err(format!("Invalid log format: {}", self.logging.format));
        }

        if self.crates_io.timeout_seconds == 0 {
            return Err("API timeout cannot be 0".to_string());
        }

        if self.crates_io.max_concurrent == 0 {
            return Err("Max concurrent requests cannot be 0".to_string());
        }

        Ok(())
    }

    /// Create a sample configuration file
    pub fn create_sample_config() -> String {
        toml::to_string_pretty(&AppConfig::default())
            .unwrap_or_else(|_| "# Failed to generate sample config".to_string())
    }

    /// Get the bind address for the server
    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

/// Environment-specific configuration overrides
#[derive(Debug, Clone)]
pub struct EnvironmentConfig {
    pub is_development: bool,
    pub is_production: bool,
    pub is_test: bool,
}

impl EnvironmentConfig {
    pub fn detect() -> Self {
        let env = std::env::var("RUST_ENV")
            .or_else(|_| std::env::var("ENVIRONMENT"))
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase();

        Self {
            is_development: env == "development" || env == "dev",
            is_production: env == "production" || env == "prod",
            is_test: env == "test" || env == "testing",
        }
    }

    /// Apply environment-specific overrides to the configuration
    pub fn apply_overrides(&self, config: &mut AppConfig) {
        if self.is_development {
            config.logging.level = "debug".to_string();
            config.cache.enabled = false;
            config.rate_limiting.enabled = false;
        } else if self.is_production {
            config.logging.level = "info".to_string();
            config.logging.structured = true;
            config.cache.enabled = true;
            config.rate_limiting.enabled = true;
        } else if self.is_test {
            config.logging.level = "warn".to_string();
            config.cache.enabled = false;
            config.rate_limiting.enabled = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server.port, DEFAULT_SERVER_PORT);
        assert_eq!(config.crates_io.api_url, DEFAULT_API_URL);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_load_from_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().with_extension("toml");

        writeln!(
            temp_file,
            r#"
[server]
port = 8080
host = "127.0.0.1"

[logging]
level = "debug"
"#
        )
        .unwrap();

        // Copy the temp file to a .toml file
        std::fs::copy(temp_file.path(), &temp_path).unwrap();

        let config = AppConfig::load_from_file(Some(&temp_path)).unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.logging.level, "debug");

        // Clean up
        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_environment_overrides() {
        let env_config = EnvironmentConfig {
            is_development: true,
            is_production: false,
            is_test: false,
        };

        let mut config = AppConfig::default();
        env_config.apply_overrides(&mut config);

        assert_eq!(config.logging.level, "debug");
        assert!(!config.cache.enabled);
        assert!(!config.rate_limiting.enabled);
    }

    #[test]
    fn test_bind_address() {
        let config = AppConfig::default();
        assert!(config
            .bind_address()
            .contains(&config.server.port.to_string()));
    }

    #[test]
    fn test_create_sample_config() {
        let sample = AppConfig::create_sample_config();
        assert!(sample.contains("[server]"));
        assert!(sample.contains("[logging]"));
        assert!(sample.contains("[cache]"));
    }
}
