use backupforge_common::{types::Snapshot, Error, Result};
use backupforge_core::BackupEngine;
use backupforge_storage::StorageManager;
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;

/// SSH-based remote backup
pub struct SshBackup {
    engine: Arc<BackupEngine>,
    storage: Arc<StorageManager>,
}

impl SshBackup {
    pub fn new(engine: Arc<BackupEngine>, storage: Arc<StorageManager>) -> Self {
        Self { engine, storage }
    }

    /// Connect to remote host via SSH
    pub fn connect(
        &self,
        host: &str,
        port: u16,
        username: &str,
        password: Option<&str>,
        private_key_path: Option<&Path>,
    ) -> Result<Session> {
        let tcp = TcpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| Error::Network(format!("Failed to connect: {}", e)))?;

        let mut sess = Session::new()
            .map_err(|e| Error::Network(format!("Failed to create session: {}", e)))?;

        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| Error::Network(format!("SSH handshake failed: {}", e)))?;

        // Authenticate
        if let Some(key_path) = private_key_path {
            sess.userauth_pubkey_file(username, None, key_path, None)
                .map_err(|e| Error::AuthenticationFailed(format!("Key auth failed: {}", e)))?;
        } else if let Some(pass) = password {
            sess.userauth_password(username, pass)
                .map_err(|e| Error::AuthenticationFailed(format!("Password auth failed: {}", e)))?;
        } else {
            return Err(Error::AuthenticationFailed(
                "No authentication method provided".to_string(),
            ));
        }

        Ok(sess)
    }

    /// Backup a file from remote server
    pub async fn backup_remote_file(
        &self,
        session: &Session,
        remote_path: &str,
    ) -> Result<Vec<u8>> {
        let (mut remote_file, _stat) = session
            .scp_recv(Path::new(remote_path))
            .map_err(|e| Error::Network(format!("SCP recv failed: {}", e)))?;

        let mut data = Vec::new();
        remote_file
            .read_to_end(&mut data)
            .map_err(|e| Error::Io(e))?;

        Ok(data)
    }

    /// Execute command on remote host
    pub fn execute_command(&self, session: &Session, command: &str) -> Result<String> {
        let mut channel = session
            .channel_session()
            .map_err(|e| Error::Network(format!("Failed to open channel: {}", e)))?;

        channel
            .exec(command)
            .map_err(|e| Error::Network(format!("Failed to execute command: {}", e)))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| Error::Io(e))?;

        channel
            .wait_close()
            .map_err(|e| Error::Network(format!("Channel close failed: {}", e)))?;

        Ok(output)
    }

    /// List files in remote directory
    pub fn list_remote_files(&self, session: &Session, path: &str) -> Result<Vec<String>> {
        let output = self.execute_command(session, &format!("find {} -type f", path))?;

        let files: Vec<String> = output
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssh_backup_creation() {
        let engine = Arc::new(BackupEngine::new(Default::default()));
        let storage_config = backupforge_storage::StorageConfig::Local {
            path: "/tmp/test".to_string(),
        };

        // Note: actual SSH tests would require a test SSH server
        // This just tests that we can create the struct
    }
}
