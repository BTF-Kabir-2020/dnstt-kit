# Set GitHub About + topics (run after: gh auth login)
$ErrorActionPreference = "Stop"
$Repo = "BTF-Kabir-2020/dnstt-kit"

gh auth status | Out-Null

gh repo edit $Repo `
  --description "DNSTT kit (Rust): DNS scan, NetMod/NekoBox/SlipNet generate, offline slipnet, secure localhost web UI, Docker"
# Clear homepage (empty) — gh --homepage "" rejects; use API
gh api -X PATCH "repos/$Repo" -f homepage="" | Out-Null

$topics = @(
  "rust","dnstt","dns","networking","cli","web-ui","docker",
  "offline-first","sqlite","security","persian","open-source","tailwindcss"
)
foreach ($t in $topics) {
  gh repo edit $Repo --add-topic $t
}

Write-Host "OK: About + topics on https://github.com/$Repo"
Write-Host "Remember: enable Discussions if you want easy Q&A."
