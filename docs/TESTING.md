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

## پوشش

| لایه | محتوا |
|------|--------|
| rustfmt | قالب اجباری در CI |
| clippy | `-D warnings` در CI |
| unit | Kryo / NetMod / SlipNet / verify / decode edge / resolvers |
| CLI | generate، doctor، verify file+URI، backup، env، completion، … |
| real DNS | اسکن عمومی + pipeline |
| stream / scale | `max_targets`، فایل ~20k خط خط‌به‌خط، CLI `--preset low` روی لیست بزرگ |
| release | `cargo build --release` + artifact Actions |

جزئیات کیفیت: `.\scripts\quality.ps1` (rustfmt + clippy + test)
