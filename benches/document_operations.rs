use criterion::{Criterion, black_box, criterion_group, criterion_main};
use docfusiondb::cache::QueryCache;
use serde_json::json;
use std::collections::HashMap;

// Benchmark JSON document parsing
fn bench_json_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_parsing");

    let sample_doc = json!({
        "title": "Benchmark Document",
        "content": "This is a benchmark document for performance testing",
        "tags": ["benchmark", "performance", "test"],
        "metadata": {
            "author": "benchmark",
            "created_at": "2024-01-01T00:00:00Z",
            "benchmark_run": true
        },
        "stats": {
            "word_count": 150,
            "reading_time": 3
        }
    });

    group.bench_function("serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&sample_doc).unwrap());
        });
    });

    group.bench_function("deserialize", |b| {
        let json_str = serde_json::to_string(&sample_doc).unwrap();
        b.iter(|| {
            black_box(serde_json::from_str::<serde_json::Value>(&json_str).unwrap());
        });
    });

    group.finish();
}

// Benchmark cache operations
fn bench_cache(c: &mut Criterion) {
    let cache = QueryCache::new(300, 100); // 5 min TTL, 100 entries

    let mut group = c.benchmark_group("cache");

    // Cache put operations
    group.bench_function("cache_put", |b| {
        b.iter(|| {
            let rand_id = black_box(rand::random::<u32>() % 1000);
            let key = format!("SELECT * FROM documents WHERE id = {rand_id}");
            let result = vec![{
                let mut map = HashMap::new();
                map.insert("id".to_string(), json!(1));
                map.insert("doc".to_string(), json!({"title": "Test"}));
                map
            }];
            cache.put(key, result);
            black_box(());
        });
    });

    // Cache get operations (populate first)
    for i in 0..50 {
        let key = format!("cached_query_{i}");
        let result = vec![{
            let mut map = HashMap::new();
            map.insert("id".to_string(), json!(i));
            map.insert(
                "doc".to_string(),
                json!({"title": format!("Cached Doc {i}")}),
            );
            map
        }];
        cache.put(key, result);
    }

    group.bench_function("cache_get_hit", |b| {
        b.iter(|| {
            let rand_num = black_box(rand::random::<usize>() % 50);
            let key = format!("cached_query_{rand_num}");
            black_box(cache.get(&key));
        });
    });

    group.bench_function("cache_get_miss", |b| {
        b.iter(|| {
            let rand_num = black_box(rand::random::<u32>());
            let key = format!("missing_query_{rand_num}");
            black_box(cache.get(&key));
        });
    });

    group.finish();
}

criterion_group!(benches, bench_json_parsing, bench_cache);
criterion_main!(benches);
