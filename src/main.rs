use async_trait::async_trait;
use datafusion::arrow::array::{Array, ArrayRef, Int32Builder, StringArray, StringBuilder};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::Session;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::logical_expr::{create_udf, Expr, Operator, TableProviderFilterPushDown};
use datafusion::logical_expr::expr::ScalarFunction;
use datafusion::logical_expr_common::signature::Volatility;
use datafusion::execution::context::SessionContext;
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
    SendableRecordBatchStream, Statistics,
};
use datafusion::physical_plan::metrics::MetricsSet;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::ColumnarValue;
use datafusion::scalar::ScalarValue;
use datafusion::error::DataFusionError;
use futures::stream;
use serde_json::Value;
use std::sync::Arc;
use tokio_postgres::{Client, NoTls};
use log::debug;

/// UDF: extract a field from a JSON string column.
fn json_extract_path_udf(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    if args.len() != 2 {
        return Err(DataFusionError::Internal(
            "json_extract_path requires 2 arguments".to_string(),
        ));
    }
    // Downcast first arg to StringArray
    let json_array = match &args[0] {
        ColumnarValue::Array(arr) => arr
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DataFusionError::Internal("Expected StringArray".to_string()))?,
        ColumnarValue::Scalar(ScalarValue::Utf8(Some(s))) => {
            let mut b = StringBuilder::new();
            b.append_value(s);
            let arr = Arc::new(b.finish()) as ArrayRef;
            return Ok(ColumnarValue::Array(arr));
        }
        _ => {
            return Err(DataFusionError::Internal(
                "Invalid first argument to json_extract_path".to_string(),
            ))
        }
    };
    // Extract key from second arg
    let key = match &args[1] {
        ColumnarValue::Scalar(ScalarValue::Utf8(Some(k))) => k.clone(),
        ColumnarValue::Array(arr) => {
            let sa = arr
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| DataFusionError::Internal("Expected StringArray".to_string()))?;
            if sa.len() != 1 {
                return Err(DataFusionError::Internal(
                    "Key array must have exactly one element".to_string(),
                ));
            }
            sa.value(0).to_string()
        }
        _ => {
            return Err(DataFusionError::Internal(
                "Invalid second argument to json_extract_path".to_string(),
            ))
        }
    };
    // Build result array
    let mut builder = StringBuilder::new();
    for i in 0..json_array.len() {
        if json_array.is_null(i) {
            builder.append_null();
        } else {
            let json_str = json_array.value(i);
            let parsed: Result<Value, _> = serde_json::from_str(json_str);
            let out = parsed
                .ok()
                .and_then(|v| v.get(&key).and_then(Value::as_str).map(String::from))
                .unwrap_or_default();
            builder.append_value(out);
        }
    }
    let result_arr = Arc::new(builder.finish()) as ArrayRef;
    Ok(ColumnarValue::Array(result_arr))
}

/// Convert DataFusion filter expressions into a SQL WHERE clause.
fn expr_to_sql(expr: &Expr) -> Option<String> {
    debug!("Translating expr: {:?}", expr);
    match expr {
        Expr::ScalarFunction(ScalarFunction { func, args })
            if func.name() == "json_extract_path" =>
        {
            if args.len() != 2 {
                return None;
            }
            let json_col = expr_to_sql(&args[0])?;
            let key = match &args[1] {
                Expr::Literal(ScalarValue::Utf8(Some(k))) => k.clone(),
                _ => return None,
            };
            let sql = format!("{}->>'{}'", json_col, key);
            debug!("Translated json_extract_path to: {}", sql);
            Some(sql)
        }
        Expr::BinaryExpr(be) => {
            // LongArrow ->> extraction
            let op_str = format!("{:?}", be.op);
            if op_str == "LongArrow" {
                let l = expr_to_sql(&be.left)?;
                let r = expr_to_sql(&be.right)?;
                let field = r.trim_matches('\'');
                let sql = format!("{}->>'{}'", l, field);
                debug!("Translated LongArrow to: {}", sql);
                return Some(sql);
            }
            // Equality
            if be.op == Operator::Eq {
                let l = expr_to_sql(&be.left)?;
                let r = expr_to_sql(&be.right)?;
                let sql = if l.starts_with("doc.") {
                    let field = l.strip_prefix("doc.").unwrap();
                    format!("doc->>'{}' = {}", field, r)
                } else {
                    format!("{} = {}", l, r)
                };
                debug!("Translated Eq to: {}", sql);
                return Some(sql);
            }
            // AND
            if be.op == Operator::And {
                let l = expr_to_sql(&be.left)?;
                let r = expr_to_sql(&be.right)?;
                let sql = format!("({} AND {})", l, r);
                debug!("Translated And to: {}", sql);
                return Some(sql);
            }
            None
        }
        Expr::Column(c) => Some(c.name.clone()),
        Expr::Literal(s) => match s {
            ScalarValue::Utf8(Some(v)) => Some(format!("'{}'", v)),
            _ => Some(s.to_string()),
        },
        _ => None,
    }
}

fn filters_to_sql(filters: &[Expr]) -> Option<String> {
    let conds: Vec<_> = filters.iter().filter_map(expr_to_sql).collect();
    if conds.is_empty() {
        None
    } else {
        Some(format!(" WHERE {}", conds.join(" AND ")))
    }
}

