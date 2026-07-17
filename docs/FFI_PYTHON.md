# FFI / shared library (Python, JNI, Android)

`scanner_core` builds as a **cdylib** so other languages can call the scanner without rewriting it.

```text
Python / Java / Kotlin  --ctypes/JNI-->  scanner_core (.dll / .so / .dylib)
Python / UI             --subprocess-->  dns-cli
```

## Prebuilt (GitHub Releases)

Each tagged release ships:

| Asset | Platform |
|-------|----------|
| `dnstt-kit-scanner_core-windows-x64.dll` | Windows |
| `dnstt-kit-scanner_core-linux-x64.so` | Linux x64 |
| `dnstt-kit-scanner_core-linux-arm64.so` | Linux ARM64 |
| `dnstt-kit-scanner_core-macos-arm64.dylib` | macOS ARM |
| `dnstt-kit-scanner_core-android-arm64-v8a.so` | Android arm64 |
| `dnstt-kit-scanner_core-android-armeabi-v7a.so` | Android 32-bit ARM |

Rename locally if your loader expects `scanner_core.dll` / `libscanner_core.so`.

## Build locally

```powershell
cargo build -p scanner_core --release
.\scripts\build-release.ps1 -Lib
```

Outputs under `target/release/`:
- Windows: `scanner_core.dll`
- Linux: `libscanner_core.so`
- macOS: `libscanner_core.dylib`

Android (needs NDK + `cargo-ndk`):

```bash
cargo ndk -t arm64-v8a -t armeabi-v7a -o dist/android build -p scanner_core --release
```

## C ABI

- `scanner_run_from_file(path) -> *char` (JSON; caller frees)
- `scanner_free_string(ptr)`

> **Large lists:** FFI `scanner_run_from_file` uses `run_scan` and returns **one JSON blob of all results** — fine for small samples, not for multi‑million IP files. For huge lists use the CLI with `--preset low` / `--stream` (line-by-line + disk). See [MEMORY.md](MEMORY.md).

## Python

```powershell
python python\scanner_ffi.py testdata\dns_sample.txt
```

For a full pipeline from Python, subprocess `dns-cli` (or `run.py`).

## Practical map

| Goal | Path |
|------|------|
| Desktop end user | Release CLI binary |
| Python script | DLL/SO + `scanner_ffi.py`, or subprocess CLI |
| Android / Java | ship Android `.so` into `jniLibs/<abi>/`, JNI or similar |
| Termux | Linux ARM64 CLI or `.so` from Releases |
