# Workflow: scan → SlipNet e2e → client configs

`scan` alone only probes UDP resolvers. Client configs and live tunnel checks need **`pipeline run`**.

Scan labels / NODATA+SOA / `--domain` extras: **[SCAN.md](SCAN.md)** (kept aligned with Amir/`scanner2` — do not over-harden the UDP success rule). **e2e** is the tunnel ground truth, not `txt_ok`.

```text
dnsir.txt (IP:port list)
        │
        ▼
   scan  ───────────────►  OK / DNS_ONLY list   (out/txt, runs/scan_*)
        │
        ▼
   pipeline  (or --skip-scan to reuse last OK list)
        │
        ├─ sync → resolvers.json
        ├─ SlipNet e2e (real DNSTT[+SSH] dial) → e2e_passed.txt
        └─ generate → configs/{netmod,dnstt,slipnet}/
```

| Step | Command | Output |
|------|---------|--------|
| 1. Save tunnel | `decode "dns://…" --save-profile mytunnel` | `config/profiles.json` (gitignored) |
| 2a. Probe only | `scan dnsir.txt --preset low --domain NS --limit 50` | `runs/scan_*/`, `out/txt/…` (OK + DNS_ONLY by default) |
| 2b. Full | `pipeline run --input dnsir.txt --profile mytunnel --preset low --limit 50` | scan + e2e + configs (+ `DMVPN/` bundle) |
| 2c. Continue | `pipeline run --input dnsir.txt --profile mytunnel --skip-scan` | e2e + configs from last working list (`out/json` / prior scan) |

`--skip-scan` with an **empty** `out/json` + `out/txt` fails (`resolvers list is empty`). A non-empty stale `out/` is reused on purpose (last working list) — prefer a fresh scan or an explicit resolvers path when unsure.

## Resolver list (`dnsir.txt`)

Copy large dumps into the kit root or `local/lists/` (both gitignored patterns). Example path after copy from another tree:

```text
dnstt-kit/dnsir.txt          ← gitignored (`dnsir.txt` in .gitignore)
```

## SlipNet e2e

- Needs vendor `slipnet` (`dns-cli slipnet which`).
- Config: `--slipnet-config "slipnet://…"` **or** `SLIPNET_CONFIG` in `.env` **or** (default) auto-built from the **profile**.
- IPs for e2e always come from this run’s `resolvers.json` (`runs/pipeline_*/e2e_candidate_ips.txt`), not a stale root list.
- Skip live dial: `--skip-slipnet` (still generates configs from the scan working list: OK + DNS_ONLY).
- A resolver can be `DNS_ONLY` (tunnel TXT timed out) and still pass e2e — that is expected under filtering.

## Config folders

After a successful pipeline:

```text
runs/pipeline_<id>/configs/
  netmod/    dns://         → NetMod
  dmvpn/     sn://dnstt?…   → DMVPN
  slipnet/   slipnet://     → SlipNet

Also (by default): DMVPN/<timestamp>_<batch>_<remark>/  → same sn:// links as a dated import bundle
```

Display names share one **batch tag** per generate (e.g. `Remark-K7HM-01`) — see [CLIENTS.md](CLIENTS.md).

If e2e ran and found survivors, generate uses **only** `e2e_passed` IPs.

## Windows launcher

`.\dns-cli.cmd` picks the **newest** of `target\release`, `target\debug`, and `dist\…` so an old release build does not hide a newer debug binary that has `decode`.
