# بکاپ

> ویندوز: `.\dns-cli.cmd` به‌جای `dns-cli` (از پوشهٔ `dnstt-kit`).

## دستی

```powershell
# فقط سورس kit (بدون target، بدون secrets)
.\dns-cli.cmd backup create --mode kit --keep 20

# sqlite + archives
.\dns-cli.cmd backup create --mode data --keep 10

# هر دو + اختیاری runs/vendor/secrets
.\dns-cli.cmd backup create --mode full --include-runs --include-vendor --include-secrets --label crisis

.\dns-cli.cmd backup list
.\dns-cli.cmd backup restore dnstt_kit_YYYYMMDD_HHMMSS.zip
.\dns-cli.cmd backup prune --keep 10
```

خروجی در `backups/` با فایل `.sha256`.

`restore` محتوا را در `_restore_tmp/` می‌ریزد تا overwrite ناخواسته نشود.

## خودکار

### بعد از pipeline

```powershell
.\dns-cli.cmd pipeline run --input testdata\dns_sample.txt --profile demo --preset low --skip-slipnet --auto-archive --auto-backup
```

### حلقهٔ watch

```powershell
.\dns-cli.cmd backup watch --mode kit --interval 3600 --keep 20
```

### Windows Task Scheduler

```powershell
powershell -File scripts\backup-scheduled.ps1 -Mode kit -Keep 20
```

یا با `-IncludeSecrets` / `-IncludeRuns`.

### Linux cron

```bash
0 */6 * * * cd /path/dnstt-kit && ./target/release/dns-cli backup create --mode kit --keep 20 --label cron
```
