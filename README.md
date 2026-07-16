# dnstt-kit (Rust) — v0.1.0

[![CI](https://github.com/BTF-Kabir-2020/dnstt-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/BTF-Kabir-2020/dnstt-kit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-Non--Commercial-blue)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)
[![Release](https://img.shields.io/github/v/release/BTF-Kabir-2020/dnstt-kit)](https://github.com/BTF-Kabir-2020/dnstt-kit/releases)

**EN:** Unified DNSTT toolkit — DNS resolver scan, config generate (NetMod / NekoBox / SlipNet), offline-first slipnet, SQLite web panel, backup, Docker.  
**FA:** ابزار یکپارچهٔ DNSTT — اسکن DNS، ساخت کانفیگ، slipnet آفلاین‌اول، پنل وب، بکاپ، داکر.

**Author:** [BTF Kabir](https://github.com/BTF-Kabir-2020) (`@BTF-Kabir-2020`)  
Related: [CS-1.6-Tool-v2](https://github.com/BTF-Kabir-2020/CS-1.6-Tool-v2)

> **Topics (GitHub About):** `rust` · `dnstt` · `dns` · `cli` · `docker` · `offline-first` · `web-ui` · `sqlite` — apply with [`scripts/github-seo.ps1`](scripts/github-seo.ps1)

**PRs welcome** — see [CONTRIBUTING.md](CONTRIBUTING.md). Let's grow this together.

---

## Downloads / دانلود

Prebuilt binaries: [Releases](https://github.com/BTF-Kabir-2020/dnstt-kit/releases)  
Asset names: `dnstt-kit-windows-x64.exe`, `dnstt-kit-linux-x64`, `dnstt-kit-linux-arm64`, `dnstt-kit-macos-arm64` (+ `LICENSE`, `README.md`, `SHA256SUMS.txt`).

### Antivirus false positives / آنتی‌ویروس

**EN:** Windows Defender and other AVs sometimes flag **unsigned** Rust CLI tools as suspicious (heuristic / “generic” malware). This project is open source under a non-commercial [LICENSE](LICENSE). Binaries are built by GitHub Actions from this repo — not packed with UPX or similar. If your AV quarantines `dns-cli.exe`:

1. Prefer `cargo build -p dns-cli --release` from this source tree.  
2. Or allowlist the file after checking `SHA256SUMS.txt` on the Release page.  
3. Code signing (Authenticode) requires a paid certificate; until then, unsigned builds are normal for indie projects.

**FA:** آنتی‌ویروس گاهی باینری **بدون امضای دیجیتال** را اشتباه ویروس می‌زند. این پروژه متن‌باز و غیرتجاری است. اگر Defender گیر داد: از سورس بساز، یا با چک‌سام ریلیز allowlist کن.

---

## Quick Start / شروع سریع

**Beginner (no CLI knowledge):** download a Release binary, put it inside the kit folder (with `testdata/` + `config/`), then run with **no arguments** → starter guide + interactive **menu**.

```powershell
cd dnstt-kit
# after cargo build:
.\target\release\dns-cli.exe
# or Release asset:
.\dnstt-kit-windows-x64.exe

.\dns-cli.cmd doctor
.\dns-cli.cmd serve --bind 127.0.0.1:8787
.\dns-cli.cmd menu
```

Browser: http://127.0.0.1:8787  

Docker: [docs/DOCKER.md](docs/DOCKER.md) · `docker compose up --build`

---

## Features / ویژگی‌ها

| Area | Status | Notes |
|------|--------|-------|
| Scan + presets `low/normal/fast` | ✅ | `low` = ~512 MB / 1 core |
| Pipeline + generate | ✅ | NetMod · DNSTT · SlipNet URI |
| Slipnet offline-first | ✅ | vendor / fetch optional |
| Web UI (Tailwind offline CDN) | ✅ | `static/tailwindcss.js` = cdn.tailwindcss.com |
| CLI + `menu` TUI-lite | ✅ | |
| Backup / archive / clean | ✅ | |
| SQLite run history | ✅ | |
| Docker + multi-OS CI | ✅ | win/linux/mac + aarch64 |
| UTF-8 / Unicode | ✅ | console CP 65001 on Windows |

---

## Security / امنیت وب

- Default bind **localhost**
- Optional `DNS_CLI_WEB_TOKEN`
- API **masks** `work_dir` (no full disk path)
- Relative paths only (`..` rejected)
- See [SECURITY.md](SECURITY.md) · [docs/SECURITY_WEB.md](docs/SECURITY_WEB.md)

---

## Low RAM / رم کم

Preset **`low`**: 1 worker + stream mode. Details: [docs/MEMORY.md](docs/MEMORY.md)

---

## Offline Tailwind (like Bootstrap file)

Local copy of the official Play CDN script — no internet, no Node build:

- `dns-cli/static/tailwindcss.js` ← `https://cdn.tailwindcss.com`
- loaded in HTML as `<script src="/tailwindcss.js">`

See [docs/WEB.md](docs/WEB.md) · [vendor/tailwind/README.md](vendor/tailwind/README.md)

---

## Docs

| File | Topic |
|------|--------|
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to PR fast |
| [docs/TUTORIAL.md](docs/TUTORIAL.md) | Windows walkthrough |
| [docs/WEB.md](docs/WEB.md) | Web panel + offline Tailwind |
| [docs/GITHUB_SEO.md](docs/GITHUB_SEO.md) | About + topics |
| [docs/DOCKER.md](docs/DOCKER.md) | Docker |
| [docs/MEMORY.md](docs/MEMORY.md) | 512 MB |
| [docs/ENV.md](docs/ENV.md) | `.env` |
| [docs/CLI.md](docs/CLI.md) | Commands |
| [SECURITY.md](SECURITY.md) | Security policy |
| [LICENSE](LICENSE) | Non-commercial |

---

## Disclaimer / سلب مسئولیت

> **EN:** Provided **AS IS** for personal / educational / research use. No warranty. Commercial use prohibited — see [LICENSE](LICENSE).

> **FA:** «همان‌طور که هست» برای استفاده شخصی/آموزشی. تضمینی نیست. استفاده تجاری ممنوع — [LICENSE](LICENSE).

**Copyright © 2026 BTF Kabir**
