#!/bin/bash

# DocFusionDB API Test Script
# Make sure to start the server first: cargo run -- serve

BASE_URL="http://localhost:8080"

echo "🚀 Testing DocFusionDB API"
echo "=========================="

# Test health endpoint
echo "1. Testing health endpoint..."
curl -s "$BASE_URL/health" | jq '.'
echo ""

# Test creating a document
echo "2. Creating a test document..."
RESPONSE=$(curl -s -X POST "$BASE_URL/documents" \
  -H "Content-Type: application/json" \
  -d '{
    "document": {
      "title": "My First Document",
      "content": "This is a test document created via API",
      "tags": ["test", "api", "demo"],
      "metadata": {
        "author": "DocFusionDB Test",
        "version": 1
      }
    }
  }')

echo "$RESPONSE" | jq '.'
DOC_ID=$(echo "$RESPONSE" | jq -r '.data.id')
echo "Created document with ID: $DOC_ID"
echo ""

# Test getting the document
echo "3. Retrieving the document..."
curl -s "$BASE_URL/documents/$DOC_ID" | jq '.'
echo ""

# Test listing documents
echo "4. Listing all documents..."
curl -s "$BASE_URL/documents?limit=5" | jq '.'
echo ""

# Test bulk insert
echo "5. Bulk creating documents..."
curl -s -X POST "$BASE_URL/documents/bulk" \
  -H "Content-Type: application/json" \
  -d '{
    "documents": [
      {
        "title": "Bulk Document 1",
        "content": "First bulk document",
        "type": "bulk_test"
      },
      {
        "title": "Bulk Document 2", 
        "content": "Second bulk document",
        "type": "bulk_test"
      },
      {
        "title": "Bulk Document 3",
        "content": "Third bulk document", 
        "type": "bulk_test"
      }
    ]
  }' | jq '.'
echo ""

# Test query endpoint
echo "6. Executing a custom query..."
curl -s -X POST "$BASE_URL/query" \
  -H "Content-Type: application/json" \
  -d '{
    "sql": "SELECT json_extract_path(doc, '\''title'\'') as title, json_extract_path(doc, '\''type'\'') as type FROM documents WHERE json_extract_path(doc, '\''type'\'') = '\''bulk_test'\''"
  }' | jq '.'
echo ""

echo "✅ API tests completed!"
