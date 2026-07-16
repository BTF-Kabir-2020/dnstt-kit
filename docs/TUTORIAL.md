# آموزش گام‌به‌گام (ویندوز)

## Beginner (آسون‌ترین)

1. از [Releases](https://github.com/BTF-Kabir-2020/dnstt-kit/releases) فایل `dnstt-kit-windows-x64.exe` بگیر  
2. **Source code (zip)** را هم دانلود/extract کن (برای `testdata` و `config`)  
3. `.exe` را داخل همان پوشه بگذار  
4. فقط دوبار کلیک / بدون آرگومان اجرا کن → **menu** باز می‌شود  
5. یا: `dnstt-kit-windows-x64.exe serve` → http://127.0.0.1:8787  

## از سورس (بیلد)

> بعد از بیلد، `dns-cli` alone در PATH نیست مگر اضافه کنی. از راه‌های زیر استفاده کن.

## ۰) از پوشهٔ کیت شروع کن

```powershell
cd ...\dnstt-kit
```

## ۱) بیلد (یک‌بار)

```powershell
cargo build -p dns-cli --release
```

خروجی: `target\release\dns-cli.exe`

## ۲) اجرا

### راه ۱ — لانچر

```powershell
.\dns-cli.cmd
.\dns-cli.cmd doctor
.\dns-cli.cmd serve --bind 127.0.0.1:8787
.\dns-cli.cmd menu
```

### راه ۲ — مسیر کامل exe

```powershell
.\target\release\dns-cli.exe doctor
.\target\release\dns-cli.exe serve --bind 127.0.0.1:8787
```

### راه ۳ — alias موقت در همین نشست PowerShell

```powershell
Set-Alias dns-cli .\target\release\dns-cli.exe
dns-cli doctor
dns-cli serve --bind 127.0.0.1:8787
```

### راه ۴ — Python launcher

```powershell
python run.py serve --bind 127.0.0.1:8787
```

---

## ۳) خودآزمایی و اسکن

```powershell
.\dns-cli.cmd doctor
.\dns-cli.cmd scan testdata\dns_sample.txt --preset low --limit 3 --quiet
```

## ۴) تولید کانفیگ

```powershell
.\dns-cli.cmd generate all --profile demo --resolvers testdata\resolvers_sample.json --limit 10 --no-dmvpn
```

## ۵) پنل وب / منوی ترمینال

```powershell
.\dns-cli.cmd serve --bind 127.0.0.1:8787
# یا
.\dns-cli.cmd menu
```

زبانه‌ها: شروع · گزینه‌ها (فلگ‌های کامل) · راهنما · وضعیت  
اگر بنر زرد دیدی = work_dir اشتباه بوده؛ برنامه معمولاً خودش اصلاح می‌کند.

جزئیات: [WEB.md](WEB.md)

## ۶) init و .env

```powershell
.\dns-cli.cmd init
Copy-Item .env.example .env
.\dns-cli.cmd info
```

راهنما: [ENV.md](ENV.md)

## ۷) slipnet / pipeline

```powershell
.\dns-cli.cmd slipnet which
.\dns-cli.cmd pipeline run --input testdata\dns_sample.txt --profile demo --preset low --skip-slipnet
```

## ۸) کیفیت و تست

```powershell
.\scripts\quality.ps1
```

---

اگر باز هم `not recognized` دیدی: یا داخل `dns-cli\` (زیرپوشه) هستی، یا هنوز بیلد نزدی.
باید در **`dnstt-kit`** باشی و `.\dns-cli.cmd` یا `.\target\release\dns-cli.exe` بزنی.
