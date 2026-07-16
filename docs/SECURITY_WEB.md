# Web security notes

## Threat model

The panel is for **the operator on the same machine**. If you bind to a public interface, anyone who can reach the port can start scans/pipelines (and with a stolen token, more).

## Checklist before exposing beyond localhost

1. Set a long random `DNS_CLI_WEB_TOKEN` in `.env`
2. Put TLS + auth reverse proxy in front (Caddy/nginx)
3. Keep `--bind 127.0.0.1:8787` and proxy locally
4. Never commit `.env`

## What the UI shows

| Field | Shown? |
|-------|--------|
| Full absolute `work_dir` | **No** (masked: `…/dnstt-kit`) |
| SQLite absolute path | Console only, not JSON API |
| Job logs | Truncated; avoid pasting secrets into cmdline |
| Profiles / passwords | Not via API |

## Path rules

Inputs like `testdata/dns_sample.txt` must be **relative** to work_dir.  
Rejected: `C:\…`, `/etc/passwd`, `../…`, UNC paths.

## UTF-8

All HTTP responses use `charset=utf-8`. On Windows, the CLI enables console UTF-8 (code page 65001) at startup.
