pub mod filesystem;
pub mod ssh;
pub mod proxmox;
pub mod docker;
pub mod database;
pub mod cloudvm;
pub mod agent;

pub use agent::BackupAgent;
pub use filesystem::FilesystemBackup;
pub use ssh::SshBackup;
pub use proxmox::ProxmoxBackup;
pub use docker::DockerBackup;
pub use database::DatabaseBackup;
pub use cloudvm::CloudVMBackup;
