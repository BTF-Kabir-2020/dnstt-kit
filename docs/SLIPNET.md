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

Needs vendor slipnet (`slipnet which`). Config resolution order:

1. `--slipnet-config "slipnet://…"`
2. `SLIPNET_CONFIG` / `.env`
3. **Auto from pipeline `--profile`** (built with `generate` SlipNet URI helpers)

IPs always come from the run’s synced `resolvers.json` (`e2e_candidate_ips.txt` under the pipeline run dir), not a leftover root `dns_ok_and_dnsonly_ips.txt`.

Candidates often include **DNS_ONLY** rows (A ok, tunnel TXT timed out). That is fine — e2e decides tunnel usability. Do not tighten the scan header rule to “prove” the tunnel; see [SCAN.md](SCAN.md).

```powershell
Copy-Item .env.example .env   # optional; profile auto-config is enough after decode --save-profile
.\dns-cli.cmd pipeline run --input testdata\dns_sample.txt --profile demo --preset low --limit 3
```

Without a live dial: `.\dns-cli.cmd slipnet probe` or `pipeline … --skip-slipnet` / `--slipnet-probe`.

Full flow: [WORKFLOW.md](WORKFLOW.md).
## تولید URI

```powershell
.\dns-cli.cmd generate slipnet-uri --profile demo --resolvers resolvers.json
```

خروجی: `slipnet_all.txt` / `slipnet_per.txt` / `slipnet_links.json` + `per/*.slipnet` (یک فایل مجزا برای هر DNS)
