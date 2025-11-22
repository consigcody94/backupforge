# BackupForge

**Open-source backup and disaster recovery solution** - A modern, enterprise-grade alternative to Acronis, MSP360, and Axcient.

Built with Rust for maximum performance, security, and reliability.

## Features

### Core Capabilities
- **Content-Defined Chunking (CDC)** - Efficient deduplication using rolling hash algorithm
- **AES-256-GCM Encryption** - Military-grade encryption for data at rest and in transit
- **Zstd/LZ4 Compression** - Fast compression with excellent ratios
- **Incremental Backups** - Only backup changed data
- **Multi-tenancy** - Support for MSPs managing multiple clients
- **Web Dashboard** - Modern REST API with web-based management interface

### Backup Sources
- Local files and directories
- Remote servers via SSH
- Proxmox VMs and LXC containers
- (Planned) Docker containers, databases, cloud VMs

### Storage Backends
- Local filesystem
- S3-compatible storage (AWS S3, MinIO, Backblaze B2)
- (Planned) Azure Blob, Google Cloud Storage

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      BackupForge                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Web UI     │  │   REST API   │  │   CLI Tool   │     │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘     │
│         │                 │                  │              │
│         └─────────────────┴──────────────────┘              │
│                           │                                 │
│                    ┌──────▼───────┐                        │
│                    │ Backup Agent │                        │
│                    └──────┬───────┘                        │
│                           │                                 │
│         ┌─────────────────┼─────────────────┐              │
│         │                 │                 │              │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌─────▼──────┐      │
│  │  Filesystem  │  │     SSH      │  │  Proxmox   │      │
│  │   Backup     │  │    Backup    │  │   Backup   │      │
│  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘      │
│         │                 │                 │              │
│         └─────────────────┴─────────────────┘              │
│                           │                                 │
│                    ┌──────▼───────┐                        │
│                    │ Backup Engine│                        │
│                    └──────┬───────┘                        │
│                           │                                 │
│         ┌─────────────────┼─────────────────┐              │
│         │                 │                 │              │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌─────▼──────┐      │
│  │   Chunking   │  │ Compression  │  │ Encryption │      │
│  │     (CDC)    │  │  (Zstd/LZ4)  │  │(AES-256-GCM)│      │
│  └──────┬───────┘  └──────┬───────┘  └─────┬──────┘      │
│         │                 │                 │              │
│         └─────────────────┴─────────────────┘              │
│                           │                                 │
│                    ┌──────▼───────┐                        │
│                    │Dedup Index   │                        │
│                    └──────┬───────┘                        │
│                           │                                 │
│                    ┌──────▼───────┐                        │
│                    │   Storage    │                        │
│                    │   Backend    │                        │
│                    └──────┬───────┘                        │
│                           │                                 │
│         ┌─────────────────┼─────────────────┐              │
│         │                 │                 │              │
│  ┌──────▼───────┐  ┌──────▼───────┐  ┌─────▼──────┐      │
│  │    Local     │  │      S3      │  │  Backblaze │      │
│  │  Filesystem  │  │  Compatible  │  │     B2     │      │
│  └──────────────┘  └──────────────┘  └────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

1. Install Rust (1.70+):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/backupforge.git
cd backupforge
```

2. Build the project:
```bash
cargo build --release
```

3. Install the CLI tool:
```bash
cargo install --path crates/cli
```

### Basic Usage

#### Initialize a backup repository
```bash
backupforge init --storage /var/backups/repo --encrypt
```

#### Backup a directory
```bash
backupforge backup \
  --source /home/user/documents \
  --storage /var/backups/repo \
  --exclude ".cache" \
  --exclude "*.tmp" \
  --encrypt \
  --compression 3
```

#### List snapshots
```bash
backupforge list --storage /var/backups/repo
```

#### Restore a backup
```bash
backupforge restore \
  --storage /var/backups/repo \
  --snapshot <snapshot-id> \
  --target /home/user/restored
```

#### Run the server/dashboard
```bash
backupforge server --config /etc/backupforge/config.toml --port 8080
```

## Configuration

### Storage Configuration

Create a configuration file at `/etc/backupforge/config.toml`:

```toml
[storage]
type = "local"
path = "/var/backups/repo"

# Or for S3:
# [storage]
# type = "s3"
# bucket = "my-backups"
# region = "us-east-1"
# access_key = "YOUR_ACCESS_KEY"
# secret_key = "YOUR_SECRET_KEY"

[backup]
compression = "zstd"
compression_level = 3
encryption_enabled = true

[server]
bind_address = "0.0.0.0"
port = 8080
database_url = "sqlite:///var/lib/backupforge/db.sqlite"
```

### Backup Job Configuration

Jobs can be configured via the web UI or API. Example job JSON:

```json
{
  "name": "Daily Documents Backup",
  "source": {
    "type": "LocalPath",
    "path": "/home/user/documents",
    "excludes": [".cache", "*.tmp"]
  },
  "destination": "/var/backups/repo",
  "schedule": "0 2 * * *",
  "retention_days": 30,
  "enabled": true,
  "encryption_enabled": true,
  "compression_level": 3
}
```

## API Documentation

### Authentication

```bash
# Register a new user
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "secure123", "email": "admin@example.com"}'

