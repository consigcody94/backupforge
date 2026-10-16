# Governance

BackupForge is a small project with one maintainer. This document says how
decisions are made so contributors know what to expect.

## Roles

- **Maintainer**: currently Cody ([@consigcody94](https://github.com/consigcody94)).
  Merges changes, cuts releases, handles security reports, and has final say.
- **Contributor**: anyone who opens an issue, pull request, or review.
- **Committer**: a contributor with merge rights. Granted by the maintainer after
  several accepted changes and a record of careful review. Not yet used.

## How decisions are made

- Day-to-day changes: pull request, at least one review when a second reviewer
  exists, passing CI.
- Changes to the repository format, cryptography, or the threat model need an entry
  in `docs/adr/` and an update to `docs/THREAT_MODEL.md` and `docs/FORMAT.md` in the
  same pull request.
- Disagreement is settled by discussion on the issue. If it stays open, the
  maintainer decides and records the reason.

## Releases

Tags of the form `vX.Y.Z` trigger the release workflow. Releases are built in CI,
checksummed, and carry build provenance attestations. The maintainer reviews and
publishes the draft release.

## Security

Vulnerabilities are reported privately (see `SECURITY.md`) and are not discussed in
public issues until a fix is available.

## Becoming a maintainer

Sustained, high-quality contributions, plus agreement to follow `CODE_OF_CONDUCT.md`
and `SECURITY.md`. The maintainer invites; existing maintainers must agree.

## Continuity

If the maintainer is unreachable for 90 days, a committer (if any) may publish
security fixes and may ask GitHub to transfer the repository. Until a second
committer exists this is a risk the project accepts and says so openly.

## Changes to this document

By pull request, approved by the maintainer.
