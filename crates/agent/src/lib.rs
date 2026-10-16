//! Backup agent: runs backups, restores and verification against a repository.
//!
//! Only local-path sources are wired up. The remaining source modules are
//! experimental, were never finished, and are compiled only with the
//! `experimental-sources` feature. They are not reachable from the CLI.

pub mod agent;
pub mod filesystem;

#[cfg(feature = "experimental-sources")]
pub mod cloudvm;
#[cfg(feature = "experimental-sources")]
pub mod database;
#[cfg(feature = "experimental-sources")]
pub mod docker;
#[cfg(feature = "experimental-sources")]
pub mod proxmox;
#[cfg(feature = "experimental-sources")]
pub mod ssh;

pub use agent::BackupAgent;
pub use filesystem::{FilesystemBackup, RestoreSummary, VerifyReport};

#[cfg(feature = "experimental-sources")]
pub use cloudvm::CloudVMBackup;
#[cfg(feature = "experimental-sources")]
pub use database::DatabaseBackup;
#[cfg(feature = "experimental-sources")]
pub use docker::DockerBackup;
#[cfg(feature = "experimental-sources")]
pub use proxmox::ProxmoxBackup;
#[cfg(feature = "experimental-sources")]
pub use ssh::SshBackup;
