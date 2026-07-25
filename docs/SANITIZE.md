# sanitize — پاکسازی فایل ورودی IP

> به‌روز: 2026-07-25

دستور `sanitize` فایل لیست IP (مثل `dnsir.txt`) را **تمیز + مرتب** می‌کند. جایگزین مناسب اسکریپت‌های پایтонی `remove_ips.py` / `sort_ip_*.py` است.

## عملیات

1. **حذف خطوط خالی** و کامنت (`#` / `//`)
2. **حذف فرمت نامعتبر** (غیر از `IP:port` یا `IP`)
3. **حذف تکراری‌ها** — بر اساس IP:port (پیش‌فرض) یا فقط IP (`--dedup-by-ip`)
4. **مرتب‌سازی عددی** — ابتدا IP سپس port (نه الفبایی)
5. **حذف exclude** — IPهای لیست سیاه (`--exclude`)

## استفاده

```powershell
# ساده‌ترین حالت — تمیز + مرتب + جایگزینی فایل اصلی
.\dns-cli.cmd sanitize dnsir.txt --inplace

# خروجی جداگانه (فایل اصلی دست‌نخورده)
.\dns-cli.cmd sanitize dnsir.txt

# فقط آمار (بدون نوشتن فایل)
.\dns-cli.cmd sanitize dnsir.txt --dry-run

# تکراری فقط بر اساس IP (پورت فرقی نکنه)
.\dns-cli.cmd sanitize dnsir.txt --inplace --dedup-by-ip

# حذف IPهای خاص
.\dns-cli.cmd sanitize dnsir.txt --inplace --exclude bad_ips.txt
```

## خروجی نمونه

```
📊 sanitize results:
   raw lines       : 455120
   blank removed   : 0
   comments removed: 0
   invalid removed : 0
   excluded removed: 0
   before dedup    : 455120
   duplicates      : 334704
   final unique    : 120416
   output          : dnsir.txt
```

## فلگ‌ها

| فلگ | توضیح |
|-----|--------|
| `<INPUT>` | فایل ورودی (اجباری) |
| `--output` | فایل خروجی (پیش‌فرض: `<input>.clean.txt`) |
| `--inplace` | جایگزینی فایل اصلی به‌جای نوشتن فایل جداگانه |
| `--dedup-by-ip` | تکراری فقط بر اساس IP (بدون پورت) |
| `--exclude` | فایل exclude (هر خط یک IP یا IP:port) |
| `--dry-run` | فقط چاپ آمار بدون نوشتن فایل |

## تفاوت با اسکریپت‌های پایتونی

| ویژگی | remove_ips.py / sort_ip.py | `dns-cli sanitize` |
|--------|---------------------------|---------------------|
| حذف تکراری | بله (ساده) | بله (IP:port یا فقط IP) |
| مرتب‌سازی | الفبایی یا ناقص | عددی واقعی (IPv4 octets) |
| حذف فرمت نامعتبر | خیر | بله |
| حذف کامنت | خیر | بله (`#` / `//`) |
| exclude list | خیر | بله (`--exclude`) |
| درجا نوشتن | خیر | بله (`--inplace`) |
| سرعت | کند (Python) | سریع (Rust) |

## پیشنهاد workflow

```powershell
# ۱. تمیز کن
.\dns-cli.cmd sanitize dnsir.txt --inplace

# ۲. اسکن کن
.\dns-cli.cmd pipeline run --input dnsir.txt --profile mytunnel --preset low
```
