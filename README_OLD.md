# DocFusionDB

DocFusionDB is an experimental document database built in Rust that combines Postgres’ high‑performance JSONB storage and indexing with Apache Arrow’s [DataFusion](https://arrow.apache.org/datafusion/) query engine. By using custom User-Defined Functions (UDFs) to push down JSON filtering to Postgres, DocFusionDB can deliver fast query performance while preserving document flexibility.

---

## Table of Contents

- [Features](#features)
- [Prerequisites](#prerequisites)
- [Getting Started](#getting-started)
  - [Building and Running Locally](#building-and-running-locally)
  - [Using Docker Compose](#using-docker-compose)
- [Example CLI Commands](#example-cli-commands)
- [Architecture Overview](#architecture-overview)
- [API Reference](#api-reference)
- [Tutorial / Demo App](#tutorial--demo-app)
- [Contributing](#contributing)
- [License](#license)

---

## Features

- **JSONB Document Storage**: Store rich JSON documents in Postgres.
- **Push-Down Querying**: Custom DataFusion UDFs to transform SQL expressions (like `json_extract_path`, `json_contains`, and `json_multi_contains`) into SQL that Postgres can optimize with its GIN indexes.
- **Flexible CLI**: A command-line interface for inserting, querying, and updating documents.
- **Dockerized MVP**: Ready to deploy via Docker Compose.

---

## Prerequisites

- **Docker & Docker Compose**: To run the system in a containerized environment.
- **Rust & Cargo**: For local development (optional if using the Docker image).
- **PostgreSQL 15+**: Used as the storage backend.

---

## Getting Started

### Building and Running Locally

1. **Clone the repository:**

   ```bash
   git clone https://github.com/youruser/docfusiondb.git
   cd docfusiondb
   ```

2. **Build the project using Cargo:**

   ```bash
   cargo build --release
   ```

3. **Run the CLI (for example, to show help):**

   ```bash
   cargo run -- --help
   ```

### Using Docker Compose

A sample `docker-compose.yml` is provided. Ensure it is in the repo root.

Start the services:

```bash
docker-compose up -d
```

This will start a PostgreSQL container and the DocFusionDB app container.

Run CLI commands in the container:

For example, to query the database:

```bash
docker-compose exec app docfusiondb query "SELECT json_extract_path(doc, 'title') AS title FROM documents"
```

Set Environment Variables:

The app uses the environment variable `DATABASE_URL` to connect to PostgreSQL. In your `docker-compose.yml`, ensure it’s set, e.g.:

```yaml
environment:
  DATABASE_URL: "postgres://postgres:yourpassword@db:5432/docfusiondb"
```

---

## Example CLI Commands

### Insert a Document

```bash
docfusiondb insert '{"title":"Hello","body":"World","tags":["example","test"]}'
```

Inserts a new document into the `documents` table.

### Query Documents

```bash
docfusiondb query "SELECT json_extract_path(doc, 'status') AS status, json_extract_path(doc, 'title') AS title FROM documents WHERE json_extract_path(doc, 'status') = 'active'"
```

Retrieves all documents where the `status` field equals `"active"`, using the UDF to extract fields from your JSONB.

### Update a Document

```bash
docfusiondb update 42 '{"status":"complete","title":"Updated Title","body":"New content"}'
```

Updates the document with ID 42.

---

## Architecture Overview

DocFusionDB’s architecture combines two powerful systems:

### Overview

**DataFusion Query Engine**  
Acts as the query planner and optimizer. It converts SQL queries into an execution plan. Custom UDFs (such as `json_extract_path`, `json_contains`, and `json_multi_contains`) transform JSON-related expressions into SQL that PostgreSQL can understand.

**Postgres Storage Layer**  
Utilizes Postgres’ native JSONB support and its fast GIN indexing for document storage and efficient query execution.

### Data Flow Diagram (Conceptual)

```pgsql
+----------------------+        +--------------------+        +------------------------+
|   CLI / API Input    | -----> | DataFusion Query   | -----> |   Postgres Database    |
| (SQL + UDF functions)|        | Planner & Executor |        | (JSONB storage, GIN    |
|                      |        | (translates UDFs   |        |  indexes for pushdown) |
|                      |        |  via expr_to_sql)  |        |                        |
+----------------------+        +--------------------+        +------------------------+
```

### UDF Processing:

Custom UDFs let you write SQL like:

```sql
SELECT json_extract_path(doc, 'status') 
FROM documents 
WHERE json_extract_path(doc,'status') = 'active'
```

This is translated internally into a SQL query that Postgres can optimize using its native JSONB support.

---

## API Reference

### CLI Commands

| Command | Description | Usage Example |
|---------|-------------|----------------|
| `query` | Execute a SQL query against the `documents` table | `docfusiondb query "SELECT json_extract_path(doc,'title') FROM documents"` |
| `insert` | Insert a new document (JSON string) into `documents` | `docfusiondb insert '{"title":"Hello","body":"World"}'` |
| `update` | Update an existing document by specifying its ID and new JSON | `docfusiondb update 42 '{"status":"complete"}'` |

### UDF Functions Available in SQL

- `json_extract_path(doc, 'field')`  
  Extracts the value associated with `'field'` from the JSON document in `doc`.

- `json_contains(doc, '{"field": "value"}')`  
  Returns true if the JSON in `doc` contains the key-value pair.

- `json_multi_contains(doc, '{"field1": "value1", "field2": "value2"}')`  
  Performs a multi‑key containment check in a single operation.

---

## Tutorial / Demo App

Coming soon

## Contributing

We welcome contributions! Some ideas for “good first issues” include:

- Improving CLI documentation (e.g. add more examples in the README).
- Writing integration tests with a real Postgres container.
- Creating a sample dataset loader script.

Please see `CONTRIBUTING.md` for guidelines.

---

## License

This project is licensed under the MIT License. See `LICENSE` for details.
