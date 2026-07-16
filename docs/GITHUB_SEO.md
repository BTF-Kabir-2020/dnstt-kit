# GitHub About / SEO / Discoverability

Use after `gh auth login` (account: **BTF-Kabir-2020**).

## One-shot (PowerShell)

```powershell
cd ...\dnstt-kit
.\scripts\github-seo.ps1
```

## Manual

```powershell
gh repo edit BTF-Kabir-2020/dnstt-kit `
  --description "DNSTT kit (Rust): DNS scan, NetMod/NekoBox/SlipNet generate, offline slipnet, secure localhost web UI, Docker" `
  --homepage "" `
  --add-topic rust `
  --add-topic dnstt `
  --add-topic dns `
  --add-topic networking `
  --add-topic cli `
  --add-topic web-ui `
  --add-topic docker `
  --add-topic offline-first `
  --add-topic sqlite `
  --add-topic security `
  --add-topic persian `
  --add-topic open-source `
  --add-topic tailwindcss
```

## About blurb (copy-paste)

`DNSTT toolkit in Rust — scan resolvers, generate client configs, offline slipnet, localhost web panel. Educational / non-commercial.`

## Growth tips

1. Good README + badges (done)  
2. Topics above (SEO for GitHub Explore)  
3. Releases with binaries (`release.yml` + tag `v*`)  
4. Friendly `CONTRIBUTING.md` + PR template (done)  
5. Related repos on GitHub only (no external brand links required)  
6. Answer issues quickly — stars follow usefulness  

## Releases vs Packages

- **Releases** = جایی برای باینری‌های `dns-cli` (Windows / Linux / macOS). همین را استفاده کنید.  
- **Packages** = برای npm/Docker registry روی GitHub؛ برای این پروژه لازم نیست خالی بماند.
