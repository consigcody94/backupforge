use anyhow::{anyhow, Result};
use backupforge_agent::BackupAgent;
use serde_json::{json, Value};

pub fn get_resources_list() -> Vec<Value> {
    vec![
        json!({
            "uri": "backupforge://snapshots",
            "name": "Backup Snapshots",
            "description": "List of all backup snapshots",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "backupforge://jobs",
            "name": "Backup Jobs",
            "description": "List of configured backup jobs",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "backupforge://storage/stats",
            "name": "Storage Statistics",
            "description": "Current storage usage and deduplication statistics",
            "mimeType": "application/json"
        }),
        json!({
            "uri": "backupforge://config",
            "name": "Configuration",
            "description": "Current BackupForge configuration",
            "mimeType": "application/json"
        }),
    ]
}

pub async fn read_resource(uri: &str, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    match uri {
        "backupforge://snapshots" => read_snapshots(agent).await,
        "backupforge://jobs" => read_jobs(agent).await,
        "backupforge://storage/stats" => read_storage_stats(agent).await,
        "backupforge://config" => read_config(agent).await,
        _ => {
            if uri.starts_with("backupforge://snapshots/") {
                let snapshot_id = uri.strip_prefix("backupforge://snapshots/").unwrap();
                read_snapshot_detail(snapshot_id, agent).await
            } else {
                Err(anyhow!("Unknown resource URI: {}", uri))
            }
        }
    }
}

async fn read_snapshots(agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;

    // In a real implementation, this would query the database
    Ok(vec![json!({
        "uri": "backupforge://snapshots",
        "mimeType": "application/json",
        "text": json!({
            "snapshots": [],
            "total": 0,
            "message": "Snapshot listing requires database integration (coming in v0.2.0)"
        }).to_string()
    })])
}

async fn read_jobs(agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;

    Ok(vec![json!({
        "uri": "backupforge://jobs",
        "mimeType": "application/json",
        "text": json!({
            "jobs": [],
            "total": 0,
            "message": "Job management requires server component (coming in v0.2.0)"
        }).to_string()
    })])
}

async fn read_storage_stats(agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;

    match agent.get_stats().await {
        Ok(stats) => Ok(vec![json!({
            "uri": "backupforge://storage/stats",
            "mimeType": "application/json",
            "text": json!({
                "total_bytes": stats.total_bytes,
                "total_files": stats.total_files,
                "new_chunks": stats.new_chunks,
                "reused_chunks": stats.reused_chunks,
                "compressed_bytes": stats.compressed_bytes,
                "deduplication_ratio": if stats.total_bytes > 0 {
                    ((stats.total_bytes - stats.compressed_bytes) as f64 / stats.total_bytes as f64) * 100.0
                } else {
                    0.0
                }
            }).to_string()
        })]),
        Err(e) => Err(anyhow!("Failed to get storage stats: {}", e)),
    }
}

async fn read_config(agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;

    Ok(vec![json!({
        "uri": "backupforge://config",
        "mimeType": "application/json",
        "text": json!({
            "version": env!("CARGO_PKG_VERSION"),
            "storage_path": std::env::var("BACKUPFORGE_STORAGE")
                .unwrap_or_else(|_| "/var/lib/backupforge/storage".to_string()),
            "features": {
                "encryption": true,
                "compression": true,
                "deduplication": true,
                "multi_tenancy": true
            }
        }).to_string()
    })])
}

async fn read_snapshot_detail(snapshot_id: &str, agent: &Option<BackupAgent>) -> Result<Vec<Value>> {
    let _agent = agent.as_ref().ok_or_else(|| anyhow!("Agent not initialized"))?;

    Ok(vec![json!({
        "uri": format!("backupforge://snapshots/{}", snapshot_id),
        "mimeType": "application/json",
        "text": json!({
            "snapshot_id": snapshot_id,
            "message": "Snapshot details require database integration (coming in v0.2.0)"
        }).to_string()
    })])
}
