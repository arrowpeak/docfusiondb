use async_trait::async_trait;
use futures::stream;
use log::debug;
use serde_json::Value;
use std::sync::Arc;
use tokio_postgres::NoTls;
use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};

pub mod error;
pub mod config;
pub mod logging;
pub mod api;
pub mod cache;

#[cfg(test)]
mod tests;

pub use error::{DocFusionError, DocFusionResult};
pub use config::Config;

use datafusion::arrow::array::{
    Array, ArrayRef, BooleanBuilder, Int32Builder, StringArray, StringBuilder,
};
use datafusion::arrow::datatypes::{DataType, Field, Schema};
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::catalog::Session;
use datafusion::datasource::{TableProvider, TableType};
use datafusion::error::DataFusionError;
use datafusion::logical_expr::expr::ScalarFunction;
use datafusion::logical_expr::{Expr, Operator, TableProviderFilterPushDown};
use datafusion::physical_expr::EquivalenceProperties;
use datafusion::physical_plan::ColumnarValue;
use datafusion::physical_plan::execution_plan::{Boundedness, EmissionType};
use datafusion::physical_plan::metrics::MetricsSet;
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{
    DisplayAs, DisplayFormatType, ExecutionPlan, Partitioning, PlanProperties,
    SendableRecordBatchStream, Statistics,
};
use datafusion::scalar::ScalarValue;

/// UDF: extract a field from a JSON string column.
pub fn json_extract_path_udf(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
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
            ));
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
            ));
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
    Ok(ColumnarValue::Array(Arc::new(builder.finish())))
}

/// UDF: single-key JSONB containment (`@>`).
pub fn json_contains_udf(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    if args.len() != 2 {
        return Err(DataFusionError::Internal(
            "json_contains requires 2 arguments".to_string(),
        ));
    }
    let arr_doc = match &args[0] {
        ColumnarValue::Array(a) => a
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DataFusionError::Internal("Expected StringArray".to_string()))?,
        _ => {
            return Err(DataFusionError::Internal(
                "Invalid first argument to json_contains".to_string(),
            ));
        }
    };
    let arr_pat = match &args[1] {
        ColumnarValue::Array(a) => a
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DataFusionError::Internal("Expected StringArray".to_string()))?,
        _ => {
            return Err(DataFusionError::Internal(
                "Invalid second argument to json_contains".to_string(),
            ));
        }
    };

    let mut builder = BooleanBuilder::new();
    for i in 0..arr_doc.len() {
        if arr_doc.is_null(i) || arr_pat.is_null(i) {
            builder.append_null();
        } else {
            let v_doc = serde_json::from_str::<Value>(arr_doc.value(i)).unwrap_or(Value::Null);
            let v_pat = serde_json::from_str::<Value>(arr_pat.value(i)).unwrap_or(Value::Null);
            let contains = match (&v_doc, &v_pat) {
                (Value::Object(map1), Value::Object(map2)) => {
                    map2.iter().all(|(k, v)| map1.get(k) == Some(v))
                }
                _ => false,
            };
            builder.append_value(contains);
        }
    }
    Ok(ColumnarValue::Array(Arc::new(builder.finish())))
}

/// UDF: JSONB containment for multiple keys at once (`@>`).
pub fn json_multi_contains_udf(args: &[ColumnarValue]) -> datafusion::error::Result<ColumnarValue> {
    if args.len() != 2 {
        return Err(DataFusionError::Internal(
            "json_multi_contains requires 2 arguments".to_string(),
        ));
    }
    let arr_doc = match &args[0] {
        ColumnarValue::Array(a) => a
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DataFusionError::Internal("Expected StringArray".to_string()))?,
        _ => {
            return Err(DataFusionError::Internal(
                "Invalid first argument to json_multi_contains".to_string(),
            ));
        }
    };
    let arr_pat = match &args[1] {
        ColumnarValue::Array(a) => a
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DataFusionError::Internal("Expected StringArray".to_string()))?,
        _ => {
            return Err(DataFusionError::Internal(
                "Invalid second argument to json_multi_contains".to_string(),
            ));
        }
    };

    let mut builder = BooleanBuilder::new();
    for i in 0..arr_doc.len() {
        if arr_doc.is_null(i) || arr_pat.is_null(i) {
            builder.append_null();
        } else {
            let v_doc = serde_json::from_str::<Value>(arr_doc.value(i)).unwrap_or(Value::Null);
            let v_pat = serde_json::from_str::<Value>(arr_pat.value(i)).unwrap_or(Value::Null);
            let contains = match (&v_doc, &v_pat) {
                (Value::Object(map1), Value::Object(map2)) => {
                    map2.iter().all(|(k, v)| map1.get(k) == Some(v))
                }
                _ => false,
            };
            builder.append_value(contains);
        }
    }
    Ok(ColumnarValue::Array(Arc::new(builder.finish())))
}

