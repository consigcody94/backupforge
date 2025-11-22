#!/bin/bash
#
# Test script for BackupForge MCP server
# This demonstrates the MCP server functionality without needing Claude Desktop
#

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║         BackupForge MCP Server Test                        ║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Check if the MCP binary exists
MCP_BIN="./target/release/backupforge-mcp"
if [ ! -f "$MCP_BIN" ]; then
    echo -e "${YELLOW}Building MCP server...${NC}"
    cargo build --release --bin backupforge-mcp
    echo -e "${GREEN}✓ Build complete${NC}"
    echo ""
fi

# Function to send MCP request and get response
send_request() {
    local request="$1"
    local description="$2"

    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}Test: $description${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo "Request:"
    echo "$request" | jq '.'
    echo ""
    echo "Response:"
    echo "$request" | $MCP_BIN | jq '.'
    echo ""
}

# Test 1: Initialize
send_request '{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {}
}' "Initialize MCP Server"

# Test 2: List Tools
send_request '{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}' "List Available Tools"

# Test 3: List Resources
send_request '{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "resources/list",
  "params": {}
}' "List Available Resources"

# Test 4: List Prompts
send_request '{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "prompts/list",
  "params": {}
}' "List Available Prompts"

# Test 5: Get Storage Stats
send_request '{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "get_storage_stats",
    "arguments": {}
  }
}' "Get Storage Statistics"

# Test 6: Estimate Backup Size
send_request '{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "estimate_backup_size",
    "arguments": {
      "source_path": "/tmp"
    }
  }
}' "Estimate Backup Size for /tmp"

# Test 7: Read Config Resource
send_request '{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "resources/read",
  "params": {
    "uri": "backupforge://config"
  }
}' "Read Configuration Resource"

# Test 8: Get Disaster Recovery Prompt
send_request '{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "prompts/get",
  "params": {
    "name": "disaster_recovery",
    "arguments": {
      "scenario": "ransomware"
    }
  }
}' "Get Disaster Recovery Prompt (Ransomware)"

echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║              All Tests Completed Successfully!             ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "Next steps:"
echo "1. Install the MCP server:"
echo "   sudo cp target/release/backupforge-mcp /usr/local/bin/"
echo ""
echo "2. Configure Claude Desktop (see docs/MCP_SETUP.md):"
echo "   Edit: ~/.config/Claude/claude_desktop_config.json"
echo ""
echo "3. Add this configuration:"
echo '{
  "mcpServers": {
    "backupforge": {
      "command": "/usr/local/bin/backupforge-mcp",
      "env": {
        "BACKUPFORGE_STORAGE": "/var/lib/backupforge/storage"
      }
    }
  }
}'
echo ""
echo "4. Restart Claude Desktop and try:"
echo "   - 'Backup my Documents folder with encryption'"
echo "   - 'Show me my storage statistics'"
echo "   - 'Help me create a disaster recovery plan'"
echo ""
