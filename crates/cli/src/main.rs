use anyhow::{bail, Context};
use backupforge_agent::BackupAgent;
use backupforge_common::types::{BackupJob, BackupSource};
use backupforge_core::{BackupConfig, CompressionAlgorithm};
use backupforge_storage::StorageConfig;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "backupforge")]
#[command(about = "Open-source backup tool: deduplicated, compressed, optionally encrypted", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a backup repository
    Init {
        /// Repository path
        #[arg(short = 'd', long)]
        storage: PathBuf,

        /// Encrypt the repository (password from BACKUPFORGE_PASSWORD or a prompt)
        #[arg(short, long)]
        encrypt: bool,
    },

    /// Back up a directory or file (creates the repository on first use)
    Backup {
        /// Source path to back up
        #[arg(short, long)]
        source: PathBuf,

        /// Repository path
        #[arg(short = 'd', long)]
        storage: PathBuf,

        /// Exclude paths containing this text (repeatable)
        #[arg(short = 'x', long)]
        exclude: Vec<String>,

        /// Encrypt a new repository (existing repositories keep their mode)
        #[arg(short = 'e', long)]
        encrypt: bool,

        /// Zstd compression level (1-22)
        #[arg(short, long, default_value = "3", value_parser = clap::value_parser!(i32).range(1..=22))]
        compression: i32,
    },

    /// Restore a snapshot (the newest unless --snapshot is given)
    Restore {
        /// Repository path
        #[arg(short = 'd', long)]
        storage: PathBuf,

        /// Directory to restore into
        #[arg(short, long)]
        target: PathBuf,

        /// Snapshot ID
        #[arg(short, long)]
        snapshot: Option<String>,
    },

    /// List snapshots
    List {
        /// Repository path
        #[arg(short = 'd', long)]
        storage: PathBuf,
    },

    /// Check that every chunk of a snapshot is present and uncorrupted
    Verify {
        /// Repository path
        #[arg(short = 'd', long)]
        storage: PathBuf,

        /// Snapshot ID (newest if omitted)
        #[arg(short, long)]
        snapshot: Option<String>,
    },

    /// Show repository statistics
    Stats {
        /// Repository path
        #[arg(short = 'd', long)]
        storage: PathBuf,
    },
}

/// Read the password from `BACKUPFORGE_PASSWORD`, else prompt without echo.
/// New repositories ask twice. The password is never hardcoded or logged.
fn read_password(confirm: bool) -> anyhow::Result<String> {
    if let Ok(pw) = std::env::var("BACKUPFORGE_PASSWORD") {
        if pw.is_empty() {
            bail!("BACKUPFORGE_PASSWORD is set but empty");
        }
        return Ok(pw);
    }
    let pw = rpassword::prompt_password("Repository password: ")?;
    if pw.is_empty() {
        bail!("Password must not be empty");
    }
    if confirm {
        let again = rpassword::prompt_password("Repeat password: ")?;
        if pw != again {
            bail!("Passwords do not match");
        }
    }
    Ok(pw)
}

fn local(path: &PathBuf) -> StorageConfig {
    StorageConfig::Local {
        path: path.to_string_lossy().to_string(),
    }
}

/// True when the path already holds a repository header.
fn repo_exists(path: &PathBuf) -> bool {
    path.join("metadata").join(backupforge_core::REPO_CONFIG_KEY).exists()
}

async fn open(
    storage: &PathBuf,
    config: BackupConfig,
    encrypt_if_new: bool,
    create: bool,
) -> anyhow::Result<BackupAgent> {
    let exists = repo_exists(storage);
    if !exists && !create {
        bail!("No repository at {} (run `backupforge init` or `backupforge backup` first)", storage.display());
    }
    // Only ask for a password when one is needed: creating an encrypted
    // repository, or opening one that is encrypted.
    let needs_password = if exists {
        let header = std::fs::read(storage.join("metadata").join(backupforge_core::REPO_CONFIG_KEY))
            .context("Cannot read repository header")?;
        backupforge_core::RepoConfig::from_bytes(&header)?.encrypted
    } else {
        encrypt_if_new
    };
    let password = if needs_password {
        Some(read_password(!exists)?)
    } else {
        None
    };

    Ok(BackupAgent::open_repository(config, local(storage), password.as_deref(), encrypt_if_new).await?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

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
        Commands::Init { storage, encrypt } => {
            if repo_exists(&storage) {
                bail!("A repository already exists at {}", storage.display());
            }
            tokio::fs::create_dir_all(&storage).await?;
            open(&storage, BackupConfig::default(), encrypt, true).await?;
            println!(
                "Initialized {} repository at {}",
                if encrypt { "encrypted" } else { "unencrypted" },
                storage.display()
            );
        }

        Commands::Backup {
            source,
            storage,
            exclude,
            encrypt,
            compression,
        } => {
            let mut config = BackupConfig::default();
            config.compression = CompressionAlgorithm::Zstd(compression);

            tokio::fs::create_dir_all(&storage).await?;
            let agent = open(&storage, config, encrypt, true).await?;

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

            println!("Snapshot: {}", snapshot.id.0);
            println!("Files:    {}", snapshot.file_count);
            println!("Size:     {} bytes", snapshot.total_size);
            println!("Stored:   {} bytes (before deduplication)", snapshot.compressed_size);

            if let Some(tag) = snapshot.tags.iter().find(|t| t.starts_with("partial:")) {
                let n = tag.trim_start_matches("partial:");
                eprintln!("WARNING: {} entries could not be read and are NOT in this backup.", n);
                std::process::exit(2);
            }
        }

        Commands::Restore {
            storage,
            target,
            snapshot,
        } => {
            let agent = open(&storage, BackupConfig::default(), false, false).await?;
            let summary = agent.restore_snapshot(snapshot.as_deref(), &target).await?;
            println!(
                "Restored {} files, {} directories, {} bytes to {}",
                summary.files,
                summary.directories,
                summary.bytes,
                target.display()
            );
        }

        Commands::List { storage } => {
            let agent = open(&storage, BackupConfig::default(), false, false).await?;
            let snapshots = agent.list_snapshots().await?;
            if snapshots.is_empty() {
                println!("No snapshots.");
            }
            for s in snapshots {
                println!(
                    "{}  {}  {} files  {} bytes  {}",
                    s.id.0,
                    s.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                    s.file_count,
                    s.total_size,
                    s.source_path
                );
            }
        }

        Commands::Verify { storage, snapshot } => {
            let agent = open(&storage, BackupConfig::default(), false, false).await?;
            let report = agent.verify_snapshot(snapshot.as_deref()).await?;
            println!("Checked {} chunks", report.chunks_checked);
            if !report.is_ok() {
                for id in &report.missing {
                    eprintln!("MISSING {}", id);
                }
                for id in &report.corrupt {
                    eprintln!("CORRUPT {}", id);
                }
                bail!(
                    "Verification failed: {} missing, {} corrupt",
                    report.missing.len(),
                    report.corrupt.len()
                );
            }
            println!("OK");
        }

        Commands::Stats { storage } => {
            let agent = open(&storage, BackupConfig::default(), false, false).await?;
            let stats = agent.get_stats().await?;
            println!("Files in snapshots: {}", stats.total_files);
            println!("Bytes stored:       {}", stats.total_bytes);
        }
    }

    Ok(())
}
