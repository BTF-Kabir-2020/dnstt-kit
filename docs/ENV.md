# متغیرهای محیطی و فایل `.env`

این سند برای کسی است که کد را ننوشته — فقط می‌خواهد ابزار را راه بیندازد.

## این فایل اصلاً چیست؟

| فایل | نقش |
|------|-----|
| `.env.example` | نمونهٔ **امن برای git** — پر از توضیح و مقدار MOCK |
| `.env` | کپی شخصی تو — ممکن است رمز/کانفیگ واقعی داشته باشد (**gitignore**) |

`dns-cli` موقع اجرا `.env` را **خودکار می‌خواند** (اگر باشد).  
قبل از این اصلاح، فقط یک فایل خالی بود و اصلاً لود نمی‌شد — الان لود می‌شود.

> ویندوز: به‌جای `dns-cli` بنویس `.\dns-cli.cmd` (از پوشهٔ `dnstt-kit`).

## راه‌اندازی در ۳۰ ثانیه

```powershell
cd saaboon\dnstt-kit
Copy-Item .env.example .env
# اختیاری: notepad .env  و SLIPNET_CONFIG واقعی را بگذار
.\dns-cli.cmd info
.\dns-cli.cmd doctor
```

خروجی `info` چیزی شبیه این است:

```text
.env=present (...)
--- env vars (masked) ---
DNS_CLI_WORK_DIR=(unset)
SLIPNET_PATH=(unset)
SLIPNET_CONFIG=set(len=..., starts_with="slipnet://...")
DNS_CLI_BIND=(unset)
```

اگر `(unset)` دیدی یعنی متغیر ست نشده (طبیعی است اگر هنوز لازم نداری).

## جدول متغیرها

| متغیر | اجباری؟ | معنی |
|--------|---------|------|
| `DNS_CLI_WORK_DIR` | خیر | ریشهٔ پروژه؛ معادل `--work-dir` |
| `SLIPNET_PATH` | خیر | مسیر باینری slipnet؛ وگرنه از `vendor/...` پیدا می‌شود |
| `SLIPNET_CONFIG` | فقط برای **e2e واقعی** | لینک `slipnet://...` |
| `DNS_CLI_BIND` | خیر | آدرس `serve`؛ پیش‌فرض `127.0.0.1:8787` |
| `DNS_CLI_ENV_DEBUG` | خیر | اگر ست شود، مسیر فایل `.env` لود‌شده را چاپ می‌کند |

## اولویت‌ها (کدام برنده است؟)

1. فلگ CLI (مثلاً `--slipnet-config "..."`)
2. متغیر محیط سیستم‌عامل که خودت `export` / `$env:` کردی
3. مقدار داخل فایل `.env`
4. پیش‌فرض داخل برنامه

## MOCK داخل `.env.example` چیست؟

یک `slipnet://...` جعلی با دامنه `mock.example.invalid` و پسورد `MOCK_PASS_NOT_REAL`.

- برای **یادگیری فرمت** و تست اینکه `.env` لود می‌شود
- برای **اتصال واقعی به سرور کافی نیست**
- برای e2e واقعی: خروجی `generate slipnet-uri` یا کانفیگ سرور خودت را بگذار

## بدون `.env` هم کار می‌کند؟

بله. بیشتر کارها (scan / generate / doctor / backup / وب) بدون `.env` اجرا می‌شوند.

`.env` فقط برای راحتی است وقتی:
- نمی‌خواهی هر بار `SLIPNET_CONFIG` را دستی ست کنی
- یا مسیر work_dir / slipnet را ثابت می‌خواهی

## تست لود (خودت هم می‌توانی تکرار کنی)

```powershell
# یک .env موقت با MOCK
@"
SLIPNET_CONFIG=slipnet://MTh8ZG5zdHRfc3NofE1PQ0tfUFJPRklMRV9ET19OT1RfVVNFfG1vY2suZXhhbXBsZS5pbnZhbGlkfDEuMS4xLjE6NTM6MHwwfDUwMDB8YmJyfDEwODB8MTI3LjAuMC4xfDB8YWFiYmNjZGRlZWZmMDAxMTIyMzM0NDU1NjY3Nzg4OTlhYWJiY2NkZGVlZmYwMDExMjIzMzQ0NTU2Njc3ODg5OXx8fDF8cm9vdHxNT0NLX1BBU1NfTk9UX1JFQUx8MjJ8MHwxMjcuMC4wLjF8fHx1ZHB8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHx8fHw=
"@ | Set-Content -Encoding utf8 .env

.\dns-cli.cmd info
# باید SLIPNET_CONFIG=set(...) ببینی نه (unset)
```

## ارتباط با پروفایل‌ها

رمز SSH / pubkey داخل `config/profiles.json` است (نه `.env`).  
`.env` بیشتر برای مسیرها و `SLIPNET_CONFIG` است.

نمونه پروفایل: `config/profiles.example.json` → کپی به `profiles.json`.
