use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use datafusion::arrow::datatypes::DataType;
use datafusion::execution::context::SessionContext;
use datafusion::logical_expr::create_udf;
use docfusiondb::{
    PostgresTable, json_contains_udf, json_extract_path_udf, json_multi_contains_udf,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

async fn run_query(ctx: &SessionContext, sql: &str) {
    let df = ctx.sql(sql).await.unwrap();
    let _ = df.collect().await.unwrap();
}

fn bench_json_filters(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let ctx = rt.block_on(async {
        let ctx = SessionContext::new();
        ctx.register_udf(create_udf(
            "json_extract_path",
            vec![DataType::Utf8, DataType::Utf8],
            DataType::Utf8,
            datafusion::logical_expr_common::signature::Volatility::Immutable,
            Arc::new(json_extract_path_udf),
        ));
        ctx.register_udf(create_udf(
            "json_contains",
            vec![DataType::Utf8, DataType::Utf8],
            DataType::Boolean,
            datafusion::logical_expr_common::signature::Volatility::Immutable,
            Arc::new(json_contains_udf),
        ));
        ctx.register_udf(create_udf(
            "json_multi_contains",
            vec![DataType::Utf8, DataType::Utf8],
            DataType::Boolean,
            datafusion::logical_expr_common::signature::Volatility::Immutable,
            Arc::new(json_multi_contains_udf),
        ));
        let config = docfusiondb::config::Config::load().unwrap();
        let table = PostgresTable::new(&config.database).await.unwrap();
        ctx.register_table("documents", Arc::new(table)).unwrap();
        ctx
    });

    let q1 = "SELECT json_extract_path(doc,'status') \
              FROM documents \
              WHERE json_extract_path(doc,'status')='active' \
                AND json_contains(doc,'{\"category\":\"urgent\"}')";
    let q2 = "SELECT json_extract_path(doc,'status') \
              FROM documents \
              WHERE json_multi_contains(doc,'{\"status\":\"active\",\"category\":\"urgent\"}')";

    let mut group = c.benchmark_group("json_filters");
    group.throughput(Throughput::Elements(1));

    group.bench_function("separate UDFs", |b| {
        b.to_async(&rt).iter(|| run_query(&ctx, q1))
    });
    group.bench_function("multi_contains", |b| {
        b.to_async(&rt).iter(|| run_query(&ctx, q2))
    });

    group.finish();
}

criterion_group!(benches, bench_json_filters);
criterion_main!(benches);
