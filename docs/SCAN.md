# Scan semantics (UDP resolver probe)

> Updated: 2026-07-24 · Aligned with the historical Amir/`scanner2` core (do not over-harden).

`dns-cli scan` only probes resolvers over UDP. It does **not** dial the tunnel and does **not** emit NetMod/DMVPN/SlipNet configs. For that use [`pipeline run`](WORKFLOW.md).

## Success rule (header)

Same as `scanner2` / its `docs/dns_query_hex.md`:

```text
RCODE == NOERROR (0)  AND  (ANCOUNT > 0  OR  NSCOUNT > 0)
```

- **Answer RRs** (`an > 0`) — normal A/TXT success.
- **Authority-only** (`an == 0`, `ns > 0`) — e.g. NODATA + SOA — still counts as a **meaningful reply**. Under filtered / intermittent networks this is common; treating it as hard FAIL drops usable resolvers.

Do **not** require Answer RRs for TXT alone. Ground truth for “tunnel works” is **SlipNet e2e**, not `txt_ok`.

## Status labels

| Status | Meaning | In working list? (default) |
|--------|---------|----------------------------|
| `OK` | A probe ok **and** at least one TXT probe ok (`txt_ok`) | Yes |
| `DNS_ONLY` | A probe ok, TXT did not | Yes (`include_dns_only`, unless `--ok-only`) |
| `FAIL` | A probe failed | No |

Pipeline / generate feed from the working list (OK + DNS_ONLY by default). e2e then filters further.

## Probes

1. **A** — `cloudflare.com` by default (`--a-domain`).
2. **TXT** — `--domain` first, then each `--extra-domains` entry; first success sets `txt_ok`.

### `--domain` vs extras

| Invocation | TXT domains probed |
|------------|--------------------|
| No `--domain` | Default extras (`cloudflare.com`, `example.com`) via preset |
| `--domain NS` only | **Only** `NS` (avoids soft false-OK when cloudflare/example TXT succeeds but tunnel TXT times out) |
| `--extra-domains a,b` | Explicit list (overrides the “domain-only” shortcut) |

Pipeline uses the profile’s `tunnel_domain` (and profile `extra_domains` if set; otherwise tunnel-only).

To mimic classic `scanner2` soft extras (e.g. google/youtube beside the tunnel), pass them yourself:

```powershell
.\dns-cli.cmd scan list.txt --preset low --domain YOUR.NS --extra-domains google.com,youtube.com --limit 50
```

## Balance (do not over-filter)

| Keep soft (scanner2) | Keep strict (operator accuracy) |
|----------------------|----------------------------------|
| Header `an \|\| ns` | `--domain` alone → no unrelated extras |
| `DNS_ONLY` in working list | e2e IPs from **this run** (`e2e_candidate_ips.txt`) |
| Intermittent timeouts → retry/backoff, not permanent ban | `scan` must not claim configs/e2e were built |

UDP hardening that stays: peer `connect`, TXID match, ignore QR≠1 (anti spoof) — not a status filter.

## Related

- Flow: [WORKFLOW.md](WORKFLOW.md)
- Flags: [OPTIONS.md](OPTIONS.md)
- e2e: [SLIPNET.md](SLIPNET.md)
- Large lists: [MEMORY.md](MEMORY.md)
