# DocFusionDB

DocFusionDB is an experimental document database built in Rust that combines PostgreSQL's high‑performance JSONB storage with Apache Arrow's [DataFusion](https://arrow.apache.org/datafusion/) query engine. By using custom User-Defined Functions (UDFs), DocFusionDB enables fast SQL queries over JSON documents while maintaining flexibility.

## ✨ Features

- **🚀 HTTP API**: RESTful API for document operations and custom queries
- **📊 JSONB Document Storage**: Store rich JSON documents in PostgreSQL with GIN indexing
- **⚡ DataFusion Integration**: Fast SQL queries with custom JSON UDFs
- **🔌 Connection Pooling**: Production-ready database connection management  
- **📝 Structured Logging**: JSON logging with performance metrics
- **🧪 CLI Interface**: Command-line tools for development and testing
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

logging:
  level: info
  format: json
```

Or use environment variables:
```bash
export DATABASE_URL="postgres://postgres:yourpassword@localhost:5432/docfusiondb"
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

### List Documents

```bash
curl "http://localhost:8080/documents?limit=10&offset=0"
```

### Get Specific Document

```bash
curl "http://localhost:8080/documents/1"
```

### Custom SQL Queries

```bash
curl -X POST "http://localhost:8080/query" \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT json_extract_path(doc, '\''title'\'') as title FROM documents WHERE json_extract_path(doc, '\''published'\'') = '\''true'\''"
  }'
```

## 🛠️ CLI Usage

```bash
# Insert a document
cargo run -- insert '{"title":"Test","content":"Hello World"}'

# Query documents  
cargo run -- query "SELECT json_extract_path(doc, 'title') as title FROM documents"

# Update a document
cargo run -- update 1 '{"title":"Updated","content":"New content"}'
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
│                 │    │   + Custom UDFs │    │   + GIN Indexes │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

**DataFusion** handles query planning and optimization, while **PostgreSQL** provides durable JSONB storage with efficient indexing. Custom UDFs bridge the gap, translating JSON operations into optimized PostgreSQL queries.

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Test the API (requires running server)
./test_api.sh
```

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
- `LOG_LEVEL` - Logging level (debug, info, warn, error)

## 🎯 Roadmap

- ✅ **Phase 1**: Foundation (Error handling, Config, Connection pooling, Logging, Tests)
- ✅ **Phase 2**: HTTP API Server
- 🔄 **Phase 2**: Bulk operations for efficient data ingestion
- ⏳ **Phase 3**: Performance optimizations and caching
- ⏳ **Phase 4**: Production features (Auth, Monitoring, Backup)

## 🤝 Contributing

DocFusionDB is experimental and welcomes contributions! Areas of interest:

- Performance benchmarking and optimization
- Additional JSON query functions
- Integration tests
- Documentation improvements

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.

---

**⚠️ Experimental Status**: DocFusionDB is experimental software. Use in production at your own risk.
