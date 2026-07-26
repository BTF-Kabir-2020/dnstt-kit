# Client compatibility

This kit **scans resolvers** and **builds import links**. It is not a VPN app.

| Client | Status in dnstt-kit | What we emit |
|--------|---------------------|--------------|
| **NetMod** | Supported | `dns://` + base64(JSON) |
| **DMVPN** | Supported | `sn://dnstt?…` under `configs/dmvpn/` + `per/*.sn` (one file per DNS) + root `DMVPN/<ts>_<batch>_…/` — Android client: [@irbitnet](https://t.me/irbitnet) |
| **SlipNet** | Supported | `slipnet://…` under `configs/slipnet/` + `per/*.slipnet` (one file per DNS) |
| **MasterDnsVPN** | Resolvers only | Different protocol. Export scan hits with `dns-cli resolvers export-txt` → `client_resolvers.txt`. Do **not** paste a DNSTT Noise pubkey as its encryption key |

## Batch labels (same-day runs)

Each `generate` / `pipeline` run mints a short **batch tag** (e.g. `K7HM`). Display names across NetMod / DMVPN / SlipNet share it:

```text
BTFJang891-K7HM-01
BTFJang891-K7HM-02
…
BTFJang891-K7HM-all   ← combined “all resolvers” link where applicable
```

So a second run the same day gets another tag (`P3WQ`, …) and lists do not collide. The tag is also in filenames / `*_info.json` / `*_links.json` (`"batch"`).

## Decode an existing link

```text
dns-cli decode "dns://...."
dns-cli decode "dns://...." --save-profile mytunnel
dns-cli verify "dns://...."
dns-cli verify "sn://dnstt?...."
```

Password is masked unless `--show-secrets`. Profiles land in `config/profiles.json` (gitignored).

Full operator flow (scan → SlipNet e2e → categorized configs): [WORKFLOW.md](WORKFLOW.md).

## Priority (maintainers)

1. Keep NetMod + DMVPN + SlipNet correct (phone/desktop imports).
2. Keep resolver scan solid (this is the unique value vs pure apps).
3. Extra client formats only when someone needs a concrete export — do not invent unsupported schemes.
