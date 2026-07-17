# CLI reference

> Windows: from `dnstt-kit` folder use `.\dns-cli.cmd ...` or Release `.\dnstt-kit-windows-x64.exe ...`

**No args** → starter guide + interactive **menu** (best for beginners).

```text
dns-cli [--work-dir DIR] [COMMAND]
```

## First steps

```text
dns-cli                 # guide + menu
dns-cli init
dns-cli doctor
dns-cli menu
dns-cli serve           # http://127.0.0.1:8787
dns-cli info
```

`.env`: copy from `.env.example` — [ENV.md](ENV.md)

## scan / resolvers / generate / pipeline

Flags: [OPTIONS.md](OPTIONS.md) · Large lists / low RAM: [MEMORY.md](MEMORY.md)

```text
dns-cli scan huge.txt --preset low --quiet
dns-cli scan huge.txt --preset low --limit 50000 --quiet
dns-cli resolvers exclude --input resolvers.json --exclude bad.txt
dns-cli pipeline run ... --auto-archive --auto-backup
```

## slipnet / archive / backup / clean

```text
dns-cli slipnet which|fetch|probe
dns-cli archive pack|restore|list
dns-cli backup create|list|restore|prune|watch
dns-cli clean [--runs-keep N] [--dry-run]
```

## profiles / status

```text
dns-cli profiles list|show
dns-cli status
```

Web panel: [WEB.md](WEB.md) · Tutorial: [TUTORIAL.md](TUTORIAL.md)
