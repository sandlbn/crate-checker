//! Command-line interface for the crate checker application

use crate::client::CrateClient;
use crate::config::{AppConfig, EnvironmentConfig};
use crate::error::Result;
use crate::server::start_server;
use crate::types::*;
use crate::utils::{
    create_example_batch_inputs, format_download_count, parse_json_file, parse_json_input,
    parse_timeout, truncate_text, validate_batch_input,
};
use crate::DEFAULT_SERVER_PORT;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use serde_json;
use std::path::PathBuf;
use tabled::{Table, Tabled};
use tracing::{error, info, warn};

/// Crate Checker - A comprehensive Rust crate information retrieval tool
#[derive(Parser)]
#[command(
    name = "crate-checker",
    version = "1.0.0",
    about = "Check crate existence, versions, dependencies and more from crates.io",
    long_about = "A comprehensive tool for retrieving information about Rust crates from crates.io. 
Supports checking crate existence, getting version information, searching crates, 
batch operations, and running as an HTTP API server."
)]
pub struct Cli {
    /// Output format
    #[arg(short, long, global = true, value_enum, default_value = "table")]
    pub format: OutputFormat,

    /// Enable verbose output
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Enable quiet mode (only errors)
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Configuration file path
    #[arg(long, long, global = true)]
    pub config: Option<PathBuf>,

    /// Timeout for requests (e.g. 30s, 2m, 1h)
    #[arg(long, global = true)]
    pub timeout: Option<String>,

    /// Custom crates.io API URL
    #[arg(long, global = true)]
    pub api_url: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand)]
pub enum Commands {
    /// Check if a crate exists
    Check {
        /// Name of the crate to check
        crate_name: String,

        /// Specific version to check (optional)
        #[arg(short, long)]
        version: Option<String>,
    },

    /// Check multiple crates at once with merged output
    CheckMultiple {
        /// Names of the crates to check (space-separated)
        crate_names: Vec<String>,

        /// Show only summary (don't list individual results)
        #[arg(short, long)]
        summary_only: bool,

        /// Exit with error code if any crate doesn't exist
        #[arg(long)]
        fail_on_missing: bool,
    },

    /// Get detailed information about a crate
    Info {
        /// Name of the crate
        crate_name: String,

        /// Include dependency information
        #[arg(short, long)]
        deps: bool,

        /// Include download statistics
        #[arg(short, long)]
        stats: bool,
    },

    /// List all versions of a crate
    Versions {
        /// Name of the crate
        crate_name: String,

        /// Show only non-yanked versions
        #[arg(long)]
        no_yanked: bool,

        /// Limit number of versions to show
        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Search for crates by name or keywords
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Show only exact matches
        #[arg(short, long)]
        exact: bool,
    },

    /// Show dependencies for a crate version
    Deps {
        /// Name of the crate
        crate_name: String,

        /// Version (defaults to latest)
        #[arg(short, long)]
        version: Option<String>,

        /// Show only runtime dependencies
        #[arg(long)]
        runtime_only: bool,
    },

    /// Show download statistics for a crate
    Stats {
        /// Name of the crate
        crate_name: String,

        /// Show version-specific stats
        #[arg(short, long)]
        versions: bool,
    },

    /// Process multiple crates at once
    Batch {
        /// JSON string with batch input
        #[arg(long, long, conflicts_with = "file")]
        json: Option<String>,

        /// JSON file with batch input
        #[arg(long, long, conflicts_with = "json")]
        file: Option<PathBuf>,

        /// Process requests in parallel
        #[arg(short, long)]
        parallel: bool,
    },

    /// Start HTTP API server
    Server {
        /// Port to bind to
        #[arg(short, long, default_value_t = DEFAULT_SERVER_PORT)]
        port: u16,

        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Enable CORS
        #[arg(long)]
        cors: bool,

        /// Configuration file for server
        #[arg(short, long)]
        config: Option<PathBuf>,
    },

