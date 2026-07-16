# Set GitHub About + topics (run after: gh auth login)
$ErrorActionPreference = "Stop"
$Repo = "BTF-Kabir-2020/dnstt-kit"

gh auth status | Out-Null

gh repo edit $Repo `
  --description "DNSTT kit (Rust): DNS scan, NetMod/NekoBox/SlipNet generate, offline slipnet, secure localhost web UI, Docker"
# Clear homepage (empty) — gh --homepage "" rejects; use API
gh api -X PATCH "repos/$Repo" -f homepage="" | Out-Null

# Full replace — no locale/region topics (persian/farsi/iran/…)
$body = @{
  names = @(
    "rust", "dnstt", "dns", "networking", "cli", "web-ui", "docker",
    "offline-first", "sqlite", "security", "open-source", "tailwindcss"
  )
} | ConvertTo-Json -Compress

$body | gh api -X PUT "repos/$Repo/topics" `
  -H "Accept: application/vnd.github+json" `
  --input - | Out-Null

Write-Host "OK: About + topics on https://github.com/$Repo"
Write-Host "Remember: enable Discussions if you want easy Q&A."
