# Repository format (version 1)

A repository is a directory:

```
repo/
  metadata/
    repo.json                 public header
    snapshot-<uuid>           one sealed manifest per snapshot
  chunks/<first 2 chars>/<chunk id>
```

All names read from storage are validated (`[A-Za-z0-9._-]`, no leading dot) and
all files are written to a temporary name, fsynced, then renamed.

## repo.json

```json
{
  "format_version": 1,
  "encrypted": true,
  "kdf": { "memory_kib": 65536, "iterations": 3, "parallelism": 1, "salt": [16 bytes] },
  "key_check": "<hex of nonce || AES-GCM(\"backupforge key check v1\", aad=\"bf1/key-check\")>"
}
```

A reader refuses a `format_version` newer than it supports. KDF parameters are
stored so they can be raised for new repositories without breaking old ones.

## Keys

`master = Argon2id(password, salt, memory_kib, iterations, parallelism) -> 32 bytes`

Subkeys use BLAKE3 `derive_key` with context `"backupforge 2026 v1 <purpose>"`:
`aead` for AES-256-GCM and `chunk-id` for keyed chunk IDs.

## Chunk IDs

- unencrypted: `hex(BLAKE3(plaintext chunk))`
- encrypted: `hex(BLAKE3-keyed(chunk-id subkey, plaintext chunk))`

## Sealed objects

First byte is the format tag.

- `0x01` plain chunk: `0x01 || compression_tag || compressed`
- `0x02` encrypted chunk: `0x02 || nonce(12) || AES-256-GCM(compression_tag || compressed)`,
  AAD = `"bf1/chunk/" + chunk id`
- plain blob (manifest): `0x01 || BLAKE3(plaintext) || plaintext`
- encrypted blob: `0x02 || nonce(12) || AES-256-GCM(plaintext)`,
  AAD = `"bf1/blob/snapshot/" + snapshot id`

Compression tags: `0` none, `1` zstd, `2` lz4. Decompressed chunks are capped at
128 MiB.

On read, the chunk is decrypted, decompressed, re-hashed and compared with its ID.
A repository that is encrypted rejects `0x01` objects.

## Manifest

JSON `SnapshotManifest { snapshot, files[] }`. File `path` is relative to the
backed-up root with `/` separators. Directories appear as entries with
`is_directory: true`. A manifest's snapshot ID must equal the ID in its file name.

## Compatibility rule

Any change that alters how an existing repository is read bumps `format_version`
and ships with a test that still reads a repository written by the previous
version.
