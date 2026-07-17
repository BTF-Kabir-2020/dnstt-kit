# Security

`dnstt-kit` is a local CLI + optional localhost web UI. It is not a public multi-tenant service. Treat the panel like admin-on-localhost.

## Supported

| Version | Support |
|---------|---------|
| Latest GitHub Release | Yes |
| Older tags | Best effort |
| `main` | Development |

## Please report

- Path traversal / arbitrary file access through the API
- Command injection outside the intentional job allow-list
- Secrets leaking in API/UI/logs (`.env`, slipnet config, passwords)
- Reproducible OOM / crashes on ~512 MB hosts when using `--preset low` / `--stream` (unexpected for large lists)
- Supply-chain issues in vendored scripts

## Usually not vulnerabilities

- Scanning IPs you yourself provide
- Spawning allow-listed `dns-cli` subcommands from the job queue
- Network use when you explicitly run slipnet fetch

## Web defaults

- Bind `127.0.0.1` (don’t expose `0.0.0.0` without a reverse proxy + auth)
- Optional `DNS_CLI_WEB_TOKEN` (`Authorization: Bearer …` or `X-DNS-CLI-Token`)
- Absolute `work_dir` is masked in API responses
- Relative paths only; `..` and absolute inputs rejected
- Web cannot start `serve`, `menu`, `backup watch`, or `completion`
- One background job at a time

## Antivirus

Unsigned Windows Rust CLIs often trip heuristics. Prefer building from source or verifying `SHA256SUMS.txt` on Releases before allowlisting. We don’t ship UPX-packed binaries.

## How to report

Prefer a private advisory if available; otherwise an Issue labeled `security` without live secrets. Include OS, version/commit, bind address, and steps.

If a real secret was committed: rotate it, then clean history / open an issue without pasting the value again.

See [docs/SECURITY_WEB.md](docs/SECURITY_WEB.md) and [.env.example](.env.example).
