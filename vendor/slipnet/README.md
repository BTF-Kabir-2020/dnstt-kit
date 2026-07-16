# slipnet vendor (offline-first)

Place platform binaries here. **No automatic GitHub download** by default.

```
vendor/slipnet/
  windows-x86_64/slipnet.exe
  linux-x86_64/slipnet
  linux-aarch64/slipnet
```

Override: `--slipnet PATH` or env `SLIPNET_PATH`.

Check discovery:

```bash
dns-cli slipnet which
```
