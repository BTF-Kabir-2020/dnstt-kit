# FFI و پایتون (همهٔ سیستم‌ها بدون بازنویسی)

## ایده

یک هستهٔ Rust (`scanner_core`) به‌صورت **DLL/SO/dylib** + باینری CLI برای هر OS.

```text
┌─────────────┐     ctypes / JNI      ┌──────────────────┐
│ Python/Java │ ───────────────────► │ scanner_core.dll │
└─────────────┘                        └──────────────────┘
┌─────────────┐     subprocess         ┌──────────────────┐
│ Python/UI   │ ───────────────────► │ dns-cli[.exe]    │
└─────────────┘                        └──────────────────┘
```

## بیلد lib

```powershell
cargo build -p scanner_core --release
.\scripts\build-release.ps1 -Lib
```

خروجی نمونه:
- Windows: `scanner_core.dll`
- Linux: `libscanner_core.so`
- macOS: `libscanner_core.dylib`

## پایتون

```powershell
python python\scanner_ffi.py testdata\dns_sample.txt
```

تابع: `scanner_run_from_file` / `scanner_free_string` (C ABI).

برای کل pipeline از پایتون، همان `dns-cli` را subprocess کن (یا `run.py`).

## GitHub Actions (بیلد برای همه)

`.github/workflows/ci.yml`:

| runner | خروجی |
|--------|--------|
| windows-latest | dns-cli.exe + dll |
| ubuntu-latest | dns-cli + .so |
| macos-latest | dns-cli + dylib |
| cross aarch64-gnu | dns-cli لینوکس ARM (Termux-friendly) |
| cross x86_64-musl | باینری استاتیک‌تر لینوکس |

Artifactها از Actions قابل دانلودند — نیازی به کامپایل روی همهٔ لپ‌تاپ‌ها نیست.

## توصیهٔ عملی

| هدف | راه |
|-----|-----|
| کاربر نهایی ویندوز/لینوکس/مک | artifact CI یا `build-release` همان OS |
| اندروید/جاوا | JNI روی `scanner_core` cdylib |
| اسکریپت پایتون سریع | `scanner_ffi.py` یا `run.py pipeline ...` |
| Termux | بیلد محلی aarch64 یا artifact aarch64-gnu |
