# Set GitHub About + topics (run after: gh auth login)
$ErrorActionPreference = "Stop"
$Repo = "BTF-Kabir-2020/dnstt-kit"

gh auth status | Out-Null

gh repo edit $Repo `
  --description "DNSTT toolkit (Rust): scan UDP resolvers, decode/generate NetMod NekoBox SlipNet configs, offline slipnet, localhost web UI, Docker" `
  --homepage "https://github.com/BTF-Kabir-2020/dnstt-kit/wiki"

# Full replace — no locale/region topics (persian/farsi/iran/…)
$body = @{
  names = @(
    "rust",
    "dnstt",
    "dns",
    "dns-tunnel",
    "networking",
    "cli",
    "scanner",
    "web-ui",
    "docker",
    "offline-first",
    "sqlite",
    "security",
    "ffi",
    "android",
    "open-source",
    "tailwindcss"
  )
} | ConvertTo-Json -Compress

$body | gh api -X PUT "repos/$Repo/topics" `
  -H "Accept: application/vnd.github+json" `
  --input - | Out-Null

Write-Host "OK: About + topics on https://github.com/$Repo"
Write-Host "Remember: enable Discussions if you want easy Q&A."
