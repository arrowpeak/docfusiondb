#!/usr/bin/env python3
"""
Simple benchmark script for DocFusionDB HTTP API
Tests real-world usage scenarios with concurrent requests
"""

import asyncio
import aiohttp
import json
import time
import statistics
import argparse
from typing import List, Dict, Any
from dataclasses import dataclass

@dataclass
class BenchmarkResult:
    operation: str
    total_requests: int
    duration: float
    rps: float
    avg_latency: float
    p95_latency: float
    p99_latency: float
    success_rate: float

class DocFusionBenchmark:
    def __init__(self, base_url: str = "http://localhost:8080", api_key: str = None):
        self.base_url = base_url
        self.api_key = api_key
        self.session = None
    
    async def __aenter__(self):
        headers = {"Content-Type": "application/json"}
        if self.api_key:
            headers["X-API-Key"] = self.api_key
        
        self.session = aiohttp.ClientSession(
            headers=headers,
            timeout=aiohttp.ClientTimeout(total=30)
        )
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        if self.session:
            await self.session.close()
    
    async def health_check(self) -> bool:
        """Check if the server is running"""
        try:
            async with self.session.get(f"{self.base_url}/health") as resp:
                return resp.status == 200
        except:
            return False
    
    async def create_document(self, doc: Dict[str, Any]) -> Dict[str, Any]:
        """Create a single document"""
        async with self.session.post(
            f"{self.base_url}/documents",
            json={"document": doc}
        ) as resp:
            return await resp.json(), resp.status
    
    async def bulk_create(self, docs: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Create multiple documents at once"""
        async with self.session.post(
            f"{self.base_url}/documents/bulk",
            json={"documents": docs}
        ) as resp:
            return await resp.json(), resp.status
    
    async def list_documents(self, limit: int = 10, offset: int = 0) -> Dict[str, Any]:
        """List documents"""
        async with self.session.get(
            f"{self.base_url}/documents?limit={limit}&offset={offset}"
        ) as resp:
            return await resp.json(), resp.status
    
    async def execute_query(self, sql: str) -> Dict[str, Any]:
        """Execute a custom SQL query"""
        async with self.session.post(
            f"{self.base_url}/query",
            json={"sql": sql}
        ) as resp:
            return await resp.json(), resp.status
    
    async def get_metrics(self) -> Dict[str, Any]:
        """Get server metrics"""
        async with self.session.get(f"{self.base_url}/metrics") as resp:
            return await resp.json(), resp.status
    
    async def timed_operation(self, operation, *args, **kwargs):
        """Execute an operation and measure latency"""
        start_time = time.perf_counter()
        try:
            result, status = await operation(*args, **kwargs)
            duration = time.perf_counter() - start_time
            return duration, True, result
        except Exception as e:
            duration = time.perf_counter() - start_time
            return duration, False, str(e)
    
    async def run_concurrent_benchmark(
        self,
        operation_name: str,
        operation,
        operation_args: List[tuple],
        concurrency: int = 10
    ) -> BenchmarkResult:
        """Run a benchmark with concurrent requests"""
        print(f"\n🚀 Running {operation_name} benchmark...")
        print(f"   Requests: {len(operation_args)}, Concurrency: {concurrency}")
        
        latencies = []
        successes = 0
        semaphore = asyncio.Semaphore(concurrency)
        
        async def limited_operation(args):
            async with semaphore:
                return await self.timed_operation(operation, *args)
        
        start_time = time.perf_counter()
        
        # Execute all operations concurrently
        tasks = [limited_operation(args) for args in operation_args]
        results = await asyncio.gather(*tasks)
        
        total_duration = time.perf_counter() - start_time
        
        # Analyze results
        for latency, success, _ in results:
            latencies.append(latency)
            if success:
                successes += 1
        
        total_requests = len(operation_args)
        rps = total_requests / total_duration
        avg_latency = statistics.mean(latencies)
        p95_latency = statistics.quantiles(latencies, n=20)[18] if latencies else 0  # 95th percentile
        p99_latency = statistics.quantiles(latencies, n=100)[98] if latencies else 0  # 99th percentile
        success_rate = successes / total_requests
        
        return BenchmarkResult(
            operation=operation_name,
            total_requests=total_requests,
            duration=total_duration,
            rps=rps,
            avg_latency=avg_latency * 1000,  # Convert to ms
            p95_latency=p95_latency * 1000,
            p99_latency=p99_latency * 1000,
            success_rate=success_rate
        )

def generate_sample_documents(count: int) -> List[Dict[str, Any]]:
    """Generate sample documents for testing"""
    docs = []
    categories = ["blog", "article", "note", "tutorial", "review"]
    tags_pool = ["rust", "database", "performance", "api", "web", "backend", "json", "sql"]
    
    for i in range(count):
        doc = {
            "title": f"Benchmark Document {i}",
            "content": f"This is the content of document {i} for performance testing. " * 5,
            "category": categories[i % len(categories)],
            "tags": tags_pool[i % 3:(i % 3) + 3],
            "metadata": {
                "author": f"author_{i % 10}",
                "created_at": "2024-01-01T00:00:00Z",
                "published": i % 2 == 0,
                "view_count": i * 10,
                "benchmark": True
            },
            "stats": {
                "word_count": 100 + (i % 50),
                "reading_time": 2 + (i % 5)
            }
        }
        docs.append(doc)
    
    return docs

async def run_benchmarks(args):
    """Run the complete benchmark suite"""
    async with DocFusionBenchmark(args.url, args.api_key) as bench:
        # Check server health
        if not await bench.health_check():
            print("❌ Server is not responding at", args.url)
            return
        
        print(f"✅ Server is healthy at {args.url}")
        
        results = []
        
        # Benchmark 1: Single document creation
        docs = generate_sample_documents(args.requests)
        single_args = [(doc,) for doc in docs[:args.requests]]
        result = await bench.run_concurrent_benchmark(
            "Single Document Creation",
            bench.create_document,
            single_args,
            args.concurrency
        )
        results.append(result)
        
        # Benchmark 2: Document listing
        list_args = [() for _ in range(args.requests // 2)]  # Fewer list operations
        result = await bench.run_concurrent_benchmark(
            "Document Listing",
            bench.list_documents,
            list_args,
            args.concurrency
        )
        results.append(result)
        
        # Benchmark 3: Custom queries (test caching)
        queries = [
            "SELECT json_extract_path(doc, 'title') as title FROM documents WHERE json_contains(doc, '{\"published\": true}') LIMIT 5",
            "SELECT json_extract_path(doc, 'category') as category, COUNT(*) as count FROM documents GROUP BY json_extract_path(doc, 'category') LIMIT 10",
            "SELECT * FROM documents WHERE json_extract_path(doc, 'metadata', 'author') = 'author_1' LIMIT 10",
            "SELECT json_extract_path(doc, 'title') as title FROM documents WHERE json_multi_contains(doc, '{\"benchmark\": true}') LIMIT 10"
        ]
        query_args = [(query,) for query in queries * (args.requests // len(queries))]
        result = await bench.run_concurrent_benchmark(
            "Custom Queries",
            bench.execute_query,
            query_args,
            args.concurrency
        )
        results.append(result)
        
        # Benchmark 4: Bulk operations
        if args.bulk_size > 0:
            bulk_docs = generate_sample_documents(args.bulk_size)
            bulk_batches = [bulk_docs[i:i+args.bulk_size] for i in range(0, len(bulk_docs), args.bulk_size)]
            bulk_args = [(batch,) for batch in bulk_batches[:args.requests // 10]]  # Fewer bulk operations
            result = await bench.run_concurrent_benchmark(
                "Bulk Document Creation",
                bench.bulk_create,
                bulk_args,
                min(args.concurrency, 5)  # Lower concurrency for bulk ops
            )
            results.append(result)
        
        # Benchmark 5: Metrics endpoint
        metrics_args = [() for _ in range(args.requests // 5)]  # Even fewer metrics calls
        result = await bench.run_concurrent_benchmark(
            "Metrics Endpoint",
            bench.get_metrics,
            metrics_args,
            args.concurrency
        )
        results.append(result)
        
        # Print results
        print_benchmark_results(results)

def print_benchmark_results(results: List[BenchmarkResult]):
    """Print formatted benchmark results"""
    print("\n" + "="*80)
    print("🎯 BENCHMARK RESULTS")
    print("="*80)
    
    for result in results:
        print(f"\n📊 {result.operation}")
        print(f"   Total Requests: {result.total_requests}")
        print(f"   Duration: {result.duration:.2f}s")
        print(f"   Requests/sec: {result.rps:.2f}")
        print(f"   Success Rate: {result.success_rate*100:.1f}%")
        print(f"   Avg Latency: {result.avg_latency:.2f}ms")
        print(f"   P95 Latency: {result.p95_latency:.2f}ms")
        print(f"   P99 Latency: {result.p99_latency:.2f}ms")
    
    print("\n" + "="*80)
    print("💡 TIPS:")
    print("   - Higher RPS is better")
    print("   - Lower latency is better")
    print("   - Cache hit rates improve with repeated queries")
    print("   - Use /metrics endpoint to monitor cache performance")
    print("="*80)

def main():
    parser = argparse.ArgumentParser(description="DocFusionDB Benchmark Tool")
    parser.add_argument("--url", default="http://localhost:8080", help="DocFusionDB server URL")
    parser.add_argument("--api-key", help="API key for authentication")
    parser.add_argument("--requests", type=int, default=100, help="Number of requests per benchmark")
    parser.add_argument("--concurrency", type=int, default=10, help="Concurrent requests")
    parser.add_argument("--bulk-size", type=int, default=50, help="Documents per bulk operation")
    
    args = parser.parse_args()
    
    print("🦀 DocFusionDB Benchmark Tool")
    print(f"Target: {args.url}")
    print(f"Requests: {args.requests}, Concurrency: {args.concurrency}")
    
    try:
        asyncio.run(run_benchmarks(args))
    except KeyboardInterrupt:
        print("\n❌ Benchmark interrupted by user")
    except Exception as e:
        print(f"\n❌ Benchmark failed: {e}")

if __name__ == "__main__":
    main()
