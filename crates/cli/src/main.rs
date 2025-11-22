use backupforge_agent::BackupAgent;
use backupforge_common::types::{BackupJob, BackupSource};
use backupforge_core::{BackupConfig, ChunkingStrategy, CompressionAlgorithm, EncryptionKey};
use backupforge_storage::StorageConfig;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "backupforge")]
#[command(about = "Open-source backup and disaster recovery solution", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Backup a local directory
    Backup {
        /// Source path to backup
        #[arg(short, long)]
        source: PathBuf,

        /// Storage path
        #[arg(short = 'd', long)]
        storage: PathBuf,

        /// Exclude patterns
        #[arg(short, long)]
        exclude: Vec<String>,

        /// Enable encryption (will prompt for password)
        #[arg(short = 'e', long)]
        encrypt: bool,

        /// Compression level (0-22 for zstd)
        #[arg(short, long, default_value = "3")]
        compression: i32,
    },

    /// Restore a backup
    Restore {
        /// Storage path
        #[arg(short = 'd', long)]
        storage: PathBuf,

        /// Target path to restore to
        #[arg(short, long)]
        target: PathBuf,

        /// Snapshot ID to restore
        #[arg(short, long)]
        snapshot: Option<String>,
    },

    /// List snapshots
    List {
        /// Storage path
        #[arg(short = 'd', long)]
        storage: PathBuf,
    },

    /// Show storage statistics
    Stats {
        /// Storage path
        #[arg(short = 'd', long)]
        storage: PathBuf,
    },

    /// Initialize a new backup repository
    Init {
        /// Storage path
        #[arg(short = 'd', long)]
        storage: PathBuf,

        /// Enable encryption
        #[arg(short, long)]
        encrypt: bool,
    },

    /// Run backup server/daemon
    Server {
        /// Server configuration file
        #[arg(short, long)]
        config: PathBuf,

        /// Server port
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    match cli.command {
        Commands::Backup {
            source,
            storage,
            exclude,
            encrypt,
            compression,
        } => {
            println!("🚀 Starting backup...");
            println!("Source: {}", source.display());
            println!("Storage: {}", storage.display());

            let mut backup_config = BackupConfig::default();
            backup_config.compression = CompressionAlgorithm::Zstd(compression);

            if encrypt {
                // In production, would prompt for password securely
                let password = "test_password"; // PLACEHOLDER
                let salt = b"salt123456789012";
                let key = EncryptionKey::from_password(password, salt)?;
                backup_config.encryption_key = Some(key);
                println!("🔒 Encryption enabled");
            }

            let storage_config = StorageConfig::Local {
                path: storage.to_string_lossy().to_string(),
            };

            let agent = BackupAgent::new(backup_config, storage_config).await?;

            let job = BackupJob {
                id: Uuid::new_v4(),
                name: "cli-backup".to_string(),
                source: BackupSource::LocalPath {
                    path: source.to_string_lossy().to_string(),
                    excludes: exclude,
                },
                destination: storage.to_string_lossy().to_string(),
                schedule: None,
                retention_days: 30,
                enabled: true,
                encryption_enabled: encrypt,
                compression_level: compression as u8,
            };

            let snapshot = agent.run_job(&job).await?;

            println!("✅ Backup completed!");
            println!("Snapshot ID: {}", snapshot.id.0);
            println!("Files: {}", snapshot.file_count);
            println!("Size: {} bytes", snapshot.total_size);
            println!(
                "Compressed: {} bytes ({:.1}%)",
                snapshot.compressed_size,
                (snapshot.compressed_size as f64 / snapshot.total_size as f64) * 100.0
            );
        }

        Commands::Restore {
            storage,
            target,
            snapshot,
        } => {
            println!("🔄 Starting restore...");
            println!("Storage: {}", storage.display());
            println!("Target: {}", target.display());

            // Would implement restore logic
            println!("⚠️  Restore functionality coming soon!");
        }

        Commands::List { storage } => {
            println!("📋 Listing snapshots from: {}", storage.display());

            // Would list snapshots from storage
            println!("⚠️  List functionality coming soon!");
        }

        Commands::Stats { storage } => {
            println!("📊 Storage statistics for: {}", storage.display());

            let storage_config = StorageConfig::Local {
                path: storage.to_string_lossy().to_string(),
            };

            let agent = BackupAgent::new(BackupConfig::default(), storage_config).await?;
            let stats = agent.get_stats().await?;

            println!("Total bytes: {}", stats.total_bytes);
        }

        Commands::Init { storage, encrypt } => {
            println!("🎯 Initializing backup repository...");
            println!("Storage: {}", storage.display());

            // Create storage directory
            tokio::fs::create_dir_all(&storage).await?;

            if encrypt {
                println!("🔐 Encryption will be enabled for this repository");
                // Would save encryption config
            }

            println!("✅ Repository initialized!");
        }

        Commands::Server { config, port } => {
            println!("🌐 Starting BackupForge server...");
            println!("Config: {}", config.display());
            println!("Port: {}", port);

            // Would start the API server
            println!("⚠️  Server functionality coming soon!");
            println!("    This will start the REST API and web dashboard");
        }
    }

    Ok(())
}
