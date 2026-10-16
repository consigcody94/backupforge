# Threat model

Scope: the `backupforge` CLI and the crates it uses (`core`, `storage`, `agent`,
`common`) with a **local-directory repository**. The HTTP server and MCP server are
unfinished prototypes that refuse to start unless explicitly enabled; they are not
covered by any claim here. The experimental sources (SSH, Proxmox, Docker, databases,
cloud VMs) and the S3/B2 backend are unfinished and not built by default.

## What is being protected

| Asset | Where it lives |
|---|---|
| File contents | chunks under `chunks/` |
| File names, directory layout, sizes, timestamps, permissions, link targets | snapshot manifests under `metadata/` |
| The password and derived key | user's head, process memory while running |
| The user's restored files and the rest of their disk | restore target and surroundings |

## Adversaries

1. **Storage-side attacker.** Can read, copy, modify, delete, reorder or replace
   anything in the repository directory, but has no password. Examples: a cloud
   provider, someone who steals a backup disk, malware on a NAS.
2. **Malicious repository.** The user restores from a repository they did not
   create. The manifest is hostile input.
3. **Concurrent local process.** Another process running as the same user that
   creates or swaps links in the restore target while a restore runs.
4. **Local unprivileged user** on the machine running the backup.
5. **Compromised backup client.** Out of scope: if the machine running
   `backupforge` is owned while the key is in memory, the key is lost.

## What the design gives you (encrypted repositories)

| Property | Mechanism | Test |
|---|---|---|
| Content confidentiality | AES-256-GCM per chunk, key derived by Argon2id | `encrypted_round_trip_and_nothing_leaks_to_disk` |
| File names and layout confidentiality | manifest is sealed with the same AEAD | same test |
| Tamper detection for chunks | GCM tag, plus a hash of the plaintext compared to the chunk ID | `corruption_truncation_and_loss_are_detected_*`, property `encrypted_chunk_rejects_any_single_bit_flip` |
| Chunk swapping detected | chunk ID is authenticated as AAD | `swapping_two_valid_chunks_is_detected`, property `encrypted_chunk_is_bound_to_its_id` |
| Manifest tamper and rename detected | manifest sealed with its snapshot ID as AAD, ID checked after opening | `tampered_manifest_is_rejected`, `manifest_renamed_to_another_snapshot_id_is_rejected` |
| No downgrade to plaintext | an encrypted repository rejects objects tagged plaintext | `wrong_key_and_downgrade_are_rejected` |
| Known-file confirmation prevented | chunk IDs are keyed BLAKE3 (MAC), not a bare hash of content | `encrypted_ids_depend_on_key` |
| Wrong password distinguished from corruption | encrypted key-check value in `repo.json` | `wrong_or_missing_password_is_refused` |
| Hostile manifests cannot write outside the target | every write goes through a capability handle (cap-std) rooted at the target; links are never read through; pre-existing links are unlinked, not followed | `hostile_manifest_paths_cannot_escape_the_target`, `restore_refuses_to_write_through_a_symlink`, `preexisting_link_cannot_redirect_a_restored_file` |
| Hostile link targets are not restored | absolute targets and targets that climb above the restore root are skipped and counted | `links_that_escape_the_restore_directory_are_skipped` |
| Setuid/setgid/sticky bits not restored | mode bits masked to `0o777` | `file_permissions_are_restored_without_setuid` |
| Unreadable files are not silently skipped | snapshot is tagged `partial:N`, CLI exits 2 | `unreadable_files_mark_the_snapshot_partial` |
| Retention cannot delete data it cannot account for | `forget` verifies every manifest first and aborts if any fails | `forget_refuses_zero_and_aborts_on_an_unverifiable_manifest` |
| Backup and forget do not run concurrently | exclusive lock object; released on success and failure | `backups_refuse_to_run_while_the_repository_is_locked` |

Unencrypted repositories still get integrity checks (chunk hash, manifest checksum)
but no confidentiality and no protection against an attacker who rewrites a chunk
and its hash together.

