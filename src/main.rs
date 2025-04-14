use clap::{Parser, Subcommand};
use datafusion::arrow::datatypes::DataType;
use datafusion::execution::context::SessionContext;
use datafusion::logical_expr::create_udf;
use datafusion::logical_expr_common::signature::Volatility;
use docfusiondb::{
    PostgresTable, json_contains_udf, json_extract_path_udf, json_multi_contains_udf,
};
use serde_json::Value as JsonValue;
use std::env;
use std::sync::Arc;
use std::time::Instant;
use tokio_postgres::NoTls;

/// DocFusionDB CLI
#[derive(Parser)]
#[command(name = "docfusiondb", version, about = "CLI for DocFusionDB")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::init();

    // Read the DATABASE_URL env var, or default to a local Postgres
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:yourpassword@localhost:5432/docfusiondb".into());

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
    let df_table = PostgresTable::new().await?;
    df_ctx.register_table("documents", Arc::new(df_table))?;

    // Open a direct Postgres client for writes, using DATABASE_URL
    let (pg_client, pg_conn) = tokio_postgres::connect(&database_url, NoTls).await?;
    tokio::spawn(async move {
        let _ = pg_conn.await;
    });

    match cli.command {
        Commands::Query { sql } => {
            println!("Running query: {}", sql);
            let df = df_ctx.sql(&sql).await?;
            let batches = df.collect().await?;
            let rows: usize = batches.iter().map(|b| b.num_rows()).sum();
            println!("Rows returned: {}", rows);

            let start = Instant::now();
            let df2 = df_ctx.sql(&sql).await?;
            df2.collect().await?;
            println!("Time taken: {:.3?}", start.elapsed());
        }
        Commands::Insert { json } => {
            // Parse the JSON string into a serde_json::Value
            let json_value: JsonValue =
                serde_json::from_str(&json).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;
            let stmt = "INSERT INTO documents (doc) VALUES ($1::jsonb)";
            let n = pg_client.execute(stmt, &[&json_value]).await?;
            println!("Inserted {} row(s)", n);
        }
        Commands::Update { id, json } => {
            let json_value: JsonValue =
                serde_json::from_str(&json).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?;
            let stmt = "UPDATE documents SET doc = $1::jsonb WHERE id = $2";
            let n = pg_client.execute(stmt, &[&json_value, &id]).await?;
            println!("Updated {} row(s)", n);
        }
    }

    Ok(())
}
