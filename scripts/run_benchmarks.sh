#!/bin/bash

# DocFusionDB Benchmark Runner
# Simple script to run performance tests against DocFusionDB

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
DOCFUSION_URL="${DOCFUSION_URL:-http://localhost:8080}"
API_KEY="${API_KEY:-demo-api-key}"
REQUESTS="${REQUESTS:-100}"
CONCURRENCY="${CONCURRENCY:-10}"
BULK_SIZE="${BULK_SIZE:-50}"

echo -e "${BLUE}🦀 DocFusionDB Benchmark Runner${NC}"
echo "=================================="

# Check if server is running
echo -e "${YELLOW}Checking server health...${NC}"
if curl -s "$DOCFUSION_URL/health" > /dev/null; then
    echo -e "${GREEN}✅ Server is running at $DOCFUSION_URL${NC}"
else
    echo -e "${RED}❌ Server is not responding at $DOCFUSION_URL${NC}"
    echo "Please start DocFusionDB first:"
    echo "  export AUTH_ENABLED=true"
    echo "  export API_KEY=$API_KEY"
    echo "  cargo run -- serve"
    exit 1
fi

# Check Python dependencies
echo -e "${YELLOW}Checking Python dependencies...${NC}"
if ! python3 -c "import aiohttp" 2>/dev/null; then
    echo -e "${YELLOW}Installing aiohttp...${NC}"
    pip3 install aiohttp
fi

# Run benchmarks
echo -e "${BLUE}Starting benchmarks...${NC}"
echo "Configuration:"
echo "  URL: $DOCFUSION_URL"
echo "  API Key: $API_KEY"
echo "  Requests: $REQUESTS"
echo "  Concurrency: $CONCURRENCY"
echo "  Bulk Size: $BULK_SIZE"
echo ""

python3 scripts/benchmark.py \
    --url "$DOCFUSION_URL" \
    --api-key "$API_KEY" \
    --requests "$REQUESTS" \
    --concurrency "$CONCURRENCY" \
    --bulk-size "$BULK_SIZE"

echo -e "\n${GREEN}✅ Benchmarks completed!${NC}"
echo -e "${YELLOW}💡 Tips:${NC}"
echo "  - Run multiple times to see cache performance improve"
echo "  - Check /metrics endpoint for detailed cache statistics"
echo "  - Adjust REQUESTS and CONCURRENCY env vars for different loads"
echo "  - Use 'cargo bench' for low-level Rust benchmarks"
