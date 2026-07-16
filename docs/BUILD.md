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

۱) **GitHub Actions** — بهترین راه برای artifact هر OS: ببین `.github/workflows/ci.yml`  
۲) **DLL + Python** — `docs/FFI_PYTHON.md` + `python/scanner_ffi.py`  
۳) **run.py** — لانچر بدون منطق تکراری

جزئیات FFI: [FFI_PYTHON.md](FFI_PYTHON.md) · وب: [WEB.md](WEB.md)
