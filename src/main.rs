use async_trait::async_trait;
use datafusion::catalog::Session;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::execution::context::SessionContext;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, SendableRecordBatchStream,
    Statistics, Partitioning,
};
use datafusion::physical_plan::metrics::MetricsSet;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::{Field, DataType, Schema};
use datafusion::physical_expr::EquivalenceProperties;
use futures::stream::{self};
use std::sync::Arc;
use tokio_postgres::{Client, NoTls};

/// Define a struct to represent our Postgres table.
#[derive(Debug)]
struct PostgresTable {
    client: Client,
}

impl PostgresTable {
    async fn new() -> Result<Self, tokio_postgres::Error> {
        // Connect to Postgres.
        let (client, connection) = tokio_postgres::connect(
            "host=localhost user=postgres password=yourpassword dbname=docfusiondb",
            NoTls,
        )
        .await?;
        // Spawn a task to manage the connection.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Postgres connection error: {}", e);
            }
        });
        Ok(Self { client })
    }
}

/// Custom ExecutionPlan implementation that stores computed plan properties.
#[derive(Debug)]
struct SimpleExec {
    batches: Vec<RecordBatch>,
    schema: Arc<Schema>,
    properties: PlanProperties,
}

impl SimpleExec {
    fn new(batches: Vec<RecordBatch>, schema: Arc<Schema>) -> Self {
        let properties = PlanProperties::new(
            EquivalenceProperties::new(schema.clone()),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,
        );
        Self { batches, schema, properties }
    }
}

/// Implement DisplayAs for SimpleExec to satisfy the ExecutionPlan trait.
impl DisplayAs for SimpleExec {
    fn fmt_as(
        &self,
        _t: DisplayFormatType,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "SimpleExec({:?})", self)
    }
}

/// Implement ExecutionPlan for SimpleExec.
impl ExecutionPlan for SimpleExec {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn schema(&self) -> Arc<Schema> {
        self.schema.clone()
    }
    // Return no children.
    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> {
        vec![]
    }
    fn with_new_children(
        self: Arc<Self>,
        _children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }
    fn execute(
        &self,
        _partition: usize,
        _context: Arc<datafusion::execution::TaskContext>,
    ) -> datafusion::error::Result<SendableRecordBatchStream> {
        // Convert the vector of RecordBatches into an async stream.
        let stream = stream::iter(self.batches.clone().into_iter().map(Ok));
        Ok(Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(), stream)))
    }
    fn metrics(&self) -> Option<MetricsSet> {
        Some(MetricsSet::new())
    }
    fn statistics(&self) -> datafusion::error::Result<Statistics> {
        Ok(Statistics::new_unknown(&self.schema()))
    }
    // Provide a fixed name.
    fn name(&self) -> &str {
        "SimpleExec"
    }
    // Return the computed plan properties.
    fn properties(&self) -> &PlanProperties {
        &self.properties
    }
}

/// Implement TableProvider for PostgresTable.
#[async_trait]
impl TableProvider for PostgresTable {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    // Always return the full table schema.
    fn schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("doc", DataType::Utf8, false),
        ]))
    }
    fn table_type(&self) -> TableType {
        TableType::Base
    }
    // For simplicity, ignore the projection parameter and always produce full schema data.
    async fn scan(
        &self,
        _state: &dyn Session,
        _projection: Option<&Vec<usize>>,
        _filters: &[datafusion::logical_expr::Expr],
        _limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        // Query Postgres for the data.
        let rows = self
            .client
            .query("SELECT id, doc::text FROM documents", &[])
            .await
            .map_err(|e| datafusion::error::DataFusionError::Execution(e.to_string()))?;
        // Build Arrow arrays.
        let mut id_builder = datafusion::arrow::array::Int32Builder::new();
        let mut doc_builder = datafusion::arrow::array::StringBuilder::new();
        for row in rows {
            id_builder.append_value(row.get::<usize, i32>(0));
            doc_builder.append_value(row.get::<usize, String>(1));
        }
        let id_array = Arc::new(id_builder.finish())
            as Arc<dyn datafusion::arrow::array::Array>;
        let doc_array = Arc::new(doc_builder.finish())
            as Arc<dyn datafusion::arrow::array::Array>;
        // Use the full schema.
        let full_schema = self.schema();
        let batch = RecordBatch::try_new(full_schema.clone(), vec![id_array, doc_array])?;
        // Create our physical plan using the full schema.
        let exec = SimpleExec::new(vec![batch], full_schema);
        Ok(Arc::new(exec))
    }
}

/// The main function creates a DataFusion SessionContext, registers the Postgres table,
/// executes a simple query, and prints the results.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a DataFusion session context.
    let ctx = SessionContext::new();
    // Connect to Postgres and create the table provider.
    let table = PostgresTable::new().await?;
    let table_provider: Arc<dyn TableProvider> = Arc::new(table);
    // Register the table with DataFusion.
    ctx.register_table("documents", table_provider)?;
    // Execute a simple query: SELECT doc FROM documents.
    let df = ctx.sql("SELECT doc FROM documents").await?;
    let results = df.collect().await?;
    // Print the results.
    for batch in results {
        println!("Batch: {:?}", batch);
    }
    // Measure query performance.
    let start = std::time::Instant::now();
    let df = ctx.sql("SELECT doc FROM documents").await?;
    df.collect().await?;
    let duration = start.elapsed();
    println!("Query took: {:?}", duration);
    Ok(())
}
