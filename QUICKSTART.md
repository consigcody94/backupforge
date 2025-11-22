# BackupForge Quick Start Guide

Get up and running with BackupForge in 5 minutes!

## Installation

### Automated Setup

```bash
git clone https://github.com/consigcody94/backupforge.git
cd backupforge
./scripts/setup.sh
```

The setup script will:
- Install Rust if needed
- Build BackupForge
- Create configuration directories
- Initialize a backup repository

### Manual Setup

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/consigcody94/backupforge.git
cd backupforge
cargo build --release

# Install
sudo cp target/release/backupforge /usr/local/bin/
```

## Your First Backup

### 1. Initialize Repository

```bash
backupforge init --storage /var/backups/repo --encrypt
```

### 2. Create a Backup

```bash
backupforge backup \
  --source ~/Documents \
  --storage /var/backups/repo \
  --exclude ".cache" \
  --encrypt
```

Output:
```
🚀 Starting backup...
Source: /home/user/Documents
Storage: /var/backups/repo
🔒 Encryption enabled
✅ Backup completed!
Snapshot ID: 550e8400-e29b-41d4-a716-446655440000
Files: 245
Size: 1048576 bytes
Compressed: 524288 bytes (50.0%)
```

### 3. List Your Backups

```bash
backupforge list --storage /var/backups/repo
```

### 4. Restore a Backup

```bash
backupforge restore \
  --storage /var/backups/repo \
  --snapshot 550e8400-e29b-41d4-a716-446655440000 \
  --target ~/Restored
```

## Common Use Cases

### Backup to S3

```toml
# ~/.config/backupforge/config.toml
[storage]
type = "s3"
bucket = "my-backups"
region = "us-east-1"
access_key = "YOUR_ACCESS_KEY"
secret_key = "YOUR_SECRET_KEY"
```

Then:
```bash
backupforge backup --source ~/Data --storage s3://my-backups
```

### Backup Remote Server via SSH

```bash
backupforge backup \
  --source ssh://user@server:/var/www \
  --storage /var/backups/repo
```

### Backup Proxmox VM

```bash
# Coming soon - via configuration file or API
```

### Schedule Daily Backups

Create a cron job:
```bash
crontab -e
```

Add:
```cron
0 2 * * * /usr/local/bin/backupforge backup --source /home/user/data --storage /var/backups/repo
```

## Run the Web Dashboard

### Start Server

```bash
backupforge server --config ~/.config/backupforge/config.toml --port 8080
```

### Access Dashboard

Open browser to: http://localhost:8080

### Create Account

```bash
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "secure123",
    "email": "admin@example.com"
  }'
```

### Login and Get Token

```bash
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "admin",
    "password": "secure123"
  }'
```

### Create Backup Job

```bash
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Daily Backup",
    "source": {
      "type": "LocalPath",
      "path": "/home/user/data",
      "excludes": [".cache"]
    },
    "destination": "/var/backups/repo",
    "schedule": "0 2 * * *",
    "retention_days": 30,
    "enabled": true,
    "encryption_enabled": true,
    "compression_level": 3
  }'
```

## Configuration Examples

### High Compression

```toml
[backup]
compression = "zstd"
compression_level = 15  # Slower but better ratio
```

### Fast Backups

```toml
[backup]
compression = "lz4"  # Fastest compression
```

### Larger Chunks (for big files)

In code:
```rust
ChunkingStrategy::ContentDefined {
    min_size: 512 * 1024,      // 512 KB
    avg_size: 2 * 1024 * 1024, // 2 MB
    max_size: 8 * 1024 * 1024, // 8 MB
}
```

## Monitoring

### Check Storage Stats

```bash
backupforge stats --storage /var/backups/repo
```

### Enable Debug Logging

```bash
RUST_LOG=debug backupforge backup --source /data --storage /repo
```

## Troubleshooting

### Build Fails

```bash
# Install dependencies
sudo apt install build-essential pkg-config libssl-dev

# Clean and rebuild
cargo clean
cargo build --release
```

### Permission Denied

```bash
sudo chown -R $USER:$USER /var/backups/repo
chmod 700 /var/backups/repo
```

### Out of Memory

Reduce chunk size or use LZ4 compression instead of Zstd.

## Next Steps

- Read the [Full Documentation](README.md)
- Explore [Advanced Features](docs/ADVANCED.md)
- Check out the [API Reference](docs/API.md)
- Join the community on [GitHub Discussions](https://github.com/consigcody94/backupforge/discussions)

## Need Help?

- Documentation: [docs/](docs/)
- Issues: [GitHub Issues](https://github.com/consigcody94/backupforge/issues)
- Discussions: [GitHub Discussions](https://github.com/consigcody94/backupforge/discussions)

## Tips

1. **Always test restores** - Backups are useless if you can't restore!
2. **Use encryption** for sensitive data
3. **Store backups off-site** - Use S3 or another cloud provider
4. **Monitor backup status** - Set up alerts for failed backups
5. **Test your disaster recovery plan** regularly

Happy backing up! 🚀
