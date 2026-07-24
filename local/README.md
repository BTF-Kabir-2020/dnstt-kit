# local/ (not published)

Put large resolver dumps and personal scratch files here. Everything under `local/` except this README is gitignored.

Suggested layout:

```text
local/lists/tokhmi.txt          # big UDP resolver candidates
local/lists/dnsir.txt           # optional copy of IR resolver dumps
local/lists/ok_after_scan.txt   # survivors from your last scan
local/notes/                    # personal notes — never commit secrets
local/VERIFY_SESSION_*.md       # private verify/debug writeups (gitignored)
```

Also allowed at kit root (gitignored): `dnsir.txt`, `dns_ok_and_dnsonly_ips.txt`, `resolvers.json`.

Public sample lists stay in `testdata/`. End-to-end usage: [docs/WORKFLOW.md](../docs/WORKFLOW.md). Scan labels: [docs/SCAN.md](../docs/SCAN.md).
