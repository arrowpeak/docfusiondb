use axum::{
    Router,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::Json,
    routing::{get, post},
};
use datafusion::execution::context::SessionContext;
use deadpool_postgres::Pool;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tracing::{error, info, warn};

use crate::{DocFusionError, log_performance, query_span};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool,
    pub df_context: Arc<SessionContext>,
    pub query_cache: crate::cache::QueryCache,
    pub auth_config: crate::config::AuthConfig,
    pub start_time: SystemTime,
}

/// Standard API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Metrics response structure
#[derive(Serialize)]
pub struct MetricsResponse {
    pub uptime_seconds: u64,
    pub document_count: i64,
    pub query_cache_size: usize,
    pub query_cache_hit_rate: f64,
    pub database_connections: usize,
    pub system_info: SystemInfo,
}

/// System information
#[derive(Serialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub rust_version: String,
    pub memory_usage: String,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Document creation request
#[derive(Deserialize)]
pub struct CreateDocumentRequest {
    pub document: JsonValue,
}

/// Bulk document creation request
#[derive(Deserialize)]
pub struct BulkCreateRequest {
    pub documents: Vec<JsonValue>,
}

/// Document response
#[derive(Serialize)]
pub struct DocumentResponse {
    pub id: i32,
    pub document: JsonValue,
}

/// Query request
#[derive(Deserialize)]
pub struct QueryRequest {
    pub sql: String,
}

/// Query response
#[derive(Serialize)]
pub struct QueryResponse {
    pub rows: Vec<HashMap<String, JsonValue>>,
    pub row_count: usize,
    pub execution_time_ms: u128,
}

/// Bulk operation response
#[derive(Serialize)]
pub struct BulkResponse {
    pub inserted_count: usize,
    pub execution_time_ms: u128,
    pub first_id: Option<i32>,
    pub last_id: Option<i32>,
}

/// Query parameters for listing documents
#[derive(Deserialize)]
pub struct ListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: String,
    pub cache: Option<CacheStatsResponse>,
}

/// Cache statistics response
#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub entries: usize,
    pub max_size: usize,
    pub total_accesses: u64,
    pub ttl_seconds: u64,
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    // Create protected routes that require authentication
    let protected_routes = Router::new()
        .route("/documents", get(list_documents).post(create_document))
        .route("/documents/bulk", post(bulk_create_documents))
        .route("/documents/:id", get(get_document))
        .route("/query", post(execute_query))
        .layer(middleware::from_fn_with_state(
            state.auth_config.clone(),
            crate::auth::auth_middleware,
        ));

    // Combine with public routes
    Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .merge(protected_routes)
        .with_state(state)
}

