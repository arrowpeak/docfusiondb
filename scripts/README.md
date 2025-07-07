# DocFusionDB Benchmarking Tools

Performance testing and benchmarking utilities for DocFusionDB.

## Quick Start

```bash
# Start DocFusionDB server
export AUTH_ENABLED=true
export API_KEY=demo-api-key
cargo run -- serve

# Run HTTP API benchmarks (in another terminal)
./scripts/run_benchmarks.sh

# Run low-level Rust benchmarks
cargo bench
```

## HTTP API Benchmarks

The `benchmark.py` script tests real-world HTTP API performance with concurrent requests.

### Features Tested
- 📝 **Document Creation**: Single document insert performance
- 📋 **Document Listing**: Pagination and retrieval performance  
- 🔍 **Custom Queries**: SQL query execution and caching
- 📦 **Bulk Operations**: Multi-document insert efficiency
- 📊 **Metrics Endpoint**: Monitoring endpoint performance

### Usage

**Basic benchmarking:**
```bash
python3 scripts/benchmark.py --url http://localhost:8080 --api-key demo-api-key
```

**Custom configuration:**
```bash
python3 scripts/benchmark.py \
  --url http://localhost:8080 \
  --api-key your-api-key \
  --requests 200 \
  --concurrency 20 \
  --bulk-size 100
```

**Environment variables:**
```bash
export DOCFUSION_URL=http://localhost:8080
export API_KEY=demo-api-key
export REQUESTS=500
export CONCURRENCY=25
./scripts/run_benchmarks.sh
```

### Sample Output

```
🎯 BENCHMARK RESULTS
================================================================================

📊 Single Document Creation
   Total Requests: 100
   Duration: 2.34s
   Requests/sec: 42.74
   Success Rate: 100.0%
   Avg Latency: 23.4ms
   P95 Latency: 45.2ms
   P99 Latency: 67.8ms

📊 Custom Queries
   Total Requests: 100
   Duration: 0.89s
   Requests/sec: 112.36
   Success Rate: 100.0%
   Avg Latency: 8.9ms
   P95 Latency: 15.2ms
   P99 Latency: 23.1ms
```

## Low-Level Rust Benchmarks

The `cargo bench` command runs criterion-based benchmarks that test internal components.

### Features Tested
- 🗄️ **Database Operations**: Direct PostgreSQL performance
- 📦 **Bulk Inserts**: Batch insert optimization
- 🔍 **Query Performance**: Raw SQL execution speed
- ⚡ **Cache Operations**: In-memory cache performance

### Usage

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench cache

# Save results for comparison
cargo bench -- --save-baseline main

# Compare with baseline
cargo bench -- --baseline main
```

### Sample Output

```
document_insert/single/1   time:   [23.45 ms 24.12 ms 24.89 ms]
document_insert/single/10  time:   [234.1 ms 241.2 ms 248.9 ms]
bulk_insert/bulk/50        time:   [89.34 ms 91.23 ms 93.45 ms]
cache/cache_get_hit        time:   [245.67 ns 251.23 ns 257.89 ns]
```

## Performance Tips

### For Better Results
1. **Warm up the cache**: Run queries multiple times to see caching benefits
2. **Use appropriate concurrency**: Start low (5-10) and increase gradually
3. **Monitor system resources**: Check CPU, memory, and disk I/O during tests
4. **Test with realistic data**: Use documents similar to your production data

### Expected Performance
- **Single inserts**: 50-200 requests/sec (depends on document size)
- **Bulk inserts**: 500-2000 documents/sec (batch size dependent)
- **Cached queries**: 200-1000+ requests/sec (cache hit rate dependent)
- **Cold queries**: 20-100 requests/sec (query complexity dependent)

## Interpreting Results

### Key Metrics
- **RPS (Requests/Second)**: Higher is better
- **Latency**: Lower is better
- **P95/P99**: 95th/99th percentile latency (outliers)
- **Success Rate**: Should be 100% for healthy systems

### Cache Performance
- **First run**: Low cache hit rate, higher latency
- **Subsequent runs**: Higher cache hit rate, lower latency
- **Cache effectiveness**: Monitor via `/metrics` endpoint

### Troubleshooting

**Low performance?**
- Check database connection limits
- Monitor PostgreSQL performance
- Verify network latency
- Check system resource usage

**High error rates?**
- Verify server is running and healthy
- Check API key authentication
- Review server logs for errors
- Reduce concurrency if overwhelmed

## Custom Benchmarks

You can create custom benchmarks by:

1. **Modifying the Python script**: Add new operations or test scenarios
2. **Creating new Rust benchmarks**: Add to `benches/` directory
3. **Using curl for simple tests**: Quick one-off performance checks

Example curl benchmark:
```bash
# Warm up
for i in {1..10}; do
  curl -s -X POST localhost:8080/documents \
    -H "X-API-Key: demo-api-key" \
    -d '{"document": {"test": true}}' > /dev/null
done

# Time 100 requests
time for i in {1..100}; do
  curl -s -X POST localhost:8080/documents \
    -H "X-API-Key: demo-api-key" \
    -d '{"document": {"test": '$i'}}' > /dev/null
done
```

## Integration with CI/CD

The benchmarks can be integrated into CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Performance Tests
  run: |
    cargo run -- serve &
    sleep 5
    ./scripts/run_benchmarks.sh
    cargo bench
```

This helps catch performance regressions early in development.
