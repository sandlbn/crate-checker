//! Data types and structures for the crate checker application

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main crate information structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrateInfo {
    /// Crate name
    pub name: String,

    /// Crate description
    pub description: Option<String>,

    /// Latest version number
    pub newest_version: String,

    /// Total download count
    pub downloads: u64,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Homepage URL
    pub homepage: Option<String>,

    /// Repository URL
    pub repository: Option<String>,

    /// Documentation URL
    pub documentation: Option<String>,

    /// Associated keywords
    pub keywords: Vec<String>,

    /// Associated categories
    pub categories: Vec<String>,

    /// Maximum upload size in bytes
    pub max_upload_size: Option<u64>,

    /// License information
    pub license: Option<String>,

    /// Whether the crate is yanked
    pub yanked: Option<bool>,

    /// Links to various resources
    pub links: Option<CrateLinks>,
}

/// Links associated with a crate
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrateLinks {
    pub owner_team: Option<String>,
    pub owner_user: Option<String>,
    pub reverse_dependencies: Option<String>,
    pub version_downloads: Option<String>,
    pub versions: Option<String>,
}

/// Version information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Version {
    /// Version number
    pub num: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Download count for this version
    pub downloads: u64,

    /// Whether this version is yanked
    pub yanked: bool,

    /// Version ID
    pub id: Option<u64>,

    /// Crate size in bytes
    pub crate_size: Option<u64>,

    /// Published by user
    pub published_by: Option<User>,

    /// Audit actions
    pub audit_actions: Option<Vec<AuditAction>>,

    /// License information
    pub license: Option<String>,

    /// Links for this version
    pub links: Option<VersionLinks>,
}

/// User information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: Option<String>,
}

/// Audit action information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditAction {
    pub action: String,
    pub user: User,
    pub time: DateTime<Utc>,
}

/// Links for a specific version
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionLinks {
    pub dependencies: Option<String>,
    pub version_downloads: Option<String>,
    pub authors: Option<String>,
}

/// Crate status enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum CrateStatus {
    /// Crate exists and is available
    Exists,
    /// Crate was not found
    NotFound,
    /// All versions are yanked
    Yanked,
    /// Some versions are yanked
    PartiallyYanked,
}

/// Search result for crates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrateSearchResult {
    pub name: String,
    pub description: Option<String>,
    pub newest_version: String,
    pub downloads: u64,
    pub exact_match: bool,
}

/// Dependency information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dependency {
    #[serde(rename = "crate_id")]
    pub name: String,
    pub req: String,
    #[serde(default)]
    pub features: Vec<String>,
    pub optional: bool,
    pub default_features: bool,
    pub target: Option<String>,
    pub kind: String,
    #[serde(default)]
    pub downloads: Option<u64>,
}

impl Dependency {
    /// Get the dependency name (alias for crate_id)
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version requirement
    pub fn version_req(&self) -> &str {
        &self.req
    }
}

/// Download statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DownloadStats {
    /// Total all-time downloads
    pub total: u64,
    /// Per-version download statistics  
    pub versions: Vec<VersionDownload>,
}

/// Download stats for a specific version
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VersionDownload {
    pub version: String,
    pub downloads: u64,
    pub date: DateTime<Utc>,
}

/// Crate owner information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Owner {
    pub id: u64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub url: Option<String>,
    pub kind: String, // "user" or "team"
}

// Batch processing types

/// Batch input format - supports multiple input types
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum BatchInput {
    /// Map of crate names to specific versions
    CrateVersionMap(HashMap<String, String>),

    /// List of crate names (will check latest versions)
    CrateList { crates: Vec<String> },

    /// Advanced operations format
    Operations { operations: Vec<BatchOperation> },
}

/// A single batch operation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchOperation {
    /// The target(s) for this operation
    #[serde(flatten)]
    pub target: BatchTarget,

    /// The operation to perform
    pub operation: String,
}

/// Target for batch operations
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum BatchTarget {
    /// Single crate with optional version
    Single {
        #[serde(rename = "crate")]
        crate_name: String,
        version: Option<String>,
    },

    /// Multiple crates
    Multiple { crates: Vec<String> },
}

/// Result for checking a single crate
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CrateCheckResult {
    pub crate_name: String,
    pub exists: bool,
    pub latest_version: Option<String>,
    pub requested_version: Option<String>,
    pub version_exists: Option<bool>,
    pub error: Option<String>,
    pub info: Option<CrateInfo>,
}

