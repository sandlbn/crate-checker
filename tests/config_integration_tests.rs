use crate_checker::config::{AppConfig, EnvironmentConfig};
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::TempDir;

// Helper to clean up all environment variables
fn cleanup_env_vars() {
    env::remove_var("CRATE_CHECKER__SERVER__PORT");
    env::remove_var("CRATE_CHECKER__SERVER__HOST");
    env::remove_var("CRATE_CHECKER__LOGGING__LEVEL");
    env::remove_var("CRATE_CHECKER__CACHE__ENABLED");
    env::remove_var("CRATE_CHECKER__CRATES_IO__API_URL");
    env::remove_var("CRATE_CHECKER__CRATES_IO__TIMEOUT_SECONDS");
    env::remove_var("CRATE_CHECKER__CRATES_IO__MAX_CONCURRENT");
    env::remove_var("CRATE_CHECKER__RATE_LIMITING__REQUESTS_PER_MINUTE");
    env::remove_var("RUST_ENV");
    env::remove_var("ENVIRONMENT");
}

/// Test loading default configuration
#[test]
#[serial]
fn test_load_default_config() {
    cleanup_env_vars();

    let config = AppConfig::load().expect("Failed to load default config");

    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.host, "0.0.0.0");
    assert_eq!(config.crates_io.api_url, "https://crates.io/api/v1");
    assert_eq!(config.crates_io.timeout_seconds, 30);
    assert!(config.cache.enabled);
    assert!(!config.rate_limiting.enabled);

    cleanup_env_vars();
}

/// Test loading configuration from file
#[test]
#[serial]
fn test_load_config_from_file() {
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("test_config.toml");

    let config_content = r#"
[server]
port = 8080
host = "127.0.0.1"
workers = 8
request_timeout = 60
enable_cors = false

[cache]
enabled = false
ttl_seconds = 600
max_entries = 2000

[logging]
level = "debug"
format = "json"
structured = true

[rate_limiting]
enabled = true
requests_per_minute = 200
burst_size = 50

[crates_io]
api_url = "https://test.crates.io/api/v1"
timeout_seconds = 45
max_concurrent = 20
"#;

    fs::write(&config_path, config_content).expect("Failed to write config file");

    let config =
        AppConfig::load_from_file(Some(&config_path)).expect("Failed to load config from file");

    assert_eq!(config.server.port, 8080);
    assert_eq!(config.server.host, "127.0.0.1");
    assert_eq!(config.server.workers, 8);
    assert_eq!(config.server.request_timeout, 60);
    assert!(!config.server.enable_cors);

    assert!(!config.cache.enabled);
    assert_eq!(config.cache.ttl_seconds, 600);
    assert_eq!(config.cache.max_entries, 2000);

    assert_eq!(config.logging.level, "debug");
    assert_eq!(config.logging.format, "json");
    assert!(config.logging.structured);

    assert!(config.rate_limiting.enabled);
    assert_eq!(config.rate_limiting.requests_per_minute, 200);
    assert_eq!(config.rate_limiting.burst_size, 50);

    assert_eq!(config.crates_io.api_url, "https://test.crates.io/api/v1");
    assert_eq!(config.crates_io.timeout_seconds, 45);
    assert_eq!(config.crates_io.max_concurrent, 20);

    cleanup_env_vars();
}

/// Test partial configuration file (should merge with defaults)
#[test]
#[serial]
fn test_partial_config_file() {
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("partial_config.toml");

    let config_content = r#"
[server]
port = 9000

[logging]
level = "warn"
"#;

    fs::write(&config_path, config_content).expect("Failed to write config file");

    let config =
        AppConfig::load_from_file(Some(&config_path)).expect("Failed to load config from file");

    // Custom values should be loaded
    assert_eq!(config.server.port, 9000);
    assert_eq!(config.logging.level, "warn");

    // Default values should remain
    assert_eq!(config.server.host, "0.0.0.0"); // default
    assert_eq!(config.crates_io.timeout_seconds, 30); // default

    cleanup_env_vars();
}

