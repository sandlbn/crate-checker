use crate_checker::config::AppConfig;
use crate_checker::server::start_server;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::timeout;

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

/// Test server health check endpoint
#[tokio::test]
async fn test_health_endpoint() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/health",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(10), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert_eq!(body["status"], "healthy");
    assert!(body["timestamp"].is_string());
    assert_eq!(body["version"], "1.0.0");
    assert!(body["uptime_seconds"].is_number());
}

/// Test API documentation endpoint
#[tokio::test]
async fn test_api_docs_endpoint() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!("http://{}:{}/", config.server.host, config.server.port);

    let response = timeout(Duration::from_secs(10), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body = response.text().await.expect("Failed to read response");
    assert!(body.contains("# Crate Checker API"));
    assert!(body.contains("Available Endpoints"));
}

/// Test metrics endpoint
#[tokio::test]
async fn test_metrics_endpoint() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/metrics",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(10), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert!(body["requests_total"].is_number());
    assert!(body["requests_successful"].is_number());
    assert!(body["requests_failed"].is_number());
    assert!(body["average_response_time_ms"].is_number());
    assert!(body["cache_hits"].is_number());
    assert!(body["cache_misses"].is_number());
    assert!(body["uptime_seconds"].is_number());
}

/// Test getting crate information via API
#[tokio::test]
async fn test_get_crate_info_api() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/crates/serde",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(30), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert_eq!(body["name"], "serde");
    assert!(body["newest_version"].is_string());
    assert!(body["downloads"].is_number());
    assert!(body["created_at"].is_string());
}

/// Test getting non-existent crate
#[tokio::test]
async fn test_get_nonexistent_crate() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/crates/this-crate-definitely-does-not-exist-12345",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(30), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 404);
}

/// Test checking specific version
#[tokio::test]
async fn test_get_crate_version_api() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/crates/serde/latest",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(30), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert_eq!(body["crate_name"], "serde");
    assert_eq!(body["exists"], true);
    assert!(body["latest_version"].is_string());
    assert_eq!(body["requested_version"], "latest");
    assert_eq!(body["version_exists"], true);
}

/// Test getting crate dependencies
#[tokio::test]
async fn test_get_crate_dependencies_api() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/crates/serde/latest/deps",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(30), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert!(body.is_array());
}

/// Test getting crate download stats
#[tokio::test]
async fn test_get_crate_stats_api() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/crates/serde/stats",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(30), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert!(body["total"].is_number());
    assert!(body["versions"].is_array());
}

/// Test crate search API
#[tokio::test]
async fn test_search_crates_api() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/search?q=serde&limit=5",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(30), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert!(body.is_array());
    assert!(body.as_array().unwrap().len() <= 5);

    // Check first result structure if any results
    if let Some(first) = body.as_array().unwrap().first() {
        assert!(first["name"].is_string());
        assert!(first["newest_version"].is_string());
        assert!(first["downloads"].is_number());
    }
}

/// Test search with missing query parameter
#[tokio::test]
async fn test_search_missing_query() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/search",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(10), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 400);
}

/// Test batch processing API
#[tokio::test]
async fn test_batch_processing_api() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/batch",
        config.server.host, config.server.port
    );

    let batch_input = serde_json::json!({
        "serde": "latest",
        "tokio": "latest"
    });

    let response = timeout(
        Duration::from_secs(60),
        client.post(&url).json(&batch_input).send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert!(body["request_id"].is_string());
    assert_eq!(body["status"], "completed");
    assert!(body["results"].is_array());
    assert!(body["total_processed"].is_number());
    assert!(body["successful"].is_number());
    assert!(body["failed"].is_number());
}