fn expr_to_sql(expr: &Expr) -> Option<String> {
    match expr {
        Expr::ScalarFunction(ScalarFunction { func, args })
            if func.name() == "json_extract_path" =>
        {
            let col = expr_to_sql(&args[0])?;
            let key = match &args[1] {
                Expr::Literal(ScalarValue::Utf8(Some(k))) => k.clone(),
                _ => return None,
            };
            Some(format!("{}->>'{}'", col, key))
        }
        Expr::ScalarFunction(ScalarFunction { func, args }) if func.name() == "json_contains" => {
            let col = expr_to_sql(&args[0])?;
            let pat = expr_to_sql(&args[1])?;
            Some(format!("{} @> {}", col, pat))
        }
        Expr::ScalarFunction(ScalarFunction { func, args })
            if func.name() == "json_multi_contains" =>
        {
            let col = expr_to_sql(&args[0])?;
            let pat = expr_to_sql(&args[1])?;
            Some(format!("{} @> {}", col, pat))
        }
        Expr::BinaryExpr(be) if be.op == Operator::Eq => {
            let l = expr_to_sql(&be.left)?;
            let r = expr_to_sql(&be.right)?;
            if l.starts_with("doc.") {
                let field = l.strip_prefix("doc.").unwrap();
                Some(format!("doc->>'{}' = {}", field, r))
            } else {
                Some(format!("{} = {}", l, r))
            }
        }
        Expr::BinaryExpr(be) if be.op == Operator::And => {
            let l = expr_to_sql(&be.left)?;
            let r = expr_to_sql(&be.right)?;
            Some(format!("({} AND {})", l, r))
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

/// A DataFusion TableProvider backed by Postgres.
#[derive(Debug)]
pub struct PostgresTable {
    pool: Pool,
}

impl PostgresTable {
    /// Create a new PostgresTable with connection pooling.
    pub async fn new(config: &config::DatabaseConfig) -> DocFusionResult<Self> {
        let mut cfg = PoolConfig::new();
        cfg.host = Some(config.host.clone());
        cfg.port = Some(config.port);
        cfg.user = Some(config.user.clone());
        cfg.password = Some(config.password.clone());
        cfg.dbname = Some(config.database.clone());
        cfg.pool = Some(deadpool_postgres::PoolConfig::new(config.max_connections));
        
        let pool = cfg.create_pool(Some(Runtime::Tokio1), NoTls)?;
        
        // Test the connection
        let _conn = pool.get().await?;
        
        Ok(Self { pool })
    }
    
    /// Create a new PostgresTable from a connection pool.
    pub fn from_pool(pool: Pool) -> Self {
        Self { pool }
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
            Boundedness::Bounded,
        );
        Self {
            batches,
            schema,
            properties: props,
        }
    }
}

impl DisplayAs for SimpleExec {
    fn fmt_as(&self, _: DisplayFormatType, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SimpleExec({:?})", self)
    }
}

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
        _: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        Ok(self)
    }
    fn execute(
        &self,
        _part: usize,
        _ctx: Arc<datafusion::execution::TaskContext>,
    ) -> datafusion::error::Result<SendableRecordBatchStream> {
        let s = stream::iter(self.batches.clone().into_iter().map(Ok));
        Ok(Box::pin(RecordBatchStreamAdapter::new(
            self.schema.clone(),
            s,
        )))
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

#[async_trait]
impl TableProvider for PostgresTable {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int32, false),
            Field::new("doc", DataType::Utf8, true),
        ]))
    }
    fn table_type(&self) -> TableType {
        TableType::Base
    }

    fn supports_filters_pushdown(
        &self,
        filters: &[&Expr],
    ) -> datafusion::error::Result<Vec<TableProviderFilterPushDown>> {
        Ok(filters
            .iter()
            .map(|e| {
                if expr_to_sql(e).is_some() {
                    TableProviderFilterPushDown::Exact
                } else {
                    TableProviderFilterPushDown::Unsupported
                }
            })
            .collect())
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
        
        let client = self.pool.get().await.map_err(|e| {
            DataFusionError::Execution(format!("Failed to get connection from pool: {}", e))
        })?;
        
        let rows = client
            .query(&q, &[])
            .await
            .map_err(|e| DataFusionError::Execution(e.to_string()))?;

        // Build Arrow arrays
        let mut ib = Int32Builder::new();
        let mut sb = StringBuilder::new();
        for r in rows {
            ib.append_value(r.get(0));
            sb.append_value(r.get::<usize, String>(1));
        }
        let id_arr = Arc::new(ib.finish()) as ArrayRef;
        let doc_arr = Arc::new(sb.finish()) as ArrayRef;

        // E0716 fix: bind schema before borrowing fields
        let full_schema = self.schema();
        let fields = full_schema.fields();
        let projected_fields: Vec<_> = match proj {
            Some(indices) => indices.iter().map(|&i| fields[i].clone()).collect(),
            None => fields.iter().cloned().collect(),
        };
        let projected_schema = Arc::new(Schema::new(projected_fields));

        // Projected arrays
        let projected_arrays: Vec<ArrayRef> = match proj {
            Some(indices) => {
                let mut cols = Vec::with_capacity(indices.len());
                for &i in indices {
                    match i {
                        0 => cols.push(id_arr.clone()),
                        1 => cols.push(doc_arr.clone()),
                        _ => {
                            return Err(DataFusionError::Internal(format!(
                                "Invalid projection index {}",
                                i
                            )));
                        }
                    }
                }
                cols
            }
            None => vec![id_arr.clone(), doc_arr.clone()],
        };

        let batch = RecordBatch::try_new(projected_schema.clone(), projected_arrays)?;
        Ok(Arc::new(SimpleExec::new(vec![batch], projected_schema)))
    }
}

/// Helper function to convert deadpool config
impl From<&config::DatabaseConfig> for PoolConfig {
    fn from(config: &config::DatabaseConfig) -> Self {
        let mut cfg = PoolConfig::new();
        cfg.host = Some(config.host.clone());
        cfg.port = Some(config.port);
        cfg.user = Some(config.user.clone());
        cfg.password = Some(config.password.clone());
        cfg.dbname = Some(config.database.clone());
        cfg.pool = Some(deadpool_postgres::PoolConfig::new(config.max_connections));
        cfg
    }
}