/// Test environment variable overrides
#[test]
#[serial]
fn test_environment_overrides() {
    cleanup_env_vars();

    // Set up environment variables
    env::set_var("CRATE_CHECKER__SERVER__PORT", "7777");
    env::set_var("CRATE_CHECKER__SERVER__HOST", "localhost");
    env::set_var("CRATE_CHECKER__LOGGING__LEVEL", "trace");
    env::set_var("CRATE_CHECKER__CACHE__ENABLED", "false");

    let config = AppConfig::load().expect("Failed to load config with env vars");

    assert_eq!(config.server.port, 7777);
    assert_eq!(config.server.host, "localhost");
    assert_eq!(config.logging.level, "trace");
    assert!(!config.cache.enabled);

    cleanup_env_vars();
}

/// Test configuration validation
#[test]
#[serial]
fn test_config_validation() {
    cleanup_env_vars();

    let mut config = AppConfig::default();

    // Valid config should pass
    assert!(config.validate().is_ok());

    // Invalid port
    config.server.port = 0;
    assert!(config.validate().is_err());
    assert!(config.validate().unwrap_err().contains("port cannot be 0"));

    // Reset to valid state
    config.server.port = 3000;
    assert!(config.validate().is_ok());

    // Invalid workers count
    config.server.workers = 0;
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .contains("workers cannot be 0"));

    // Reset to valid state
    config.server.workers = 4;
    assert!(config.validate().is_ok());

    // Invalid log level
    config.logging.level = "invalid".to_string();
    assert!(config.validate().is_err());
    assert!(config.validate().unwrap_err().contains("Invalid log level"));

    // Reset to valid state
    config.logging.level = "info".to_string();
    assert!(config.validate().is_ok());

    // Invalid log format
    config.logging.format = "invalid".to_string();
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .contains("Invalid log format"));

    // Reset to valid state
    config.logging.format = "pretty".to_string();
    assert!(config.validate().is_ok());

    // Invalid cache config (enabled but max_entries is 0)
    config.cache.enabled = true;
    config.cache.max_entries = 0;
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .contains("Cache max entries cannot be 0"));

    // Reset to valid state
    config.cache.max_entries = 1000;
    assert!(config.validate().is_ok());

    // Invalid timeout
    config.crates_io.timeout_seconds = 0;
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .contains("API timeout cannot be 0"));

    // Reset to valid state
    config.crates_io.timeout_seconds = 30;
    assert!(config.validate().is_ok());

    // Invalid max concurrent
    config.crates_io.max_concurrent = 0;
    assert!(config.validate().is_err());
    assert!(config
        .validate()
        .unwrap_err()
        .contains("Max concurrent requests cannot be 0"));

    cleanup_env_vars();
}

/// Test creating sample configuration
#[test]
#[serial]
fn test_create_sample_config() {
    cleanup_env_vars();

    let sample = AppConfig::create_sample_config();

    assert!(sample.contains("[server]"));
    assert!(sample.contains("port = 3000"));
    assert!(sample.contains("[cache]"));
    assert!(sample.contains("enabled = true"));
    assert!(sample.contains("[logging]"));
    assert!(sample.contains("level = \"info\""));
    assert!(sample.contains("[rate_limiting]"));
    assert!(sample.contains("[crates_io]"));
    assert!(sample.contains("api_url = \"https://crates.io/api/v1\""));

    // Verify it's valid TOML by trying to parse it back
    let parsed: toml::Value = toml::from_str(&sample).expect("Sample config is not valid TOML");
    assert!(parsed.get("server").is_some());
    assert!(parsed.get("cache").is_some());
    assert!(parsed.get("logging").is_some());
    assert!(parsed.get("rate_limiting").is_some());
    assert!(parsed.get("crates_io").is_some());

    cleanup_env_vars();
}

/// Test bind address generation
#[test]
#[serial]
fn test_bind_address() {
    cleanup_env_vars();

    let mut config = AppConfig::default();

    config.server.host = "127.0.0.1".to_string();
    config.server.port = 8080;

    assert_eq!(config.bind_address(), "127.0.0.1:8080");

    config.server.host = "0.0.0.0".to_string();
    config.server.port = 3000;

    assert_eq!(config.bind_address(), "0.0.0.0:3000");

    cleanup_env_vars();
}

