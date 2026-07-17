# Memory & CPU (large lists / ~512 MB / single core)

## Short answer

With **`--preset low`** (or `--stream`), peak RAM stays roughly **O(workers + unique working IPs)** — not O(file size). The scanner reads the input **line-by-line**, bounds in-flight work, and appends each result to disk.

That keeps small VPS hosts usable for **huge resolver lists**. It does **not** make a multi‑million scan fast on one core (still network-bound), and you still need free disk for `dns_all_results.txt`.

| Preset | Workers | Stream → disk | Use when |
|--------|---------|---------------|----------|
| **low** | 16 | yes | ~512 MB / 1 core, large lists |
| normal | 64 | no (collects in RAM) | desktop / small lists |
| fast | 128 | yes | strong host |

## How it stays light

- Input: `BufReader` line-by-line — no full-file load into a giant `Vec`
- Concurrency: bounded `JoinSet` (in-flight ≤ workers)
- CLI stream path: append to `dns_all_results.txt`; only working IPs kept in a `HashSet`
- `--limit N`: stops after N valid targets without loading the rest of the file
- `--quiet`: skips debug stdout **and** the debug log file (important for huge runs)

## Recommended commands

```powershell
.\dns-cli.cmd scan huge_resolvers.txt --preset low --quiet
.\dns-cli.cmd scan huge_resolvers.txt --preset low --limit 50000 --quiet
.\dns-cli.cmd scan huge_resolvers.txt --preset low -j 32 --quiet
.\dns-cli.cmd pipeline run --input huge_resolvers.txt --profile demo --preset low --skip-slipnet --quiet
```

## What still costs RAM / time

| Item | Notes |
|------|--------|
| Working IP `HashSet` | Grows with **successful** resolvers only |
| `dns_all_results.txt` | Disk growth with every scanned line |
| `run_scan` / FFI JSON | Still aggregates **all** results in memory — small lists only |
| `--preset normal` without `--stream` | Full results in RAM — avoid on huge files |
| Wall clock | Network-bound even when RAM stays flat |

## Small smoke check

```powershell
.\dns-cli.cmd scan testdata\dns_sample.txt --preset low --limit 5 --quiet
```

If you still OOM: stay on `--preset low` / `--stream`, lower `-j`, close other apps, give Docker ≥ 384 MB, and avoid FFI `run_scan` on huge lists.
