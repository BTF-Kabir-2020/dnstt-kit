# گزینه‌ها و قابلیت‌های اضافه‌شده (changelog عملیاتی)

> به‌روز: 2026-07-24

معنی وضعیت اسکن / قانون هدر DNS / `--domain` و extras: **[SCAN.md](SCAN.md)** (سینک با `scanner2`: `NOERROR && (an>0 || ns>0)`؛ NODATA+SOA را FAIL نکن).

## دستورات جدید

| دستور | کار |
|--------|-----|
| `sanitize` | پاکسازی فایل ورودی IP: حذف تکراری، مرتب‌سازی عددی، حذف خطوط خالی/کامنت/فرمت نامعتبر |
| `doctor` | سلامت محیط (پروفایل / slipnet / sqlite / testdata) |
| `decode <uri>` | decode لینک `dns://` / `slipnet://` / `sn://dnstt?` (+ `--save-profile` برای dns/slipnet) |
| `verify <file\|uri>` | اعتبارسنجی فایل لینک‌ها **یا** یک URI اینلاین (`dns://` / `slipnet://` / `sn://dnstt?`) — در ویندوز URI را کوت کنید |
| `profiles list\|show` | لیست / نمایش امن پروفایل |
| `archive restore` | برگرداندن ZIP آرشیو به `runs/` |
| `resolvers sort\|take\|shuffle\|merge` | مدیریت لیست IP |

## فلگ‌های اختیاری جدید

| محل | فلگ |
|-----|------|
| scan | `--limit` (خط‌به‌خط) `--stream` `--ok-only` `--enable-tcp` `--quiet` `--no-legacy-out` `--domain` `--a-domain` `--extra-domains` |
| resolvers sync | `--limit` |
| generate * | `--limit` `--no-dmvpn` `--ns` `--pubkey` `--remark` |
| pipeline | `--limit` `-j` `--dry-run` `--no-dmvpn` `--generate-kinds` `--quiet` |

## دستورات جدید (نسخه کامل)

| دستور | کار |
|--------|-----|
| `init` | ساخت پوشه‌ها + profiles از نمونه |
| `backup *` | بکاپ kit/data/full + watch |
| `clean` | prune runs/archives/backups/logs |
| `info` | مسیرها و نسخه |
| `completion` | autocomplete شل |
| `slipnet probe` | تست اجرای باینری بدون e2e |

## فلگ‌های pipeline

`--auto-archive` · `--auto-backup` · `--slipnet-probe` · `--skip-scan` · `--skip-slipnet` · `--no-dmvpn` (opt-out of the default **DMVPN** bundle)

- اگر `SLIPNET_CONFIG` خالی باشد، e2e از `--profile` یک `slipnet://` می‌سازد.
- لیست IP برای e2e همیشه از `resolvers` همان run است (`e2e_candidate_ips.txt`)، نه فایل کهنهٔ ریشه.
- با `--domain NS` و بدون `--extra-domains`، فقط همان NS برای TXT پروب می‌شود (false-OK از cloudflare/example نه).
- جریان کامل: [WORKFLOW.md](WORKFLOW.md) · اسکن: [SCAN.md](SCAN.md)


## تست‌های مرتبط

- unit: Kryo / NetMod / SlipNet URI / verify / decode edge (scheme/b64) / resolvers sort
- CLI: doctor, profiles, verify file **and** inline URI, resolvers sort/take, pipeline dry-run, generate `--limit`
- stream scale: `scanner_core` `max_targets` + فایل بزرگ خط‌به‌خط؛ CLI `--preset low`
- real DNS: `tests/real_dns.rs` + smoke دستی گزینه‌ها
- UDP scan: peer `connect` + TXID/QR match (کاهش پاسخ تزریقی از منبع اشتباه)

لیست‌های میلیون‌تایی / رم کم: [MEMORY.md](MEMORY.md)

```powershell
cargo test -p dns-cli
.\target\debug\dns-cli.exe doctor
.\target\debug\dns-cli.exe scan testdata\dns_sample.txt --preset low --limit 3 --quiet
.\target\debug\dns-cli.exe generate all --resolvers testdata\resolvers_sample.json --limit 2 --out-dir runs\tmp_gen
.\target\debug\dns-cli.exe verify runs\tmp_gen\netmod\....txt
.\target\debug\dns-cli.exe verify "dns://...."
```
