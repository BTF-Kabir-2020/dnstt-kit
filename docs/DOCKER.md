# Docker

Multi-stage image: Rust build → `debian:bookworm-slim`. UI assets are compiled into `dns-cli`. Slipnet is not in the image.

Host port is bound to `127.0.0.1` only. Compose runs as root so a bind-mount of `./` stays writable for `runs/` / `backups/`.

```bash
docker build -t dnstt-kit:local .

docker run --rm --user 0:0 -v "$PWD:/work" -w /work dnstt-kit:local doctor

docker run --rm --user 0:0 -p 127.0.0.1:8787:8787 -v "$PWD:/work" -w /work \
  -e DNS_CLI_WEB_TOKEN=change-me \
  dnstt-kit:local serve --bind 0.0.0.0:8787
```

```bash
docker compose up --build
# http://127.0.0.1:8787
```

## Without Docker

Windows: `.\scripts\build-release.ps1` then `.\dns-cli.cmd serve --bind 127.0.0.1:8787`  
Linux: `./scripts/build-release.sh` then `./target/release/dns-cli serve --bind 127.0.0.1:8787`

CI builds the multi-OS binaries — see `.github/workflows/`.
