//! HTTP server implementation for the crate checker API

use crate::client::CrateClient;
use crate::config::AppConfig;
use crate::error::{CrateCheckerError, Result};
use crate::types::*;
use crate::utils::validate_batch_input;
use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use dashmap::DashMap;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{error, info};

/// Server state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub client: CrateClient,
    pub config: AppConfig,
    pub metrics: Arc<ServerMetrics>,
    pub cache: Arc<DashMap<String, CacheEntry>>,
    pub start_time: Instant,
}

/// Cached response entry
#[derive(Clone)]
pub struct CacheEntry {
    pub data: Value,
    pub expires_at: Instant,
}

/// Server metrics
#[derive(Default)]
pub struct ServerMetrics {
    pub requests_total: AtomicU64,
    pub requests_successful: AtomicU64,
    pub requests_failed: AtomicU64,
    pub cache_hits: AtomicU64,
    pub cache_misses: AtomicU64,
    pub total_response_time_ms: AtomicU64,
}

impl ServerMetrics {
    pub fn record_request(&self, success: bool, response_time_ms: u64) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms
            .fetch_add(response_time_ms, Ordering::Relaxed);

        if success {
            self.requests_successful.fetch_add(1, Ordering::Relaxed);
        } else {
            self.requests_failed.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_metrics(&self) -> MetricsResponse {
        let total = self.requests_total.load(Ordering::Relaxed);
        let total_time = self.total_response_time_ms.load(Ordering::Relaxed);

        MetricsResponse {
            requests_total: total,
            requests_successful: self.requests_successful.load(Ordering::Relaxed),
            requests_failed: self.requests_failed.load(Ordering::Relaxed),
            average_response_time_ms: if total > 0 {
                total_time as f64 / total as f64
            } else {
                0.0
            },
            cache_hits: self.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.cache_misses.load(Ordering::Relaxed),
            uptime_seconds: 0, // Will be set by the handler
        }
    }
}

/// Start the HTTP server
pub async fn start_server(config: AppConfig) -> Result<()> {
    info!("Starting server on {}", config.bind_address());

    // Validate configuration
    config.validate().map_err(CrateCheckerError::validation)?;

    // Create client with configuration
    let client = CrateClient::builder()
        .base_url(&config.crates_io.api_url)
        .user_agent(&config.crates_io.user_agent)
        .timeout(Duration::from_secs(config.crates_io.timeout_seconds))
        .build()?;

    // Create shared state
    let state = AppState {
        client,
        config: config.clone(),
        metrics: Arc::new(ServerMetrics::default()),
        cache: Arc::new(DashMap::new()),
        start_time: Instant::now(),
    };

    // Build the application router
    let app = create_router(state);

    // Configure server
    let listener = tokio::net::TcpListener::bind(&config.bind_address()).await?;

    info!("Server listening on {}", config.bind_address());
    info!("Health check: http://{}/health", config.bind_address());
    info!("API docs: http://{}/", config.bind_address());

    // Start server
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the application router
fn create_router(state: AppState) -> Router {
    let mut app = Router::new()
        // Health check
        .route("/health", get(health_check))
        // API documentation
        .route("/", get(api_docs))
        // Core API endpoints
        .route("/api/crates/:name", get(get_crate))
        .route("/api/crates/:name/:version", get(get_crate_version))
        .route(
            "/api/crates/:name/:version/deps",
            get(get_crate_dependencies),
        )
        .route("/api/crates/:name/stats", get(get_crate_stats))
        .route("/api/search", get(search_crates))
        .route("/api/batch", post(handle_batch))
        // Metrics and monitoring
        .route("/metrics", get(get_metrics))
        // Add state
        .with_state(state.clone());

    // Add middleware
    let service = ServiceBuilder::new().layer(TraceLayer::new_for_http());

    app = app.layer(service);

    // Add CORS if enabled
    if state.config.server.enable_cors {
        app = app.layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_headers(Any)
                .allow_origin(Any),
        );
    }

    app
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: Utc::now(),
        version: "1.0.0".to_string(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    })
}

/// API documentation endpoint
async fn api_docs() -> &'static str {
    r#"# Crate Checker API

## Available Endpoints

### Health Check
- `GET /health` - Server health status

### Crate Information
- `GET /api/crates/{name}` - Get crate information
- `GET /api/crates/{name}/{version}` - Check specific version
- `GET /api/crates/{name}/{version}/deps` - Get dependencies
- `GET /api/crates/{name}/stats` - Get download statistics

### Search
- `GET /api/search?q={query}&limit={limit}` - Search crates

### Batch Operations
- `POST /api/batch` - Process multiple crates

### Monitoring
- `GET /metrics` - Server metrics

## Examples

```bash
# Check if crate exists
curl http://localhost:3000/api/crates/serde

# Search for crates
curl "http://localhost:3000/api/search?q=http%20client&limit=5"

# Batch processing
curl -X POST http://localhost:3000/api/batch \
  -H "Content-Type: application/json" \
  -d '{"serde": "1.0.0", "tokio": "latest"}'
```
"#
}

