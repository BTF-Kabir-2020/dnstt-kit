## Summary
<!-- 1–3 bullets: what & why -->

-

## Type
- [ ] Bug fix
- [ ] Feature
- [ ] Docs
- [ ] CI / build
- [ ] Web UI (Tailwind via local `tailwindcss.js` — no build step)

## Test plan
- [ ] `.\scripts\quality.ps1` **or** `cargo test --workspace` + clippy
- [ ] Manual: `.\dns-cli.cmd doctor` (if CLI touched)
- [ ] Manual: web smoke / browser (if UI touched)

## Checklist
- [ ] No secrets (`.env`, real slipnet links, passwords)
- [ ] LICENSE / attribution kept
- [ ] Docs updated if behavior changed

Thanks — PRs help us grow faster 🚀
