use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to create a command for testing
fn crate_checker_cmd() -> Command {
    Command::cargo_bin("crate-checker").unwrap()
}

/// Test basic help output
#[test]
fn test_help_output() {
    crate_checker_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("comprehensive tool"))
        .stdout(predicate::str::contains("Check if a crate exists"));
}

/// Test version output
#[test]
fn test_version_output() {
    crate_checker_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("1.0.0"));
}

/// Test checking an existing crate
#[test]
fn test_check_existing_crate() {
    crate_checker_cmd()
        .args(["check", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test checking a non-existing crate
#[test]
fn test_check_non_existing_crate() {
    crate_checker_cmd()
        .args(["check", "this-crate-definitely-does-not-exist-12345"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure()
        .code(1);
}

/// Test JSON output format
#[test]
fn test_json_output_format() {
    crate_checker_cmd()
        .args(["--format", "json", "check", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"crate\""))
        .stdout(predicate::str::contains("\"exists\""));
}

/// Test YAML output format
#[test]
fn test_yaml_output_format() {
    crate_checker_cmd()
        .args(["--format", "yaml", "check", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("crate:"))
        .stdout(predicate::str::contains("exists:"));
}

/// Test checking multiple crates
#[test]
fn test_check_multiple_crates() {
    crate_checker_cmd()
        .args(["check-multiple", "serde", "tokio"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("SUMMARY"))
        .stdout(predicate::str::contains("Total checked: 2"));
}

/// Test check multiple with summary only
#[test]
fn test_check_multiple_summary_only() {
    crate_checker_cmd()
        .args(["check-multiple", "serde", "tokio", "--summary-only"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("SUMMARY"))
        .stdout(predicate::str::contains("Total checked: 2"));
}

/// Test check multiple with fail on missing
#[test]
fn test_check_multiple_fail_on_missing() {
    crate_checker_cmd()
        .args([
            "check-multiple",
            "serde",
            "this-crate-definitely-does-not-exist-12345",
            "--fail-on-missing",
        ])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .failure()
        .code(1);
}

/// Test check multiple JSON output
#[test]
fn test_check_multiple_json_output() {
    crate_checker_cmd()
        .args(["--format", "json", "check-multiple", "serde", "tokio"])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"results\""))
        .stdout(predicate::str::contains("\"summary\""));
}

/// Test getting crate info
#[test]
fn test_crate_info() {
    crate_checker_cmd()
        .args(["info", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("serde"));
}

/// Test crate info with dependencies
#[test]
fn test_crate_info_with_deps() {
    crate_checker_cmd()
        .args(["info", "serde", "--deps"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test getting crate versions
#[test]
fn test_crate_versions() {
    crate_checker_cmd()
        .args(["versions", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("Version"));
}

/// Test versions with limit
#[test]
fn test_crate_versions_with_limit() {
    crate_checker_cmd()
        .args(["versions", "serde", "--limit", "5"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test searching crates
#[test]
fn test_search_crates() {
    crate_checker_cmd()
        .args(["search", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("serde"));
}

/// Test search with limit
#[test]
fn test_search_with_limit() {
    crate_checker_cmd()
        .args(["search", "http", "--limit", "5"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test getting dependencies
#[test]
fn test_crate_deps() {
    crate_checker_cmd()
        .args(["deps", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test getting download stats
#[test]
fn test_crate_stats() {
    crate_checker_cmd()
        .args(["stats", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("Downloads"));
}

/// Test batch processing with JSON input
#[test]
fn test_batch_json_input() {
    let json_input = r#"{"serde": "latest", "tokio": "latest"}"#;

    crate_checker_cmd()
        .args(["batch", "--json", json_input])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

/// Test batch processing with file input
#[test]
fn test_batch_file_input() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("batch_input.json");

    let json_content = r#"{"serde": "latest", "tokio": "latest"}"#;
    fs::write(&file_path, json_content).unwrap();

    crate_checker_cmd()
        .args(["batch", "--file", file_path.to_str().unwrap()])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

/// Test batch with crates list format
#[test]
fn test_batch_crates_list() {
    let json_input = r#"{"crates": ["serde", "tokio"]}"#;

    crate_checker_cmd()
        .args(["batch", "--json", json_input])
        .timeout(std::time::Duration::from_secs(60))
        .assert()
        .success();
}

/// Test configuration generation
#[test]
fn test_config_generation() {
    crate_checker_cmd()
        .args(["config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[server]"))
        .stdout(predicate::str::contains("[logging]"));
}

/// Test config output to file
#[test]
fn test_config_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.toml");

    crate_checker_cmd()
        .args(["config", "--output", config_path.to_str().unwrap()])
        .assert()
        .success();

    assert!(config_path.exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("[server]"));
}

/// Test examples command
#[test]
fn test_examples_command() {
    crate_checker_cmd()
        .args(["examples"])
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON Batch Input Examples"))
        .stdout(predicate::str::contains("Crate version map"));
}

/// Test verbose output
#[test]
fn test_verbose_output() {
    crate_checker_cmd()
        .args(["--verbose", "check", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test quiet mode
#[test]
fn test_quiet_mode() {
    crate_checker_cmd()
        .args(["--quiet", "check", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test custom timeout
#[test]
fn test_custom_timeout() {
    crate_checker_cmd()
        .args(["--timeout", "10s", "check", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test invalid timeout format
#[test]
fn test_invalid_timeout() {
    crate_checker_cmd()
        .args(["--timeout", "invalid", "check", "serde"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid timeout format"));
}

/// Test checking specific version
#[test]
fn test_check_specific_version() {
    crate_checker_cmd()
        .args(["check", "serde", "--version", "1.0.100"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test invalid crate name
#[test]
fn test_invalid_crate_name() {
    crate_checker_cmd()
        .args(["check", "invalid@crate#name"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a valid crate name"));
}

/// Test empty crate name
#[test]
fn test_empty_crate_name() {
    crate_checker_cmd().args(["check", ""]).assert().failure();
}

/// Test batch with invalid JSON
#[test]
fn test_batch_invalid_json() {
    let invalid_json = r#"{"invalid": json"#;

    crate_checker_cmd()
        .args(["batch", "--json", invalid_json])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

/// Test batch with empty input
#[test]
fn test_batch_empty_input() {
    let empty_json = r#"{}"#;

    crate_checker_cmd()
        .args(["batch", "--json", empty_json])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));
}

/// Test check multiple with no arguments
#[test]
fn test_check_multiple_no_args() {
    crate_checker_cmd()
        .args(["check-multiple"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "At least one crate name must be provided",
        ));
}

/// Test config with custom file
#[test]
fn test_custom_config_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("custom_config.toml");

    // Create a custom config file
    let config_content = r#"
[server]
port = 8080
host = "127.0.0.1"

[logging]
level = "debug"
"#;
    fs::write(&config_path, config_content).unwrap();

    crate_checker_cmd()
        .args(["--config", config_path.to_str().unwrap(), "check", "serde"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test with custom API URL (using a mock or test environment)
#[test]
fn test_custom_api_url() {
    // This test would fail with an invalid URL, but tests the argument parsing
    crate_checker_cmd()
        .args(["--api-url", "https://example.com/api", "check", "serde"])
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .failure(); // Expected to fail due to invalid API
}

/// Integration test for complete workflow
#[test]
fn test_complete_workflow() {
    // Test a complete workflow: check -> info -> versions -> deps
    let crate_name = "serde";

    // First check if crate exists
    crate_checker_cmd()
        .args(["check", crate_name])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Get info
    crate_checker_cmd()
        .args(["info", crate_name])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Get versions
    crate_checker_cmd()
        .args(["versions", crate_name, "--limit", "3"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();

    // Get dependencies
    crate_checker_cmd()
        .args(["deps", crate_name])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}

/// Test CSV output format
#[test]
fn test_csv_output() {
    crate_checker_cmd()
        .args(["--format", "csv", "search", "serde", "--limit", "3"])
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}
