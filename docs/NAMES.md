# Names & labels (spelling)

## **BTF** — person’s name (not a brand)

**BTF** is a **human person’s name**. It is **not** a product brand.

In user-facing text (remarks, NetMod `ps`, SlipNet display names, docs) those three letters are always ASCII uppercase: `BTF`.

| Surface | Rule |
|---------|------|
| Remarks / NetMod `ps` / SlipNet names / docs | Always `BTF` when that person-name appears |
| Source / tests | Do **not** embed a lowercased spelling of those three letters as a string literal; use `BTF_NAME.to_ascii_lowercase()` when a probe is required |
| **`--profile` keys** | Ordinary tunnel nicknames (`demo`, `zenadartabestan`, …). **Do not** use the person’s name as the profile id |

Example: remark / `ps` can be `BTFJang891` (from the server link); profile key should be something like `zenadartabestan` (from the tunnel NS), not `BTFJang`.

## **DMVPN** — client app (like SlipNet)

**DMVPN** is a **filter-bypass / tunnel client** (same class of tool as SlipNet / NetMod), not a random folder nickname.

This kit can emit an import bundle under `DMVPN/` for that app. Spelling in paths and docs is always **`DMVPN`** (ASCII uppercase).

| Surface | Rule |
|---------|------|
| App name / docs / log lines | Always `DMVPN` |
| Kit export folder | `DMVPN/` (`DMVPN_LABEL`) |
| Source / tests | No lowercased display-string literal; probe via `DMVPN_LABEL.to_ascii_lowercase()` |
| CLI flag | `--no-dmvpn` (clap kebab-case) = skip writing the **DMVPN** export bundle |

## Code

`dns-cli/src/names.rs` — `BTF_NAME`, `DMVPN_LABEL`, and the normalize helpers.

Operator flow: [WORKFLOW.md](WORKFLOW.md). Clients: [CLIENTS.md](CLIENTS.md).
