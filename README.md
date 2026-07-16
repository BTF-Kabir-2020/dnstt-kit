# dnstt-kit (Rust) — v0.1.0

[![CI](https://github.com/BTF-Kabir-2020/dnstt-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/BTF-Kabir-2020/dnstt-kit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Non--Commercial-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![Release](https://img.shields.io/github/v/release/BTF-Kabir-2020/dnstt-kit)](https://github.com/BTF-Kabir-2020/dnstt-kit/releases)

DNSTT toolkit: scan DNS resolvers, generate NetMod / NekoBox / SlipNet configs, optional offline slipnet, localhost web UI, backup helpers, Docker.

Author: [BTF Kabir](https://github.com/BTF-Kabir-2020) · also: [CS-1.6-Tool-v2](https://github.com/BTF-Kabir-2020/CS-1.6-Tool-v2)

PRs welcome — [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Downloads

Prebuilt: [Releases](https://github.com/BTF-Kabir-2020/dnstt-kit/releases)

| Asset | Platform |
|-------|----------|
| `dnstt-kit-windows-x64.exe` | Windows |
| `dnstt-kit-linux-x64` | Linux x64 |
| `dnstt-kit-linux-arm64` | Linux ARM64 |
| `dnstt-kit-macos-arm64` | macOS Apple Silicon |

Plus `LICENSE`, `README.md`, `SHA256SUMS.txt`.

### Antivirus note

Unsigned Rust CLIs get heuristic hits sometimes (Defender “generic”). Builds are plain GitHub Actions artifacts, no UPX. If something quarantines the exe: build with `cargo build -p dns-cli --release`, or allowlist after checking the release checksums. Code signing needs a paid cert — not in place yet.

---

## Quick start

Put a Release binary (or a local build) next to `testdata/` and `config/`, then run with no args for the starter guide + `menu`.

```powershell
cd dnstt-kit
.\target\release\dns-cli.exe
# or: .\dnstt-kit-windows-x64.exe

.\dns-cli.cmd doctor
.\dns-cli.cmd serve --bind 127.0.0.1:8787
.\dns-cli.cmd menu
```

Web UI: http://127.0.0.1:8787  
Docker notes: [docs/DOCKER.md](docs/DOCKER.md)

Default profile name in samples is `demo` (domains under `*.example.com` — replace with yours in `config/profiles.json`).

---

## What’s in the box

- Scan with presets `low` / `normal` / `fast` (`low` targets ~512 MB / 1 core)
- Generate NetMod, DNSTT, SlipNet URI
- Slipnet offline-first (`vendor/slipnet/…`; fetch is opt-in)
- Web UI with local Tailwind Play CDN (`dns-cli/static/tailwindcss.js`)
- Backup / archive / clean + SQLite run history
- CI for Windows / Linux / macOS (+ aarch64)

---

## Security (web)

- Binds to localhost by default
- Optional `DNS_CLI_WEB_TOKEN`
- API masks absolute `work_dir`
- Relative paths only (`..` rejected)

Details: [SECURITY.md](SECURITY.md), [docs/SECURITY_WEB.md](docs/SECURITY_WEB.md)

---

## Docs

| File | Topic |
|------|--------|
| [docs/TUTORIAL.md](docs/TUTORIAL.md) | Windows walkthrough |
| [docs/CLI.md](docs/CLI.md) | Commands |
| [docs/WEB.md](docs/WEB.md) | Web panel |
| [docs/ENV.md](docs/ENV.md) | `.env` |
| [docs/DOCKER.md](docs/DOCKER.md) | Docker |
| [docs/MEMORY.md](docs/MEMORY.md) | Low-RAM preset |
| [docs/SLIPNET.md](docs/SLIPNET.md) | Slipnet |
| [LICENSE](LICENSE) | Non-commercial |

---

## Disclaimer

AS IS — personal / educational / research. No warranty. Commercial use not allowed; see [LICENSE](LICENSE).

Copyright © 2026 BTF Kabir
