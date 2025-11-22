# BackupForge Architecture

## Overview

BackupForge is built using a modular architecture with clear separation of concerns. The system is divided into several crates (Rust modules) that handle specific aspects of the backup process.

## Core Components

### 1. Common (`backupforge-common`)

Shared types and utilities used across all components.

**Key Types:**
- `ChunkId` - Unique identifier for data chunks
- `SnapshotId` - Unique identifier for backup snapshots
- `Chunk` - Represents a chunk of data
- `Snapshot` - Metadata for a complete backup
- `FileMetadata` - Information about backed-up files
- `BackupJob` - Configuration for backup jobs
- `BackupSource` - Enum for different backup sources

### 2. Core Engine (`backupforge-core`)

The heart of the backup system, handling:

#### Chunking (`chunker.rs`)
- **Fixed-size chunking**: Simple, predictable chunk boundaries
- **Content-Defined Chunking (CDC)**: Rolling hash for better deduplication
  - Uses simplified Rabin fingerprinting
  - Configurable min/avg/max chunk sizes
  - Default: 256KB min, 1MB avg, 4MB max

#### Deduplication (`dedup.rs`)
- In-memory hash index for fast duplicate detection
- Reference counting for chunk lifecycle management
- O(1) chunk existence lookups
- Thread-safe using Arc and RwLock

#### Compression (`compression.rs`)
- **Zstd**: Best compression ratio, configurable levels (1-22)
- **LZ4**: Fastest compression, lower ratio
- Automatic algorithm selection based on configuration

#### Encryption (`encryption.rs`)
- **AES-256-GCM**: Authenticated encryption
- **Argon2**: Password-based key derivation
- Random nonce per chunk for security
- Nonce prepended to ciphertext

#### Backup Engine (`engine.rs`)
- Orchestrates the entire backup pipeline:
  1. Read file data
  2. Chunk the data (CDC)
  3. Check for duplicates
  4. Compress chunks
  5. Encrypt chunks (if enabled)
  6. Store in backend

### 3. Storage (`backupforge-storage`)

Abstract storage layer supporting multiple backends.

#### Storage Backend Trait
```rust
trait StorageBackend {
    async fn put_chunk(&self, chunk_id: &ChunkId, data: Vec<u8>) -> Result<()>;
    async fn get_chunk(&self, chunk_id: &ChunkId) -> Result<Vec<u8>>;
    async fn chunk_exists(&self, chunk_id: &ChunkId) -> Result<bool>;
    async fn delete_chunk(&self, chunk_id: &ChunkId) -> Result<()>;
    async fn list_chunks(&self) -> Result<Vec<ChunkId>>;
}
```

#### Implementations:
- **Local Storage**: Filesystem-based, uses subdirectories for performance
- **S3 Storage**: S3-compatible (AWS, MinIO, B2)
- **Manager**: Unified interface for all backends

### 4. Agent (`backupforge-agent`)

Handles different backup sources.

#### Filesystem Backup
- Recursive directory walking
- Exclude pattern matching
- Metadata preservation (permissions, timestamps)

#### SSH Backup
- Remote file transfer via SCP
- Command execution on remote hosts
- Public key and password authentication

#### Proxmox Backup
- VM snapshot creation
- Container filesystem backup
- Integration with Proxmox API
- Support for MCP tools

### 5. Server (`backupforge-server`)

REST API server for web dashboard and remote management.

#### API Routes:
- `/api/auth/*` - Authentication endpoints
- `/api/jobs/*` - Backup job management
- `/api/snapshots/*` - Snapshot operations
- `/api/storage/*` - Storage statistics
- `/api/tenants/*` - Multi-tenancy management

#### Components:
- **Handlers**: Request processing logic
- **State**: Shared application state
- **Auth**: JWT-based authentication
- **Database**: Metadata persistence (SQLite/PostgreSQL)

### 6. CLI (`backupforge-cli`)

