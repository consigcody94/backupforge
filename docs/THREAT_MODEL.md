# Threat model

Scope: the `backupforge` CLI and the crates it uses (`core`, `storage`, `agent`,
`common`) with a **local-directory repository**. The `server` and `mcp-server`
crates and the `experimental-sources` feature are prototypes and are **not**
covered by any claim here.

## What is being protected

| Asset | Where it lives |
|---|---|
| File contents | chunks under `chunks/` |
| File names, directory layout, sizes, timestamps, permissions | snapshot manifests under `metadata/` |
| The password and derived key | user's head, process memory while running |
| The user's restored files and the rest of their disk | restore target and surroundings |

## Adversaries

1. **Storage-side attacker.** Can read, copy, modify, delete, reorder or replace
   anything in the repository directory or bucket, but has no password. Examples:
   a cloud provider, someone who steals a backup disk, malware on a NAS.
2. **Malicious repository.** The user restores from a repository they did not
   create. The manifest is hostile input.
3. **Local unprivileged user** on the machine running the backup.
4. **Compromised backup client.** Out of scope: if the machine running
   `backupforge` is owned while the key is in memory, the key is lost.

## What the design gives you (encrypted repositories)

| Property | Mechanism | Test |
|---|---|---|
| Content confidentiality | AES-256-GCM per chunk, key derived by Argon2id | `encrypted_round_trip_and_nothing_leaks_to_disk` |
| File names and layout confidentiality | manifest is sealed with the same AEAD | same test |
| Tamper detection for chunks | GCM tag, plus a hash of the plaintext compared to the chunk ID | `corruption_truncation_and_loss_are_detected_*` |
| Chunk swapping detected | chunk ID is authenticated as AAD | `swapping_two_valid_chunks_is_detected` |
| Manifest tamper and rename detected | manifest sealed with its snapshot ID as AAD, ID checked after opening | `tampered_manifest_is_rejected`, `manifest_renamed_to_another_snapshot_id_is_rejected` |
| No downgrade to plaintext | an encrypted repository rejects objects tagged plaintext | `wrong_key_and_downgrade_are_rejected` |
| Known-file confirmation prevented | chunk IDs are keyed BLAKE3 (MAC), not a bare hash of content | `encrypted_ids_depend_on_key` |
| Wrong password distinguished from corruption | encrypted key-check value in `repo.json` | `wrong_or_missing_password_is_refused` |
| Safe restore from a hostile manifest | absolute paths, `..`, and symlinks under the target are refused; restored mode bits drop setuid/setgid/sticky | `hostile_manifest_paths_cannot_escape_the_target`, `restore_refuses_to_write_through_a_symlink`, `file_permissions_are_restored_without_setuid` |
| Unreadable files are not silently skipped | snapshot is tagged `partial:N`, CLI exits 2 | `unreadable_files_mark_the_snapshot_partial` |

Unencrypted repositories still get integrity checks (chunk hash, manifest
checksum) but no confidentiality and no protection against an attacker who
rewrites a chunk and its hash together.

## What it does NOT give you

- **No freshness or rollback protection.** A storage-side attacker can delete
  snapshots or chunks (detected only when you verify or restore), and can put back
  an older, valid copy of the whole repository. Nothing records "latest".
- **Metadata leaks.** An attacker sees the number and approximate sizes of chunks
  (compressed sizes), the number of snapshots and their file modification times,
  and total repository size. Chunk identity within one repository leaks equality
  of chunks (inherent to deduplication).
- **Compression before encryption.** Sizes reflect compressibility. This matters
  mainly if an attacker can influence file contents before backup.
- **No key rotation or password change.** The key is derived directly from the
  password; changing it means a new repository.
- **Password handling in memory.** The derived key is zeroized on drop. The
  password string, the CLI's copy of it, and copies made inside the KDF library
  are not guaranteed to be wiped.
- **Nonce limit.** Chunks use random 96-bit nonces under one key. NIST guidance
  limits that to 2^32 messages per key; at 1 MiB average chunks that is about
  4 PiB per repository. See `docs/adr/0001-aead-and-nonces.md`.
- **Weak chunker.** The content-defined chunker is a simple hash, not FastCDC.
  It affects deduplication quality, not confidentiality.
- **Files are not snapshotted atomically.** Files changing during a backup can be
  captured inconsistently.
- **Symlinks, special files, ownership, extended attributes and timestamps on
  restore are not preserved.** Symlinks are skipped with a warning.
- **File permissions of the repository** follow the process umask; the tool does
  not tighten them.
- **Local multi-user attackers** can read a repository that is world-readable and
  can observe `BACKUPFORGE_PASSWORD` if the environment is exposed. Prefer the
  interactive prompt.

## Not in scope

S3/B2 backends (the S3 backend does not use the configured credentials yet and
depends on an unmaintained SDK), the HTTP server (authentication is not
implemented; login and register return 501), the MCP server, and the
experimental sources (SSH, Proxmox, Docker, databases, cloud VMs).
