# Security policy

## Supported versions

Only the `main` branch and the latest release receive security fixes. The project
is pre-1.0.

## Reporting a vulnerability

Please do **not** open a public issue.

Report privately through GitHub:
<https://github.com/consigcody94/backupforge/security/advisories/new>

Include what you found, how to reproduce it, and the version or commit.

## What to expect

- Acknowledgement within 3 business days.
- A fix or a written assessment within 30 days for confirmed issues.
- Coordinated disclosure: a public advisory once a fix is released, normally within
  90 days of the report. Reporters are credited unless they prefer otherwise.

## Scope

In scope: the `backupforge` CLI and the `backupforge-core`, `-storage`, `-agent`
and `-common` crates, including the repository format and cryptography described in
`docs/THREAT_MODEL.md` and `docs/FORMAT.md`.

Out of scope: the `server` and `mcp-server` crates and the `experimental-sources`
feature (prototypes with known gaps, listed in the threat model), and attacks that
need a compromised client machine.

## Known limitations

Documented, with their reasoning, in `docs/THREAT_MODEL.md`. Reports that restate a
listed limitation are welcome as design feedback but are not treated as new
vulnerabilities.
