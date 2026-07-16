# Security Policy

## Scope

`dnstt-kit` is a **local** DNSTT / DNS-resolver toolkit (CLI + optional localhost web UI).  
It is **not** a public multi-tenant SaaS. Treat the web panel as **admin-on-localhost**.

## Supported versions

| Version | Supported |
|---------|-----------|
| Latest release on GitHub Releases | Yes |
| Older tags | Best effort |
| `main` branch | Development only |

## What to report

**Do report:**

- Path traversal / arbitrary file read-write via API
- Command injection beyond the intentional CLI exec allow-list
- Accidental exposure of secrets (`.env`, `SLIPNET_CONFIG`, passwords) in API/UI/logs
- Crashes / memory exhaustion reproducible on constrained hosts (~512 MB)
- Supply-chain issues in vendored scripts

**Do not report as vulnerabilities (by design):**

- Running `scan` / `pipeline` against IPs you supply (operator responsibility)
- Local process spawning of `dns-cli` subcommands from the job queue
- Features requiring network when you explicitly enable slipnet fetch

## Web / API hardening (built-in)

- Default bind: `127.0.0.1` (do **not** expose `0.0.0.0` without a reverse proxy + auth)
- Optional token: `DNS_CLI_WEB_TOKEN` — required as `Authorization: Bearer …` or `X-DNS-CLI-Token`
- API never returns full absolute `work_dir` (masked label only)
- Relative paths only; `..` and absolute paths rejected for scan/pipeline inputs
- Blocked from web: `serve`, `menu`, `backup watch`, `completion`
- Single background job at a time

## Antivirus false positives

Unsigned Windows builds of small Rust CLIs are often flagged by heuristic scanners. That is a **false positive risk**, not a known malware implant in this project. Mitigations for users: build from source, verify `SHA256SUMS.txt` on Releases, allowlist after review. Maintainers: prefer GitHub Actions artifacts from this repo; do not distribute UPX-packed binaries.

## Reporting

1. Prefer a private channel / GitHub Security Advisory when available  
2. Or open an Issue with label `security` — **do not** paste live secrets  
3. Include: OS, version/commit, bind address, steps, expected vs actual

## Secrets in git

If `.env`, real `SLIPNET_CONFIG`, or profile passwords were committed:

1. Rotate the secret immediately  
2. Remove from history / open an issue without re-pasting the secret  

See [docs/SECURITY_WEB.md](docs/SECURITY_WEB.md) and [.env.example](.env.example).
