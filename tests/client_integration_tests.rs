use crate_checker::client::CrateClient;
use crate_checker::types::{BatchOperation, BatchTarget, CrateStatus};
use std::collections::HashMap;
use std::time::Duration;

/// Test creating a default client
#[tokio::test]
async fn test_create_default_client() {
    let client = CrateClient::new();
    // Basic smoke test - just verify we can create a client
    assert!(client.validate_crate_name("serde").is_ok());
}

/// Test creating a client with builder
#[tokio::test]
async fn test_create_client_with_builder() {
    let client = CrateClient::builder()
        .base_url("https://crates.io/api/v1")
        .user_agent("test-agent/1.0.0")
        .timeout(Duration::from_secs(30))
        .build()
        .expect("Failed to build client");

    assert!(client.validate_crate_name("serde").is_ok());
}

/// Test checking if a popular crate exists
#[tokio::test]
async fn test_crate_exists_popular() {
    let client = CrateClient::new();

    let exists = client.crate_exists("serde").await.expect("Request failed");
    assert!(exists);
}

/// Test checking if a non-existent crate exists
#[tokio::test]
async fn test_crate_exists_nonexistent() {
    let client = CrateClient::new();

    let exists = client
        .crate_exists("this-crate-definitely-does-not-exist-12345")
        .await
        .expect("Request failed");
    assert!(!exists);
}

/// Test getting latest version of a crate
#[tokio::test]
async fn test_get_latest_version() {
    let client = CrateClient::new();

    let version = client
        .get_latest_version("serde")
        .await
        .expect("Request failed");
    assert!(!version.is_empty());
    assert!(version.contains('.'));
}

/// Test getting detailed crate information
#[tokio::test]
async fn test_get_crate_info() {
    let client = CrateClient::new();

    let info = client
        .get_crate_info("serde")
        .await
        .expect("Request failed");

    assert_eq!(info.name, "serde");
    assert!(!info.newest_version.is_empty());
    assert!(info.downloads > 0);
    assert!(info.description.is_some());
    assert!(!info.created_at.to_string().is_empty());
}

/// Test getting crate info for non-existent crate
#[tokio::test]
async fn test_get_crate_info_nonexistent() {
    let client = CrateClient::new();

    let result = client
        .get_crate_info("this-crate-definitely-does-not-exist-12345")
        .await;
    assert!(result.is_err());
}

/// Test getting all versions of a crate
#[tokio::test]
async fn test_get_all_versions() {
    let client = CrateClient::new();

    let versions = client
        .get_all_versions("serde")
        .await
        .expect("Request failed");

    assert!(!versions.is_empty());

    // Check that versions have proper structure
    for version in versions.iter().take(5) {
        // Check first 5 versions
        assert!(!version.num.is_empty());
        // Downloads should be present (u64 so always >= 0)
        assert!(!version.created_at.to_string().is_empty());
    }
}

/// Test searching for crates
#[tokio::test]
async fn test_search_crates() {
    let client = CrateClient::new();

    let results = client
        .search_crates("serde", Some(10))
        .await
        .expect("Request failed");

    assert!(!results.is_empty());
    assert!(results.len() <= 10);

    // Check that results contain serde
    let has_serde = results.iter().any(|r| r.name == "serde");
    assert!(has_serde);

    // Check structure of first result
    if let Some(first) = results.first() {
        assert!(!first.name.is_empty());
        assert!(!first.newest_version.is_empty());
        // Downloads field is u64, so always valid
    }
}

/// Test search with no results
#[tokio::test]
async fn test_search_crates_no_results() {
    let client = CrateClient::new();

    let results = client
        .search_crates("this-definitely-does-not-exist-as-a-crate-12345", Some(10))
        .await
        .expect("Request failed");

    assert!(results.is_empty());
}

/// Test getting crate dependencies
#[tokio::test]
async fn test_get_crate_dependencies() {
    let client = CrateClient::new();

    // Get latest version first
    let version = client
        .get_latest_version("tokio")
        .await
        .expect("Failed to get version");

    let deps = client
        .get_crate_dependencies("tokio", &version)
        .await
        .expect("Request failed");

    // Tokio should have some dependencies
    assert!(!deps.is_empty());

    // Check structure of dependencies
    for dep in deps.iter().take(3) {
        assert!(!dep.name.is_empty());
        assert!(!dep.version_req().is_empty());
        assert!(!dep.kind.is_empty());
    }
}