/// Test environment detection
#[test]
#[serial]
fn test_environment_detection() {
    cleanup_env_vars();

    // Test default (development) environment
    env::remove_var("RUST_ENV");
    env::remove_var("ENVIRONMENT");

    let env_config = EnvironmentConfig::detect();
    assert!(env_config.is_development);
    assert!(!env_config.is_production);
    assert!(!env_config.is_test);

    // Test production environment
    env::set_var("RUST_ENV", "production");
    let env_config = EnvironmentConfig::detect();
    assert!(!env_config.is_development);
    assert!(env_config.is_production);
    assert!(!env_config.is_test);

    // Test test environment - need to clear RUST_ENV first
    env::remove_var("RUST_ENV"); // Clear RUST_ENV so it doesn't override ENVIRONMENT
    env::set_var("ENVIRONMENT", "test");
    let env_config = EnvironmentConfig::detect();
    assert!(!env_config.is_development);
    assert!(!env_config.is_production);
    assert!(env_config.is_test);

    cleanup_env_vars();
}

/// Test environment-specific overrides
#[test]
#[serial]
fn test_environment_specific_overrides() {
    cleanup_env_vars();

    let mut config = AppConfig::default();

    // Test development overrides
    let env_config = EnvironmentConfig {
        is_development: true,
        is_production: false,
        is_test: false,
    };

    env_config.apply_overrides(&mut config);

    assert_eq!(config.logging.level, "debug");
    assert!(!config.cache.enabled);
    assert!(!config.rate_limiting.enabled);

    // Reset config
    config = AppConfig::default();

    // Test production overrides
    let env_config = EnvironmentConfig {
        is_development: false,
        is_production: true,
        is_test: false,
    };

    env_config.apply_overrides(&mut config);

    assert_eq!(config.logging.level, "info");
    assert!(config.logging.structured);
    assert!(config.cache.enabled);
    assert!(config.rate_limiting.enabled);

    // Reset config
    config = AppConfig::default();

    // Test test environment overrides
    let env_config = EnvironmentConfig {
        is_development: false,
        is_production: false,
        is_test: true,
    };

    env_config.apply_overrides(&mut config);

    assert_eq!(config.logging.level, "warn");
    assert!(!config.cache.enabled);
    assert!(!config.rate_limiting.enabled);

    cleanup_env_vars();
}