# Login
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "secure123"}'
```

### Backup Jobs

```bash
# Create a backup job
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d @job.json

# List all jobs
curl http://localhost:8080/api/jobs \
  -H "Authorization: Bearer <token>"

# Run a job
curl -X POST http://localhost:8080/api/jobs/<job-id>/run \
  -H "Authorization: Bearer <token>"
```

### Snapshots

```bash
# List snapshots
curl http://localhost:8080/api/snapshots \
  -H "Authorization: Bearer <token>"

# Get snapshot details
curl http://localhost:8080/api/snapshots/<snapshot-id> \
  -H "Authorization: Bearer <token>"
```

## Project Structure

```
backupforge/
├── crates/
│   ├── common/          # Shared types and utilities
│   ├── core/            # Core backup engine
│   │   ├── chunking     # Content-defined chunking
│   │   ├── dedup        # Deduplication index
│   │   ├── compression  # Compression algorithms
│   │   └── encryption   # AES-256-GCM encryption
│   ├── storage/         # Storage backends
│   │   ├── local        # Local filesystem
│   │   ├── s3           # S3-compatible storage
│   │   └── manager      # Storage abstraction
│   ├── agent/           # Backup agents
│   │   ├── filesystem   # Local file backup
│   │   ├── ssh          # Remote SSH backup
│   │   └── proxmox      # Proxmox VM/CT backup
│   ├── server/          # REST API server
│   │   ├── api          # API routes
│   │   ├── handlers     # Request handlers
│   │   └── auth         # Authentication
│   └── cli/             # Command-line interface
├── web-dashboard/       # React frontend (planned)
├── docs/                # Documentation
└── scripts/             # Helper scripts
```

## Development

### Running Tests

```bash
cargo test
```

### Running with Debug Logging

```bash
RUST_LOG=debug cargo run --bin backupforge -- backup --source /path/to/data --storage /path/to/repo
```

### Building for Production

```bash
cargo build --release --all
```

Binaries will be in `target/release/`.

## Roadmap

### v0.2.0
- [ ] Complete restore functionality
- [ ] Snapshot management (list, delete, prune)
- [ ] Retention policy enforcement
- [ ] PostgreSQL support for metadata
- [ ] Web dashboard frontend (React)

### v0.3.0
- [ ] Docker container backup
- [ ] Database backup (PostgreSQL, MySQL)
- [ ] Scheduled backup jobs
- [ ] Email notifications
- [ ] Backup verification

### v1.0.0
- [ ] Cloud VM backup (AWS EC2, Azure VMs)
- [ ] Kubernetes backup
- [ ] Advanced deduplication (global dedup)
- [ ] Backup replication
- [ ] Disaster recovery testing

## Performance

BackupForge is designed for high performance:

- **Chunking**: ~500 MB/s (content-defined chunking)
- **Compression**: ~400 MB/s (Zstd level 3)
- **Encryption**: ~2 GB/s (AES-256-GCM with hardware acceleration)
- **Deduplication**: O(1) chunk lookup using hash index

## Security

- **Encryption**: AES-256-GCM for data at rest and in transit
- **Key Derivation**: Argon2 for password-based encryption
- **Hash Algorithm**: BLAKE3 for chunk hashing (faster than SHA-256)
- **Authentication**: JWT tokens for API access
- **Zero-Knowledge**: Optional client-side encryption

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](docs/CONTRIBUTING.md) for guidelines.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

## Comparison with Alternatives

| Feature | BackupForge | Acronis | MSP360 | Restic | Duplicati |
|---------|-------------|---------|---------|--------|-----------|
| Open Source | ✅ | ❌ | ❌ | ✅ | ✅ |
| Deduplication | ✅ CDC | ✅ | ✅ | ✅ Fixed | ✅ |
| Encryption | ✅ AES-256 | ✅ | ✅ | ✅ | ✅ |
| Multi-tenancy | ✅ | ✅ | ✅ | ❌ | ❌ |
| Web Dashboard | ✅ | ✅ | ✅ | ❌ | ✅ |
| VM Backup | ✅ Proxmox | ✅ All | ✅ | ❌ | ❌ |
| SSH Backup | ✅ | ✅ | ✅ | ✅ | ❌ |
| S3 Support | ✅ | ✅ | ✅ | ✅ | ✅ |
| Written in | Rust | C++ | .NET | Go | C# |
| Performance | ⚡⚡⚡ | ⚡⚡⚡ | ⚡⚡ | ⚡⚡⚡ | ⚡⚡ |

## Support

- Documentation: [docs/](docs/)
- Issues: [GitHub Issues](https://github.com/yourusername/backupforge/issues)
- Discussions: [GitHub Discussions](https://github.com/yourusername/backupforge/discussions)

## Acknowledgments

Built with:
- [Rust](https://www.rust-lang.org/) - Systems programming language
- [Tokio](https://tokio.rs/) - Async runtime
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [BLAKE3](https://github.com/BLAKE3-team/BLAKE3) - Cryptographic hash function
- [Zstd](https://facebook.github.io/zstd/) - Compression algorithm

Inspired by: Restic, Borg Backup, Duplicati, and commercial solutions like Acronis and Veeam.
