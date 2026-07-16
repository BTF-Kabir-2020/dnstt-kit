# slipnet

> ویندوز: `.\dns-cli.cmd` به‌جای `dns-cli` (از پوشهٔ `dnstt-kit`).

## آفلاین‌اول

1. `--slipnet PATH`
2. `SLIPNET_PATH`
3. کنار `dns-cli`
4. `vendor/slipnet/<platform>/slipnet(.exe)`

## دانلود اختیاری از GitHub

```powershell
.\dns-cli.cmd slipnet fetch --tag v2.5.3
# یا داخل pipeline:
.\dns-cli.cmd pipeline run ... --fetch-slipnet
```

- Repo: `anonvector/SlipNet`
- Tag پیش‌فرض CLI دسکتاپ: **v2.5.3** (v2.5.5 فقط APK دارد)
- Assetها: `slipnet-windows-amd64.exe` / `slipnet-linux-amd64` / `slipnet-linux-arm64`
- اگر فایل محلی سالم باشد، بدون `--force` دوباره دانلود نمی‌شود

## e2e

نیاز به `--slipnet-config` یا متغیر/` .env ` با `SLIPNET_CONFIG` دارد.

```powershell
Copy-Item .env.example .env
# SLIPNET_CONFIG را با لینک واقعی پر کن — راهنما: docs/ENV.md
.\dns-cli.cmd pipeline run --input testdata\dns_sample.txt --profile mame --preset low
```

بدون کانفیگ واقعی: `.\dns-cli.cmd slipnet probe` یا `--slipnet-probe`.

## تولید URI

```powershell
.\dns-cli.cmd generate slipnet-uri --profile mame --resolvers resolvers.json
```

خروجی: `slipnet_all.txt` / `slipnet_per.txt` / `slipnet_links.json`