/// Test configuration file with invalid TOML
#[test]
#[serial]
fn test_invalid_toml_file() {
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("invalid_config.toml");

    let invalid_content = r#"
[server
port = 8080  # Missing closing bracket
"#;

    fs::write(&config_path, invalid_content).expect("Failed to write config file");

    let result = AppConfig::load_from_file(Some(&config_path));
    assert!(result.is_err());

    cleanup_env_vars();
}

/// Test configuration with missing file
#[test]
#[serial]
fn test_missing_config_file() {
    cleanup_env_vars();

    let non_existent_path = "/this/path/does/not/exist/config.toml";

    // Should succeed and use defaults when file doesn't exist
    let config = AppConfig::load_from_file(Some(non_existent_path))
        .expect("Should fall back to defaults when file doesn't exist");

    assert_eq!(config.server.port, 3000); // Should be default

    cleanup_env_vars();
}

/// Test nested environment variable configuration
#[test]
#[serial]
fn test_nested_env_vars() {
    cleanup_env_vars();

    env::set_var(
        "CRATE_CHECKER__CRATES_IO__API_URL",
        "https://custom.api.com/v1",
    );
    env::set_var("CRATE_CHECKER__CRATES_IO__TIMEOUT_SECONDS", "60");
    env::set_var("CRATE_CHECKER__CRATES_IO__MAX_CONCURRENT", "15");
    env::set_var("CRATE_CHECKER__RATE_LIMITING__REQUESTS_PER_MINUTE", "500");

    let config = AppConfig::load().expect("Failed to load config");

    assert_eq!(config.crates_io.api_url, "https://custom.api.com/v1");
    assert_eq!(config.crates_io.timeout_seconds, 60);
    assert_eq!(config.crates_io.max_concurrent, 15);
    assert_eq!(config.rate_limiting.requests_per_minute, 500);

    cleanup_env_vars();
}

/// Test configuration with complex file and environment mix
#[test]
#[serial]
fn test_file_and_env_precedence() {
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("mixed_config.toml");

    let config_content = r#"
[server]
port = 4000
host = "0.0.0.0"

[logging]
level = "error"
"#;

    fs::write(&config_path, config_content).expect("Failed to write config file");

    // Set env var that should override the file
    env::set_var("CRATE_CHECKER__SERVER__PORT", "5000");
    env::set_var("CRATE_CHECKER__LOGGING__LEVEL", "debug");

    let config = AppConfig::load_from_file(Some(&config_path)).expect("Failed to load config");

    // Environment variables should take precedence
    assert_eq!(config.server.port, 5000);
    assert_eq!(config.logging.level, "debug");

    // Values not overridden by env should come from file
    assert_eq!(config.server.host, "0.0.0.0");

    cleanup_env_vars();
}

/// Test default values for all configuration sections
#[test]
#[serial]
fn test_all_default_values() {
    cleanup_env_vars();

    let config = AppConfig::default();

    // Server defaults
    assert_eq!(config.server.port, 3000);
    assert_eq!(config.server.host, "0.0.0.0");
    assert!(config.server.workers > 0);
    assert_eq!(config.server.request_timeout, 30);
    assert!(config.server.enable_cors);
    assert!(config.server.enable_tracing);

    // Cache defaults
    assert!(config.cache.enabled);
    assert_eq!(config.cache.ttl_seconds, 300);
    assert_eq!(config.cache.max_entries, 1000);

    // Logging defaults
    assert_eq!(config.logging.level, "info");
    assert_eq!(config.logging.format, "pretty");
    assert!(config.logging.file.is_none());
    assert!(!config.logging.structured);

    // Rate limiting defaults
    assert_eq!(config.rate_limiting.requests_per_minute, 100);
    assert_eq!(config.rate_limiting.burst_size, 20);
    assert!(!config.rate_limiting.enabled);

    // Crates.io defaults
    assert_eq!(config.crates_io.api_url, "https://crates.io/api/v1");
    assert_eq!(config.crates_io.user_agent, "crate-checker/1.0.0");
    assert_eq!(config.crates_io.timeout_seconds, 30);
    assert_eq!(config.crates_io.max_concurrent, 10);
    assert_eq!(config.crates_io.retry_attempts, 3);

    cleanup_env_vars();
}

/// Test configuration serialization roundtrip
#[test]
#[serial]
fn test_config_serialization_roundtrip() {
    cleanup_env_vars();

    let original_config = AppConfig::default();

    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&original_config).expect("Failed to serialize config");

    // Write to temp file
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = temp_dir.path().join("roundtrip_config.toml");
    fs::write(&config_path, &toml_string).expect("Failed to write config file");

    // Load back from file without env vars interfering
    let loaded_config =
        AppConfig::load_from_file(Some(&config_path)).expect("Failed to load config from file");

    // Should be equivalent (we can't use PartialEq because of the nested structs)
    assert_eq!(original_config.server.port, loaded_config.server.port);
    assert_eq!(original_config.server.host, loaded_config.server.host);
    assert_eq!(original_config.cache.enabled, loaded_config.cache.enabled);
    assert_eq!(original_config.logging.level, loaded_config.logging.level);
    assert_eq!(
        original_config.crates_io.api_url,
        loaded_config.crates_io.api_url
    );

    cleanup_env_vars();
}

/// Test boolean environment variable parsing
#[test]
#[serial]
fn test_boolean_env_vars() {
    cleanup_env_vars();

    // Test various boolean representations
    let bool_tests = vec![
        ("true", true),
        ("false", false),
        ("1", true),
        ("0", false),
        ("yes", true),
        ("no", false),
        ("on", true),
        ("off", false),
    ];

    for (env_val, expected) in bool_tests {
        cleanup_env_vars(); // Clean between each test
        env::set_var("CRATE_CHECKER__CACHE__ENABLED", env_val);

        let config = AppConfig::load().expect("Failed to load config");
        assert_eq!(
            config.cache.enabled, expected,
            "Failed for env value: {}",
            env_val
        );
    }

    cleanup_env_vars();
}
