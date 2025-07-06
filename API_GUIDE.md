# DocFusionDB API Guide

## Overview

DocFusionDB provides a RESTful HTTP API for document storage and querying. The API supports JSON document operations and custom SQL queries through DataFusion.

## Base URL

```
http://localhost:8080
```

## API Endpoints

### Health Check

**GET /health**

Check the health status of the service and database connection.

**Response:**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "0.1.1",
    "database": "connected"
  },
  "error": null,
  "timestamp": "2025-01-07T10:30:00Z"
}
```

### Documents

#### List Documents

**GET /documents**

Retrieve a paginated list of documents.

**Query Parameters:**
- `limit` (optional): Number of documents to return (default: 10, max: 100)
- `offset` (optional): Number of documents to skip (default: 0)

**Example:**
```bash
curl "http://localhost:8080/documents?limit=5&offset=0"
```

**Response:**
```json
{
  "success": true,
  "data": [
    {
      "id": 1,
      "document": {
        "title": "Sample Document",
        "content": "Document content here"
      }
    }
  ],
  "error": null,
  "timestamp": "2025-01-07T10:30:00Z"
}
```

#### Get Document

**GET /documents/{id}**

Retrieve a specific document by ID.

**Path Parameters:**
- `id`: Document ID (integer)

**Example:**
```bash
curl "http://localhost:8080/documents/1"
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": 1,
    "document": {
      "title": "Sample Document",
      "content": "Document content here",
      "tags": ["example", "test"]
    }
  },
  "error": null,
  "timestamp": "2025-01-07T10:30:00Z"
}
```

#### Create Document

**POST /documents**

Create a new document.

**Request Body:**
```json
{
  "document": {
    "title": "New Document",
    "content": "Document content",
    "metadata": {
      "author": "User",
      "created": "2025-01-07"
    }
  }
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/documents" \
  -H "Content-Type: application/json" \
  -d '{
    "document": {
      "title": "My Document",
      "content": "Hello World"
    }
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": 2,
    "document": {
      "title": "My Document",
      "content": "Hello World"
    }
  },
  "error": null,
  "timestamp": "2025-01-07T10:30:00Z"
}
```

### Custom Queries

#### Execute Query

**POST /query**

Execute a custom SQL query using DataFusion.

**Request Body:**
```json
{
  "sql": "SELECT json_extract_path(doc, 'title') as title FROM documents WHERE json_extract_path(doc, 'status') = 'active'"
}
```

**Example:**
```bash
curl -X POST "http://localhost:8080/query" \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT COUNT(*) as total FROM documents"
  }'
```

**Response:**
```json
{
  "success": true,
  "data": {
    "rows": [
      {"total": "5"}
    ],
    "row_count": 1,
    "execution_time_ms": 23
  },
  "error": null,
  "timestamp": "2025-01-07T10:30:00Z"
}
```

## Error Responses

All error responses follow this format:

```json
{
  "success": false,
  "data": null,
  "error": "Error description",
  "timestamp": "2025-01-07T10:30:00Z"
}
```

### HTTP Status Codes

- `200 OK` - Success
- `400 Bad Request` - Invalid request data or malformed JSON
- `404 Not Found` - Document not found
- `500 Internal Server Error` - Server error
- `503 Service Unavailable` - Database connection issues

## JSON Functions in Queries

DocFusionDB supports custom JSON functions in SQL queries:

### json_extract_path
Extract a value from a JSON document by path.

```sql
SELECT json_extract_path(doc, 'title') as title FROM documents
```

### json_contains
Check if a JSON document contains specific key-value pairs.

```sql
SELECT * FROM documents WHERE json_contains(doc, '{"status": "active"}')
```

### json_multi_contains
Check multiple key-value pairs at once.

```sql
SELECT * FROM documents WHERE json_multi_contains(doc, '{"status": "active", "type": "article"}')
```

## Example Workflows

### 1. Basic Document Storage

```bash
# Create a document
curl -X POST "http://localhost:8080/documents" \
  -H "Content-Type: application/json" \
  -d '{
    "document": {
      "title": "Meeting Notes",
      "content": "Discussed API improvements",
      "attendees": ["Alice", "Bob"],
      "date": "2025-01-07"
    }
  }'

# List documents
curl "http://localhost:8080/documents"

# Get specific document
curl "http://localhost:8080/documents/1"
```

### 2. Advanced Querying

```bash
# Find documents by title
curl -X POST "http://localhost:8080/query" \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT id, json_extract_path(doc, '\''title'\'') as title FROM documents WHERE json_extract_path(doc, '\''title'\'') LIKE '\''%Meeting%'\''"
  }'

# Count documents by type
curl -X POST "http://localhost:8080/query" \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT json_extract_path(doc, '\''type'\'') as doc_type, COUNT(*) as count FROM documents GROUP BY json_extract_path(doc, '\''type'\'')"
  }'
```

## Server Configuration

Start the API server:

```bash
# Using default configuration (localhost:8080)
cargo run -- serve

# Override port
cargo run -- serve --port 3000

# Override host and port
cargo run -- serve --host 127.0.0.1 --port 3000
```

## Performance Notes

- The API uses connection pooling for efficient database access
- Query results are streamed for large datasets
- All operations include performance timing in logs
- Pagination is recommended for large document collections

## Security Considerations

- Currently no authentication is implemented (planned for Phase 4)
- All SQL queries are executed through DataFusion with predefined UDFs
- Input validation is recommended for production use
