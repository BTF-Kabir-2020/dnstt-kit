# گزینه‌ها و قابلیت‌های اضافه‌شده (changelog عملیاتی)

> به‌روز: 2026-07-17

## دستورات جدید

| دستور | کار |
|--------|-----|
| `doctor` | سلامت محیط (پروفایل / slipnet / sqlite / testdata) |
| `verify <file>` | decode و اعتبارسنجی لینک‌های تولیدشده |
| `profiles list\|show` | لیست / نمایش امن پروفایل |
| `archive restore` | برگرداندن ZIP آرشیو به `runs/` |
| `resolvers sort\|take\|shuffle\|merge` | مدیریت لیست IP |

## فلگ‌های اختیاری جدید

| محل | فلگ |
|-----|------|
| scan | `--limit` (خط‌به‌خط) `--stream` `--ok-only` `--enable-tcp` `--quiet` `--no-legacy-out` |
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

`--auto-archive` · `--auto-backup` · `--slipnet-probe`


## تست‌های مرتبط

- unit: Kryo / NetMod / SlipNet URI / verify / resolvers sort
- CLI: doctor, profiles, verify, resolvers sort/take, pipeline dry-run, generate `--limit`
- stream scale: `scanner_core` `max_targets` + فایل بزرگ خط‌به‌خط؛ CLI `--preset low`
- real DNS: `tests/real_dns.rs` + smoke دستی گزینه‌ها

لیست‌های میلیون‌تایی / رم کم: [MEMORY.md](MEMORY.md)

```powershell
cargo test -p dns-cli
.\target\debug\dns-cli.exe doctor
.\target\debug\dns-cli.exe scan testdata\dns_sample.txt --preset low --limit 3 --quiet
.\target\debug\dns-cli.exe generate all --resolvers testdata\resolvers_sample.json --limit 2 --no-dmvpn --out-dir runs\tmp_gen
.\target\debug\dns-cli.exe verify runs\tmp_gen\netmod\...
```
