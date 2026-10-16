# BackupForge

[![CI](https://github.com/consigcody94/backupforge/actions/workflows/ci.yml/badge.svg)](https://github.com/consigcody94/backupforge/actions/workflows/ci.yml)
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/consigcody94/backupforge/badge)](https://scorecard.dev/viewer/?uri=github.com/consigcody94/backupforge)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

A command-line backup tool written in Rust. It splits files into chunks,
deduplicates them, compresses them, and can encrypt everything, including file
names, with a password. Backups are verifiable and restores check every chunk.

**Status: early development (pre-0.1).** Local-directory backup, restore and
verification work and are covered by end-to-end tests. Several things shown in
older versions of this README did not exist; see [What works](#what-works) for the
accurate list.

## Quick start

```bash
cargo install --path crates/cli      # installs the `backupforge` binary

# Back up a directory into a new encrypted repository (prompts for a password)
backupforge backup -s ~/Documents -d /mnt/backup/docs --encrypt

backupforge list    -d /mnt/backup/docs
backupforge verify  -d /mnt/backup/docs            # re-reads and checks every chunk
backupforge restore -d /mnt/backup/docs -t ~/restored
```

- The password comes from an interactive prompt (no echo). For scripts you can set
  `BACKUPFORGE_PASSWORD`, but environment variables can leak; prefer the prompt.
- A second `backup` into the same repository only stores chunks it does not already
  have.
- `backup` exits with status 2 and prints a warning if some files could not be read;
  the snapshot is tagged `partial:N`.
- Exclude paths with `-x node_modules` (repeatable, substring match).

## What works

| Feature | State |
|---|---|
| Backup of a local directory or file | works, tested |
| Content-defined chunking, zstd compression, deduplication across runs | works |
| Password encryption (AES-256-GCM, Argon2id), file names included | works, tested |
| Restore, `verify`, `list`, `stats` | works, tested |
| Corruption, truncation, deletion and chunk-swap detection | works, tested |
| Safe restore from a hostile repository (no path escape, no symlink writes) | works, tested |
| S3 / Backblaze backends | **not usable**: credentials are not wired up |
| HTTP server and web dashboard | **prototype**: authentication is not implemented, login returns 501 |
| MCP server | **prototype**, unreviewed |
| SSH, Proxmox, Docker, database, cloud-VM sources | **unfinished**, off by default (`experimental-sources` feature) |
| Scheduling, retention, pruning, incremental parent snapshots | not implemented |
| Symlinks, ownership, extended attributes, timestamps on restore | not preserved |

## Security model

Short version; the full reasoning is in [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md)
and the byte-level layout is in [docs/FORMAT.md](docs/FORMAT.md).

**Design**

- Each chunk is compressed, then encrypted with AES-256-GCM under a key derived from
  your password with Argon2id (64 MiB, 3 passes; parameters are stored in the
  repository header). Every ciphertext is bound to its chunk ID as associated data.
- Chunk IDs in encrypted repositories are keyed BLAKE3 values, so someone with the
  repository cannot test whether a known file is in it.
- Snapshot manifests (file names, sizes, layout) are sealed the same way.
- Every restored chunk is decrypted, decompressed and re-hashed against its ID.
- The derived key is zeroized when dropped.

**What it protects against:** someone who can read or modify the repository but does
not have the password (stolen disk, cloud provider, malware on the storage box).

**What it does not protect against**

- Deletion or rollback of the whole repository to an older valid state.
- Leaks of chunk count and sizes, snapshot count and times, and total size.
- A compromised machine running the backup.
- Very large repositories: random 96-bit nonces limit one key to roughly 4 PiB.
- A weak password. Argon2id slows guessing; it cannot stop it.

This project has not had an independent security audit. Do not rely on it as your
only copy of data you cannot lose.

Report vulnerabilities privately: see [SECURITY.md](SECURITY.md).

## Build and test

```bash
cargo build --release -p backupforge
cargo test --workspace          # unit and end-to-end tests
cargo clippy --workspace --all-targets -- -D warnings
```

Layout:

```
crates/common    shared types, errors, hashing
crates/core      chunking, compression, encryption, repository header, engine
crates/storage   storage backends (local directory; S3 unfinished)
crates/agent     backup, restore, verify
crates/cli       the `backupforge` binary
crates/server    HTTP API prototype
crates/mcp-server  MCP server prototype
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [GOVERNANCE.md](GOVERNANCE.md). Good
first issues are labelled. Security-sensitive changes need an ADR in `docs/adr/`.

## License

Apache-2.0. See [LICENSE](LICENSE).