## Restore containment, precisely

Restore resolves every path relative to a directory handle opened on the target.
That is what closes the race a path-string check cannot: an attacker who can create
a link in the target between "check" and "write" cannot redirect the write, because
the write is relative to the handle and never follows a link out of the root.

Remaining limits:
- A concurrent process running as the same user can still modify the target itself
  (for example delete files a restore just wrote). Containment prevents writes
  outside the target, not tampering inside it.
- Restore is not atomic. An error part-way leaves a partial tree; re-run into an
  empty directory.
- Metadata (mode, mtime) is applied best-effort after content is verified. Owner,
  extended attributes and ACLs are not restored.

## What it does NOT give you

- **No freshness or rollback protection.** A storage-side attacker can delete
  snapshots or chunks (detected only when you verify or restore), and can put back an
  older, valid copy of the whole repository. Nothing records "latest".
- **Metadata leaks.** An attacker sees the number and approximate sizes of chunks
  (compressed sizes), the number of snapshots and their file modification times, and
  total repository size. Chunk identity within one repository leaks equality of chunks
  (inherent to deduplication).
- **Compression before encryption.** Sizes reflect compressibility.
- **No key rotation or password change.** The key is derived directly from the
  password; changing it means a new repository.
- **Password handling in memory.** The derived key is zeroized on drop. The password
  string and copies made inside the KDF library are not guaranteed to be wiped.
- **Nonce limit.** Chunks use random 96-bit nonces under one key. NIST guidance limits
  that to 2^32 messages per key; at 1 MiB average chunks that is about 4 PiB per
  repository. See `docs/adr/0001-aead-and-nonces.md`.
- **Locking is advisory and local.** The lock is an object in the repository. It stops
  two `backupforge` operations from overlapping. It does not stop an unrelated process
  from editing the repository. A killed operation leaves the lock behind until
  `backupforge unlock` is run. Storage backends that cannot create objects exclusively
  refuse backup and forget rather than run without the lock.
- **Retention is not safe against concurrent readers.** Forgetting a snapshot while it
  is being restored can make that restore fail partway.
- **Files are not snapshotted atomically.** Files changing during a backup can be
  captured inconsistently.
- **Special files are not backed up.** Sockets, FIFOs and device files are skipped and
  counted as `partial`.
- **File permissions of the repository** follow the process umask.
- **Local multi-user attackers** can read a repository that is world-readable and can
  observe `BACKUPFORGE_PASSWORD` if the environment is exposed. Prefer the prompt.

## Testing status

- Unit, end-to-end, CLI and property tests pass on every push (CI runs them on Linux
  and macOS; Windows is run but not yet required).
- Fuzzing, 5 minutes per target, on 2026-10-16 (about 20 minutes total):

| Target | Inputs run | Crashes |
|---|---|---|
| `open_chunk` (random bytes as a stored chunk) | 2,965,173 | 0 |
| `open_blob` (random bytes as a sealed manifest) | 12,220,103 | 0 |
| `repo_config` (random header, unlock attempted) | 19,631,863 | 0 |
| `chunker` (FastCDC and fixed chunks reassemble to input) | 2,635,390 | 0 |

  The first `chunker` run crashed. The fault was in the fuzz target, which still used
  chunk sizes the library now rejects. The library returned an error, as intended.
  The target was corrected and the rerun is the result above.

- Reproducible build: two release builds of `backupforge` from separate target
  directories, with paths remapped, produced identical SHA-256 hashes
  (`f446eb61…`). This was checked on one machine and toolchain only.

Fuzzing is not a proof of absence of bugs. Five minutes per target is a smoke test, not
a campaign. The weekly workflow `.github/workflows/fuzz.yml` runs each target for 10 minutes.

## Not in scope

S3/B2 backends, the HTTP server, the MCP server, and the experimental sources. These
are behind feature flags or explicit opt-in environment variables and are listed in the
README with their status.