/// Test batch with crates list format
#[tokio::test]
async fn test_batch_crates_list_api() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/batch",
        config.server.host, config.server.port
    );

    let batch_input = serde_json::json!({
        "crates": ["serde", "tokio"]
    });

    let response = timeout(
        Duration::from_secs(60),
        client.post(&url).json(&batch_input).send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 200);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert_eq!(body["status"], "completed");
    assert!(
        body["total_processed"]
            .as_number()
            .unwrap()
            .as_u64()
            .unwrap()
            >= 2
    );
}

/// Test batch with invalid JSON
#[tokio::test]
async fn test_batch_invalid_json() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/batch",
        config.server.host, config.server.port
    );

    let invalid_json = r#"{"invalid": json"#;

    let response = timeout(
        Duration::from_secs(10),
        client
            .post(&url)
            .header("content-type", "application/json")
            .body(invalid_json)
            .send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 400);
}

/// Test batch with empty input
#[tokio::test]
async fn test_batch_empty_input() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/api/batch",
        config.server.host, config.server.port
    );

    let empty_input = serde_json::json!({});

    let response = timeout(
        Duration::from_secs(10),
        client.post(&url).json(&empty_input).send(),
    )
    .await
    .expect("Request timeout")
    .expect("Request failed");

    assert_eq!(response.status(), 400);
}

/// Test CORS headers when enabled
#[tokio::test]
async fn test_cors_headers() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/health",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(10), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);

    // Check for CORS headers
    let headers = response.headers();
    assert!(headers.contains_key("access-control-allow-origin"));
}

/// Test API error handling
#[tokio::test]
async fn test_api_error_responses() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();

    // Test 404 error
    let url = format!(
        "http://{}:{}/api/crates/non-existent-crate-12345",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(30), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 404);

    let body: Value = response.json().await.expect("Invalid JSON");
    assert!(body["error"].is_string());
    assert!(body["timestamp"].is_string());
}

/// Test server with different configurations
#[tokio::test]
async fn test_server_configuration() {
    let mut config = AppConfig::default();
    config.server.port = 0; // Let OS choose port
    config.server.host = "127.0.0.1".to_string();
    config.server.enable_cors = false;
    config.cache.enabled = true;
    config.cache.ttl_seconds = 60;
    config.cache.max_entries = 100;

    // Find available port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    config.server.port = addr.port();
    drop(listener);

    let server_config = config.clone();
    let _handle = tokio::spawn(async move {
        if let Err(e) = start_server(server_config).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = Client::new();
    let url = format!(
        "http://{}:{}/health",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(10), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 200);
}

/// Test concurrent requests
#[tokio::test]
async fn test_concurrent_requests() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();

    let mut tasks = Vec::new();

    // Launch multiple concurrent requests
    for i in 0..10 {
        let client = client.clone();
        let config = config.clone();

        let task = tokio::spawn(async move {
            let url = format!(
                "http://{}:{}/health",
                config.server.host, config.server.port
            );

            let response = timeout(Duration::from_secs(10), client.get(&url).send())
                .await
                .expect("Request timeout")
                .expect("Request failed");

            assert_eq!(response.status(), 200);
            i
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    for task in tasks {
        task.await.expect("Task failed");
    }
}

/// Test server graceful handling of malformed requests
#[tokio::test]
async fn test_malformed_requests() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();

    // Test invalid path
    let url = format!(
        "http://{}:{}/api/invalid/path",
        config.server.host, config.server.port
    );

    let response = timeout(Duration::from_secs(10), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    assert_eq!(response.status(), 404);
}

/// Test server response times under load
#[tokio::test]
async fn test_response_times() {
    let (config, _handle) = start_test_server().await;
    let client = Client::new();
    let url = format!(
        "http://{}:{}/health",
        config.server.host, config.server.port
    );

    let start = std::time::Instant::now();

    let response = timeout(Duration::from_secs(5), client.get(&url).send())
        .await
        .expect("Request timeout")
        .expect("Request failed");

    let duration = start.elapsed();

    assert_eq!(response.status(), 200);
    assert!(duration < Duration::from_millis(1000)); // Should respond within 1 second
}
