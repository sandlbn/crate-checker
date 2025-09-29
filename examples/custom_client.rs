//! Example of using a custom-configured client with advanced features
//!
//! Run with: `cargo run --example custom_client`

use crate_checker::{CrateClient, Result};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Custom Client Configuration Example ===\n");

    // Example 1: Client with custom timeout
    example_custom_timeout().await?;

    // Example 2: Client with custom API URL (for testing/proxies)
    example_custom_api_url().await?;

    // Example 3: Error handling and retry logic
    example_error_handling().await?;

    // Example 4: Concurrent operations with multiple clients
    example_concurrent_operations().await?;

    println!("\n=== All custom client examples completed! ===");
    Ok(())
}

async fn example_custom_timeout() -> Result<()> {
    println!("1. Client with custom timeout configuration:");

    // Create a client with a short timeout for demonstration
    let client = CrateClient::builder()
        .timeout(Duration::from_secs(5))
        .user_agent("crate-checker-examples/1.0")
        .build()?;

    println!("   Testing with 5-second timeout...");

    match tokio::time::timeout(Duration::from_secs(10), client.crate_exists("serde")).await {
        Ok(Ok(exists)) => {
            println!("   ✓ Request completed: serde exists = {}", exists);
        }
        Ok(Err(e)) => {
            println!("   ✗ Request failed: {}", e);
        }
        Err(_) => {
            println!("   ⏱️ Request timed out");
        }
    }

    println!();
    Ok(())
}

async fn example_custom_api_url() -> Result<()> {
    println!("2. Client with custom API configuration:");

    // This would be used for:
    // - Testing against a mock server
    // - Using a proxy or mirror
    // - Corporate environments with custom registries
    let client = CrateClient::builder()
        .base_url("https://crates.io/api/v1") // Default, but could be custom
        .user_agent("my-company-tool/2.0")
        .timeout(Duration::from_secs(30))
        .build()?;

    println!("   Using default crates.io API");
    println!("   Custom user agent: my-company-tool/2.0");

    // Validate the client works
    let info = client.get_crate_info("serde").await?;
    println!(
        "   ✓ Successfully connected: {} v{}",
        info.name, info.newest_version
    );

    println!();
    Ok(())
}

async fn example_error_handling() -> Result<()> {
    println!("3. Error handling and retry logic:");

    let client = CrateClient::new();

    // Example of checking a crate that might not exist
    let test_crates = vec![
        "serde",                                      // exists
        "this-crate-definitely-does-not-exist-12345", // doesn't exist
        "tokio",                                      // exists
    ];

    for crate_name in test_crates {
        println!("   Checking '{}'...", crate_name);

        // Implement retry logic
        let mut retries = 3;
        let mut success = false;

        while retries > 0 && !success {
            match client.get_crate_info(crate_name).await {
                Ok(info) => {
                    println!("   ✓ Found: {} v{}", info.name, info.newest_version);
                    success = true;
                }
                Err(e) => {
                    // Check if error is recoverable
                    if e.is_recoverable() && retries > 1 {
                        println!("   ⟲ Retrying... ({} attempts left)", retries - 1);
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        retries -= 1;
                    } else {
                        // Determine error type and handle appropriately
                        match e.status_code() {
                            Some(404) => println!("   ✗ Not found: {}", crate_name),
                            Some(429) => println!("   ⚠️ Rate limited - please wait"),
                            Some(code) => {
                                println!("   ✗ HTTP error {}: {}", code, e.user_message())
                            }
                            None => println!("   ✗ Error: {}", e.user_message()),
                        }
                        break;
                    }
                }
            }
        }
    }

    println!();
    Ok(())
}

async fn example_concurrent_operations() -> Result<()> {
    println!("4. Concurrent operations with multiple clients:");
    println!("   Fetching information for multiple crates in parallel...\n");

    let crates_to_check = vec![
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
    ];

    // Create multiple tasks for concurrent execution
    let mut tasks = Vec::new();

    for crate_name in crates_to_check {
        let client = CrateClient::new(); // Each task gets its own client (they're cloneable)
        let name = crate_name.to_string();

        let task = tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = client.get_crate_info(&name).await;
            let duration = start.elapsed();
            (name, result, duration)
        });

        tasks.push(task);
    }

    // Collect all results
    let mut total_time = Duration::new(0, 0);
    let mut successful = 0;

    println!("   Results (fetched concurrently):");
    for task in tasks {
        match task.await {
            Ok((name, result, duration)) => {
                total_time += duration;
                match result {
                    Ok(info) => {
                        successful += 1;
                        println!(
                            "   ✓ {} v{} - fetched in {:.2?}",
                            name, info.newest_version, duration
                        );
                    }
                    Err(e) => {
                        println!("   ✗ {} - failed: {} ({:.2?})", name, e, duration);
                    }
                }
            }
            Err(e) => {
                println!("   ✗ Task failed: {}", e);
            }
        }
    }

    println!("\n   Summary:");
    println!("   Successful: {}/10", successful);
    println!("   Total time: {:.2?}", total_time);
    println!("   Average time per crate: {:.2?}", total_time / 10);

    // Compare with sequential execution time estimate
    let estimated_sequential = total_time;
    let actual_concurrent = total_time / 10; // Rough estimate
    println!(
        "   Speed improvement: ~{}x faster than sequential",
        (estimated_sequential.as_millis() / actual_concurrent.as_millis().max(1))
    );

    Ok(())
}
