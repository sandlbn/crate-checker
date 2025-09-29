//! # Crate Checker
//!
//! Rust crate information retrieval tool that provides both a powerful
//! CLI interface and a library API for querying crates.io.
//!
//! ## Features
//!
//! - **Crate existence checking** - Quickly verify if a crate exists
//! - **Version information** - Get detailed version history and metadata
//! - **Dependency analysis** - Explore dependencies and their relationships
//! - **Download statistics** - Access download metrics and trends
//! - **Batch processing** - Process multiple crates efficiently
//! - **REST API server** - Run as an HTTP server for integration
//! - **Multiple output formats** - JSON, YAML, Table, CSV, and compact
//! - **Async/concurrent** - Built on Tokio for excellent performance
//!
//! ## Quick Start
//!
//! Add to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! crate-checker = "0.1.0"
//! ```
//!
//! ## Library Usage
//!
//! ### Basic Example
//!
//! ```rust,no_run
//! use crate_checker::{CrateClient, Result};
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create a client with default settings
//!     let client = CrateClient::new();
//!     
//!     // Check if a crate exists
//!     let exists = client.crate_exists("serde").await?;
//!     println!("Serde exists: {}", exists);
//!     
//!     // Get detailed information
//!     let info = client.get_crate_info("tokio").await?;
//!     println!("Tokio version: {}", info.newest_version);
//!     println!("Downloads: {}", info.downloads);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Custom Client Configuration
//!
//! ```rust,no_run
//! use crate_checker::{CrateClient, Result};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<()> {
//! let client = CrateClient::builder()
//!     .base_url("https://crates.io/api/v1")
//!     .timeout(Duration::from_secs(30))
//!     .user_agent("my-app/1.0")
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! ### Batch Processing
//!
//! ```rust,no_run
//! use crate_checker::{CrateClient, Result};
//! use std::collections::HashMap;
//!
//! # async fn example() -> Result<()> {
//! let client = CrateClient::new();
//!
//! // Process multiple crates with specific versions
//! let mut versions = HashMap::new();
//! versions.insert("serde".to_string(), "0.1.0".to_string());
//! versions.insert("tokio".to_string(), "latest".to_string());
//!
//! let result = client.process_crate_version_map(versions).await?;
//! println!("Processed {} crates", result.total_processed);
//! println!("Successful: {}, Failed: {}", result.successful, result.failed);
//! # Ok(())
//! # }
//! ```
//!
//! ### Error Handling
//!
//! ```rust,no_run
//! use crate_checker::{CrateClient, CrateCheckerError, Result};
//!
//! # async fn example() -> Result<()> {
//! let client = CrateClient::new();
//!
//! match client.get_crate_info("unknown-crate").await {
//!     Ok(info) => println!("Found: {}", info.name),
//!     Err(CrateCheckerError::CrateNotFound(name)) => {
//!         println!("Crate '{}' not found", name);
//!     }
//!     Err(e) if e.is_recoverable() => {
//!         println!("Temporary error, can retry: {}", e);
//!     }
//!     Err(e) => {
//!         println!("Error: {}", e.user_message());
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## CLI Usage
//!
//! The crate-checker binary provides a comprehensive command-line interface:
//!
//! ```bash
//! # Check if a crate exists
//! crate-checker check serde
//!
//! # Get detailed information with dependencies
//! crate-checker info tokio --deps --stats
//!
//! # Search for crates
//! crate-checker search "http client" --limit 10
//!
//! # Check multiple crates
//! crate-checker check-multiple serde tokio reqwest
//!
//! # Start API server
//! crate-checker server --port 8080
//! ```
//!
//! ## API Server
//!
//! Run crate-checker as an HTTP server:
//!
//! ```bash
//! crate-checker server --port 8080 --cors
//! ```
//!
//! Available endpoints:
//! - `GET /health` - Health check
//! - `GET /api/crates/{name}` - Get crate info
//! - `GET /api/search?q={query}` - Search crates
//! - `POST /api/batch` - Batch processing
//!
//! ## Configuration
//!
//! Configure via file (`crate-checker.toml`) or environment variables:
//!
//! ```toml
//! [server]
//! port = 8080
//! host = "0.0.0.0"
//!
//! [cache]
//! enabled = true
//! ttl_seconds = 300
//!
//! [crates_io]
//! timeout_seconds = 30
//! ```
//!
//! Environment variables: `CRATE_CHECKER__SECTION__KEY`
//!
//! ## Performance Tips
//!
//! - Enable caching to reduce API calls
//! - Use batch operations for multiple crates
//! - Configure appropriate timeouts
//! - Use parallel processing when available
//!
//! ## Examples
//!
//! See the `examples/` directory for more usage patterns:
//! - `basic_usage.rs` - Simple API usage
//! - `batch_processing.rs` - Batch operations
//! - `monitor_updates.rs` - Version monitoring
//! - `custom_client.rs` - Advanced configuration

pub mod cli;
pub mod client;
pub mod config;
pub mod error;
pub mod server;
pub mod types;
pub mod utils;

// Re-export commonly used items at the crate root for convenience
pub use client::{CrateClient, CrateClientBuilder};
pub use error::{CrateCheckerError, Result};
pub use types::{
    BatchInput, BatchOperation, BatchRequest, BatchResponse, BatchResult, BatchTarget,
    CrateCheckResult, CrateInfo, CrateSearchResult, CrateStatus, Dependency, DownloadStats, Owner,
    Version, VersionDownload,
};

// Re-export configuration types for server users
pub use config::{AppConfig, EnvironmentConfig};

/// Default crates.io API base URL
pub const DEFAULT_API_URL: &str = "https://crates.io/api/v1";

/// Default user agent for requests
pub const DEFAULT_USER_AGENT: &str = "crate-checker/0.1.0";

/// Default request timeout in seconds
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Default server port
pub const DEFAULT_SERVER_PORT: u16 = 3000;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Library name
pub const NAME: &str = env!("CARGO_PKG_NAME");
