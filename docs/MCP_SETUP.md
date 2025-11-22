# BackupForge MCP Server Setup

BackupForge provides a Model Context Protocol (MCP) server that allows AI assistants like Claude to interact with your backup system.

## What is MCP?

The Model Context Protocol (MCP) is an open protocol that standardizes how AI applications interact with external data sources and tools. With BackupForge's MCP server, you can:

- Create backups through natural language
- List and query snapshots
- Restore files using AI assistance
- Get storage statistics
- Create and manage backup jobs
- Access disaster recovery prompts and guidance

## Features

### Tools
- `backup_directory` - Backup a local directory
- `list_snapshots` - List all backup snapshots
- `get_snapshot_info` - Get details about a snapshot
- `restore_snapshot` - Restore from a snapshot
- `get_storage_stats` - View storage statistics
- `create_backup_job` - Create scheduled backup jobs
- `verify_backup` - Verify backup integrity
- `estimate_backup_size` - Estimate backup size

### Resources
- `backupforge://snapshots` - List of all snapshots
- `backupforge://jobs` - Configured backup jobs
- `backupforge://storage/stats` - Storage statistics
- `backupforge://config` - Current configuration

### Prompts
- `create_backup_plan` - Get help creating a backup strategy
- `disaster_recovery` - Disaster recovery guidance
- `optimize_storage` - Storage optimization tips

## Installation

### 1. Build the MCP Server

```bash
cd /home/ajs/backup/backupforge
cargo build --release --bin backupforge-mcp
```

The binary will be at: `target/release/backupforge-mcp`

### 2. Configure Claude Desktop

Edit your Claude Desktop configuration file:

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
**Windows:** `%APPDATA%\Claude\claude_desktop_config.json`
**Linux:** `~/.config/Claude/claude_desktop_config.json`

Add the BackupForge MCP server:

```json
{
  "mcpServers": {
    "backupforge": {
      "command": "/home/ajs/backup/backupforge/target/release/backupforge-mcp",
      "env": {
        "BACKUPFORGE_STORAGE": "/var/lib/backupforge/storage"
      }
    }
  }
}
```

### 3. Restart Claude Desktop

Close and reopen Claude Desktop to load the MCP server.

## Configuration

### Environment Variables

- `BACKUPFORGE_STORAGE` - Path to backup storage (default: `/var/lib/backupforge/storage`)
- `RUST_LOG` - Log level (debug, info, warn, error)

### Example Configuration

```json
{
  "mcpServers": {
    "backupforge": {
      "command": "/usr/local/bin/backupforge-mcp",
      "args": [],
      "env": {
        "BACKUPFORGE_STORAGE": "/mnt/backups/storage",
        "RUST_LOG": "info"
      }
    }
  }
}
```

## Usage Examples

Once configured, you can interact with BackupForge through Claude:

### Create a Backup

```
User: Backup my Documents folder using BackupForge with encryption

Claude: I'll backup your Documents folder with encryption enabled.
[Uses backup_directory tool]
✅ Backup completed successfully!
Snapshot ID: 550e8400-e29b-41d4-a716-446655440000
Files: 1,245
Total Size: 524288000 bytes
Compressed: 262144000 bytes (50.0% ratio)
```

### List Snapshots

```
User: Show me all my backups

Claude: Here are your backup snapshots:
[Uses list_snapshots tool]
...
```

### Get Storage Statistics

```
User: How much storage am I using for backups?

Claude: Let me check your storage statistics.
[Uses get_storage_stats tool]
📊 Storage Statistics
Total Bytes: 10737418240
Deduplication Ratio: 45.2%
...
```

### Disaster Recovery Guidance

```
User: I need help recovering from ransomware

Claude: I'll guide you through ransomware recovery.
[Uses disaster_recovery prompt]
⚠️ CRITICAL: DO NOT PAY THE RANSOM
...
```

## Testing the MCP Server

### Manual Testing

You can test the MCP server manually using stdio:

```bash
cd /home/ajs/backup/backupforge
cargo build --release --bin backupforge-mcp

# Run the server
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | ./target/release/backupforge-mcp
```

Expected response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "protocolVersion": "2024-11-05",
    "capabilities": {
      "tools": {},
      "resources": {},
      "prompts": {}
    },
    "serverInfo": {
      "name": "backupforge-mcp",
      "version": "0.1.0"
    }
  },
  "id": 1
}
```

### List Available Tools

```bash
echo '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | ./target/release/backupforge-mcp
```

### Call a Tool

```bash
echo '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"estimate_backup_size","arguments":{"source_path":"/home/user/test"}}}' | ./target/release/backupforge-mcp
```

## Debugging

### Enable Debug Logging

Set the `RUST_LOG` environment variable:

```json
{
  "mcpServers": {
    "backupforge": {
      "command": "/usr/local/bin/backupforge-mcp",
      "env": {
        "RUST_LOG": "debug"
      }
    }
  }
}
```

Logs will be written to stderr and can be viewed in Claude Desktop's logs.

### Check Claude Desktop Logs

**macOS:**
```bash
tail -f ~/Library/Logs/Claude/mcp*.log
```

**Linux:**
```bash
tail -f ~/.config/Claude/logs/mcp*.log
```

## Troubleshooting

### MCP Server Not Appearing

1. Check that the binary path is correct
2. Ensure the binary is executable: `chmod +x /path/to/backupforge-mcp`
3. Verify environment variables are set
4. Check Claude Desktop logs for errors

### Permission Errors

Ensure the MCP server has permission to access storage:

```bash
sudo chown -R $USER:$USER /var/lib/backupforge
chmod 700 /var/lib/backupforge/storage
```

### Tool Execution Failures

- Check that storage path exists
- Verify you have read/write permissions
- Ensure enough disk space is available

## Security Considerations

1. **File System Access**: The MCP server runs with your user permissions and can access any files you can access
2. **Storage Path**: Ensure the storage path is on a secure, backed-up volume
3. **Encryption**: Use encryption for sensitive data
4. **Network**: The MCP server uses stdio (no network exposure)

## Advanced Usage

### Custom Storage Location

```json
{
  "mcpServers": {
    "backupforge": {
      "command": "/usr/local/bin/backupforge-mcp",
      "env": {
        "BACKUPFORGE_STORAGE": "/mnt/nas/backups"
      }
    }
  }
}
```

### Multiple Backup Repositories

You can configure multiple MCP servers for different backup repositories:

```json
{
  "mcpServers": {
    "backupforge-personal": {
      "command": "/usr/local/bin/backupforge-mcp",
      "env": {
        "BACKUPFORGE_STORAGE": "/home/user/backups"
      }
    },
    "backupforge-work": {
      "command": "/usr/local/bin/backupforge-mcp",
      "env": {
        "BACKUPFORGE_STORAGE": "/mnt/work-backups"
      }
    }
  }
}
```

## Limitations

Current version (v0.1.0) limitations:

- Snapshot listing requires database integration (coming in v0.2.0)
- Restore functionality requires database integration (coming in v0.2.0)
- Job scheduling requires server component (coming in v0.2.0)

## Next Steps

- Read the [BackupForge Documentation](../README.md)
- Learn about [MCP Protocol](https://modelcontextprotocol.io/)
- Join the [Discussion](https://github.com/consigcody94/backupforge/discussions)

## Contributing

Found a bug or have a feature request? [Open an issue](https://github.com/consigcody94/backupforge/issues)!
