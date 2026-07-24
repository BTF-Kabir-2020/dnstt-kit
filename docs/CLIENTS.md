# Client compatibility

This kit **scans resolvers** and **builds import links**. It is not a VPN app and does not replace SlipNet / NetMod / MasterDnsVPN.

| Client | Status in dnstt-kit | What we emit |
|--------|---------------------|--------------|
| **NetMod** | Supported | `dns://` + base64(JSON) |
| **NekoBox / sn** | Supported | `sn://dnstt?…` |
| **SlipNet** | Supported | `slipnet://…` (+ optional local `slipnet` binary) |
| **VayDNS** | Via SlipNet | Use SlipNet tunnel types `vaydns` / `vaydns_ssh` on the app/server side; this kit’s default URI is classic `dnstt` / `dnstt_ssh` |
| **MasterDnsVPN** | Resolvers only | Different protocol (TOML + shared encrypt key). Export scan hits with `dns-cli resolvers export-txt` → `client_resolvers.txt`. Do **not** paste a DNSTT Noise pubkey into MasterDnsVPN as its encryption key |

## Decode an existing link

```text
dns-cli decode "dns://...."
dns-cli decode "dns://...." --save-profile mytunnel
dns-cli verify "dns://...."
```

Password is masked unless `--show-secrets`. Profiles land in `config/profiles.json` (gitignored).

## Priority (maintainers)

1. Keep NetMod + SlipNet + NekoBox correct (most phone users).
2. Keep resolver scan solid (this is the unique value vs pure apps).
3. MasterDnsVPN / VayDNS-native URI: only if someone needs a concrete export format — do not pretend they are the same wire protocol as DNSTT.
