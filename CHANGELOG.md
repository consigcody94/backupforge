# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-01-21

### Added

#### Core Features
- Content-Defined Chunking (CDC) with rolling hash algorithm
- Deduplication engine with reference counting
- AES-256-GCM encryption for data at rest
- Argon2 password-based key derivation
- Zstd and LZ4 compression support
- BLAKE3 hashing for chunk identification

#### Storage Backends
- Local filesystem storage with subdirectory optimization
- S3-compatible storage (AWS S3, MinIO, Backblaze B2)
- Storage abstraction layer for easy backend extension

#### Backup Sources
- Local file and directory backup
- Remote server backup via SSH
- Proxmox VM backup support (framework)
- Proxmox LXC container backup support (framework)
- Recursive directory walking with exclude patterns
- File metadata preservation (permissions, timestamps)

#### API Server
- REST API with authentication
- JWT-based authentication system
- Multi-tenancy support
- Backup job management endpoints
- Snapshot management endpoints
- Storage statistics endpoints
- Tenant management endpoints
- CORS support for web dashboard

#### CLI Tool
- `backup` command for creating backups
- `restore` command for restoring backups
- `list` command for listing snapshots
- `stats` command for storage statistics
- `init` command for repository initialization
- `server` command for running API server
- Verbose logging support
- Encryption and compression configuration

#### Documentation
- Comprehensive README with feature comparison
- Quick Start guide
- Installation guide with multiple methods
- Architecture documentation
- API documentation
- Contributing guidelines
- Code of Conduct
- Security policy

#### Development Tools
- Automated setup script
- GitHub Actions CI/CD pipeline
- Issue and PR templates
- Comprehensive test suite
- Docker support

### Security
- AES-256-GCM authenticated encryption
- Argon2 for secure password hashing
- JWT token-based authentication
- Secure random nonce generation
- No plaintext password storage

### Performance
- ~500 MB/s chunking throughput
- ~400 MB/s compression (Zstd level 3)
- ~2 GB/s encryption (with AES-NI)
- O(1) deduplication lookup
- Async I/O for concurrent operations

## [0.0.1] - 2024-01-20

### Added
- Initial project structure
- Basic workspace setup
- Core module scaffolding

---

[Unreleased]: https://github.com/consigcody94/backupforge/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/consigcody94/backupforge/releases/tag/v0.1.0
[0.0.1]: https://github.com/consigcody94/backupforge/releases/tag/v0.0.1
