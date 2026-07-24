# تست‌ها و کیفیت

```powershell
.\scripts\quality.ps1
# یا جدا:
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\scripts\smoke.ps1
.\scripts\build-release.ps1
```

جریان عملیاتی کاربر (scan / pipeline / e2e): [WORKFLOW.md](WORKFLOW.md)  
قانون موفقیت UDP / تعادل با `scanner2`: [SCAN.md](SCAN.md)

## پوشش

| لایه | محتوا |
|------|--------|
| rustfmt | قالب اجباری در CI |
| clippy | `-D warnings` در CI |
| unit | Kryo / NetMod / SlipNet / verify / decode edge / resolvers / `dns_udp_header_ok` (= scanner2) |
| CLI | generate، doctor، verify file+URI، backup، env، completion، … |
| real DNS | اسکن عمومی + pipeline |
| stream / scale | `max_targets`، فایل ~20k خط خط‌به‌خط، CLI `--preset low` روی لیست بزرگ |
| release | `cargo build --release` + artifact Actions |

رگرسیون مهم اسکنر: `NOERROR` + فقط Authority (NODATA+SOA) باید **موفق** بماند — سخت‌تر کردن به `an>0` فقط برای TXT، از مرجع منحرف می‌شود و زیر فیلتر رزولور زنده را می‌کشد.

جزئیات کیفیت: `.\scripts\quality.ps1` (rustfmt + clippy + test)
