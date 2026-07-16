# dnstt-kit (Rust) — v0.1.0

[![CI](https://github.com/BTF-Kabir-2020/dnstt-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/BTF-Kabir-2020/dnstt-kit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Non--Commercial-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![Release](https://img.shields.io/github/v/release/BTF-Kabir-2020/dnstt-kit)](https://github.com/BTF-Kabir-2020/dnstt-kit/releases)

DNSTT toolkit: scan DNS resolvers, generate NetMod / NekoBox / SlipNet configs, optional offline slipnet, localhost web UI, backup helpers, Docker.

Author: [BTF Kabir](https://github.com/BTF-Kabir-2020) · also: [CS-1.6-Tool-v2](https://github.com/BTF-Kabir-2020/CS-1.6-Tool-v2)

**PRs welcome** — humans helping with code, docs, and tests are encouraged. See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Downloads

Prebuilt: [Releases](https://github.com/BTF-Kabir-2020/dnstt-kit/releases)

### CLI

| Asset | Platform |
|-------|----------|
| `dnstt-kit-windows-x64.exe` | Windows |
| `dnstt-kit-linux-x64` | Linux x64 |
| `dnstt-kit-linux-arm64` | Linux ARM64 |
| `dnstt-kit-macos-arm64` | macOS Apple Silicon |

### Shared library (FFI — Python / JNI / Android)

| Asset | Platform |
|-------|----------|
| `dnstt-kit-scanner_core-windows-x64.dll` | Windows |
| `dnstt-kit-scanner_core-linux-x64.so` | Linux x64 |
| `dnstt-kit-scanner_core-linux-arm64.so` | Linux ARM64 |
| `dnstt-kit-scanner_core-macos-arm64.dylib` | macOS |
| `dnstt-kit-scanner_core-android-arm64-v8a.so` | Android arm64 |
| `dnstt-kit-scanner_core-android-armeabi-v7a.so` | Android armeabi-v7a |

Plus `LICENSE`, `README.md`, `SHA256SUMS.txt`. Details: [docs/FFI_PYTHON.md](docs/FFI_PYTHON.md).

### Antivirus note

Unsigned Rust CLIs get heuristic hits sometimes (Defender “generic”). Builds are plain GitHub Actions artifacts, no UPX. If something quarantines the exe: build with `cargo build -p dns-cli --release`, or allowlist after checking the release checksums.

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
Docker: [docs/DOCKER.md](docs/DOCKER.md)

Default sample profile is `demo` (`*.example.com` — edit `config/profiles.json`).

---

## What’s in the box

- Scan with presets `low` / `normal` / `fast` (`low` ≈ 512 MB / 1 core)
- Generate NetMod, DNSTT, SlipNet URI
- Slipnet offline-first (`vendor/slipnet/…`; fetch is opt-in)
- Web UI with local Tailwind Play CDN (`dns-cli/static/tailwindcss.js`)
- `scanner_core` shared lib for embedding (desktop + Android `.so`)
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
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to PR |
| [docs/TUTORIAL.md](docs/TUTORIAL.md) | Windows walkthrough |
| [docs/CLI.md](docs/CLI.md) | Commands |
| [docs/WEB.md](docs/WEB.md) | Web panel |
| [docs/FFI_PYTHON.md](docs/FFI_PYTHON.md) | DLL / SO / Android FFI |
| [docs/ENV.md](docs/ENV.md) | `.env` |
| [docs/DOCKER.md](docs/DOCKER.md) | Docker |
| [LICENSE](LICENSE) | Non-commercial |

---

## Disclaimer

AS IS — personal / educational / research. No warranty. Commercial use not allowed; see [LICENSE](LICENSE).

Copyright © 2026 BTF Kabir
