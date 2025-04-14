use async_trait::async_trait;
use datafusion::catalog::Session;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::execution::context::SessionContext;
use datafusion::logical_expr::{Expr, Operator};
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, PlanProperties, SendableRecordBatchStream,
    Statistics, Partitioning,
};
use datafusion::physical_plan::metrics::MetricsSet;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::datatypes::{Field, DataType, Schema};
use datafusion::scalar::ScalarValue;
use datafusion::physical_expr::EquivalenceProperties;
use futures::stream::{self};
use std::sync::Arc;
use tokio_postgres::{Client, NoTls};

/// Convert a DataFusion expression to a SQL snippet.
///
/// This simplified translator supports:
/// - BinaryExpr where the operator is equality (Operator::Eq) or logical AND (Operator::And).
/// - Column references and literal values.
///
/// For a JSONB filter, it expects JSONB columns to be referenced with a prefix of "doc.",
/// e.g. a column reference "doc.status" is converted into "doc->>'status'".
fn expr_to_sql(expr: &Expr) -> Option<String> {
    match expr {
        // Use tuple pattern to destructure BinaryExpr.
        Expr::BinaryExpr(binary_expr) => {
            let left = &binary_expr.left;
            let op = &binary_expr.op;
            let right = &binary_expr.right;
            if *op == Operator::Eq {
                let left_sql = expr_to_sql(left)?;
                let right_sql = expr_to_sql(right)?;
                // Check if left is a JSONB column represented as "doc.field"
                if left_sql.starts_with("doc.") {
                    let field = left_sql.strip_prefix("doc.").unwrap();
                    Some(format!("doc->>'{}' = {}", field, right_sql))
                } else {
                    Some(format!("{} = {}", left_sql, right_sql))
                }
            } else if *op == Operator::And {
                let left_sql = expr_to_sql(left)?;
                let right_sql = expr_to_sql(right)?;
                Some(format!("({} AND {})", left_sql, right_sql))
            } else {
                None
            }
        },
        // For column references, just return the column name.
        Expr::Column(col) => Some(col.name.clone()),
        // For literals, if it's a Utf8 string, add quotes.
        Expr::Literal(scalar) => match scalar {
            ScalarValue::Utf8(Some(s)) => Some(format!("'{}'", s)),
            _ => Some(scalar.to_string()),
        },
        _ => None,
    }
}

/// Combine multiple filter expressions into a single SQL WHERE clause.
fn filters_to_sql(filters: &[Expr]) -> Option<String> {
    let mut conditions = Vec::new();
    for expr in filters {
        if let Some(cond) = expr_to_sql(expr) {
            conditions.push(cond);
        }
    }
    if conditions.is_empty() {
        None
    } else {
        Some(format!(" WHERE {}", conditions.join(" AND ")))
    }
}

/// Define a struct representing our Postgres table.
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

/// Implement DisplayAs for SimpleExec.
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
        let stream = stream::iter(self.batches.clone().into_iter().map(Ok));
        Ok(Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(), stream)))
    }
    fn metrics(&self) -> Option<MetricsSet> {
        Some(MetricsSet::new())
    }
    fn statistics(&self) -> datafusion::error::Result<Statistics> {
        Ok(Statistics::new_unknown(&self.schema()))
    }
    fn name(&self) -> &str {
        "SimpleExec"
    }
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
    async fn scan(
        &self,
        _state: &dyn Session,
        _projection: Option<&Vec<usize>>,
        filters: &[Expr],
        _limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        // Translate filter expressions into a SQL WHERE clause.
        let filter_clause = filters_to_sql(filters).unwrap_or_default();
        // Construct the SQL query.
        let query = format!("SELECT id, doc::text FROM documents{}", filter_clause);
        println!("Executing query: {}", query);
        let rows = self
            .client
            .query(&query, &[])
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
    // Execute a query with a filter.
    // For instance, this query is expected to match JSONB documents where doc->>'status' equals 'active'.
    let df = ctx.sql("SELECT doc FROM documents WHERE doc->>'status' = 'active'").await?;
    let results = df.collect().await?;
    for batch in results {
        println!("Batch: {:?}", batch);
    }
    let start = std::time::Instant::now();
    let df = ctx.sql("SELECT doc FROM documents WHERE doc->>'status' = 'active'").await?;
    df.collect().await?;
    let duration = start.elapsed();
    println!("Query took: {:?}", duration);
    Ok(())
}
