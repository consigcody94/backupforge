# BackupForge Dashboard - Quick Start

## Running the Dashboard on Localhost

### Option 1: Quick Start (Recommended)

```bash
cd /home/ajs/backup/backupforge

# Build and run the server
cargo run --bin backupforge-server
```

The dashboard will be available at: **http://localhost:8080**

### Option 2: Production Build

```bash
# Build the server
cargo build --release --bin backupforge-server

# Run it
./target/release/backupforge-server
```

### Option 3: Custom Port

```bash
# Set custom port via environment
PORT=3000 cargo run --bin backupforge-server
```

## What You'll See

The dashboard includes:

### 📊 Dashboard Page
- **Storage Statistics** - Total storage, deduplication ratio, active jobs
- **Recent Backups** - Latest backup snapshots
- **Storage Usage Chart** - Visual representation of storage utilization

### 📸 Snapshots Page
- View all backup snapshots
- Search and filter
- Restore or delete snapshots

### ⚙️ Backup Jobs Page
- Configure scheduled backup jobs
- View job status
- Run jobs manually

### 💾 Storage Page
- Detailed storage statistics
- Deduplication metrics
- Compression ratios

### ⚙️ Settings Page
- Configure API endpoint
- Set storage paths
- General settings

## API Endpoints

The dashboard connects to these API endpoints:

- `GET /health` - Server health check
- `GET /api/snapshots` - List all snapshots
- `POST /api/jobs` - Create backup job
- `GET /api/jobs` - List backup jobs
- `GET /api/storage/stats` - Storage statistics

## Creating Your First Backup

1. Click the **"➕ New Backup"** button
2. Enter source path (e.g., `/home/user/Documents`)
3. Give it a name
4. Choose encryption and compression settings
5. Click **"Create Backup"**

## Troubleshooting

### Server Won't Start

Make sure you have Rust installed:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Dashboard Shows "Disconnected"

Check that the server is running:
```bash
curl http://localhost:8080/health
```

Expected response:
```json
{"status":"ok","version":"0.1.0"}
```

### API Errors

Check server logs in the terminal where you ran the server.

### Port Already in Use

Change the port:
```bash
# Edit crates/server/src/state.rs
# Or use environment variable
PORT=3000 cargo run --bin backupforge-server
```

## Development Mode

For development with auto-reload:

```bash
# Install cargo-watch
cargo install cargo-watch

# Run with auto-reload
cargo watch -x 'run --bin backupforge-server'
```

## Features

✅ **Real-time Dashboard** - Live storage statistics
✅ **Backup Management** - Create and manage backups
✅ **Job Scheduling** - Configure scheduled backups
✅ **Storage Insights** - Deduplication and compression metrics
✅ **Modern UI** - Clean, responsive interface
✅ **REST API** - Full-featured backend API

## Next Steps

- Configure backup jobs
- Set up scheduled backups
- Integrate with S3 storage
- Enable encryption for sensitive data
- Configure multi-tenancy (for MSPs)

## Screenshots

### Dashboard
![Dashboard](https://via.placeholder.com/800x600?text=Dashboard)

### Snapshots
![Snapshots](https://via.placeholder.com/800x600?text=Snapshots)

### Jobs
![Jobs](https://via.placeholder.com/800x600?text=Jobs)

## Questions?

- Documentation: [README.md](README.md)
- Issues: [GitHub Issues](https://github.com/consigcody94/backupforge/issues)
- Discussions: [GitHub Discussions](https://github.com/consigcody94/backupforge/discussions)
