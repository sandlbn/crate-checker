use assert_cmd::Command;
use crate_checker::config::AppConfig;
use crate_checker::server::start_server;
use predicates::prelude::*;
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Helper to create a command for testing
fn crate_checker_cmd() -> Command {
    Command::cargo_bin("crate-checker").unwrap()
}

/// Helper to start a test server on a random port
async fn start_test_server() -> (AppConfig, tokio::task::JoinHandle<()>) {
    let mut config = AppConfig::default();
    config.server.port = 0; // Let OS choose port
    config.server.host = "127.0.0.1".to_string();
    config.cache.enabled = false; // Disable cache for tests

    // Find available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    config.server.port = addr.port();
    drop(listener);

    let server_config = config.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = start_server(server_config).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    (config, handle)
}

/// End-to-end test: CLI -> API -> Response cycle
#[tokio::test]
async fn test_e2e_cli_to_api_workflow() {
    // Start server
    let (config, _handle) = start_test_server().await;

    // Test that server is running via CLI health check equivalent
    let client = Client::new();
    let health_url = format!(
        "http://{}:{}/health",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(10), client.get(&health_url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    // Test various API endpoints that mirror CLI functionality

    // Test crate existence check (equivalent to CLI check command)
    let crate_url = format!(
        "http://{}:{}/api/crates/serde",
        config.server.host, config.server.port
    );
    let response = timeout(Duration::from_secs(30), client.get(&crate_url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.expect("Invalid JSON");
    assert_eq!(body["name"], "serde");

    // Test search (equivalent to CLI search command)
    let search_url = format!(
        "http://{}:{}/api/search?q=serde&limit=5",
        config.server.host, config.server.port
    );
    let response = timeout(Duration::from_secs(30), client.get(&search_url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.expect("Invalid JSON");
    assert!(body.is_array());

    // Test batch processing (equivalent to CLI batch command)
    let batch_url = format!(
        "http://{}:{}/api/batch",
        config.server.host, config.server.port
    );
    let batch_input = serde_json::json!({
        "serde": "latest",
        "tokio": "latest"
    });

    let response = timeout(
        Duration::from_secs(60),
        client.post(&batch_url).json(&batch_input).send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    let body: Value = response.json().await.expect("Invalid JSON");
    assert_eq!(body["status"], "completed");
    assert!(body["total_processed"].as_u64().unwrap() >= 2);
}

/// End-to-end test: Full workflow from file input to output
#[test]
fn test_e2e_file_workflow() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create input file
    let input_file = temp_dir.path().join("batch_input.json");
    let batch_content = r#"{"serde": "latest", "tokio": "latest", "reqwest": "latest"}"#;
    fs::write(&input_file, batch_content).expect("Failed to write batch input");

    // Create output file
    let output_file = temp_dir.path().join("config_output.toml");

    // Test config generation to file
    crate_checker_cmd()
        .args(["config", "--output", output_file.to_str().unwrap()])
        .assert()
        .success();

    assert!(output_file.exists());
    let config_content = fs::read_to_string(&output_file).expect("Failed to read config file");
    assert!(config_content.contains("[server]"));

    // Test batch processing from file
    crate_checker_cmd()
        .args(["batch", "--file", input_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();

    // Test with custom config file
    crate_checker_cmd()
        .args(["--config", output_file.to_str().unwrap(), "check", "serde"])
        .timeout(Duration::from_secs(30))
        .assert()
        .success();
}

/// End-to-end test: Complete multi-check workflow
#[test]
fn test_e2e_multi_check_workflow() {
    // Test various combinations of existing and non-existing crates
    let test_cases = vec![
        // All existing crates
        vec!["serde", "tokio", "reqwest"],
        // Mix of existing and non-existing
        vec!["serde", "this-crate-does-not-exist-12345", "tokio"],
        // Single crate
        vec!["serde"],
    ];

    for crates in test_cases {
        let mut cmd = crate_checker_cmd();
        cmd.args(&["check-multiple"]);

        for crate_name in &crates {
            cmd.arg(crate_name);
        }

        cmd.timeout(Duration::from_secs(60))
            .assert()
            .success()
            .stdout(predicate::str::contains("SUMMARY"))
            .stdout(predicate::str::contains(&format!(
                "Total checked: {}",
                crates.len()
            )));
    }
}

/// End-to-end test: Error handling across the stack
#[test]
fn test_e2e_error_handling() {
    // Test various error conditions

    // Invalid crate name
    crate_checker_cmd()
        .args(["check", "invalid@crate#name"])
        .timeout(Duration::from_secs(30))
        .assert()
        .failure()
        .stderr(predicate::str::contains("is not a valid crate name"));

    // Invalid JSON for batch
    crate_checker_cmd()
        .args(["batch", "--json", "invalid json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));

    // Empty batch input
    crate_checker_cmd()
        .args(["batch", "--json", "{}"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be empty"));

    // Invalid timeout format
    crate_checker_cmd()
        .args(["--timeout", "invalid", "check", "serde"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid timeout format"));
}

/// End-to-end test: Performance and concurrency
#[tokio::test]
async fn test_e2e_performance_workflow() {
    // Start server
    let (config, _handle) = start_test_server().await;
    let client = Client::new();

    // Test concurrent API requests
    let mut tasks = Vec::new();

    for i in 0..10 {
        let client = client.clone();
        let config = config.clone();

        let task = tokio::spawn(async move {
            let url = format!(
                "http://{}:{}/api/crates/serde",
                config.server.host, config.server.port
            );

            let start = std::time::Instant::now();
            let response = timeout(Duration::from_secs(30), client.get(&url).send())
                .await
                .expect("Request timeout")
                .expect("Request failed");
            let duration = start.elapsed();

            assert_eq!(response.status(), 200);
            assert!(duration < Duration::from_secs(10)); // Should be reasonably fast

            (i, duration)
        });

        tasks.push(task);
    }

    // Wait for all tasks and verify performance
    let mut total_time = Duration::new(0, 0);
    for task in tasks {
        let (_, duration) = task.await.expect("Task failed");
        total_time += duration;
    }

    let avg_time = total_time / 10;
    assert!(avg_time < Duration::from_secs(5)); // Average response time should be reasonable
}

/// End-to-end test: Different output formats consistency
#[test]
fn test_e2e_output_formats() {
    let crate_name = "serde";
    let formats = vec!["json", "yaml", "table", "csv"];

    for format in formats {
        // Test check command with different formats
        // Use --quiet to suppress logs for structured formats
        let mut cmd = crate_checker_cmd();
        let args = if format == "json" || format == "yaml" || format == "csv" {
            vec!["--quiet", "--format", format, "check", crate_name]
        } else {
            vec!["--format", format, "check", crate_name]
        };

        cmd.args(&args).timeout(Duration::from_secs(30));

        let output = cmd.output().expect("Failed to execute command");
        assert!(
            output.status.success(),
            "Command failed for format: {}",
            format
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.is_empty(), "Empty output for format: {}", format);

        // Verify format-specific characteristics
        match format {
            "json" => {
                assert!(stdout.contains("\"crate\""));
                assert!(stdout.contains("\"exists\""));
                // Verify it's valid JSON
                let _: Value = serde_json::from_str(&stdout)
                    .expect(&format!("Invalid JSON output: {}", stdout));
            }
            "yaml" => {
                assert!(stdout.contains("crate:"));
                assert!(stdout.contains("exists:"));
                // Verify it's valid YAML
                let _: serde_yaml::Value = serde_yaml::from_str(&stdout)
                    .expect(&format!("Invalid YAML output: {}", stdout));
            }
            "table" => {
                // Table output should be human-readable
                // It may contain JSON if table format falls back to JSON for this command
                assert!(!stdout.is_empty());
            }
            "csv" => {
                // CSV output should contain comma-separated values or be simple output
                assert!(!stdout.is_empty());
            }
            _ => {}
        }
    }
}

/// End-to-end test: Multi-check with different scenarios
#[test]
fn test_e2e_multi_check_scenarios() {
    // Test scenario 1: All existing crates with summary only
    crate_checker_cmd()
        .args(["check-multiple", "serde", "tokio", "--summary-only"])
        .timeout(Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("Total checked: 2"))
        .stdout(predicate::str::contains("Existing: 2"));

    // Test scenario 2: Mix with fail-on-missing (should fail)
    crate_checker_cmd()
        .args([
            "check-multiple",
            "serde",
            "this-definitely-does-not-exist-12345",
            "--fail-on-missing",
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .failure()
        .code(1);

    // Test scenario 3: JSON output format
    crate_checker_cmd()
        .args(["--format", "json", "check-multiple", "serde", "tokio"])
        .timeout(Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"results\""))
        .stdout(predicate::str::contains("\"summary\""));
}

/// End-to-end test: Configuration and environment integration
#[test]
fn test_e2e_config_integration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config_file = temp_dir.path().join("test_config.toml");

    // Create a test configuration
    let config_content = r#"
[server]
port = 8080

[logging]
level = "debug"

[crates_io]
timeout_seconds = 45
"#;

    fs::write(&config_file, config_content).expect("Failed to write config");

    // Test with custom config
    crate_checker_cmd()
        .args([
            "--config",
            config_file.to_str().unwrap(),
            "--verbose",
            "check",
            "serde",
        ])
        .timeout(Duration::from_secs(30))
        .assert()
        .success();

    // Test config generation and validation cycle
    let generated_config = temp_dir.path().join("generated_config.toml");

    crate_checker_cmd()
        .args(["config", "--output", generated_config.to_str().unwrap()])
        .assert()
        .success();

    assert!(generated_config.exists());

    // Use generated config
    crate_checker_cmd()
        .args([
            "--config",
            generated_config.to_str().unwrap(),
            "check",
            "serde",
        ])
        .timeout(Duration::from_secs(30))
        .assert()
        .success();
}

/// End-to-end test: Batch processing with various input formats
#[test]
fn test_e2e_batch_variations() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Test 1: Crate version map format
    let version_map_file = temp_dir.path().join("version_map.json");
    let version_map_content = r#"{"serde": "latest", "tokio": "1.0.0"}"#;
    fs::write(&version_map_file, version_map_content).expect("Failed to write file");

    crate_checker_cmd()
        .args(["batch", "--file", version_map_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();

    // Test 2: Crate list format
    let crate_list_file = temp_dir.path().join("crate_list.json");
    let crate_list_content = r#"{"crates": ["serde", "tokio", "reqwest"]}"#;
    fs::write(&crate_list_file, crate_list_content).expect("Failed to write file");

    crate_checker_cmd()
        .args(["batch", "--file", crate_list_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();

    // Test 3: Operations format
    let operations_file = temp_dir.path().join("operations.json");
    let operations_content = r#"{
        "operations": [
            {"crate": "serde", "version": "latest", "operation": "check"},
            {"crates": ["tokio", "reqwest"], "operation": "batch_check"}
        ]
    }"#;
    fs::write(&operations_file, operations_content).expect("Failed to write file");

    crate_checker_cmd()
        .args(["batch", "--file", operations_file.to_str().unwrap()])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();
}

/// End-to-end test: API and CLI equivalence
#[tokio::test]
async fn test_e2e_api_cli_equivalence() {
    // Start server
    let (config, _handle) = start_test_server().await;
    let client = Client::new();

    let test_crate = "serde";

    // Test CLI check with --quiet flag to suppress logs for clean JSON output
    let cli_output = crate_checker_cmd()
        .args(["--quiet", "--format", "json", "check", test_crate])
        .timeout(Duration::from_secs(30))
        .output()
        .expect("CLI command failed");

    assert!(cli_output.status.success());
    let cli_result: Value =
        serde_json::from_slice(&cli_output.stdout).expect("Invalid CLI JSON output");

    // Test API equivalent
    let api_url = format!(
        "http://{}:{}/api/crates/{}",
        config.server.host, config.server.port, test_crate
    );
    let api_response = timeout(Duration::from_secs(30), client.get(&api_url).send())
        .await
        .expect("Request timeout")
        .expect("API request failed");

    assert_eq!(api_response.status(), 200);
    let api_result: Value = api_response.json().await.expect("Invalid API JSON");

    // Compare key fields (allowing for format differences)
    assert_eq!(cli_result["crate"], test_crate);
    assert_eq!(api_result["name"], test_crate);
    assert_eq!(cli_result["exists"], true);
    assert!(api_result["newest_version"].is_string());
}

/// End-to-end test: Stress test with large batch
#[test]
#[ignore] // This test takes a long time, run with --ignored flag
fn test_e2e_stress_large_batch() {
    let popular_crates = vec![
        "serde",
        "tokio",
        "reqwest",
        "clap",
        "anyhow",
        "thiserror",
        "chrono",
        "uuid",
        "rand",
        "regex",
        "log",
        "env_logger",
        "async-trait",
        "futures",
        "hyper",
        "tonic",
        "diesel",
        "sqlx",
        "actix-web",
        "axum",
        "warp",
        "rocket",
    ];

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let batch_file = temp_dir.path().join("large_batch.json");

    // Create batch input with all popular crates
    let batch_input = serde_json::json!({
        "crates": popular_crates
    });

    fs::write(
        &batch_file,
        serde_json::to_string_pretty(&batch_input).unwrap(),
    )
    .expect("Failed to write batch file");

    // Process large batch
    crate_checker_cmd()
        .args(["batch", "--file", batch_file.to_str().unwrap()])
        .timeout(Duration::from_secs(300)) // 5 minutes timeout for large batch
        .assert()
        .success();

    // Also test multi-check with the same crates
    let mut cmd = crate_checker_cmd();
    cmd.args(&["check-multiple"]);
    for crate_name in &popular_crates[..10] {
        // Test first 10 to avoid command line length limits
        cmd.arg(crate_name);
    }

    cmd.timeout(Duration::from_secs(180))
        .assert()
        .success()
        .stdout(predicate::str::contains("Total checked: 10"));
}

/// End-to-end test: Comprehensive workflow simulation
#[tokio::test]
async fn test_e2e_comprehensive_workflow() {
    // This test simulates a complete user workflow

    // Step 1: Check if some crates exist
    crate_checker_cmd()
        .args(["check-multiple", "serde", "tokio", "unknown-crate-12345"])
        .timeout(Duration::from_secs(60))
        .assert()
        .success();

    // Step 2: Get detailed info about existing crates
    crate_checker_cmd()
        .args(["info", "serde", "--deps", "--stats"])
        .timeout(Duration::from_secs(30))
        .assert()
        .success();

    // Step 3: Search for related crates
    crate_checker_cmd()
        .args(["search", "serialization", "--limit", "5"])
        .timeout(Duration::from_secs(30))
        .assert()
        .success();

    // Step 4: Process a batch of crates
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let batch_file = temp_dir.path().join("workflow_batch.json");
    let batch_content = r#"{"serde": "latest", "serde_json": "latest"}"#;
    fs::write(&batch_file, batch_content).expect("Failed to write batch file");

    crate_checker_cmd()
        .args([
            "batch",
            "--file",
            batch_file.to_str().unwrap(),
            "--format",
            "json",
        ])
        .timeout(Duration::from_secs(60))
        .assert()
        .success()
        .stdout(predicate::str::contains("results"));

    // Step 5: Start server and test API
    let (config, _handle) = start_test_server().await;
    let client = Client::new();

    // Test server health
    let health_url = format!(
        "http://{}:{}/health",
        config.server.host, config.server.port
    );
    let response = timeout(Duration::from_secs(10), client.get(&health_url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    // Test API batch processing
    let batch_url = format!(
        "http://{}:{}/api/batch",
        config.server.host, config.server.port
    );
    let api_batch_input = serde_json::json!({
        "serde": "latest",
        "tokio": "latest"
    });

    let response = timeout(
        Duration::from_secs(60),
        client.post(&batch_url).json(&api_batch_input).send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);
    let result: Value = response.json().await.expect("Invalid JSON");
    assert_eq!(result["status"], "completed");

    // Step 6: Get metrics from server
    let metrics_url = format!(
        "http://{}:{}/metrics",
        config.server.host, config.server.port
    );
    let response = timeout(Duration::from_secs(10), client.get(&metrics_url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);
    let metrics: Value = response.json().await.expect("Invalid JSON");
    assert!(metrics["requests_total"].as_u64().unwrap() > 0);
}