/// Overall batch processing result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchResult {
    pub results: Vec<CrateCheckResult>,
    pub total_processed: usize,
    pub successful: usize,
    pub failed: usize,
    pub processing_time_ms: u64,
}

// Server API types

/// Request format for batch API endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchRequest {
    /// The batch input data
    #[serde(flatten)]
    pub input: BatchInput,

    /// Processing options
    #[serde(default)]
    pub options: BatchOptions,
}

/// Options for batch processing
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct BatchOptions {
    /// Include detailed crate information
    #[serde(default)]
    pub include_details: bool,

    /// Process requests in parallel
    #[serde(default)]
    pub parallel: bool,

    /// Timeout for the entire batch operation
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Maximum number of concurrent requests
    #[serde(default = "default_concurrency")]
    pub max_concurrent: usize,
}

fn default_timeout() -> u64 {
    30
}

fn default_concurrency() -> usize {
    10
}

/// Response format for batch API endpoint
#[derive(Debug, Serialize, Deserialize)]
pub struct BatchResponse {
    pub request_id: String,
    pub status: String,

    /// The batch processing result
    #[serde(flatten)]
    pub result: BatchResult,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: DateTime<Utc>,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Search request parameters
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort: Option<String>,
}

/// Metrics response
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub requests_total: u64,
    pub requests_successful: u64,
    pub requests_failed: u64,
    pub average_response_time_ms: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub uptime_seconds: u64,
}

// Crates.io API response types (internal)

/// Response from crates.io for crate information
#[derive(Debug, Deserialize)]
pub struct CrateResponse {
    #[serde(rename = "crate")]
    pub crate_info: CrateApiInfo,
    pub versions: Option<Vec<Version>>,
    pub keywords: Option<Vec<Keyword>>,
    pub categories: Option<Vec<Category>>,
}

/// Crate information from crates.io API
#[derive(Debug, Deserialize)]
pub struct CrateApiInfo {
    pub name: String,
    pub description: Option<String>,
    pub newest_version: String,
    pub downloads: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub max_upload_size: Option<u64>,
    #[serde(rename = "recent_downloads")]
    pub recent_downloads: Option<u64>,
}

/// Keyword information
#[derive(Debug, Deserialize)]
pub struct Keyword {
    pub keyword: String,
}

/// Category information
#[derive(Debug, Deserialize)]
pub struct Category {
    pub category: String,
}

/// Response from crates.io versions endpoint
#[derive(Debug, Deserialize)]
pub struct VersionsResponse {
    pub versions: Vec<Version>,
}

/// Response from crates.io search endpoint
#[derive(Debug, Deserialize)]
pub struct SearchResponse {
    pub crates: Vec<CrateSearchResult>,
    pub meta: SearchMeta,
}

/// Search metadata
#[derive(Debug, Deserialize)]
pub struct SearchMeta {
    pub total: u32,
}

/// Response from dependencies endpoint
#[derive(Debug, Deserialize)]
pub struct DependenciesResponse {
    pub dependencies: Vec<Dependency>,
}

/// Response from downloads endpoint
#[derive(Debug, Deserialize)]
pub struct DownloadsResponse {
    pub version_downloads: Vec<VersionDownloadApi>,
    pub meta: DownloadsMeta,
}

/// Downloads metadata
#[derive(Debug, Deserialize)]
pub struct DownloadsMeta {
    pub extra_downloads: Vec<ExtraDownload>,
}

/// Version download info from API
#[derive(Debug, Deserialize)]
pub struct VersionDownloadApi {
    pub version: String,
    pub downloads: u64,
    pub date: String, // API returns date as string, not DateTime
}

/// Extra download information
#[derive(Debug, Deserialize)]
pub struct ExtraDownload {
    pub date: String, // API returns date as string
    pub downloads: u64,
}

impl From<CrateApiInfo> for CrateInfo {
    fn from(api_info: CrateApiInfo) -> Self {
        Self {
            name: api_info.name,
            description: api_info.description,
            newest_version: api_info.newest_version,
            downloads: api_info.downloads,
            created_at: api_info.created_at,
            updated_at: api_info.updated_at,
            homepage: api_info.homepage,
            repository: api_info.repository,
            documentation: api_info.documentation,
            keywords: Vec::new(),   // Will be populated separately
            categories: Vec::new(), // Will be populated separately
            max_upload_size: api_info.max_upload_size,
            license: None,
            yanked: None,
            links: None,
        }
    }
}
