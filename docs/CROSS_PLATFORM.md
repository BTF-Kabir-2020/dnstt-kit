# Cross-platform

One binary does not cover all OS/arch — build per target (CI + Release do this).

| Target | Release asset | Notes |
|--------|---------------|--------|
| Windows x64 | `dnstt-kit-windows-x64.exe` | Release + CI |
| Linux x64 | `dnstt-kit-linux-x64` | Release + CI |
| Linux ARM64 | `dnstt-kit-linux-arm64` | Release + CI cross |
| macOS Apple Silicon | `dnstt-kit-macos-arm64` | `macos-latest` |
| Linux musl x64 | `dnstt-kit-linux-x64-musl` | CI best-effort only |

## slipnet (optional)

CI/Release **never** download slipnet. Layout (offline):

```text
vendor/slipnet/windows-x86_64/slipnet.exe
vendor/slipnet/linux-x86_64/slipnet
vendor/slipnet/linux-aarch64/slipnet
```

macOS: no upstream asset mapping in kit today — use `--skip-slipnet` or supply `SLIPNET_PATH`.

```bash
dns-cli pipeline run --skip-slipnet ...
dns-cli slipnet which   # local only
# dns-cli slipnet fetch # only when YOU want network download
```

Local scripts: [BUILD.md](BUILD.md) · `scripts/build-release.ps1` / `.sh`
