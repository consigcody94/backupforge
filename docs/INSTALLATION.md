# BackupForge Installation Guide

## Prerequisites

### System Requirements

**Minimum:**
- 2 CPU cores
- 4 GB RAM
- 10 GB disk space
- Linux, macOS, or Windows

**Recommended:**
- 4+ CPU cores
- 8+ GB RAM
- 100+ GB disk space for backups
- Linux (Ubuntu 20.04+, Debian 11+, RHEL 8+)

### Dependencies

1. **Rust** (1.70 or later)
2. **Git**
3. **OpenSSL** development libraries
4. **pkg-config**

## Installation Methods

### Method 1: From Source (Recommended)

#### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### 2. Install System Dependencies

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev git
```

**RHEL/CentOS/Fedora:**
```bash
sudo dnf install -y gcc pkg-config openssl-devel git
```

**macOS:**
```bash
brew install openssl pkg-config
```

#### 3. Clone and Build

```bash
git clone https://github.com/yourusername/backupforge.git
cd backupforge
cargo build --release
```

This will take several minutes. Binaries will be in `target/release/`.

#### 4. Install System-Wide

```bash
sudo cp target/release/backupforge /usr/local/bin/
sudo chmod +x /usr/local/bin/backupforge
```

#### 5. Verify Installation

```bash
backupforge --version
```

### Method 2: Using Cargo Install

```bash
cargo install backupforge-cli
```

### Method 3: Pre-built Binaries (Coming Soon)

Download from [GitHub Releases](https://github.com/yourusername/backupforge/releases).

## Initial Configuration

### 1. Create Configuration Directory

```bash
sudo mkdir -p /etc/backupforge
sudo mkdir -p /var/lib/backupforge
sudo mkdir -p /var/backups/backupforge
```

### 2. Create Configuration File

Create `/etc/backupforge/config.toml`:

```toml
[storage]
type = "local"
path = "/var/backups/backupforge"

[backup]
compression = "zstd"
compression_level = 3
encryption_enabled = true

[server]
bind_address = "0.0.0.0"
port = 8080
database_url = "sqlite:///var/lib/backupforge/db.sqlite"
jwt_secret = "CHANGE_THIS_TO_RANDOM_STRING"
```

### 3. Set Permissions

```bash
sudo chown -R $USER:$USER /var/lib/backupforge
sudo chown -R $USER:$USER /var/backups/backupforge
sudo chmod 700 /var/backups/backupforge
```

### 4. Initialize Repository

```bash
backupforge init --storage /var/backups/backupforge --encrypt
```

## Running as a Service

### SystemD Service (Linux)

Create `/etc/systemd/system/backupforge.service`:

```ini
[Unit]
Description=BackupForge Backup Server
After=network.target

[Service]
Type=simple
User=backupforge
Group=backupforge
WorkingDirectory=/var/lib/backupforge
ExecStart=/usr/local/bin/backupforge server --config /etc/backupforge/config.toml
Restart=on-failure
RestartSec=10

# Security
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/backupforge /var/backups/backupforge

[Install]
WantedBy=multi-user.target
```

Create user and start service:

```bash
sudo useradd -r -s /bin/false backupforge
sudo chown -R backupforge:backupforge /var/lib/backupforge /var/backups/backupforge
sudo systemctl daemon-reload
sudo systemctl enable backupforge
sudo systemctl start backupforge
sudo systemctl status backupforge
```

### Docker Installation

Create `Dockerfile`:

```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y openssl ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/backupforge /usr/local/bin/
RUN mkdir -p /data /config
VOLUME ["/data", "/config"]
EXPOSE 8080
ENTRYPOINT ["backupforge"]
CMD ["server", "--config", "/config/config.toml"]
```

Build and run:

```bash
docker build -t backupforge .
docker run -d \
  --name backupforge \
  -p 8080:8080 \
  -v /var/backups/backupforge:/data \
  -v /etc/backupforge:/config \
  backupforge
```

## S3 Storage Setup

### AWS S3

1. Create S3 bucket:
```bash
aws s3 mb s3://my-backupforge-bucket --region us-east-1
```

2. Configure in `config.toml`:
```toml
[storage]
type = "s3"
bucket = "my-backupforge-bucket"
region = "us-east-1"
access_key = "YOUR_ACCESS_KEY"
secret_key = "YOUR_SECRET_KEY"
```

### MinIO (Self-Hosted S3)

1. Install MinIO:
```bash
wget https://dl.min.io/server/minio/release/linux-amd64/minio
chmod +x minio
sudo mv minio /usr/local/bin/

