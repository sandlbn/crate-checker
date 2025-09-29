//! Basic usage examples for the crate-checker library
//!
//! Run with: `cargo run --example basic_usage`

use crate_checker::{CrateClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Crate Checker Basic Usage Examples ===\n");

    // Create a new client with default settings
    let client = CrateClient::new();

    // Example 1: Check if a crate exists
    println!("1. Checking if 'serde' exists:");
    let exists = client.crate_exists("serde").await?;
    println!("   Serde exists: {}\n", exists);

    // Example 2: Get the latest version
    println!("2. Getting latest version of 'tokio':");
    let version = client.get_latest_version("tokio").await?;
    println!("   Latest tokio version: {}\n", version);

    // Example 3: Get detailed crate information
    println!("3. Getting detailed info for 'reqwest':");
    let info = client.get_crate_info("reqwest").await?;
    println!("   Name: {}", info.name);
    println!("   Version: {}", info.newest_version);
    println!("   Downloads: {}", info.downloads);
    if let Some(desc) = &info.description {
        println!("   Description: {}", desc);
    }
    println!();

    // Example 4: Search for crates
    println!("4. Searching for HTTP client crates:");
    let results = client.search_crates("http client", Some(5)).await?;
    for (i, crate_info) in results.iter().enumerate() {
        println!(
            "   {}. {} v{} - {} downloads",
            i + 1,
            crate_info.name,
            crate_info.newest_version,
            crate_info.downloads
        );
    }
    println!();

    // Example 5: Get crate dependencies
    println!("5. Getting dependencies for 'clap':");
    let latest_version = client.get_latest_version("clap").await?;
    let deps = client
        .get_crate_dependencies("clap", &latest_version)
        .await?;

    let runtime_deps: Vec<_> = deps.iter().filter(|d| d.kind == "normal").collect();
    println!("   Runtime dependencies: {}", runtime_deps.len());
    for dep in runtime_deps.iter().take(5) {
        println!("   - {} {}", dep.name(), dep.version_req());
    }
    println!();

    // Example 6: Check crate status
    println!("6. Checking status of various crates:");
    let crates_to_check = vec!["serde", "non-existent-crate-12345"];
    for crate_name in crates_to_check {
        let status = client.check_crate_status(crate_name).await?;
        println!("   {}: {:?}", crate_name, status);
    }

    println!("\n=== Examples completed successfully! ===");
    Ok(())
}