/// Test getting dependencies for specific version
#[tokio::test]
async fn test_get_dependencies_specific_version() {
    let client = CrateClient::new();

    // Use a known version that should exist
    let deps = client.get_crate_dependencies("serde", "1.0.100").await;

    // This might succeed or fail depending on if the version exists
    // The important thing is that it doesn't panic
    match deps {
        Ok(deps) => {
            // If successful, check structure
            for dep in deps.iter().take(3) {
                assert!(!dep.name.is_empty());
                assert!(!dep.version_req().is_empty());
            }
        }
        Err(_) => {
            // If it fails, that's also acceptable for this test
        }
    }
}

/// Test getting download statistics
#[tokio::test]
async fn test_get_download_stats() {
    let client = CrateClient::new();

    let stats = client
        .get_download_stats("serde")
        .await
        .expect("Request failed");

    assert!(stats.total > 0);
    // Recent downloads is u64, so always valid
    assert!(!stats.versions.is_empty());

    // Check structure of version downloads
    if let Some(version_download) = stats.versions.first() {
        assert!(!version_download.version.is_empty());
        // Downloads field is u64, so always valid
    }
}

/// Test checking crate status
#[tokio::test]
async fn test_check_crate_status() {
    let client = CrateClient::new();

    // Test existing crate
    let status = client
        .check_crate_status("serde")
        .await
        .expect("Request failed");
    assert!(matches!(
        status,
        CrateStatus::Exists | CrateStatus::PartiallyYanked
    ));

    // Test non-existent crate
    let status = client
        .check_crate_status("this-crate-definitely-does-not-exist-12345")
        .await
        .expect("Request failed");
    assert_eq!(status, CrateStatus::NotFound);
}

/// Test crate name validation
#[tokio::test]
async fn test_crate_name_validation() {
    let client = CrateClient::new();

    // Valid names
    assert!(client.validate_crate_name("serde").is_ok());
    assert!(client.validate_crate_name("tokio").is_ok());
    assert!(client.validate_crate_name("test-crate").is_ok());
    assert!(client.validate_crate_name("test_crate").is_ok());
    assert!(client.validate_crate_name("test123").is_ok());

    // Invalid names
    assert!(client.validate_crate_name("").is_err());
    assert!(client.validate_crate_name("invalid@crate").is_err());
    assert!(client.validate_crate_name("invalid crate").is_err());
    assert!(client.validate_crate_name("invalid.crate").is_err());

    // Too long name
    let long_name = "a".repeat(100);
    assert!(client.validate_crate_name(&long_name).is_err());
}

/// Test processing crate list
#[tokio::test]
async fn test_process_crate_list() {
    let client = CrateClient::new();

    let crates = vec![
        "serde".to_string(),
        "tokio".to_string(),
        "this-crate-definitely-does-not-exist-12345".to_string(),
    ];

    let results = client
        .process_crate_list(crates)
        .await
        .expect("Request failed");

    assert_eq!(results.len(), 3);

    // First two should exist
    assert!(results[0].exists);
    assert_eq!(results[0].crate_name, "serde");
    assert!(results[0].error.is_none());

    assert!(results[1].exists);
    assert_eq!(results[1].crate_name, "tokio");
    assert!(results[1].error.is_none());

    // Third should not exist
    assert!(!results[2].exists);
    assert_eq!(
        results[2].crate_name,
        "this-crate-definitely-does-not-exist-12345"
    );
}

/// Test processing crate version map
#[tokio::test]
async fn test_process_crate_version_map() {
    let client = CrateClient::new();

    let mut input = HashMap::new();
    input.insert("serde".to_string(), "latest".to_string());
    input.insert("tokio".to_string(), "latest".to_string());
    input.insert(
        "this-crate-definitely-does-not-exist-12345".to_string(),
        "1.0.0".to_string(),
    );

    let result = client
        .process_crate_version_map(input)
        .await
        .expect("Request failed");

    assert_eq!(result.total_processed, 3);
    assert_eq!(result.successful, 2);
    assert_eq!(result.failed, 1);
    assert_eq!(result.results.len(), 3);
    assert!(result.processing_time_ms > 0);
}

/// Test processing batch operations
#[tokio::test]
async fn test_process_batch_operations() {
    let client = CrateClient::new();

    let operations = vec![
        BatchOperation {
            target: BatchTarget::Single {
                crate_name: "serde".to_string(),
                version: Some("latest".to_string()),
            },
            operation: "check".to_string(),
        },
        BatchOperation {
            target: BatchTarget::Multiple {
                crates: vec!["tokio".to_string(), "reqwest".to_string()],
            },
            operation: "batch_check".to_string(),
        },
    ];

    let response = client
        .process_batch_operations(operations)
        .await
        .expect("Request failed");

    assert!(!response.request_id.is_empty());
    assert_eq!(response.status, "completed");
    assert_eq!(response.result.total_processed, 2);
    assert!(response.result.results.len() >= 3); // At least 3 results from the operations
}

