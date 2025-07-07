# DocFusionDB Demo Application

A simple web-based demo showcasing DocFusionDB's capabilities.

## Features Demonstrated

- 📝 **Document Creation**: Create JSON documents via REST API
- 📋 **Document Listing**: Browse stored documents with pagination
- 🔍 **Custom Queries**: Execute SQL queries with custom JSON functions
- 📦 **Bulk Operations**: Insert multiple documents at once
- 📊 **Real-time Metrics**: Live server status and performance metrics
- ⚡ **Query Caching**: See cache hit rates in real-time

## Quick Start

1. **Start DocFusionDB server**:
   ```bash
   cd ..  # Go back to project root
   export AUTH_ENABLED=true
   export API_KEY=demo-api-key
   cargo run -- serve
   ```

2. **Open the demo**:
   ```bash
   cd demo
   python3 -m http.server 8000
   # Or open index.html directly in your browser
   ```

3. **Access the demo**: Open http://localhost:8000 in your browser

## Demo Scenarios

### Basic Document Storage
1. Create a few sample documents using the JSON editor
2. List documents to see them stored
3. Notice the document count updating in real-time

### Query Performance
1. Execute the sample query to see custom JSON functions in action
2. Run the same query multiple times to see caching improve performance
3. Watch the cache hit rate increase in the metrics

### Bulk Operations
1. Use the bulk create feature to add multiple documents
2. Try different JSON structures to test flexibility
3. Query the bulk-inserted data

### Real-time Monitoring
1. Watch the uptime counter increase
2. See document counts update as you add data
3. Monitor cache performance in real-time

## Sample Queries to Try

```sql
-- Find all published articles
SELECT json_extract_path(doc, 'title') as title 
FROM documents 
WHERE json_contains(doc, '{"published": true}')

-- Get documents by tag
SELECT json_extract_path(doc, 'title') as title,
       json_extract_path(doc, 'tags') as tags
FROM documents 
WHERE json_contains(doc, '{"tags": ["rust"]}')

-- Complex nested query
SELECT json_extract_path(doc, 'title') as title,
       json_extract_path(doc, 'metadata', 'author') as author
FROM documents 
WHERE json_multi_contains(doc, '{"type": "article", "published": true}')
```

## Troubleshooting

**Server Offline**: Make sure DocFusionDB is running on port 8080 with auth enabled:
```bash
export AUTH_ENABLED=true
export API_KEY=demo-api-key
cargo run -- serve
```

**CORS Issues**: If running the demo from `file://`, try serving it via HTTP:
```bash
python3 -m http.server 8000
```

**Authentication Errors**: Ensure the API key in the demo matches your server configuration.

## Customization

The demo uses a hardcoded API key (`demo-api-key`) for simplicity. In production, you'd want to:

1. Use environment variables for the API key
2. Implement proper authentication
3. Add error handling for network issues
4. Add input validation and sanitization

## Architecture

The demo is a single HTML file with vanilla JavaScript that:
- Makes REST API calls to DocFusionDB
- Displays responses in a clean, readable format
- Updates metrics in real-time
- Provides a user-friendly interface for all core features

Perfect for understanding how to integrate DocFusionDB into your own applications!