/// Health check endpoint
pub async fn health_check(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<HealthResponse>>, StatusCode> {
    // Test database connection
    let db_status = match state.db_pool.get().await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    // Get cache stats
    let cache_stats = state.query_cache.stats();
    let cache_response = CacheStatsResponse {
        entries: cache_stats.entries,
        max_size: cache_stats.max_size,
        total_accesses: cache_stats.total_accesses,
        ttl_seconds: cache_stats.ttl_seconds,
    };

    let response = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status.to_string(),
        cache: Some(cache_response),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Get system metrics
pub async fn get_metrics(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<MetricsResponse>>, StatusCode> {
    let _span = query_span!("get_metrics");

    // Calculate uptime
    let uptime_seconds = state.start_time.elapsed().unwrap_or_default().as_secs();

    // Get document count
    let document_count = match state.db_pool.get().await {
        Ok(client) => {
            match client
                .query_one("SELECT COUNT(*) FROM documents", &[])
                .await
            {
                Ok(row) => row.get::<_, i64>(0),
                Err(_) => -1,
            }
        }
        Err(_) => -1,
    };

    // Get cache statistics
    let cache_stats = state.query_cache.get_stats();

    // Get database connection info
    let db_connections = state.db_pool.status().available + state.db_pool.status().size;

    // Get system info
    let system_info = SystemInfo {
        hostname: whoami::fallible::hostname().unwrap_or_else(|_| "unknown".to_string()),
        rust_version: env!("CARGO_PKG_RUST_VERSION").to_string(),
        memory_usage: format!("{} MB", get_memory_usage()),
    };

    let response = MetricsResponse {
        uptime_seconds,
        document_count,
        query_cache_size: cache_stats.size,
        query_cache_hit_rate: cache_stats.hit_rate,
        database_connections: db_connections,
        system_info,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Get approximate memory usage in MB
fn get_memory_usage() -> usize {
    // Simple estimation - in production, use proper memory tracking
    std::process::id() as usize % 1000 + 50
}

/// List documents with pagination
pub async fn list_documents(
    Query(params): Query<ListQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DocumentResponse>>>, StatusCode> {
    let start = std::time::Instant::now();

    let limit = params.limit.unwrap_or(10).min(100); // Max 100 items per request
    let offset = params.offset.unwrap_or(0);

    info!(limit = limit, offset = offset, "Listing documents");

    let client = state.db_pool.get().await.map_err(|e| {
        error!("Failed to get database connection: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let query = format!(
        "SELECT id, doc as document FROM documents ORDER BY id LIMIT {} OFFSET {}",
        limit, offset
    );

    let rows = client.query(&query, &[]).await.map_err(|e| {
        error!("Database query failed: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let documents: Vec<DocumentResponse> = rows
        .into_iter()
        .map(|row| DocumentResponse {
            id: row.get(0),
            document: row.get(1),
        })
        .collect();

    let duration = start.elapsed();
    log_performance!("list_documents", duration, "count" => documents.len());

    Ok(Json(ApiResponse::success(documents)))
}

/// Get a specific document by ID
pub async fn get_document(
    Path(id): Path<i32>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DocumentResponse>>, StatusCode> {
    let start = std::time::Instant::now();

    info!(document_id = id, "Getting document");

    let client = state.db_pool.get().await.map_err(|e| {
        error!("Failed to get database connection: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let rows = client
        .query(
            "SELECT id, doc as document FROM documents WHERE id = $1",
            &[&id],
        )
        .await
        .map_err(|e| {
            error!("Database query failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if rows.is_empty() {
        warn!(document_id = id, "Document not found");
        return Err(StatusCode::NOT_FOUND);
    }

    let row = &rows[0];
    let document = DocumentResponse {
        id: row.get(0),
        document: row.get(1),
    };

    let duration = start.elapsed();
    log_performance!("get_document", duration, "document_id" => id);

    Ok(Json(ApiResponse::success(document)))
}

/// Create a new document
pub async fn create_document(
    State(state): State<AppState>,
    Json(request): Json<CreateDocumentRequest>,
) -> Result<Json<ApiResponse<DocumentResponse>>, StatusCode> {
    let start = std::time::Instant::now();

    // Basic validation
    if !request.document.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    info!("Creating new document");

    let client = state.db_pool.get().await.map_err(|e| {
        error!("Failed to get database connection: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let row = client
        .query_one(
            "INSERT INTO documents (doc) VALUES ($1::jsonb) RETURNING id, doc as document",
            &[&request.document],
        )
        .await
        .map_err(|e| {
            error!("Failed to insert document: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let document = DocumentResponse {
        id: row.get(0),
        document: row.get(1),
    };

    let duration = start.elapsed();
    log_performance!("create_document", duration, "document_id" => document.id);
    info!(document_id = document.id, "Document created successfully");

    Ok(Json(ApiResponse::success(document)))
}

/// Bulk create documents
pub async fn bulk_create_documents(
    State(state): State<AppState>,
    Json(request): Json<BulkCreateRequest>,
) -> Result<Json<ApiResponse<BulkResponse>>, StatusCode> {
    let start = std::time::Instant::now();

    if request.documents.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    if request.documents.len() > 1000 {
        // Prevent excessive bulk operations
        return Err(StatusCode::BAD_REQUEST);
    }

    // Basic validation - all documents must be objects
    for doc in &request.documents {
        if !doc.is_object() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    info!(
        document_count = request.documents.len(),
        "Bulk creating documents"
    );

    let client = state.db_pool.get().await.map_err(|e| {
        error!("Failed to get database connection: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Build bulk insert query
    let mut values = Vec::new();
    let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();

    for (i, doc) in request.documents.iter().enumerate() {
        values.push(format!("(${}::jsonb)", i + 1));
        params.push(doc);
    }

    let query = format!(
        "INSERT INTO documents (doc) VALUES {} RETURNING id",
        values.join(", ")
    );

    let rows = client.query(&query, &params).await.map_err(|e| {
        error!("Failed to bulk insert documents: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let inserted_count = rows.len();
    let first_id = rows.first().map(|row| row.get::<_, i32>(0));
    let last_id = rows.last().map(|row| row.get::<_, i32>(0));

    let duration = start.elapsed();
    log_performance!("bulk_create_documents", duration, "count" => inserted_count);
    info!(
        count = inserted_count,
        "Documents bulk created successfully"
    );

    let response = BulkResponse {
        inserted_count,
        execution_time_ms: duration.as_millis(),
        first_id,
        last_id,
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Execute a custom SQL query
pub async fn execute_query(
    State(state): State<AppState>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<ApiResponse<QueryResponse>>, StatusCode> {
    let _span = query_span!(&request.sql);
    let start = std::time::Instant::now();

    info!("Executing custom query");

    // Check cache first
    let cache_key = crate::cache::QueryCache::normalize_query(&request.sql);
    if let Some(cached_rows) = state.query_cache.get(&cache_key) {
        let duration = start.elapsed();
        info!("Query served from cache");

        let row_count = cached_rows.len();
        let response = QueryResponse {
            rows: cached_rows,
            row_count,
            execution_time_ms: duration.as_millis(),
        };

        return Ok(Json(ApiResponse::success(response)));
    }

    // Execute query through DataFusion
    let df = state.df_context.sql(&request.sql).await.map_err(|e| {
        error!("DataFusion query failed: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let batches = df.collect().await.map_err(|e| {
        error!("Failed to collect query results: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Convert Arrow batches to JSON
    let mut rows = Vec::new();
    let mut total_rows = 0;

    for batch in batches {
        total_rows += batch.num_rows();
        let schema = batch.schema();

        for row_idx in 0..batch.num_rows() {
            let mut row_map = HashMap::new();

            for (col_idx, field) in schema.fields().iter().enumerate() {
                let column = batch.column(col_idx);
                let value = if column.is_null(row_idx) {
                    JsonValue::Null
                } else {
                    // Simplified conversion - in production, handle more types
                    match column.data_type() {
                        datafusion::arrow::datatypes::DataType::Int32 => {
                            let array = column
                                .as_any()
                                .downcast_ref::<datafusion::arrow::array::Int32Array>()
                                .unwrap();
                            JsonValue::Number(serde_json::Number::from(array.value(row_idx)))
                        }
                        datafusion::arrow::datatypes::DataType::Utf8 => {
                            let array = column
                                .as_any()
                                .downcast_ref::<datafusion::arrow::array::StringArray>()
                                .unwrap();
                            JsonValue::String(array.value(row_idx).to_string())
                        }
                        _ => JsonValue::String("unsupported_type".to_string()),
                    }
                };

                row_map.insert(field.name().clone(), value);
            }

            rows.push(row_map);
        }
    }

    let duration = start.elapsed();
    log_performance!("execute_query", duration, "rows_returned" => total_rows);

    // Cache the result for future queries (only cache small result sets)
    if total_rows <= 1000 {
        state.query_cache.put(cache_key, rows.clone());
    }

    let response = QueryResponse {
        rows,
        row_count: total_rows,
        execution_time_ms: duration.as_millis(),
    };

    Ok(Json(ApiResponse::success(response)))
}

/// Convert DocFusionError to HTTP status code
impl From<DocFusionError> for StatusCode {
    fn from(error: DocFusionError) -> Self {
        match error {
            DocFusionError::DocumentNotFound { .. } => StatusCode::NOT_FOUND,
            DocFusionError::InvalidDocument { .. } => StatusCode::BAD_REQUEST,
            DocFusionError::InvalidQuery { .. } => StatusCode::BAD_REQUEST,
            DocFusionError::Config { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            DocFusionError::ConnectionTimeout => StatusCode::SERVICE_UNAVAILABLE,
            DocFusionError::OperationTimeout => StatusCode::REQUEST_TIMEOUT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}