Command-line interface for interactive and scripted use.

**Commands:**
- `backup` - Create a new backup
- `restore` - Restore from snapshot
- `list` - List all snapshots
- `stats` - Show storage statistics
- `init` - Initialize repository
- `server` - Run API server

## Data Flow

### Backup Process

```
File System
    │
    ▼
Read File
    │
    ▼
Chunk Data (CDC)
    │
    ▼
┌─────────────┐
│Check Dedup? │
└──┬──────┬───┘
   │      │
   │ Yes  │ No
   │      │
   │      ▼
   │   Compress
   │      │
   │      ▼
   │   Encrypt (optional)
   │      │
   │      ▼
   │   Store Chunk
   │      │
   └──────┴──────►
            │
            ▼
    Update Dedup Index
            │
            ▼
    Record Chunk ID
            │
            ▼
    Create Snapshot
```

### Restore Process

```
Load Snapshot Metadata
    │
    ▼
For Each Chunk ID
    │
    ▼
Fetch Chunk from Storage
    │
    ▼
Decrypt (if encrypted)
    │
    ▼
Decompress
    │
    ▼
Reassemble File
    │
    ▼
Write to Target Path
    │
    ▼
Restore Metadata (permissions, timestamps)
```

## Security Architecture

### Encryption

1. **Key Derivation**:
   - User password + salt → Argon2 → 256-bit key
   - High memory/CPU cost prevents brute force

2. **Chunk Encryption**:
   - Each chunk gets unique random nonce
   - AES-256-GCM provides confidentiality + authentication
   - Format: `[12-byte nonce][ciphertext + 16-byte auth tag]`

3. **Metadata**:
   - Snapshot metadata can be encrypted separately
   - Keys stored securely (user-managed)

### Authentication (API)

1. **User Registration**:
   - Password hashed with bcrypt
   - Stored in database

2. **Login**:
   - Verify bcrypt hash
   - Generate JWT token (expiry: 24h)

3. **Request Authentication**:
   - Extract JWT from Authorization header
   - Validate signature and expiry
   - Extract user ID from claims

## Performance Optimizations

### Deduplication
- Hash-based index in memory for O(1) lookups
- Chunk hashes computed using BLAKE3 (faster than SHA-256)
- Reference counting to track chunk usage

### Compression
- Zstd level 3 balances speed and ratio
- LZ4 option for maximum speed
- Compression before encryption (better ratio)

### Async I/O
- Tokio async runtime for concurrent operations
- Non-blocking file I/O
- Parallel chunk processing (future enhancement)

### Storage
- Local: Subdirectories prevent too many files in one directory
- S3: Batch operations where possible
- Caching layer (planned)

## Multi-Tenancy

Each tenant has:
- Isolated storage namespace
- Separate quota tracking
- Independent backup jobs
- Own user accounts

Implemented via:
- Tenant ID in all data structures
- Storage path prefixing
- Database row-level filtering
- API middleware for tenant context

## Scalability Considerations

### Current Design (Single Node)
- In-memory dedup index
- Local or S3 storage
- SQLite metadata database
- Good for: Small to medium deployments

### Future Enhancements
- Distributed dedup index (Redis/Cassandra)
- PostgreSQL for metadata
- Multi-node agent deployment
- Backup job distribution
- Horizontal scaling of API servers

## Testing Strategy

### Unit Tests
- Each module has comprehensive tests
- Mock storage backends for testing
- Crypto operations tested for correctness

### Integration Tests
- End-to-end backup/restore
- Multiple storage backends
- API endpoint testing

### Performance Tests
- Chunking throughput
- Compression benchmarks
- Dedup efficiency measurements

## Monitoring and Observability

- Tracing using `tracing` crate
- Structured logging
- Metrics endpoints (planned):
  - Backup success/failure rates
  - Dedup ratios
  - Storage usage
  - API latencies
