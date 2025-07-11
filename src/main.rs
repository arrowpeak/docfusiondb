use clap::{Parser, Subcommand};
use datafusion::arrow::datatypes::DataType;
use datafusion::execution::context::SessionContext;
use datafusion::logical_expr::create_udf;
use datafusion::logical_expr_common::signature::Volatility;
use docfusiondb::{
    Config, DocFusionError, DocFusionResult, PostgresTable,
    api::{AppState, create_router},
    json_contains_udf, json_extract_path_udf, json_multi_contains_udf, log_performance, logging,
    query_span,
};
use serde_json::Value as JsonValue;

use axum::serve;
use deadpool_postgres::{Config as PoolConfig, Runtime};
use std::fs;
use std::io::Write;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use tokio_postgres::NoTls;
use tracing::{info, warn};

/// DocFusionDB CLI
#[derive(Parser)]
#[command(name = "docfusiondb", version, about = "CLI for DocFusionDB")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the HTTP API server
    Serve {
        /// Port to bind to (overrides config)
        #[arg(short, long)]
        port: Option<u16>,
        /// Host to bind to (overrides config)
        #[arg(long)]
        host: Option<String>,
    },

    /// Run a SQL query against the `documents` table
    Query {
        /// The SQL to execute
        sql: String,
    },
    /// Insert a new JSON document into `documents`
    Insert {
        /// The JSON document to insert
        json: String,
    },
    /// Update an existing document by ID
    Update {
        /// The ID of the document to update
        id: i32,
        /// The new JSON document
        json: String,
    },
    /// Backup documents to a JSON file
    Backup {
        /// Output file path
        #[arg(short, long, default_value = "backup.json")]
        output: String,
    },
    /// Restore documents from a JSON file
    Restore {
        /// Input file path
        #[arg(short, long, default_value = "backup.json")]
        input: String,
        /// Clear existing documents before restore
        #[arg(long)]
        clear: bool,
    },
}

