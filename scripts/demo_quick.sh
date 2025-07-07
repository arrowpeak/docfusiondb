#!/bin/bash

# Quick Demo Script for DocFusionDB
# Demonstrates all major features in a few minutes

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}🦀 DocFusionDB Quick Demo${NC}"
echo "=========================="
echo ""

# Configuration
API_KEY="demo-api-key"
BASE_URL="http://localhost:8080"

echo -e "${YELLOW}📋 Checking prerequisites...${NC}"
if ! command -v curl &> /dev/null; then
    echo "❌ curl is required but not installed"
    exit 1
fi

if ! command -v jq &> /dev/null; then
    echo "⚠️  jq not found - output will be less pretty"
    JQ_CMD="cat"
else
    JQ_CMD="jq"
fi

echo -e "${GREEN}✅ Prerequisites OK${NC}"
echo ""

# Check server
echo -e "${YELLOW}🔍 Checking DocFusionDB server...${NC}"
if curl -s "$BASE_URL/health" > /dev/null; then
    echo -e "${GREEN}✅ Server is running${NC}"
else
    echo -e "${YELLOW}⚠️  Server not running. Starting it for you...${NC}"
    echo "Please run in another terminal:"
    echo "  export AUTH_ENABLED=true"
    echo "  export API_KEY=$API_KEY"
    echo "  cargo run -- serve"
    echo ""
    echo "Then press Enter to continue..."
    read
fi

echo ""

# Demo functions
demo_health() {
    echo -e "${BLUE}📊 1. Health Check${NC}"
    curl -s "$BASE_URL/health" | $JQ_CMD
    echo ""
    echo ""
}

demo_create_documents() {
    echo -e "${BLUE}📝 2. Creating Sample Documents${NC}"
    
    # Create individual documents
    for i in {1..3}; do
        echo "Creating document $i..."
        curl -s -X POST "$BASE_URL/documents" \
            -H "Content-Type: application/json" \
            -H "X-API-Key: $API_KEY" \
            -d "{\"document': {\"title\": \"Demo Article $i\", \"content\": \"This is demo content for article $i\", \"tags\": [\"demo\", \"article\"], \"published\": $([ $((i % 2)) -eq 0 ] && echo true || echo false)}}" | $JQ_CMD '.success'
    done
    echo ""
    
    # Bulk create
    echo "Creating bulk documents..."
    curl -s -X POST "$BASE_URL/documents/bulk" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: $API_KEY" \
        -d '{"documents": [
            {"title": "Bulk Doc 1", "type": "note", "tags": ["bulk", "demo"]},
            {"title": "Bulk Doc 2", "type": "tutorial", "tags": ["bulk", "guide"]},
            {"title": "Bulk Doc 3", "type": "review", "tags": ["bulk", "feedback"]}
        ]}' | $JQ_CMD '.success'
    echo ""
    echo ""
}

demo_list_documents() {
    echo -e "${BLUE}📋 3. Listing Documents${NC}"
    curl -s "$BASE_URL/documents?limit=5" \
        -H "X-API-Key: $API_KEY" | $JQ_CMD '.data[] | {id: .id, title: .document.title}'
    echo ""
    echo ""
}

demo_queries() {
    echo -e "${BLUE}🔍 4. Custom Queries${NC}"
    
    echo "Query 1: Published articles"
    curl -s -X POST "$BASE_URL/query" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: $API_KEY" \
        -d '{"sql": "SELECT json_extract_path(doc, '\''title'\'') as title FROM documents WHERE json_contains(doc, '\''{\"published\": true}'\'') LIMIT 3"}' | $JQ_CMD '.data[]'
    echo ""
    
    echo "Query 2: Documents by tag"
    curl -s -X POST "$BASE_URL/query" \
        -H "Content-Type: application/json" \
        -H "X-API-Key: $API_KEY" \
        -d '{"sql": "SELECT json_extract_path(doc, '\''title'\'') as title, json_extract_path(doc, '\''type'\'') as type FROM documents WHERE json_contains(doc, '\''{\"tags\": [\"bulk\"]}'\'') LIMIT 3"}' | $JQ_CMD '.data[]'
    echo ""
    echo ""
}

demo_metrics() {
    echo -e "${BLUE}📊 5. System Metrics${NC}"
    curl -s "$BASE_URL/metrics" | $JQ_CMD '.data | {document_count, uptime_seconds, cache_hit_rate: (.query_cache_hit_rate * 100 | floor), hostname: .system_info.hostname}'
    echo ""
    echo ""
}

demo_cache_performance() {
    echo -e "${BLUE}⚡ 6. Cache Performance Demo${NC}"
    echo "Running the same query multiple times to show caching..."
    
    QUERY='{"sql": "SELECT json_extract_path(doc, '\''title'\'') as title FROM documents LIMIT 2"}'
    
    for i in {1..3}; do
        echo "Query run $i:"
        start_time=$(date +%s%N)
        curl -s -X POST "$BASE_URL/query" \
            -H "Content-Type: application/json" \
            -H "X-API-Key: $API_KEY" \
            -d "$QUERY" | $JQ_CMD '.data | length' | xargs echo "  Results:"
        end_time=$(date +%s%N)
        duration=$(( (end_time - start_time) / 1000000 ))
        echo "  Duration: ${duration}ms"
        sleep 1
    done
    
    echo ""
    echo "Notice how subsequent queries get faster due to caching!"
    echo ""
    echo ""
}

# Run the demo
demo_health
demo_create_documents
demo_list_documents
demo_queries
demo_metrics
demo_cache_performance

echo -e "${GREEN}🎉 Demo Complete!${NC}"
echo ""
echo -e "${YELLOW}💡 What's Next?${NC}"
echo "1. Open the web demo: cd demo && python3 -m http.server 8000"
echo "2. Run benchmarks: ./scripts/run_benchmarks.sh"
echo "3. Try backup/restore: cargo run -- backup && cargo run -- restore"
echo "4. Check the metrics endpoint: curl $BASE_URL/metrics | jq"
echo "5. Read the docs: README.md has full API documentation"
echo ""
echo "Happy experimenting with DocFusionDB! 🦀"
