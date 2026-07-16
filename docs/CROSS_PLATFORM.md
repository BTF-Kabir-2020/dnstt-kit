# Cross-platform

CI and Releases build per OS/arch. One binary does not cover everything.

| Target | Release asset | Notes |
|--------|---------------|--------|
| Windows x64 | `dnstt-kit-windows-x64.exe` | |
| Linux x64 | `dnstt-kit-linux-x64` | |
| Linux ARM64 | `dnstt-kit-linux-arm64` | cross in CI |
| macOS Apple Silicon | `dnstt-kit-macos-arm64` | `macos-latest` |
| Linux musl x64 | `dnstt-kit-linux-x64-musl` | CI only, best-effort |

## Slipnet

CI/Release never downloads slipnet. Drop binaries here if you need them:

```text
vendor/slipnet/windows-x86_64/slipnet.exe
vendor/slipnet/linux-x86_64/slipnet
vendor/slipnet/linux-aarch64/slipnet
```

macOS has no mapped upstream asset in this kit — use `--skip-slipnet` or set `SLIPNET_PATH`.

```bash
dns-cli pipeline run --skip-slipnet ...
dns-cli slipnet which
# dns-cli slipnet fetch   # only if you want a network download
```

Local builds: [BUILD.md](BUILD.md), `scripts/build-release.ps1` / `.sh`.