#[tokio::main]
async fn main() -> DocFusionResult<()> {
    // Load configuration first (needed for logging setup)
    let config = Config::load()?;

    // Initialize structured logging
    logging::init_logging(&config.logging)?;

    info!("Starting DocFusionDB CLI");
    info!(?config, "Loaded configuration");

    // Parse CLI arguments
    let cli = Cli::parse();

    // Build DataFusion context and register UDFs
    let df_ctx = SessionContext::new();
    df_ctx.register_udf(create_udf(
        "json_extract_path",
        vec![DataType::Utf8, DataType::Utf8],
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(json_extract_path_udf),
    ));
    df_ctx.register_udf(create_udf(
        "json_contains",
        vec![DataType::Utf8, DataType::Utf8],
        DataType::Boolean,
        Volatility::Immutable,
        Arc::new(json_contains_udf),
    ));
    df_ctx.register_udf(create_udf(
        "json_multi_contains",
        vec![DataType::Utf8, DataType::Utf8],
        DataType::Boolean,
        Volatility::Immutable,
        Arc::new(json_multi_contains_udf),
    ));

    // Register Postgres-backed table with DataFusion
    let df_table = PostgresTable::new(&config.database).await?;
    df_ctx.register_table("documents", Arc::new(df_table))?;

    // Create connection pool for writes
    let pool_config = PoolConfig::from(&config.database);
    let pool = pool_config.create_pool(Some(Runtime::Tokio1), NoTls)?;
    let pg_client = pool.get().await?;

    match cli.command {
        Commands::Serve { port, host } => {
            info!("Starting HTTP API server");

            // Override config with CLI args if provided
            let mut server_config = config.server.clone();
            if let Some(port) = port {
                server_config.port = port;
            }
            if let Some(host) = host {
                server_config.host = host;
            }

            // Create app state with cache
            let app_state = AppState {
                db_pool: pool.clone(),
                df_context: Arc::new(df_ctx),
                query_cache: docfusiondb::cache::QueryCache::default(),
                auth_config: config.auth.clone(),
                start_time: SystemTime::now(),
            };

            // Create router with middleware
            let app = create_router(app_state)
                .layer(tower_http::trace::TraceLayer::new_for_http())
                .layer(tower_http::cors::CorsLayer::permissive());

            let bind_addr = format!("{}:{}", server_config.host, server_config.port);
            info!("Server listening on {}", bind_addr);

            let listener = tokio::net::TcpListener::bind(&bind_addr)
                .await
                .map_err(|e| {
                    DocFusionError::internal(format!("Failed to bind to {bind_addr}: {e}"))
                })?;

            serve(listener, app)
                .await
                .map_err(|e| DocFusionError::internal(format!("Server error: {e}")))?;
        }
        Commands::Query { sql } => {
            let _span = query_span!(&sql);
            info!("Executing query");

            let start = Instant::now();
            let df = df_ctx.sql(&sql).await?;
            let batches = df.collect().await?;
            let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            let duration = start.elapsed();

            log_performance!("query_execution", duration, "rows_returned" => rows);
            println!("Rows returned: {rows}");
            println!("Time taken: {duration:?}");
        }
        Commands::Insert { json } => {
            info!("Inserting document");
            // Parse the JSON string into a serde_json::Value
            let json_value: JsonValue = serde_json::from_str(&json)
                .map_err(|e| DocFusionError::invalid_document(format!("Invalid JSON: {e}")))?;

            let start = Instant::now();
            let stmt = "INSERT INTO documents (doc) VALUES ($1::jsonb)";
            let n = pg_client.execute(stmt, &[&json_value]).await?;
            let duration = start.elapsed();

            log_performance!("document_insert", duration, "rows_affected" => n);
            info!(rows_inserted = n, "Document inserted successfully");
            println!("Inserted {n} row(s)");
        }
        Commands::Update { id, json } => {
            info!(document_id = id, "Updating document");
            let json_value: JsonValue = serde_json::from_str(&json)
                .map_err(|e| DocFusionError::invalid_document(format!("Invalid JSON: {e}")))?;

            let start = Instant::now();
            let stmt = "UPDATE documents SET doc = $1::jsonb WHERE id = $2";
            let n = pg_client.execute(stmt, &[&json_value, &id]).await?;
            let duration = start.elapsed();

            if n == 0 {
                warn!(document_id = id, "Document not found for update");
                return Err(DocFusionError::document_not_found(id));
            }

            log_performance!("document_update", duration, "rows_affected" => n);
            info!(
                document_id = id,
                rows_updated = n,
                "Document updated successfully"
            );
            println!("Updated {n} row(s)");
        }
        Commands::Backup { output } => {
            info!("Starting backup to {}", output);
            let _span = query_span!(&format!("backup_{output}"));

            let start = Instant::now();
            let client = pool.get().await?;

            // Get all documents
            let rows = client
                .query("SELECT id, doc FROM documents ORDER BY id", &[])
                .await?;
            let mut documents = Vec::new();

            for row in rows {
                let id: i32 = row.get(0);
                let doc: JsonValue = row.get(1);
                documents.push(serde_json::json!({
                    "id": id,
                    "document": doc
                }));
            }

            // Write to file
            let backup_data = serde_json::json!({
                "metadata": {
                    "version": env!("CARGO_PKG_VERSION"),
                    "timestamp": chrono::Utc::now(),
                    "document_count": documents.len()
                },
                "documents": documents
            });

            let mut file = fs::File::create(&output)?;
            file.write_all(serde_json::to_string_pretty(&backup_data)?.as_bytes())?;

            let duration = start.elapsed();
            log_performance!("backup", duration, "document_count" => documents.len());
            info!(
                file_path = output,
                document_count = documents.len(),
                "Backup completed successfully"
            );
            println!("Backed up {} documents to {output}", documents.len());
        }
        Commands::Restore { input, clear } => {
            info!("Starting restore from {}", input);
            let _span = query_span!(&format!("restore_{input}"));

            let start = Instant::now();
            let client = pool.get().await?;

            // Read backup file
            let file_content = fs::read_to_string(&input)?;
            let backup_data: JsonValue = serde_json::from_str(&file_content)?;

            let documents = backup_data["documents"].as_array().ok_or_else(|| {
                DocFusionError::internal(
                    "Invalid backup format: missing documents array".to_string(),
                )
            })?;

            // Clear existing data if requested
            if clear {
                info!("Clearing existing documents");
                let clear_result = client.execute("DELETE FROM documents", &[]).await?;
                info!(rows_deleted = clear_result, "Cleared existing documents");
            }

            // Restore documents
            let mut restored_count = 0;
            for doc in documents {
                let document = &doc["document"];
                let insert_sql = "INSERT INTO documents (doc) VALUES ($1)";
                client.execute(insert_sql, &[document]).await?;
                restored_count += 1;
            }

            let duration = start.elapsed();
            log_performance!("restore", duration, "document_count" => restored_count);
            info!(
                file_path = input,
                document_count = restored_count,
                cleared = clear,
                "Restore completed successfully"
            );
            println!("Restored {restored_count} documents from {input}");
        }
    }

    Ok(())
}
