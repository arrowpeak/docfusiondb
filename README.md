# DocFusionDB

DocFusionDB is an experimental document database built in Rust that combines PostgreSQL's high‑performance JSONB storage with Apache Arrow's [DataFusion](https://arrow.apache.org/datafusion/) query engine. By using custom User-Defined Functions (UDFs), DocFusionDB enables fast SQL queries over JSON documents while maintaining flexibility.

## ✨ Features

- **🚀 HTTP API**: RESTful API for document operations and custom queries
- **📊 JSONB Document Storage**: Store rich JSON documents in PostgreSQL with GIN indexing
- **⚡ DataFusion Integration**: Fast SQL queries with custom JSON UDFs
- **⚡ Smart Query Caching**: In-memory cache with LRU eviction for instant query responses
- **📊 System Metrics**: Built-in monitoring with performance and usage statistics
- **📦 Bulk Operations**: Efficient bulk document insertion (up to 1000 docs)
- **🔌 Connection Pooling**: Production-ready database connection management  
- **📝 Structured Logging**: JSON logging with performance metrics
- **🧪 CLI Interface**: Command-line tools for development and testing
- **💾 Backup/Restore**: Simple backup and restore functionality
- **🔐 API Authentication**: Optional API key authentication for security
- **⚙️ Flexible Configuration**: YAML config with environment variable support

## 🚀 Quick Start

### Prerequisites

- **PostgreSQL 15+**: As the storage backend
- **Rust & Cargo**: For building from source

### 1. Set up PostgreSQL

```bash
# Create database and table
createdb docfusiondb
psql docfusiondb -c "
CREATE TABLE IF NOT EXISTS documents (
    id SERIAL PRIMARY KEY,
    doc JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_documents_gin ON documents USING GIN (doc);
"
```

### 2. Configure Connection

Create `config.yaml`:
```yaml
database:
  host: localhost
  port: 5432
  user: postgres
  password: yourpassword
  database: docfusiondb

server:
  host: 0.0.0.0
  port: 8080

auth:
  enabled: true
  api_key: "your-secret-api-key"

logging:
  level: info
  format: json
```

Or use environment variables:
```bash
export DATABASE_URL="postgres://postgres:yourpassword@localhost:5432/docfusiondb"
export AUTH_ENABLED=true
export API_KEY="your-secret-api-key"
```

### 3. Start the Server

```bash
# Clone and build
git clone https://github.com/arrowpeak/docfusiondb.git
cd docfusiondb
cargo build --release

# Start HTTP API server
cargo run -- serve
```

The API will be available at `http://localhost:8080`

## 🌐 HTTP API Usage

### Create Documents

```bash
curl -X POST "http://localhost:8080/documents" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your-secret-api-key" \
  -d '{
    "document": {
      "title": "My Article",
      "content": "Article content here",
      "tags": ["rust", "database"],
      "metadata": {
        "author": "Developer",
        "published": true
      }
    }
  }'
```

### Bulk Create Documents

```bash
curl -X POST "http://localhost:8080/documents/bulk" \
  -H "Content-Type: application/json" \
  -d '{
    "documents": [
      {"title": "Doc 1", "content": "First document"},
      {"title": "Doc 2", "content": "Second document"},
      {"title": "Doc 3", "content": "Third document"}
    ]
  }'
```

### List Documents

```bash
curl "http://localhost:8080/documents?limit=10&offset=0" \
  -H "X-API-Key: your-secret-api-key"
```

### Get Specific Document

```bash
curl "http://localhost:8080/documents/1"
```

### Custom SQL Queries (with automatic caching)

```bash
curl -X POST "http://localhost:8080/query" \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT json_extract_path(doc, '\''title'\'') as title FROM documents WHERE json_extract_path(doc, '\''published'\'') = '\''true'\''"
  }'
```

### Check Health & Cache Stats

```bash
curl "http://localhost:8080/health" | jq '.data.cache'
```

### Get System Metrics

```bash
curl "http://localhost:8080/metrics" | jq
```

## 🛠️ CLI Usage

```bash
# Insert a document
cargo run -- insert '{"title":"Test","content":"Hello World"}'

# Query documents  
cargo run -- query "SELECT json_extract_path(doc, 'title') as title FROM documents"

# Update a document
cargo run -- update 1 '{"title":"Updated","content":"New content"}'

# Backup documents to JSON file
cargo run -- backup --output my_backup.json

# Restore documents from JSON file  
cargo run -- restore --input my_backup.json

# Restore and clear existing documents first
cargo run -- restore --input my_backup.json --clear
```

## 📊 Custom JSON Functions

DocFusionDB provides custom UDFs for JSON operations:

### `json_extract_path(doc, 'field')`
Extract a value from a JSON document:
```sql
SELECT json_extract_path(doc, 'title') as title FROM documents
```

### `json_contains(doc, '{"field": "value"}')`  
Check if document contains key-value pairs:
```sql
SELECT * FROM documents WHERE json_contains(doc, '{"status": "published"}')
```

### `json_multi_contains(doc, '{"field1": "value1", "field2": "value2"}')`
Multi-key containment check:
```sql
SELECT * FROM documents WHERE json_multi_contains(doc, '{"type": "article", "status": "published"}')
```

## 🏗️ Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   HTTP API      │    │   DataFusion    │    │   PostgreSQL    │
│   (Axum)        │───▶│   Query Engine  │───▶│   JSONB Storage │
│   + Query Cache │    │   + Custom UDFs │    │   + GIN Indexes │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**DataFusion** handles query planning and optimization, while **PostgreSQL** provides durable JSONB storage with efficient indexing. Custom UDFs bridge the gap, translating JSON operations into optimized PostgreSQL queries. **Smart caching** provides instant responses for repeated queries.

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Test the API (requires running server)
./test_api.sh
```

## 🎮 Demo Application

A web-based demo showcasing all DocFusionDB features:

```bash
# Start the server with demo auth
export AUTH_ENABLED=true
export API_KEY=demo-api-key
cargo run -- serve

# Open the demo (in another terminal)
cd demo
python3 -m http.server 8000
# Visit http://localhost:8000
```

The demo includes:
- Interactive document creation and querying
- Real-time metrics dashboard
- Bulk operations testing
- Query caching demonstration

## ⚡ Performance Benchmarks

Run comprehensive performance tests:

```bash
# HTTP API benchmarks
./scripts/run_benchmarks.sh

# Low-level Rust benchmarks  
cargo bench
```

Sample results on a modern laptop:
- **Single inserts**: ~50-200 req/sec
- **Bulk operations**: ~500-2000 docs/sec
- **Cached queries**: ~200-1000+ req/sec
- **Cache hit latency**: <1ms

See `scripts/README.md` for detailed benchmarking guide.

## ⚙️ Configuration

DocFusionDB supports multiple configuration methods with the following precedence:
1. Command-line arguments
2. `config.yaml` file  
3. Environment variables
4. Defaults

### Environment Variables

- `DATABASE_URL` - PostgreSQL connection string
- `DB_HOST`, `DB_PORT`, `DB_USER`, `DB_PASSWORD`, `DB_NAME` - Database connection details
- `SERVER_HOST`, `SERVER_PORT` - Server binding configuration
- `AUTH_ENABLED` - Enable/disable API authentication (true/false)
- `API_KEY` - API key for authentication
- `LOG_LEVEL` - Logging level (debug, info, warn, error)

## 🎯 Roadmap

- ✅ **Phase 1**: Foundation (Error handling, Config, Connection pooling, Logging, Tests)
- ✅ **Phase 2**: HTTP API Server with bulk operations  
- ✅ **Phase 3**: Performance optimizations and smart caching
- ✅ **Phase 4**: Production polish (Auth, Monitoring, Backup)
- ✅ **Phase 5**: Demo application and performance benchmarking
- 📋 **Phase 6**: Advanced features (Transactions, Schema validation)

## 🤝 Contributing

DocFusionDB is experimental and welcomes contributions! Areas of interest:

- Additional JSON query functions and UDFs
- Transaction support and ACID guarantees  
- Schema validation and data constraints
- Advanced caching strategies
- Integration with other query engines
- Documentation and examples

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

**⚠️ Experimental Status**: DocFusionDB is experimental software. Use in production at your own risk.
