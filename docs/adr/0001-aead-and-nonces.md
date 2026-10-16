# 0001: AEAD and nonce strategy

Status: accepted for format version 1, revisit for version 2.

## Context

Every chunk is encrypted under one repository key. Chunk count is unbounded and
chunks are written from independent invocations, so a counter nonce would need
shared state across runs and machines.

## Options

1. **AES-256-GCM, random 96-bit nonce** (current). Simple, hardware accelerated,
   no state. Safe only below about 2^32 messages per key (collision probability
   around 2^-32 at that point).
2. **XChaCha20-Poly1305, random 192-bit nonce.** Nonce collisions are not a
   practical concern. Needs a new dependency and is slower without SIMD.
3. **AES-GCM-SIV.** Nonce-misuse resistant, still 96-bit nonces.
4. **Per-chunk subkeys** derived from the master key and chunk ID, with a fixed
   nonce. Removes the limit but ties the nonce to chunk identity, which needs care
   with keyed IDs and re-encryption.

## Decision

Keep option 1 for format version 1 and document the limit in the threat model
(about 4 PiB per repository at 1 MiB average chunks). Associated data binds each
ciphertext to its chunk or snapshot ID.

## Consequences

Very large repositories must not exceed the limit. Format version 2 should move to
option 2 or 4. The `format_version` field and the tag byte on every object make that
change detectable and testable.
