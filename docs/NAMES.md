# Names & labels (spelling)

Operator/dev notes for casing in remarks and export folders.

## Remarks / `ps`

When a short person-name token appears in remarks, NetMod `ps`, or SlipNet display names, keep it ASCII uppercase (`BTF`). Use the constant `BTF_NAME` in code; probe with `BTF_NAME.to_ascii_lowercase()` — don’t hardcode alternate-case literals in tests.

`--profile` keys are ordinary nicknames (`demo`, `mytunnel`, …), not that person-name token.

## DMVPN

**DMVPN** is a first-class client export in this kit:

- Wire format: `sn://dnstt?…`
- Pipeline/generate folder: `configs/dmvpn/`
- Dated import bundle (default): `DMVPN/<timestamp>_<remark>/`

Spelling in paths/docs: `DMVPN` (`DMVPN_LABEL`). Opt out of the dated root bundle with `--no-dmvpn` (advanced). CLI: `generate dmvpn` (alias `generate dnstt`).

## Code

`dns-cli/src/names.rs` — `BTF_NAME`, `DMVPN_LABEL`, normalize helpers.

See also: [WORKFLOW.md](WORKFLOW.md), [CLIENTS.md](CLIENTS.md).
