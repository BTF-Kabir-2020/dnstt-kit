# GitHub About / topics

Needs `gh auth login` as **BTF-Kabir-2020**.

```powershell
cd ...\dnstt-kit
.\scripts\github-seo.ps1
```

Or manually:

```powershell
gh repo edit BTF-Kabir-2020/dnstt-kit `
  --description "DNSTT kit (Rust): scan resolvers, NetMod/NekoBox/SlipNet generate, offline slipnet, localhost web UI" `
  --homepage ""

# topics (no locale/region tags)
.\scripts\github-seo.ps1
```

Suggested About text:

`DNSTT toolkit in Rust — scan resolvers, generate client configs, offline slipnet, localhost web panel. Educational / non-commercial.`

Binaries go on **Releases**, not Packages.
