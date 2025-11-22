use backupforge_agent::BackupAgent;
use backupforge_core::BackupConfig;
use backupforge_storage::StorageConfig;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<RwLock<Option<BackupAgent>>>,
    pub config: Arc<ServerConfig>,
}

impl AppState {
    pub async fn new(config: ServerConfig) -> anyhow::Result<Self> {
        let agent = if let Some(ref storage_config) = config.default_storage {
            let backup_config = BackupConfig::default();
            let agent = BackupAgent::new(backup_config, storage_config.clone()).await?;
            Some(agent)
        } else {
            None
        };

        Ok(Self {
            agent: Arc::new(RwLock::new(agent)),
            config: Arc::new(config),
        })
    }
}

/// Server configuration
#[derive(Clone)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u16,
    pub database_url: Option<String>,
    pub default_storage: Option<StorageConfig>,
    pub jwt_secret: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 8080,
            database_url: None,
            default_storage: Some(StorageConfig::Local {
                path: "/var/lib/backupforge".to_string(),
            }),
            jwt_secret: "change-me-in-production".to_string(),
        }
    }
}