/// PostgresTable with an inherent async `new` method.
#[derive(Debug)]
struct PostgresTable {
    client: Client,
}

impl PostgresTable {
    pub async fn new() -> Result<Self, tokio_postgres::Error> {
        let (client, conn) = tokio_postgres::connect(
            "host=localhost user=postgres password=yourpassword dbname=docfusiondb",
            NoTls,
        )
        .await?;
        tokio::spawn(async move { let _ = conn.await; });
        Ok(Self { client })
    }
}

#[derive(Debug)]
struct SimpleExec {
    batches: Vec<RecordBatch>,
    schema: Arc<Schema>,
    properties: PlanProperties,
}

impl SimpleExec {
    fn new(batches: Vec<RecordBatch>, schema: Arc<Schema>) -> Self {
        let props = PlanProperties::new(
            EquivalenceProperties::new(schema.clone()),
            Partitioning::UnknownPartitioning(1),
            EmissionType::Incremental,
            Boundedness::Bounded,  // <— use the imported Boundedness
        );
        Self { batches, schema, properties: props }
    }
}

impl DisplayAs for SimpleExec {
    fn fmt_as(&self, _: DisplayFormatType, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimpleExec({:?})", self)
    }
}

impl ExecutionPlan for SimpleExec {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn schema(&self) -> Arc<Schema> { self.schema.clone() }
    fn children(&self) -> Vec<&Arc<dyn ExecutionPlan>> { vec![] }
    fn with_new_children(
        self: Arc<Self>,
        _: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> { Ok(self) }
    fn execute(
        &self,
        _part: usize,
        _ctx: Arc<datafusion::execution::TaskContext>,
    ) -> datafusion::error::Result<SendableRecordBatchStream> {
        let s = stream::iter(self.batches.clone().into_iter().map(Ok));
        Ok(Box::pin(RecordBatchStreamAdapter::new(self.schema.clone(), s)))
    }
    fn metrics(&self) -> Option<MetricsSet> { Some(MetricsSet::new()) }
    fn statistics(&self) -> datafusion::error::Result<Statistics> {
        Ok(Statistics::new_unknown(&self.schema()))
    }
    fn name(&self) -> &str { "SimpleExec" }
    fn properties(&self) -> &PlanProperties { &self.properties }
}

#[async_trait]
impl TableProvider for PostgresTable {
    fn as_any(&self) -> &dyn std::any::Any { self }
    fn schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("doc", DataType::Utf8, true),
        ]))
    }
    fn table_type(&self) -> TableType { TableType::Base }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> datafusion::error::Result<Vec<TableProviderFilterPushDown>> {
        let pushdown = filters
            .iter()
            .map(|expr| {
                if expr_to_sql(expr).is_some() {
                    TableProviderFilterPushDown::Exact
                } else {
                    TableProviderFilterPushDown::Unsupported
                }
            })
            .collect();
        Ok(pushdown)
    }

    async fn scan(
        &self,
        _state: &dyn Session,
        proj: Option<&Vec<usize>>,
        filters: &[Expr],
        _limit: Option<usize>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        let where_clause = filters_to_sql(filters).unwrap_or_default();
        let q = format!("SELECT id, doc::text FROM documents{}", where_clause);
        debug!("Executing SQL: {}", q);
        let rows = self.client.query(&q, &[]).await
            .map_err(|e| DataFusionError::Execution(e.to_string()))?;

        let mut ib = Int32Builder::new();
        let mut sb = StringBuilder::new();
        for r in rows {
            ib.append_value(r.get(0));
            sb.append_value(r.get::<usize, String>(1));
        }
        let id_arr = Arc::new(ib.finish()) as ArrayRef;
        let doc_arr = Arc::new(sb.finish()) as ArrayRef;

        let full_schema = self.schema();
        let fields = full_schema.fields();
        let projected_fields = match proj {
            Some(indices) => indices.iter().map(|&i| fields[i].clone()).collect(),
            None => fields.to_vec(),
        };
        let projected_schema = Arc::new(Schema::new(projected_fields));

        let projected_arrays = match proj {
            Some(indices) => indices
                .iter()
                .map(|&i| match i {
                    0 => Ok(id_arr.clone()),
                    1 => Ok(doc_arr.clone()),
                    _ => Err(DataFusionError::Internal("Invalid projection index".to_string())),
                })
                .collect::<Result<Vec<_>, _>>()?,
            None => vec![id_arr, doc_arr],
        };

        let batch = RecordBatch::try_new(projected_schema.clone(), projected_arrays)?;
        Ok(Arc::new(SimpleExec::new(vec![batch], projected_schema)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let ctx = SessionContext::new();

    let json_udf = create_udf(
        "json_extract_path",
        vec![DataType::Utf8, DataType::Utf8],
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(json_extract_path_udf),
    );
    ctx.register_udf(json_udf);

    let table = PostgresTable::new().await?;
    ctx.register_table("documents", Arc::new(table))?;

    let query = "SELECT json_extract_path(doc, 'status') AS status \
                 FROM documents \
                 WHERE json_extract_path(doc, 'status') = 'active'";
    println!("Running query: {}", query);
    let df = ctx.sql(query).await?;
    let results = df.collect().await?;
    println!("Number of batches: {}", results.len());

    let start = std::time::Instant::now();
    let df2 = ctx.sql(query).await?;
    df2.collect().await?;
    println!("Query took: {:?}", start.elapsed());

    Ok(())
}