# Start MinIO
minio server /mnt/data
```

2. Configure in `config.toml`:
```toml
[storage]
type = "s3"
bucket = "backups"
region = "us-east-1"
endpoint = "http://localhost:9000"
access_key = "minioadmin"
secret_key = "minioadmin"
```

### Backblaze B2

```toml
[storage]
type = "b2"
bucket = "my-backups"
key_id = "YOUR_KEY_ID"
application_key = "YOUR_APP_KEY"
```

## Database Setup

### SQLite (Default)

No additional setup required. Database will be created automatically.

### PostgreSQL (Recommended for Production)

1. Install PostgreSQL:
```bash
sudo apt install postgresql postgresql-contrib
```

2. Create database and user:
```bash
sudo -u postgres psql
```

```sql
CREATE DATABASE backupforge;
CREATE USER backupforge WITH ENCRYPTED PASSWORD 'secure_password';
GRANT ALL PRIVILEGES ON DATABASE backupforge TO backupforge;
\q
```

3. Update `config.toml`:
```toml
[server]
database_url = "postgresql://backupforge:secure_password@localhost/backupforge"
```

## First Backup

### CLI Backup

```bash
backupforge backup \
  --source /home/user/documents \
  --storage /var/backups/backupforge \
  --encrypt \
  --compression 3
```

### Create Backup Job via API

```bash
curl -X POST http://localhost:8080/api/jobs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "name": "Daily Backup",
    "source": {
      "type": "LocalPath",
      "path": "/home/user/data",
      "excludes": [".cache", "*.tmp"]
    },
    "destination": "/var/backups/backupforge",
    "schedule": "0 2 * * *",
    "retention_days": 30,
    "enabled": true,
    "encryption_enabled": true,
    "compression_level": 3
  }'
```

## Upgrading

### From Source

```bash
cd backupforge
git pull
cargo build --release
sudo systemctl stop backupforge
sudo cp target/release/backupforge /usr/local/bin/
sudo systemctl start backupforge
```

### Using Cargo

```bash
cargo install --force backupforge-cli
```

## Troubleshooting

### Build Errors

**Error: `openssl` not found**
```bash
# Ubuntu/Debian
sudo apt install libssl-dev pkg-config

# RHEL/CentOS
sudo dnf install openssl-devel
```

**Error: Linker errors**
```bash
sudo apt install build-essential
```

### Runtime Issues

**Check logs:**
```bash
sudo journalctl -u backupforge -f
```

**Increase verbosity:**
```bash
RUST_LOG=debug backupforge backup --source /path --storage /storage
```

**Check storage permissions:**
```bash
ls -la /var/backups/backupforge
```

### Performance Tuning

**Increase chunk size for large files:**
```rust
// In code or via config
ChunkingStrategy::ContentDefined {
    min_size: 512 * 1024,      // 512 KB
    avg_size: 2 * 1024 * 1024, // 2 MB
    max_size: 8 * 1024 * 1024, // 8 MB
}
```

**Adjust compression level:**
- Level 1-3: Fast, lower ratio
- Level 3-6: Balanced (recommended)
- Level 7-22: Slower, better ratio

**Use LZ4 for speed:**
```toml
[backup]
compression = "lz4"
```

## Security Hardening

### 1. Use Strong Encryption Passwords

```bash
# Generate strong password
openssl rand -base64 32
```

### 2. Restrict File Permissions

```bash
sudo chmod 700 /var/backups/backupforge
sudo chmod 600 /etc/backupforge/config.toml
```

### 3. Use TLS for API

Configure reverse proxy (nginx):

```nginx
server {
    listen 443 ssl http2;
    server_name backupforge.example.com;

    ssl_certificate /etc/letsencrypt/live/backupforge.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/backupforge.example.com/privkey.pem;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### 4. Regular Security Updates

```bash
sudo apt update && sudo apt upgrade -y
```

## Next Steps

- [Configure backup jobs](CONFIGURATION.md)
- [Set up monitoring](MONITORING.md)
- [Read the API documentation](API.md)
- [Explore advanced features](ADVANCED.md)
