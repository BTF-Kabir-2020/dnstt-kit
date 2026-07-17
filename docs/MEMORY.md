# Memory & CPU (large lists / 512 MB / single core)

## Answer

**Yes for large resolver lists** when you use **`--preset low`** (or `--stream`): the scanner reads the input **line-by-line**, keeps only ~`workers` tasks in flight, and writes each result to disk as it arrives. Peak RAM is roughly **O(workers + unique working IPs)**, not O(file size).

| Preset | Workers | Stream → disk | Target host |
|--------|---------|---------------|-------------|
| **low** | 16 | yes | ~512 MB / 1 core, multi‑million line lists |
| normal | 64 | no (collects in RAM) | desktop / small lists |
| fast | 128 | yes | strong host |

## What changed (true streaming)

- Input: `BufReader` line-by-line — **no** full-file `read_to_string` into a giant `Vec`
- Concurrency: bounded `JoinSet` (in-flight ≤ workers) — **no** `Vec` of millions of task handles
- CLI stream path: append to `dns_all_results.txt` while scanning; only working IPs kept in a `HashSet`
- `--limit N`: stops after N valid targets **without** loading the rest of the file
- `--quiet`: skips debug stdout **and** debug log file (important for huge runs)

## Recommended commands

```powershell
# Low-RAM / VPS / multi-million IP list
.\dns-cli.cmd scan huge_resolvers.txt --preset low --quiet

# Cap work without slicing the file yourself
.\dns-cli.cmd scan huge_resolvers.txt --preset low --limit 50000 --quiet

# Override workers if the host can take more I/O concurrency
.\dns-cli.cmd scan huge_resolvers.txt --preset low -j 32 --quiet
```

Pipeline:

```powershell
.\dns-cli.cmd pipeline run --input huge_resolvers.txt --profile demo --preset low --skip-slipnet --quiet
```

## What still costs RAM / time

| Item | Notes |
|------|--------|
| Working IP `HashSet` | Grows with **successful** resolvers only (usually ≪ total lines) |
| `dns_all_results.txt` | Disk, not RAM — can be large; ensure free disk |
| `run_scan` / FFI JSON | Still aggregates **all** results in memory — use for small lists only |
| `--preset normal` without `--stream` | Collects full results in RAM — avoid on huge files |
| Wall clock | Network-bound; 10M targets on 1 core still takes a long time even if RAM stays flat |

## Verify locally

```powershell
.\dns-cli.cmd scan testdata\dns_sample.txt --preset low --limit 5 --quiet
.\dns-cli.cmd pipeline run --input testdata\dns_sample.txt --profile demo --preset low --limit 5 --skip-slipnet --quiet
```

If you still OOM: prefer `--preset low` / `--stream`, lower `-j`, close other apps, give Docker ≥ 384 MB, and avoid FFI `run_scan` on multi‑million lists.