    /// Generate sample configuration file
    Config {
        /// Output file (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show examples of JSON batch input formats
    Examples,
}

/// Output format options
#[derive(ValueEnum, Clone, Debug, Default)]
pub enum OutputFormat {
    /// Human-readable table format
    #[default]
    Table,
    /// JSON format
    Json,
    /// YAML format
    Yaml,
    /// Compact text format
    Compact,
    /// CSV format
    Csv,
}

/// Tabled display for crate information
#[derive(Tabled)]
struct CrateInfoDisplay {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Downloads")]
    downloads: String,
    #[tabled(rename = "Description")]
    description: String,
}

/// Tabled display for version information
#[derive(Tabled)]
struct VersionDisplay {
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Downloads")]
    downloads: String,
    #[tabled(rename = "Published")]
    published: String,
    #[tabled(rename = "Yanked")]
    yanked: String,
}

/// Tabled display for search results
#[derive(Tabled)]
struct SearchResultDisplay {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Downloads")]
    downloads: String,
    #[tabled(rename = "Description")]
    description: String,
}

/// Tabled display for dependencies
#[derive(Tabled)]
struct DependencyDisplay {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Version")]
    version: String,
    #[tabled(rename = "Kind")]
    kind: String,
    #[tabled(rename = "Optional")]
    optional: String,
}

/// Tabled display for multi-check results
#[derive(Tabled)]
struct MultiCheckDisplay {
    #[tabled(rename = "Crate")]
    name: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Latest Version")]
    version: String,
}

/// Summary for multi-check results
#[derive(Serialize)]
struct MultiCheckSummary {
    total_checked: usize,
    existing: usize,
    missing: usize,
    existing_crates: Vec<String>,
    missing_crates: Vec<String>,
}

/// Run the CLI application
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, cli.quiet, &cli.format);

    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        AppConfig::load_from_file(Some(config_path))?
    } else {
        AppConfig::load()?
    };

    // Apply environment overrides
    let env_config = EnvironmentConfig::detect();
    let mut final_config = config;
    env_config.apply_overrides(&mut final_config);

    // Create client with configuration
    let mut client_builder = CrateClient::builder();

    if let Some(url) = &cli.api_url {
        client_builder = client_builder.base_url(url);
    } else {
        client_builder = client_builder.base_url(&final_config.crates_io.api_url);
    }

    if let Some(timeout_str) = &cli.timeout {
        let timeout = parse_timeout(timeout_str)?;
        client_builder = client_builder.timeout(timeout);
    } else {
        client_builder = client_builder.timeout(std::time::Duration::from_secs(
            final_config.crates_io.timeout_seconds,
        ));
    }

    let client = client_builder.build()?;

    // Execute command
    match cli.command {
        Commands::Check {
            crate_name,
            version,
        } => {
            handle_check(client, &crate_name, version.as_deref(), &cli.format).await?;
        }
        Commands::CheckMultiple {
            crate_names,
            summary_only,
            fail_on_missing,
        } => {
            handle_check_multiple(
                client,
                crate_names,
                summary_only,
                fail_on_missing,
                &cli.format,
            )
            .await?;
        }
        Commands::Info {
            crate_name,
            deps,
            stats,
        } => {
            handle_info(client, &crate_name, deps, stats, &cli.format).await?;
        }
        Commands::Versions {
            crate_name,
            no_yanked,
            limit,
        } => {
            handle_versions(client, &crate_name, no_yanked, limit, &cli.format).await?;
        }
        Commands::Search {
            query,
            limit,
            exact,
        } => {
            handle_search(client, &query, limit, exact, &cli.format).await?;
        }
        Commands::Deps {
            crate_name,
            version,
            runtime_only,
        } => {
            handle_deps(
                client,
                &crate_name,
                version.as_deref(),
                runtime_only,
                &cli.format,
            )
            .await?;
        }
        Commands::Stats {
            crate_name,
            versions,
        } => {
            handle_stats(client, &crate_name, versions, &cli.format).await?;
        }
        Commands::Batch {
            json,
            file,
            parallel,
        } => {
            handle_batch(
                client,
                json.as_deref(),
                file.as_deref(),
                parallel,
                &cli.format,
            )
            .await?;
        }
        Commands::Server {
            port,
            host,
            cors,
            config,
        } => {
            let mut server_config = final_config;
            server_config.server.port = port;
            server_config.server.host = host;
            server_config.server.enable_cors = cors;

            if let Some(config_path) = config {
                server_config = AppConfig::load_from_file(Some(config_path))?;
            }

            start_server(server_config).await?;
        }
        Commands::Config { output } => {
            handle_config(output.as_deref())?;
        }
        Commands::Examples => {
            handle_examples();
        }
    }

    Ok(())
}

