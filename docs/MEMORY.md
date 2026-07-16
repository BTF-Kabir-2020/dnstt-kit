# Memory & CPU (512 MB / single core)

## Answer

**Yes — designed to run on ~512 MB RAM and 1 CPU core** when you use preset **`low`** (default for web UI and recommended for VPS/nano).

| Preset | Workers | Stream | Target |
|--------|---------|--------|--------|
| **low** | 1 | yes | 512 MB / 1 core |
| normal | 64 | no | desktop |
| fast | 128 | stream | strong host |

## How it stays light

- `low` uses **streaming scan** (does not hold all results in one giant vec longer than needed)
- Web job queue: **one job at a time**
- Prefer `--limit N` for small batches
- Avoid `preset=fast` on tiny VMs
- Docker image can set `DNS_CLI_PRESET=low`

## Verify locally

```powershell
.\dns-cli.cmd scan testdata\dns_sample.txt --preset low --limit 5 --quiet
.\dns-cli.cmd pipeline run --input testdata\dns_sample.txt --profile demo --preset low --limit 5 --skip-slipnet --quiet
```

If you still OOM: lower `--limit`, close other apps, use Docker memory limit ≥ 384 MB for the container.