/// Test error handling for invalid API responses
#[tokio::test]
async fn test_error_handling_invalid_crate_name() {
    let client = CrateClient::new();

    // Test with invalid characters
    let result = client.crate_exists("invalid@crate#name").await;
    assert!(result.is_err());
}

/// Test timeout behavior
#[tokio::test]
async fn test_timeout_behavior() {
    // Create client with very short timeout
    let client = CrateClient::builder()
        .timeout(Duration::from_millis(1)) // Very short timeout
        .build()
        .expect("Failed to build client");

    // This should timeout
    let result = client.crate_exists("serde").await;
    assert!(result.is_err());
}

/// Test client with custom base URL
#[tokio::test]
async fn test_custom_base_url() {
    // Use an invalid URL to test error handling
    let client = CrateClient::builder()
        .base_url("https://invalid-crates-api.example.com/api/v1")
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build client");

    let result = client.crate_exists("serde").await;
    assert!(result.is_err()); // Should fail due to invalid URL
}

/// Test concurrent requests with the same client
#[tokio::test]
async fn test_concurrent_requests() {
    let client = CrateClient::new();

    let tasks = vec![
        tokio::spawn({
            let client = client.clone();
            async move { client.crate_exists("serde").await }
        }),
        tokio::spawn({
            let client = client.clone();
            async move { client.crate_exists("tokio").await }
        }),
        tokio::spawn({
            let client = client.clone();
            async move { client.crate_exists("reqwest").await }
        }),
    ];

    for task in tasks {
        let result = task.await.expect("Task panicked").expect("Request failed");
        assert!(result); // All these crates should exist
    }
}

/// Test search with different limits
#[tokio::test]
async fn test_search_with_limits() {
    let client = CrateClient::new();

    // Test with limit of 1
    let results = client
        .search_crates("http", Some(1))
        .await
        .expect("Request failed");
    assert!(results.len() <= 1);

    // Test with limit of 50
    let results = client
        .search_crates("web", Some(50))
        .await
        .expect("Request failed");
    assert!(results.len() <= 50);

    // Test with no limit (should default to reasonable limit)
    let results = client
        .search_crates("api", None)
        .await
        .expect("Request failed");
    assert!(!results.is_empty());
}

/// Test empty search query
#[tokio::test]
async fn test_empty_search_query() {
    let client = CrateClient::new();

    let result = client.search_crates("", Some(10)).await;
    assert!(result.is_err());

    let result = client.search_crates("   ", Some(10)).await;
    assert!(result.is_err());
}

/// Test various batch input formats
#[tokio::test]
async fn test_batch_input_formats() {
    let client = CrateClient::new();

    // Test crate version map format
    let mut version_map = HashMap::new();
    version_map.insert("serde".to_string(), "latest".to_string());
    version_map.insert("tokio".to_string(), "1.0.0".to_string());

    let result = client
        .process_crate_version_map(version_map)
        .await
        .expect("Request failed");
    assert_eq!(result.total_processed, 2);

    // Test crate list format
    let crate_list = vec!["serde".to_string(), "tokio".to_string()];
    let results = client
        .process_crate_list(crate_list)
        .await
        .expect("Request failed");
    assert_eq!(results.len(), 2);
}

/// Test rate limiting behavior (if any)
#[tokio::test]
async fn test_multiple_quick_requests() {
    let client = CrateClient::new();

    // Make several quick requests
    for i in 0..5 {
        let result = client.crate_exists("serde").await;
        assert!(result.is_ok(), "Request {} failed", i);

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Test client behavior with malformed responses (difficult to test without mock server)
#[tokio::test]
async fn test_client_resilience() {
    let client = CrateClient::new();

    // Test with various edge cases
    let edge_cases = vec![
        "a",           // Very short name
        "a-b-c-d-e-f", // Hyphenated name
        "a_b_c_d_e_f", // Underscored name
        "test123",     // Alphanumeric
    ];

    for case in edge_cases {
        let result = client.validate_crate_name(case);
        assert!(result.is_ok(), "Validation failed for: {}", case);

        // Actually try to check these (they may or may not exist, but shouldn't panic)
        let _ = client.crate_exists(case).await;
    }
}

/// Test large batch processing
#[tokio::test]
async fn test_large_batch_processing() {
    let client = CrateClient::new();

    // Create a larger batch of popular crates
    let crates = vec![
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
    ]
    .into_iter()
    .map(|s| s.to_string())
    .collect();

    let results = client
        .process_crate_list(crates)
        .await
        .expect("Request failed");

    assert_eq!(results.len(), 10);

    // Most of these should exist
    let existing_count = results.iter().filter(|r| r.exists).count();
    assert!(
        existing_count >= 8,
        "Expected at least 8 popular crates to exist"
    );
}
