# Web panel

## Run

```powershell
.\dns-cli.cmd serve --bind 127.0.0.1:8787
# or: dnstt-kit-windows-x64.exe serve
```

Open: http://127.0.0.1:8787

## Style (offline, like Bootstrap file)

| File | Role |
|------|------|
| `dns-cli/static/tailwindcss.js` | offline copy of Play CDN |
| `dns-cli/static/site.css` | buttons / panel only |
| `dns-cli/static/index.html` | loads `/tailwindcss.js` |

Update:

```powershell
Invoke-WebRequest https://cdn.tailwindcss.com -OutFile dns-cli\static\tailwindcss.js
```

## Security

- Full disk path masked in API
- Relative paths only
- Optional token: `DNS_CLI_WEB_TOKEN`
- [SECURITY_WEB.md](SECURITY_WEB.md)

## Tabs

**Start** · **Options** · **Guide** · **Status**  
(labels: English / Finglish for beginners)

## Test

`cargo test -p dns-cli --test web_smoke`
