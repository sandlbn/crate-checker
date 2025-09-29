//! HTTP client for interacting with the crates.io API

use crate::error::{CrateCheckerError, Result};
use crate::types::*;
use crate::{DEFAULT_API_URL, DEFAULT_TIMEOUT_SECS, DEFAULT_USER_AGENT};
use reqwest::{Client, StatusCode};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// HTTP client for crates.io API interactions
#[derive(Debug, Clone)]
pub struct CrateClient {
    client: Client,
    base_url: String,
    // Note: These fields are intentionally kept for configuration tracking and potential future use
    _user_agent: String,
    _timeout: Duration,
}

impl CrateClient {
    /// Create a new client with default settings
    pub fn new() -> Self {
        Self::builder()
            .build()
            .expect("Failed to create default client")
    }

    /// Create a new client builder
    pub fn builder() -> CrateClientBuilder {
        CrateClientBuilder::default()
    }

    /// Check if a specific crate exists on crates.io
    pub async fn crate_exists(&self, crate_name: &str) -> Result<bool> {
        self.validate_crate_name(crate_name)?;

        let url = format!("{}/crates/{}", self.base_url, crate_name);
        debug!("Checking if crate exists: {}", crate_name);

        match self.client.get(&url).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => {
                    info!("Crate '{}' exists", crate_name);
                    Ok(true)
                }
                StatusCode::NOT_FOUND => {
                    info!("Crate '{}' not found", crate_name);
                    Ok(false)
                }
                status => {
                    warn!("Unexpected status {} for crate '{}'", status, crate_name);
                    Err(CrateCheckerError::from(status))
                }
            },
            Err(e) => {
                error!("Failed to check crate '{}': {}", crate_name, e);
                Err(CrateCheckerError::from(e))
            }
        }
    }

    /// Get the latest version of a crate
    pub async fn get_latest_version(&self, crate_name: &str) -> Result<String> {
        let info = self.get_crate_info(crate_name).await?;
        Ok(info.newest_version)
    }

    /// Get detailed information about a crate
    pub async fn get_crate_info(&self, crate_name: &str) -> Result<CrateInfo> {
        self.validate_crate_name(crate_name)?;

        let url = format!("{}/crates/{}", self.base_url, crate_name);
        debug!("Fetching crate info for: {}", crate_name);

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let crate_response: CrateResponse = response.json().await?;
                let mut crate_info = CrateInfo::from(crate_response.crate_info);

                // Populate keywords and categories
                if let Some(keywords) = crate_response.keywords {
                    crate_info.keywords = keywords.into_iter().map(|k| k.keyword).collect();
                }
                if let Some(categories) = crate_response.categories {
                    crate_info.categories = categories.into_iter().map(|c| c.category).collect();
                }

                info!("Successfully fetched info for crate '{}'", crate_name);
                Ok(crate_info)
            }
            StatusCode::NOT_FOUND => Err(CrateCheckerError::CrateNotFound(crate_name.to_string())),
            status => Err(CrateCheckerError::from(status)),
        }
    }

    /// Get all versions of a crate
    pub async fn get_all_versions(&self, crate_name: &str) -> Result<Vec<Version>> {
        self.validate_crate_name(crate_name)?;

        let url = format!("{}/crates/{}/versions", self.base_url, crate_name);
        debug!("Fetching versions for crate: {}", crate_name);

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let versions_response: VersionsResponse = response.json().await?;
                info!(
                    "Found {} versions for crate '{}'",
                    versions_response.versions.len(),
                    crate_name
                );
                Ok(versions_response.versions)
            }
            StatusCode::NOT_FOUND => Err(CrateCheckerError::CrateNotFound(crate_name.to_string())),
            status => Err(CrateCheckerError::from(status)),
        }
    }

    /// Search for crates by name or keywords
    pub async fn search_crates(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<Vec<CrateSearchResult>> {
        if query.trim().is_empty() {
            return Err(CrateCheckerError::validation(
                "Search query cannot be empty",
            ));
        }

        let mut url = format!("{}/crates?q={}", self.base_url, urlencoding::encode(query));
        if let Some(limit) = limit {
            url.push_str(&format!("&per_page={}", limit.min(100))); // Limit to max 100
        }

        debug!(
            "Searching crates with query: '{}', limit: {:?}",
            query, limit
        );

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let search_response: SearchResponse = response.json().await?;
                info!(
                    "Search found {} results for query '{}'",
                    search_response.crates.len(),
                    query
                );
                Ok(search_response.crates)
            }
            status => Err(CrateCheckerError::from(status)),
        }
    }

    /// Get dependencies for a specific crate version
    pub async fn get_crate_dependencies(
        &self,
        crate_name: &str,
        version: &str,
    ) -> Result<Vec<Dependency>> {
        self.validate_crate_name(crate_name)?;

        let url = format!(
            "{}/crates/{}/{}/dependencies",
            self.base_url, crate_name, version
        );
        debug!("Fetching dependencies for {}:{}", crate_name, version);

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let deps_response: DependenciesResponse = response.json().await?;
                info!(
                    "Found {} dependencies for {}:{}",
                    deps_response.dependencies.len(),
                    crate_name,
                    version
                );
                Ok(deps_response.dependencies)
            }
            StatusCode::NOT_FOUND => Err(CrateCheckerError::VersionNotFound {
                crate_name: crate_name.to_string(),
                version: version.to_string(),
            }),
            status => Err(CrateCheckerError::from(status)),
        }
    }

    /// Get download statistics for a crate
    pub async fn get_download_stats(&self, crate_name: &str) -> Result<DownloadStats> {
        self.validate_crate_name(crate_name)?;

        // Get basic crate info which includes total downloads
        let crate_info = self.get_crate_info(crate_name).await?;
        let total_downloads = crate_info.downloads;

        // Get version-specific downloads from versions endpoint
        let versions = match self.get_all_versions(crate_name).await {
            Ok(mut versions) => {
                // Sort by downloads descending to get most popular versions first
                versions.sort_by(|a, b| b.downloads.cmp(&a.downloads));

                // Convert to VersionDownload format (take top 10)
                versions
                    .into_iter()
                    .take(10)
                    .map(|v| VersionDownload {
                        version: v.num,
                        downloads: v.downloads,
                        date: v.created_at,
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        };

        let stats = DownloadStats {
            total: total_downloads,
            versions,
        };

        info!(
            "Fetched download stats for '{}': {} total downloads, {} version entries",
            crate_name,
            total_downloads,
            stats.versions.len()
        );
        Ok(stats)
    }

    /// Check the status of a crate (exists, yanked, etc.)
    pub async fn check_crate_status(&self, crate_name: &str) -> Result<CrateStatus> {
        match self.get_all_versions(crate_name).await {
            Ok(versions) => {
                if versions.is_empty() {
                    Ok(CrateStatus::NotFound)
                } else {
                    let yanked_count = versions.iter().filter(|v| v.yanked).count();
                    if yanked_count == versions.len() {
                        Ok(CrateStatus::Yanked)
                    } else if yanked_count > 0 {
                        Ok(CrateStatus::PartiallyYanked)
                    } else {
                        Ok(CrateStatus::Exists)
                    }
                }
            }
            Err(CrateCheckerError::CrateNotFound(_)) => Ok(CrateStatus::NotFound),
            Err(e) => Err(e),
        }
    }

    /// Validate crate name format
    pub fn validate_crate_name(&self, name: &str) -> Result<()> {
        const PATTERN: &str = "^[a-zA-Z0-9_-]+$";

        if name.is_empty() {
            return Err(CrateCheckerError::InvalidCrateName(
                name.to_string(),
                "Crate name cannot be empty",
            ));
        }

        if name.len() > 64 {
            return Err(CrateCheckerError::InvalidCrateName(
                name.to_string(),
                "Crate name cannot be longer than 64 characters",
            ));
        }

        // Basic validation - crate names can contain letters, numbers, hyphens, and underscores
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(CrateCheckerError::InvalidCrateName(
                name.to_string(),
                PATTERN,
            ));
        }

        Ok(())
    }

    /// Process a batch of crate checks
    pub async fn process_crate_list(&self, crates: Vec<String>) -> Result<Vec<CrateCheckResult>> {
        info!("Processing batch of {} crates", crates.len());
        let start_time = Instant::now();

        let mut results = Vec::with_capacity(crates.len());

        for crate_name in crates {
            let result = self.process_single_crate_check(&crate_name, None).await;
            results.push(result);
        }

        let duration = start_time.elapsed();
        info!("Batch processing completed in {:?}", duration);

        Ok(results)
    }

    /// Process a crate version map
    pub async fn process_crate_version_map(
        &self,
        input: HashMap<String, String>,
    ) -> Result<BatchResult> {
        let start_time = Instant::now();
        let total_count = input.len();

        info!("Processing crate version map with {} entries", total_count);

        let mut results = Vec::with_capacity(total_count);
        let mut successful = 0;
        let mut failed = 0;

        for (crate_name, version) in input.iter() {
            let version_opt = if version == "latest" {
                None
            } else {
                Some(version.clone())
            };

            let result = self
                .process_single_crate_check(crate_name, version_opt)
                .await;

            if result.error.is_none() && result.exists {
                successful += 1;
            } else {
                failed += 1;
            }

            results.push(result);
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        info!(
            "Batch processing completed: {} total, {} successful, {} failed",
            total_count, successful, failed
        );

        Ok(BatchResult {
            results,
            total_processed: total_count,
            successful,
            failed,
            processing_time_ms,
        })
    }

    /// Process batch operations
    pub async fn process_batch_operations(
        &self,
        operations: Vec<BatchOperation>,
    ) -> Result<BatchResponse> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let start_time = Instant::now();
        let total_count = operations.len();

        info!(
            "Processing {} batch operations (request: {})",
            total_count, request_id
        );

        let mut all_results = Vec::new();

        for operation in &operations {
            match &operation.target {
                BatchTarget::Single {
                    crate_name,
                    version,
                } => {
                    let result = self
                        .process_single_crate_check(crate_name, version.clone())
                        .await;
                    all_results.push(result);
                }
                BatchTarget::Multiple { crates } => {
                    for crate_name in crates {
                        let result = self.process_single_crate_check(crate_name, None).await;
                        all_results.push(result);
                    }
                }
            }
        }

        let successful = all_results.iter().filter(|r| r.error.is_none()).count();
        let failed = all_results.len() - successful;
        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        let result = BatchResult {
            results: all_results,
            total_processed: total_count,
            successful,
            failed,
            processing_time_ms,
        };

        Ok(BatchResponse {
            request_id,
            status: "completed".to_string(),
            result,
        })
    }

    /// Process a single crate check (internal helper)
    async fn process_single_crate_check(
        &self,
        crate_name: &str,
        requested_version: Option<String>,
    ) -> CrateCheckResult {
        match self.crate_exists(crate_name).await {
            Ok(exists) => {
                if !exists {
                    return CrateCheckResult {
                        crate_name: crate_name.to_string(),
                        exists: false,
                        latest_version: None,
                        requested_version,
                        version_exists: None,
                        error: None,
                        info: None,
                    };
                }

                // Get crate info
                let info = match self.get_crate_info(crate_name).await {
                    Ok(info) => Some(info.clone()),
                    Err(_) => None,
                };

                let latest_version = info.as_ref().map(|i| i.newest_version.clone());

                // Check specific version if requested
                let version_exists = if let Some(ref req_version) = requested_version {
                    if req_version == "latest" {
                        Some(true)
                    } else {
                        match self.get_all_versions(crate_name).await {
                            Ok(versions) => Some(versions.iter().any(|v| v.num == *req_version)),
                            Err(_) => None,
                        }
                    }
                } else {
                    None
                };

                CrateCheckResult {
                    crate_name: crate_name.to_string(),
                    exists: true,
                    latest_version,
                    requested_version,
                    version_exists,
                    error: None,
                    info,
                }
            }
            Err(e) => CrateCheckResult {
                crate_name: crate_name.to_string(),
                exists: false,
                latest_version: None,
                requested_version,
                version_exists: None,
                error: Some(e.to_string()),
                info: None,
            },
        }
    }
}

impl Default for CrateClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating a CrateClient with custom configuration
#[derive(Debug)]
pub struct CrateClientBuilder {
    base_url: Option<String>,
    user_agent: Option<String>,
    timeout: Option<Duration>,
}

impl Default for CrateClientBuilder {
    fn default() -> Self {
        Self {
            base_url: None,
            user_agent: None,
            timeout: None,
        }
    }
}

impl CrateClientBuilder {
    /// Set the base URL for the crates.io API
    pub fn base_url<S: Into<String>>(mut self, url: S) -> Self {
        self.base_url = Some(url.into());
        self
    }

    /// Set the user agent string
    pub fn user_agent<S: Into<String>>(mut self, agent: S) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the CrateClient
    pub fn build(self) -> Result<CrateClient> {
        let timeout = self
            .timeout
            .unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT_SECS));
        let user_agent = self.user_agent.as_deref().unwrap_or(DEFAULT_USER_AGENT);

        let client = Client::builder()
            .timeout(timeout)
            .user_agent(user_agent)
            .build()?;

        Ok(CrateClient {
            client,
            base_url: self.base_url.unwrap_or_else(|| DEFAULT_API_URL.to_string()),
            _user_agent: user_agent.to_string(),
            _timeout: timeout,
        })
    }
}
