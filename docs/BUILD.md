# بیلدها و توزیع

## بدون warning / error (اجباری مثل CI)

```powershell
.\scripts\quality.ps1
# معادل:
#   cargo fmt --all -- --check
#   cargo clippy --workspace --all-targets -- -D warnings
#   cargo test --workspace
```

کیفیت: `.\scripts\quality.ps1` یا job `quality` در GitHub Actions.

## native

```powershell
.\scripts\build-release.ps1
.\scripts\build-release.ps1 -Lib
.\scripts\build-release.ps1 -Cross   # تلاش targetهای دیگر روی همین ماشین
```

```bash
./scripts/build-release.sh --lib --cross
```

خروجی: `dist/<triple>/`

## چندزبانه / همه سیستم‌ها

۱) **GitHub Releases** — CLI + DLL/SO/dylib + Android `.so` روی هر تگ `v*`  
۲) **CI artifacts** — `.github/workflows/ci.yml`  
۳) **DLL + Python** — [FFI_PYTHON.md](FFI_PYTHON.md) + `python/scanner_ffi.py`  
۴) **run.py** — لانچر محلی

وب: [WEB.md](WEB.md)
