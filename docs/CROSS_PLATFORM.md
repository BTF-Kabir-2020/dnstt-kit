# Cross-platform

CI and Releases build per OS/arch. One binary does not cover everything.

| Target | CLI asset | Shared lib |
|--------|-----------|------------|
| Windows x64 | `dnstt-kit-windows-x64.exe` | `…-scanner_core-windows-x64.dll` |
| Linux x64 | `dnstt-kit-linux-x64` | `…-scanner_core-linux-x64.so` |
| Linux ARM64 | `dnstt-kit-linux-arm64` | `…-scanner_core-linux-arm64.so` |
| macOS Apple Silicon | `dnstt-kit-macos-arm64` | `…-scanner_core-macos-arm64.dylib` |
| Android arm64-v8a | — | `…-scanner_core-android-arm64-v8a.so` |
| Android armeabi-v7a | — | `…-scanner_core-android-armeabi-v7a.so` |
| Linux musl x64 | `dnstt-kit-linux-x64-musl` (CI only) | — |

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
