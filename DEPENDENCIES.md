# BackupForge Dependencies

## Philosophy

BackupForge is designed to be **easy to install with minimal system dependencies**. We believe software should "just work" without hunting for system libraries.

## Zero System Dependencies Approach

### Vendored OpenSSL

**Problem:** Traditional Rust projects using OpenSSL require:
- Ubuntu/Debian: `libssl-dev`, `pkg-config`
- RHEL/Fedora: `openssl-devel`, `pkg-config`
- Different versions across systems
- Installation permission requirements

**Solution:** BackupForge uses **vendored OpenSSL**:
```toml
openssl = { version = "0.10", features = ["vendored"] }
```

This means:
- ✅ OpenSSL is compiled from source during build
- ✅ No system OpenSSL libraries needed
- ✅ Consistent OpenSSL version across all platforms
- ✅ No `pkg-config` required
- ✅ Works in containers without extra packages

**Trade-off:**
- Slightly longer initial build time (~2-3 minutes extra)
- Larger binary size (+2MB)
- **Worth it** for ease of installation!

## What You Actually Need

### Minimum Requirements
1. **Rust 1.70+** - The Rust toolchain
2. **C Compiler** - For compiling vendored OpenSSL
   - Linux: `gcc` (usually pre-installed)
   - macOS: Xcode Command Line Tools
   - Windows: MSVC or MinGW

### Ubuntu/Debian
```bash
# Usually just this:
sudo apt install build-essential

# build-essential includes: gcc, g++, make, libc-dev
```

### RHEL/CentOS/Fedora
```bash
sudo dnf install gcc
```

### macOS
```bash
# Install Xcode Command Line Tools
xcode-select --install
```

### Windows
Download and install [Build Tools for Visual Studio](https://visualstudio.microsoft.com/downloads/)

## Optional Dependencies

### For Docker Backup Support
- **Docker CLI** - `docker` command must be available
- Install: https://docs.docker.com/get-docker/

### For Database Backup Support
- **PostgreSQL Client** - `pg_dump` for PostgreSQL backups
  ```bash
  sudo apt install postgresql-client
  ```
- **MySQL Client** - `mysqldump` for MySQL backups
  ```bash
  sudo apt install mysql-client
  ```
- **MongoDB Tools** - `mongodump` for MongoDB backups
  ```bash
  sudo apt install mongo-tools
  ```
- **Redis CLI** - `redis-cli` for Redis backups
  ```bash
  sudo apt install redis-tools
  ```

### For SSH Backup Support
- **SSH Client** - Usually pre-installed on Linux/macOS
- **ssh2 crate** - Pure Rust SSH implementation (built-in)

## Why Not Use Rustls?

You might wonder why we don't use `rustls` (pure Rust TLS) instead of OpenSSL:

**Pros of rustls:**
- Pure Rust - no C dependencies
- Modern, secure implementation
- Faster compilation

**Cons of rustls:**
- Some ecosystem crates (like rusoto for AWS) require OpenSSL
- Certificate validation differences
- Less battle-tested in production

**Our approach:** Use vendored OpenSSL for maximum compatibility while maintaining easy installation.

## Build Time Optimization

### First Build (with vendored OpenSSL)
```bash
cargo build --release
# ~5-7 minutes (includes compiling OpenSSL)
```

### Subsequent Builds
```bash
cargo build --release
# ~1-2 minutes (OpenSSL is cached)
```

### Speeding Up Builds

Use `sccache` to cache compiled dependencies:
```bash
cargo install sccache
export RUSTC_WRAPPER=sccache
cargo build --release
```

## Container Images

Our Docker images are optimized:
- Multi-stage builds
- Alpine-based runtime (small size)
- Pre-compiled OpenSSL layer cached
- Final image: ~50MB

## Troubleshooting

### "linker `cc` not found"
Install a C compiler:
```bash
# Ubuntu/Debian
sudo apt install build-essential

# Fedora
sudo dnf install gcc

# macOS
xcode-select --install
```

### Build still fails?
Check you have enough disk space:
```bash
df -h
# Need at least 5GB free for Rust target directory
```

### Out of memory during build?
Reduce parallelism:
```bash
cargo build --release -j 2
```

## Comparison with Other Backup Tools

| Tool | System Deps | Install Complexity | Build Time |
|------|-------------|-------------------|------------|
| **BackupForge** | gcc only | ⭐ Simple | 5-7 min |
| Restic | None (Go) | ⭐⭐⭐ Very Simple | 2 min |
| Duplicati | .NET Runtime | ⭐⭐ Medium | N/A (binary) |
| Borg | Python, libs | ⭐ Complex | N/A (binary) |
| Bacula | Many libs | ⭐ Very Complex | N/A (binary) |

## Future Improvements

We're constantly working to improve the installation experience:

- [ ] Pre-built binaries for common platforms
- [ ] Snap/Flatpak packages (zero dependencies)
- [ ] Homebrew formula for macOS
- [ ] APT repository for Debian/Ubuntu
- [ ] Docker image on Docker Hub

## Questions?

If you have dependency issues:
1. Check this document
2. See [INSTALLATION.md](docs/INSTALLATION.md)
3. Open an issue: https://github.com/consigcody94/backupforge/issues

## Summary

**You asked a great question!** Dependencies should be built-in or minimal. That's exactly what we've done:

✅ **Before your question:** Required system OpenSSL libraries
✅ **After your question:** Zero OpenSSL system dependencies

Thanks for making BackupForge better! 🚀