/// Handle the check command
async fn handle_check(
    client: CrateClient,
    crate_name: &str,
    version: Option<&str>,
    format: &OutputFormat,
) -> Result<()> {
    if let Some(version) = version {
        // Check specific version
        let versions = client.get_all_versions(crate_name).await?;
        let version_exists = versions.iter().any(|v| v.num == version);

        let result = serde_json::json!({
            "crate": crate_name,
            "version": version,
            "exists": version_exists
        });

        output_result(&serde_json::to_value(result)?, format)?;

        if !version_exists {
            std::process::exit(1);
        }
    } else {
        // Check crate existence
        let exists = client.crate_exists(crate_name).await?;
        let result = serde_json::json!({
            "crate": crate_name,
            "exists": exists
        });

        output_result(&serde_json::to_value(&result)?, format)?;

        if !exists {
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Handle the check multiple command
async fn handle_check_multiple(
    client: CrateClient,
    crate_names: Vec<String>,
    summary_only: bool,
    fail_on_missing: bool,
    format: &OutputFormat,
) -> Result<()> {
    use crate::error::CrateCheckerError;

    if crate_names.is_empty() {
        return Err(CrateCheckerError::ValidationError(
            "At least one crate name must be provided".to_string(),
        ));
    }

    info!("Checking {} crates", crate_names.len());

    let mut existing_crates = Vec::new();
    let mut missing_crates = Vec::new();
    let mut results = Vec::new();

    // Check each crate
    for crate_name in &crate_names {
        match client.crate_exists(crate_name).await {
            Ok(exists) => {
                let version = if exists {
                    match client.get_latest_version(crate_name).await {
                        Ok(v) => v,
                        Err(_) => "unknown".to_string(),
                    }
                } else {
                    "N/A".to_string()
                };

                let status = if exists { "EXISTS" } else { "MISSING" };

                results.push(MultiCheckDisplay {
                    name: crate_name.clone(),
                    status: status.to_string(),
                    version,
                });

                if exists {
                    existing_crates.push(crate_name.clone());
                } else {
                    missing_crates.push(crate_name.clone());
                }
            }
            Err(e) => {
                error!("Error checking crate '{}': {}", crate_name, e);
                results.push(MultiCheckDisplay {
                    name: crate_name.clone(),
                    status: "ERROR".to_string(),
                    version: "N/A".to_string(),
                });
                missing_crates.push(crate_name.clone());
            }
        }
    }

    // Create summary
    let summary = MultiCheckSummary {
        total_checked: crate_names.len(),
        existing: existing_crates.len(),
        missing: missing_crates.len(),
        existing_crates: existing_crates.clone(),
        missing_crates: missing_crates.clone(),
    };

    // Output results based on format and options
    match format {
        OutputFormat::Table => {
            if !summary_only {
                println!("{}", Table::new(results));
                println!();
            }

            // Always show summary for table format
            println!("=== SUMMARY ===");
            println!("Total checked: {}", summary.total_checked);
            println!(
                "Existing: {} ({}%)",
                summary.existing,
                (summary.existing as f32 / summary.total_checked as f32 * 100.0).round()
            );
            println!(
                "Missing: {} ({}%)",
                summary.missing,
                (summary.missing as f32 / summary.total_checked as f32 * 100.0).round()
            );

            if !summary.existing_crates.is_empty() {
                println!("\nExisting crates:");
                for crate_name in &summary.existing_crates {
                    println!("  ✓ {}", crate_name);
                }
            }

            if !summary.missing_crates.is_empty() {
                println!("\nMissing crates:");
                for crate_name in &summary.missing_crates {
                    println!("  ✗ {}", crate_name);
                }
            }
        }
        _ => {
            let output_data = if summary_only {
                serde_json::to_value(&summary)?
            } else {
                serde_json::json!({
                    "results": results.into_iter().map(|r| serde_json::json!({
                        "crate": r.name,
                        "status": r.status,
                        "version": r.version
                    })).collect::<Vec<_>>(),
                    "summary": summary
                })
            };
            output_result(&output_data, format)?;
        }
    }

    // Exit with error if requested and there are missing crates
    if fail_on_missing && !missing_crates.is_empty() {
        std::process::exit(1);
    }

    Ok(())
}

/// Handle the info command
async fn handle_info(
    client: CrateClient,
    crate_name: &str,
    include_deps: bool,
    include_stats: bool,
    format: &OutputFormat,
) -> Result<()> {
    let info = client.get_crate_info(crate_name).await?;

    match format {
        OutputFormat::Table => {
            let display = CrateInfoDisplay {
                name: info.name.clone(),
                version: info.newest_version.clone(),
                downloads: format_download_count(info.downloads),
                description: info.description.as_deref().unwrap_or("N/A").to_string(),
            };
            println!("{}", Table::new([display]));

            if !info.keywords.is_empty() {
                println!("\nKeywords: {}", info.keywords.join(", "));
            }
            if !info.categories.is_empty() {
                println!("Categories: {}", info.categories.join(", "));
            }
            if let Some(repo) = &info.repository {
                println!("Repository: {}", repo);
            }
            if let Some(homepage) = &info.homepage {
                println!("Homepage: {}", homepage);
            }
        }
        _ => {
            let mut result = serde_json::to_value(&info)?;

            if include_deps {
                if let Ok(deps) = client
                    .get_crate_dependencies(crate_name, &info.newest_version)
                    .await
                {
                    result["dependencies"] = serde_json::to_value(deps)?;
                }
            }

            if include_stats {
                if let Ok(stats) = client.get_download_stats(crate_name).await {
                    result["download_stats"] = serde_json::to_value(stats)?;
                }
            }

            output_result(&result, format)?;
        }
    }

    Ok(())
}

/// Handle the versions command
async fn handle_versions(
    client: CrateClient,
    crate_name: &str,
    no_yanked: bool,
    limit: Option<usize>,
    format: &OutputFormat,
) -> Result<()> {
    let mut versions = client.get_all_versions(crate_name).await?;

    if no_yanked {
        versions.retain(|v| !v.yanked);
    }

    if let Some(limit) = limit {
        versions.truncate(limit);
    }

    match format {
        OutputFormat::Table => {
            let displays: Vec<VersionDisplay> = versions
                .into_iter()
                .map(|v| VersionDisplay {
                    version: v.num,
                    downloads: format_download_count(v.downloads),
                    published: v.created_at.format("%Y-%m-%d").to_string(),
                    yanked: if v.yanked { "Yes" } else { "No" }.to_string(),
                })
                .collect();
            println!("{}", Table::new(displays));
        }
        _ => {
            output_result(&serde_json::to_value(&versions)?, format)?;
        }
    }

    Ok(())
}

/// Handle the search command
async fn handle_search(
    client: CrateClient,
    query: &str,
    limit: usize,
    exact: bool,
    format: &OutputFormat,
) -> Result<()> {
    let mut results = client.search_crates(query, Some(limit)).await?;

    if exact {
        results.retain(|r| r.exact_match);
    }

    match format {
        OutputFormat::Table => {
            let displays: Vec<SearchResultDisplay> = results
                .into_iter()
                .map(|r| SearchResultDisplay {
                    name: r.name,
                    version: r.newest_version,
                    downloads: format_download_count(r.downloads),
                    description: truncate_text(r.description.as_deref().unwrap_or("N/A"), 50),
                })
                .collect();
            println!("{}", Table::new(displays));
        }
        _ => {
            output_result(&serde_json::to_value(&results)?, format)?;
        }
    }

    Ok(())
}

/// Handle the deps command
async fn handle_deps(
    client: CrateClient,
    crate_name: &str,
    version: Option<&str>,
    runtime_only: bool,
    format: &OutputFormat,
) -> Result<()> {
    let version = if let Some(v) = version {
        v.to_string()
    } else {
        client.get_latest_version(crate_name).await?
    };

    let mut deps = client.get_crate_dependencies(crate_name, &version).await?;

    if runtime_only {
        deps.retain(|d| d.kind == "normal");
    }

    match format {
        OutputFormat::Table => {
            let displays: Vec<DependencyDisplay> = deps
                .into_iter()
                .map(|d| {
                    DependencyDisplay {
                        name: d.name,
                        version: d.req, // Use req field directly
                        kind: d.kind,
                        optional: if d.optional { "Yes" } else { "No" }.to_string(),
                    }
                })
                .collect();
            println!("{}", Table::new(displays));
        }
        _ => {
            output_result(&serde_json::to_value(&deps)?, format)?;
        }
    }

    Ok(())
}

/// Handle the stats command
async fn handle_stats(
    client: CrateClient,
    crate_name: &str,
    show_versions: bool,
    format: &OutputFormat,
) -> Result<()> {
    let stats = client.get_download_stats(crate_name).await?;

    match format {
        OutputFormat::Table => {
            println!("Download Statistics for '{}':", crate_name);
            println!("Total Downloads: {}", format_download_count(stats.total));

            if show_versions && !stats.versions.is_empty() {
                println!("\nVersion Downloads:");
                let version_displays: Vec<_> = stats
                    .versions
                    .into_iter()
                    .take(10)
                    .map(|v| (v.version, format_download_count(v.downloads)))
                    .collect();

                for (version, downloads) in version_displays {
                    println!("  {}: {}", version, downloads);
                }
            }
        }
        _ => {
            output_result(&serde_json::to_value(&stats)?, format)?;
        }
    }

    Ok(())
}

/// Handle the batch command
async fn handle_batch(
    client: CrateClient,
    json: Option<&str>,
    file: Option<&std::path::Path>,
    parallel: bool,
    format: &OutputFormat,
) -> Result<()> {
    let batch_input = if let Some(json_str) = json {
        parse_json_input(json_str)?
    } else if let Some(file_path) = file {
        parse_json_file(file_path)?
    } else {
        return Err(crate::error::CrateCheckerError::ValidationError(
            "Either --json or --file must be provided".to_string(),
        ));
    };

    validate_batch_input(&batch_input)?;

    info!(
        "Processing batch request with {} mode",
        if parallel { "parallel" } else { "sequential" }
    );

    let result = match batch_input {
        BatchInput::CrateVersionMap(map) => client.process_crate_version_map(map).await?,
        BatchInput::CrateList { crates } => {
            let results = client.process_crate_list(crates).await?;
            BatchResult {
                results,
                total_processed: 0,
                successful: 0,
                failed: 0,
                processing_time_ms: 0,
            }
        }
        BatchInput::Operations { operations } => {
            client.process_batch_operations(operations).await?.result
        }
    };

    output_result(&serde_json::to_value(&result)?, format)?;

    Ok(())
}

/// Handle the config command
fn handle_config(output: Option<&std::path::Path>) -> Result<()> {
    let sample_config = AppConfig::create_sample_config();

    if let Some(path) = output {
        std::fs::write(path, sample_config)?;
        println!("Configuration written to: {}", path.display());
    } else {
        println!("{}", sample_config);
    }

    Ok(())
}

/// Handle the examples command
fn handle_examples() {
    println!("JSON Batch Input Examples:\n");

    let examples = create_example_batch_inputs();
    for (title, example) in examples {
        println!("{}:", title);
        println!("{}\n", example);
    }

    println!("Usage:");
    println!("  crate-checker batch --json '<json_string>'");
    println!("  crate-checker batch --file input.json");
}

/// Output a result in the specified format
fn output_result(value: &serde_json::Value, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        OutputFormat::Yaml => {
            println!("{}", serde_yaml::to_string(value)?);
        }
        OutputFormat::Compact => {
            println!("{}", serde_json::to_string(value)?);
        }
        OutputFormat::Csv => {
            // Simple CSV output for basic structures
            if let Some(array) = value.as_array() {
                if let Some(first) = array.first() {
                    if let Some(obj) = first.as_object() {
                        // Print headers
                        let headers: Vec<String> = obj.keys().map(|k| k.to_string()).collect();
                        println!("{}", headers.join(","));

                        // Print rows
                        for item in array {
                            if let Some(obj) = item.as_object() {
                                let values: Vec<_> = headers
                                    .iter()
                                    .map(|h| obj.get(h).and_then(|v| v.as_str()).unwrap_or("N/A"))
                                    .collect();
                                println!("{}", values.join(","));
                            }
                        }
                    }
                }
            } else {
                warn!("CSV format is only supported for array structures");
                println!("{}", serde_json::to_string_pretty(value)?);
            }
        }
        OutputFormat::Table => {
            // Table format should be handled by the individual command handlers
            println!("{}", serde_json::to_string_pretty(value)?);
        }
    }

    Ok(())
}

/// Initialize logging based on CLI flags
fn init_logging(verbose: bool, quiet: bool, format: &OutputFormat) {
    // For structured output formats (JSON, YAML, CSV), suppress logging to stdout
    // or set to quiet mode automatically to avoid interfering with output parsing
    let should_suppress = matches!(
        format,
        OutputFormat::Json | OutputFormat::Yaml | OutputFormat::Csv | OutputFormat::Compact
    );

    let level = if quiet || should_suppress {
        tracing::Level::ERROR
    } else if verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    // Configure logging to stderr to not interfere with stdout output
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_writer(std::io::stderr) // Always write logs to stderr
        .init();
}
