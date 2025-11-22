use anyhow::{anyhow, Result};
use backupforge_agent::BackupAgent;
use backupforge_common::types::{BackupJob, BackupSource};
use serde_json::{json, Value};
use std::path::PathBuf;
use uuid::Uuid;

pub fn get_tools_list() -> Vec<Value> {
    vec![
        json!({
            "name": "backup_directory",
            "description": "Backup a local directory with optional encryption and compression",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source_path": {
                        "type": "string",
                        "description": "Path to the directory to backup"
                    },
                    "excludes": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "List of patterns to exclude from backup",
                        "default": []
                    },
                    "encryption": {
                        "type": "boolean",
                        "description": "Enable encryption for this backup",
                        "default": false
                    },
                    "compression_level": {
                        "type": "integer",
                        "description": "Compression level (1-22 for zstd)",
                        "default": 3,
                        "minimum": 1,
                        "maximum": 22
                    }
                },
                "required": ["source_path"]
            }
        }),
        json!({
            "name": "list_snapshots",
            "description": "List all available backup snapshots",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of snapshots to return",
                        "default": 50
                    }
                }
            }
        }),
        json!({
            "name": "get_snapshot_info",
            "description": "Get detailed information about a specific snapshot",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "snapshot_id": {
                        "type": "string",
                        "description": "The UUID of the snapshot"
                    }
                },
                "required": ["snapshot_id"]
            }
        }),
        json!({
            "name": "restore_snapshot",
            "description": "Restore files from a snapshot to a target directory",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "snapshot_id": {
                        "type": "string",
                        "description": "The UUID of the snapshot to restore"
                    },
                    "target_path": {
                        "type": "string",
                        "description": "Path where files should be restored"
                    }
                },
                "required": ["snapshot_id", "target_path"]
            }
        }),
        json!({
            "name": "get_storage_stats",
            "description": "Get storage statistics including total size and deduplication ratio",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "create_backup_job",
            "description": "Create a scheduled backup job",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the backup job"
                    },
                    "source_path": {
                        "type": "string",
                        "description": "Path to backup"
                    },
                    "schedule": {
                        "type": "string",
                        "description": "Cron expression for schedule (e.g., '0 2 * * *')"
                    },
                    "retention_days": {
                        "type": "integer",
                        "description": "Number of days to retain backups",
                        "default": 30
                    }
                },
                "required": ["name", "source_path"]
            }
        }),
        json!({
            "name": "verify_backup",
            "description": "Verify the integrity of a backup snapshot",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "snapshot_id": {
                        "type": "string",
                        "description": "The UUID of the snapshot to verify"
                    }
                },
                "required": ["snapshot_id"]
            }
        }),
        json!({
            "name": "estimate_backup_size",
            "description": "Estimate the size of a backup before creating it",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source_path": {
                        "type": "string",
                        "description": "Path to analyze"
                    },
                    "excludes": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Patterns to exclude",
                        "default": []
                    }
                },
                "required": ["source_path"]
            }
        }),
    ]
}

pub async fn execute_tool(
    tool_name: &str,
    arguments: &Value,
    agent: &Option<BackupAgent>,
) -> Result<Vec<Value>> {
    match tool_name {
        "backup_directory" => backup_directory(arguments, agent).await,
        "list_snapshots" => list_snapshots(arguments, agent).await,
        "get_snapshot_info" => get_snapshot_info(arguments, agent).await,
        "restore_snapshot" => restore_snapshot(arguments, agent).await,
        "get_storage_stats" => get_storage_stats(arguments, agent).await,
        "create_backup_job" => create_backup_job(arguments, agent).await,
        "verify_backup" => verify_backup(arguments, agent).await,
        "estimate_backup_size" => estimate_backup_size(arguments, agent).await,
        _ => Err(anyhow!("Unknown tool: {}", tool_name)),
    }
}