/// Get crate information
async fn get_crate(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> std::result::Result<Json<CrateInfo>, AppError> {
    let start_time = Instant::now();

    // Check cache first
    let cache_key = format!("crate:{}", name);
    if let Some(cached) = get_from_cache(&state, &cache_key) {
        state.metrics.record_cache_hit();
        state
            .metrics
            .record_request(true, start_time.elapsed().as_millis() as u64);
        return Ok(Json(serde_json::from_value(cached.data)?));
    }

    state.metrics.record_cache_miss();

    match state.client.get_crate_info(&name).await {
        Ok(info) => {
            // Cache the result
            if state.config.cache.enabled {
                set_cache(&state, &cache_key, serde_json::to_value(&info)?);
            }

            state
                .metrics
                .record_request(true, start_time.elapsed().as_millis() as u64);
            Ok(Json(info))
        }
        Err(e) => {
            error!("Failed to get crate info for '{}': {}", name, e);
            state
                .metrics
                .record_request(false, start_time.elapsed().as_millis() as u64);
            Err(AppError::from(e))
        }
    }
}

/// Get crate version information
async fn get_crate_version(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> std::result::Result<Json<CrateCheckResult>, AppError> {
    let start_time = Instant::now();

    let cache_key = format!("crate:{}:{}", name, version);
    if let Some(cached) = get_from_cache(&state, &cache_key) {
        state.metrics.record_cache_hit();
        state
            .metrics
            .record_request(true, start_time.elapsed().as_millis() as u64);
        return Ok(Json(serde_json::from_value(cached.data)?));
    }

    state.metrics.record_cache_miss();

    let result = if version == "latest" {
        match state.client.get_crate_info(&name).await {
            Ok(info) => CrateCheckResult {
                crate_name: name.clone(),
                exists: true,
                latest_version: Some(info.newest_version.clone()),
                requested_version: Some("latest".to_string()),
                version_exists: Some(true),
                error: None,
                info: Some(info),
            },
            Err(e) => CrateCheckResult {
                crate_name: name.clone(),
                exists: false,
                latest_version: None,
                requested_version: Some(version),
                version_exists: None,
                error: Some(e.to_string()),
                info: None,
            },
        }
    } else {
        // Check specific version
        match state.client.get_all_versions(&name).await {
            Ok(versions) => {
                let version_exists = versions.iter().any(|v| v.num == version);
                let info = if version_exists {
                    state.client.get_crate_info(&name).await.ok()
                } else {
                    None
                };

                CrateCheckResult {
                    crate_name: name.clone(),
                    exists: true,
                    latest_version: info.as_ref().map(|i| i.newest_version.clone()),
                    requested_version: Some(version),
                    version_exists: Some(version_exists),
                    error: None,
                    info,
                }
            }
            Err(e) => CrateCheckResult {
                crate_name: name.clone(),
                exists: false,
                latest_version: None,
                requested_version: Some(version),
                version_exists: None,
                error: Some(e.to_string()),
                info: None,
            },
        }
    };

    // Cache the result
    if state.config.cache.enabled {
        set_cache(&state, &cache_key, serde_json::to_value(&result)?);
    }

    state
        .metrics
        .record_request(true, start_time.elapsed().as_millis() as u64);
    Ok(Json(result))
}

/// Get crate dependencies
async fn get_crate_dependencies(
    State(state): State<AppState>,
    Path((name, version)): Path<(String, String)>,
) -> std::result::Result<Json<Vec<Dependency>>, AppError> {
    let start_time = Instant::now();

    let actual_version = if version == "latest" {
        match state.client.get_latest_version(&name).await {
            Ok(v) => v,
            Err(e) => {
                state
                    .metrics
                    .record_request(false, start_time.elapsed().as_millis() as u64);
                return Err(AppError::from(e));
            }
        }
    } else {
        version
    };

    match state
        .client
        .get_crate_dependencies(&name, &actual_version)
        .await
    {
        Ok(deps) => {
            state
                .metrics
                .record_request(true, start_time.elapsed().as_millis() as u64);
            Ok(Json(deps))
        }
        Err(e) => {
            error!(
                "Failed to get dependencies for '{}:{}': {}",
                name, actual_version, e
            );
            state
                .metrics
                .record_request(false, start_time.elapsed().as_millis() as u64);
            Err(AppError::from(e))
        }
    }
}

/// Get crate download statistics
async fn get_crate_stats(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> std::result::Result<Json<DownloadStats>, AppError> {
    let start_time = Instant::now();

    match state.client.get_download_stats(&name).await {
        Ok(stats) => {
            state
                .metrics
                .record_request(true, start_time.elapsed().as_millis() as u64);
            Ok(Json(stats))
        }
        Err(e) => {
            error!("Failed to get stats for '{}': {}", name, e);
            state
                .metrics
                .record_request(false, start_time.elapsed().as_millis() as u64);
            Err(AppError::from(e))
        }
    }
}

/// Search crates
async fn search_crates(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> std::result::Result<Json<Vec<CrateSearchResult>>, AppError> {
    let start_time = Instant::now();

    let query = params
        .get("q")
        .ok_or_else(|| AppError::BadRequest("Missing 'q' parameter".to_string()))?;

    let limit = params
        .get("limit")
        .and_then(|l| l.parse().ok())
        .unwrap_or(10);

    match state.client.search_crates(query, Some(limit)).await {
        Ok(results) => {
            state
                .metrics
                .record_request(true, start_time.elapsed().as_millis() as u64);
            Ok(Json(results))
        }
        Err(e) => {
            error!("Failed to search for '{}': {}", query, e);
            state
                .metrics
                .record_request(false, start_time.elapsed().as_millis() as u64);
            Err(AppError::from(e))
        }
    }
}

/// Handle batch operations
async fn handle_batch(
    State(state): State<AppState>,
    Json(request): Json<BatchRequest>,
) -> std::result::Result<Json<BatchResponse>, AppError> {
    let start_time = Instant::now();

    validate_batch_input(&request.input).map_err(AppError::from)?;

    let result = match request.input {
        BatchInput::CrateVersionMap(map) => state.client.process_crate_version_map(map).await?,
        BatchInput::CrateList { crates } => {
            let results = state.client.process_crate_list(crates).await?;
            let successful = results.iter().filter(|r| r.error.is_none()).count();
            let failed = results.len() - successful;
            let total_processed = results.len();

            BatchResult {
                results,
                total_processed,
                successful,
                failed,
                processing_time_ms: start_time.elapsed().as_millis() as u64,
            }
        }
        BatchInput::Operations { operations } => {
            state
                .client
                .process_batch_operations(operations)
                .await?
                .result
        }
    };

    let response = BatchResponse {
        request_id: uuid::Uuid::new_v4().to_string(),
        status: "completed".to_string(),
        result,
    };

    state
        .metrics
        .record_request(true, start_time.elapsed().as_millis() as u64);
    Ok(Json(response))
}

/// Get server metrics
async fn get_metrics(State(state): State<AppState>) -> Json<MetricsResponse> {
    let mut metrics = state.metrics.get_metrics();
    metrics.uptime_seconds = state.start_time.elapsed().as_secs();
    Json(metrics)
}

/// Helper function to get from cache
fn get_from_cache(state: &AppState, key: &str) -> Option<CacheEntry> {
    if !state.config.cache.enabled {
        return None;
    }

    if let Some(entry) = state.cache.get(key) {
        if entry.expires_at > Instant::now() {
            return Some(entry.clone());
        } else {
            // Entry expired, remove it
            state.cache.remove(key);
        }
    }

    None
}

/// Helper function to set cache
fn set_cache(state: &AppState, key: &str, data: Value) {
    if !state.config.cache.enabled {
        return;
    }

    // Clean up expired entries periodically
    if state.cache.len() > state.config.cache.max_entries {
        let now = Instant::now();
        state.cache.retain(|_, entry| entry.expires_at > now);
    }

    let entry = CacheEntry {
        data,
        expires_at: Instant::now() + Duration::from_secs(state.config.cache.ttl_seconds),
    };

    state.cache.insert(key.to_string(), entry);
}

/// Application error wrapper for HTTP responses
#[derive(Debug)]
pub enum AppError {
    Internal(CrateCheckerError),
    BadRequest(String),
    NotFound(String),
}

impl From<CrateCheckerError> for AppError {
    fn from(err: CrateCheckerError) -> Self {
        match err {
            CrateCheckerError::CrateNotFound(_) | CrateCheckerError::VersionNotFound { .. } => {
                Self::NotFound(err.to_string())
            }
            CrateCheckerError::ValidationError(_) | CrateCheckerError::InvalidBatchInput(_) => {
                Self::BadRequest(err.to_string())
            }
            _ => Self::Internal(err),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::BadRequest(format!("JSON error: {}", err))
    }
}

/// Convert AppError to HTTP response
impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            AppError::Internal(e) => {
                error!("Internal error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
        };

        let body = serde_json::json!({
            "error": message,
            "timestamp": Utc::now().to_rfc3339()
        });

        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    async fn create_test_app() -> Router {
        let client = CrateClient::new();
        let config = AppConfig::default();
        let state = AppState {
            client,
            config,
            metrics: Arc::new(ServerMetrics::default()),
            cache: Arc::new(DashMap::new()),
            start_time: Instant::now(),
        };

        create_router(state)
    }

    #[tokio::test]
    async fn test_health_check() {
        let app = create_test_app().await;

        let request = Request::builder()
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_api_docs() {
        let app = create_test_app().await;

        let request = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let app = create_test_app().await;

        let request = Request::builder()
            .uri("/metrics")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
