# کراس‌پلتفرم

یک باینری برای همهٔ پلتفرم‌ها کار نمی‌کند — بیلد per-target.

| هدف | مسیر |
|-----|------|
| Windows x64 | `dist/windows-x86_64/dns-cli.exe` |
| Linux x64 | `dist/linux-x86_64/dns-cli` |
| Linux aarch64 | CI artifact یا بیلد محلی Termux |

```powershell
.\scripts\build-release.ps1
.\scripts\build-release.ps1 -Cross -Lib
```

```bash
./scripts/build-release.sh --cross --lib
```

`run.py` باینری هم‌معماری را از `target/release` یا `dist/<triple>/` اجرا می‌کند.

جزئیات کامل: [BUILD.md](BUILD.md)
