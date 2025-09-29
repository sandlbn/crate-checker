//! Batch processing examples for checking multiple crates efficiently
//!
//! Run with: `cargo run --example batch_processing`

use crate_checker::{BatchOperation, BatchTarget, CrateClient, Result};
use std::collections::HashMap;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Crate Checker Batch Processing Examples ===\n");

    let client = CrateClient::new();

    // Example 1: Process a list of crates
    example_crate_list(&client).await?;

    // Example 2: Process crates with specific versions
    example_version_map(&client).await?;

    // Example 3: Advanced batch operations
    example_batch_operations(&client).await?;

    // Example 4: Check project dependencies
    example_check_dependencies(&client).await?;

    println!("\n=== All batch examples completed! ===");
    Ok(())
}

async fn example_crate_list(client: &CrateClient) -> Result<()> {
    println!("1. Processing a list of crates:");
    println!("   Checking popular web frameworks...\n");

    let crates = vec![
        "axum".to_string(),
        "actix-web".to_string(),
        "rocket".to_string(),
        "warp".to_string(),
        "tide".to_string(),
    ];

    let start = Instant::now();
    let results = client.process_crate_list(crates.clone()).await?;
    let duration = start.elapsed();

    println!("   Results:");
    for result in &results {
        let status = if result.exists { "✓" } else { "✗" };
        let version = result.latest_version.as_deref().unwrap_or("N/A");
        println!("   {} {} - v{}", status, result.crate_name, version);
    }

    println!("\n   Processed {} crates in {:.2?}", crates.len(), duration);
    println!();

    Ok(())
}

async fn example_version_map(client: &CrateClient) -> Result<()> {
    println!("2. Checking specific crate versions:");
    println!("   Verifying project dependencies...\n");

    let mut version_map = HashMap::new();
    version_map.insert("serde".to_string(), "1.0.193".to_string());
    version_map.insert("tokio".to_string(), "1.35.0".to_string());
    version_map.insert("reqwest".to_string(), "0.11.23".to_string());
    version_map.insert("clap".to_string(), "4.4.11".to_string());
    version_map.insert("anyhow".to_string(), "latest".to_string());

    let start = Instant::now();
    let batch_result = client.process_crate_version_map(version_map).await?;
    let duration = start.elapsed();

    println!("   Results:");
    println!("   Total processed: {}", batch_result.total_processed);
    println!("   Successful: {}", batch_result.successful);
    println!("   Failed: {}", batch_result.failed);
    println!();

    for result in &batch_result.results {
        let status_icon = if result.exists { "✓" } else { "✗" };
        let version_check = if let Some(exists) = result.version_exists {
            if exists {
                "version found"
            } else {
                "version NOT found"
            }
        } else {
            "not checked"
        };

        println!(
            "   {} {} @ {} - {}",
            status_icon,
            result.crate_name,
            result.requested_version.as_deref().unwrap_or("latest"),
            version_check
        );
    }

    println!("\n   Processing time: {:.2?}", duration);
    println!();

    Ok(())
}

async fn example_batch_operations(client: &CrateClient) -> Result<()> {
    println!("3. Advanced batch operations:");
    println!("   Running mixed operations...\n");

    let operations = vec![
        // Check a single crate with specific version
        BatchOperation {
            target: BatchTarget::Single {
                crate_name: "serde".to_string(),
                version: Some("1.0.193".to_string()),
            },
            operation: "check_version".to_string(),
        },
        // Check latest version of a single crate
        BatchOperation {
            target: BatchTarget::Single {
                crate_name: "diesel".to_string(),
                version: None,
            },
            operation: "check_latest".to_string(),
        },
        // Check multiple crates at once
        BatchOperation {
            target: BatchTarget::Multiple {
                crates: vec![
                    "async-trait".to_string(),
                    "futures".to_string(),
                    "async-std".to_string(),
                ],
            },
            operation: "batch_check".to_string(),
        },
    ];

    let start = Instant::now();
    let response = client.process_batch_operations(operations).await?;
    let duration = start.elapsed();

    println!("   Batch Response:");
    println!("   Request ID: {}", response.request_id);
    println!("   Status: {}", response.status);
    println!("   Results processed: {}", response.result.total_processed);
    println!();

    // Display individual results
    for (i, result) in response.result.results.iter().enumerate() {
        let status = if result.exists {
            format!(
                "EXISTS (v{})",
                result.latest_version.as_deref().unwrap_or("unknown")
            )
        } else {
            "NOT FOUND".to_string()
        };
        println!("   {}. {} - {}", i + 1, result.crate_name, status);
    }

    println!("\n   Total processing time: {:.2?}", duration);
    println!();

    Ok(())
}

async fn example_check_dependencies(client: &CrateClient) -> Result<()> {
    println!("4. Checking project dependencies:");
    println!("   Analyzing common project dependencies...\n");

    // Simulate checking dependencies from a Cargo.toml
    let project_deps = vec![
        ("serde", "1.0"),
        ("serde_json", "1.0"),
        ("tokio", "1.35"),
        ("tracing", "0.1"),
        ("thiserror", "1.0"),
        ("anyhow", "1.0"),
        ("chrono", "0.4"),
        ("uuid", "1.6"),
    ];

    let mut all_exist = true;
    let mut outdated = Vec::new();

    for (crate_name, required_version) in project_deps {
        match client.get_latest_version(crate_name).await {
            Ok(latest) => {
                print!("   {} v{} ", crate_name, required_version);

                // Simple version comparison (just major version for this example)
                let latest_major = latest.split('.').next().unwrap_or("0");
                let required_major = required_version.split('.').next().unwrap_or("0");

                if latest_major != required_major {
                    println!("⚠️  (latest: v{})", latest);
                    outdated.push((crate_name, latest));
                } else {
                    println!("✓ (latest: v{})", latest);
                }
            }
            Err(_) => {
                println!("   {} v{} ✗ NOT FOUND", crate_name, required_version);
                all_exist = false;
            }
        }
    }

    println!("\n   Summary:");
    if all_exist {
        println!("   ✓ All dependencies exist on crates.io");
    } else {
        println!("   ⚠️  Some dependencies were not found");
    }

    if !outdated.is_empty() {
        println!("\n   Potentially outdated dependencies:");
        for (name, latest) in outdated {
            println!("   - {} could be updated to v{}", name, latest);
        }
    }

    Ok(())
}
