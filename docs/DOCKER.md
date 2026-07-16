# Docker & multi-OS

## Docker (Linux container)

```bash
# build
docker build -t dnstt-kit:local .

# CLI one-shot
docker run --rm -v "$PWD:/work" -w /work dnstt-kit:local doctor

# web UI (localhost only on host)
docker run --rm -p 127.0.0.1:8787:8787 -v "$PWD:/work" -w /work \
  -e DNS_CLI_WEB_TOKEN=change-me \
  dnstt-kit:local serve --bind 0.0.0.0:8787
```

Compose:

```bash
docker compose up --build
```

## Native Windows

```powershell
.\scripts\build-release.ps1
.\dns-cli.cmd serve --bind 127.0.0.1:8787
```

## Native Linux

```bash
./scripts/build-release.sh
./target/release/dns-cli serve --bind 127.0.0.1:8787
```

## CI artifacts

GitHub Actions builds Windows / macOS / Linux + aarch64 cross — see `.github/workflows/`.