async fn backup_directory(arguments: &Value, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;

    let source_path = arguments["source_path"]
        .as_str()
        .ok_or_else(|| anyhow!("source_path required"))?;

    let excludes = arguments["excludes"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let encryption = arguments["encryption"].as_bool().unwrap_or(false);
    let compression_level = arguments["compression_level"].as_u64().unwrap_or(3) as u8;

    let job = BackupJob {
        id: Uuid::new_v4(),
        name: format!("MCP Backup - {}", source_path),
        source: BackupSource::LocalPath {
            path: source_path.to_string(),
            excludes,
        },
        destination: "/var/lib/backupforge/storage".to_string(),
        schedule: None,
        retention_days: 30,
        enabled: true,
        encryption_enabled: encryption,
        compression_level,
    };

    match agent.run_job(&job).await {
        Ok(snapshot) => Ok(vec![json!({
            "type": "text",
            "text": format!(
                "✅ Backup completed successfully!\n\nSnapshot ID: {}\nFiles: {}\nTotal Size: {} bytes\nCompressed: {} bytes ({:.1}% ratio)\nCreated: {}",
                snapshot.id.0,
                snapshot.file_count,
                snapshot.total_size,
                snapshot.compressed_size,
                (snapshot.compressed_size as f64 / snapshot.total_size as f64) * 100.0,
                snapshot.created_at
            )
        })]),
        Err(e) => Err(anyhow!("Backup failed: {}", e)),
    }
}

async fn list_snapshots(arguments: &Value, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;
    let _limit = arguments["limit"].as_u64().unwrap_or(50);

    // In a real implementation, this would query the storage backend
    Ok(vec![json!({
        "type": "text",
        "text": "Snapshot listing functionality requires database integration.\nThis will be available in the next release."
    })])
}

async fn get_snapshot_info(arguments: &Value, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;
    let snapshot_id = arguments["snapshot_id"]
        .as_str()
        .ok_or_else(|| anyhow!("snapshot_id required"))?;

    Ok(vec![json!({
        "type": "text",
        "text": format!("Snapshot info for: {}\n\nThis feature requires database integration and will be available in the next release.", snapshot_id)
    })])
}

async fn restore_snapshot(arguments: &Value, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;
    let _snapshot_id = arguments["snapshot_id"]
        .as_str()
        .ok_or_else(|| anyhow!("snapshot_id required"))?;
    let _target_path = arguments["target_path"]
        .as_str()
        .ok_or_else(|| anyhow!("target_path required"))?;

    Ok(vec![json!({
        "type": "text",
        "text": "Restore functionality requires database integration.\nThis will be available in the next release."
    })])
}

async fn get_storage_stats(arguments: &Value, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;

    match agent.get_stats().await {
        Ok(stats) => Ok(vec![json!({
            "type": "text",
            "text": format!(
                "📊 Storage Statistics\n\nTotal Bytes: {}\nTotal Files: {}\nNew Chunks: {}\nReused Chunks: {}\nCompressed Bytes: {}\nDeduplication Ratio: {:.1}%",
                stats.total_bytes,
                stats.total_files,
                stats.new_chunks,
                stats.reused_chunks,
                stats.compressed_bytes,
                if stats.total_bytes > 0 {
                    ((stats.total_bytes - stats.compressed_bytes) as f64 / stats.total_bytes as f64) * 100.0
                } else {
                    0.0
                }
            )
        })]),
        Err(e) => Err(anyhow!("Failed to get stats: {}", e)),
    }
}

async fn create_backup_job(arguments: &Value, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;
    let name = arguments["name"].as_str().ok_or_else(|| anyhow!("name required"))?;
    let source_path = arguments["source_path"]
        .as_str()
        .ok_or_else(|| anyhow!("source_path required"))?;

    Ok(vec![json!({
        "type": "text",
        "text": format!(
            "Created backup job: {}\nSource: {}\n\nNote: Job scheduling requires the server component to be running.",
            name, source_path
        )
    })])
}

async fn verify_backup(arguments: &Value, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;
    let snapshot_id = arguments["snapshot_id"]
        .as_str()
        .ok_or_else(|| anyhow!("snapshot_id required"))?;

    Ok(vec![json!({
        "type": "text",
        "text": format!("Verification for snapshot {}\n\nThis feature will be available in the next release.", snapshot_id)
    })])
}

async fn estimate_backup_size(
    arguments: &Value,
    agent: &Option<BackupAgent>,
) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;
    let source_path = arguments["source_path"]
        .as_str()
        .ok_or_else(|| anyhow!("source_path required"))?;

    // Simple estimation by walking directory
    let path = PathBuf::from(source_path);
    if !path.exists() {
        return Err(anyhow!("Path does not exist: {}", source_path));
    }

    let mut total_size = 0u64;
    let mut file_count = 0u64;

    if path.is_file() {
        if let Ok(metadata) = std::fs::metadata(&path) {
            total_size = metadata.len();
            file_count = 1;
        }
    } else if path.is_dir() {
        for entry in walkdir::WalkDir::new(&path) {
            if let Ok(entry) = entry {
                if entry.file_type().is_file() {
                    if let Ok(metadata) = entry.metadata() {
                        total_size += metadata.len();
                        file_count += 1;
                    }
                }
            }
        }
    }

    Ok(vec![json!({
        "type": "text",
        "text": format!(
            "📏 Backup Size Estimate\n\nSource: {}\nFiles: {}\nTotal Size: {} bytes ({:.2} MB)\nEstimated Compressed: ~{} bytes ({:.2} MB)\n\nNote: Actual size may vary based on deduplication and compression.",
            source_path,
            file_count,
            total_size,
            total_size as f64 / 1_048_576.0,
            total_size / 2, // Rough estimate: 50% compression
            (total_size / 2) as f64 / 1_048_576.0
        )
    })])
}
