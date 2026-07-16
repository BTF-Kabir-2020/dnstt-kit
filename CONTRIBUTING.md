# Contributing

Thanks for looking at **dnstt-kit**.

Maintainer: [BTF Kabir](https://github.com/BTF-Kabir-2020)

## PR checklist

1. Fork and branch off `main`
2. Run `cargo test --workspace` and either `.\scripts\quality.ps1` or `./scripts/quality.sh`
3. Web CSS is the vendored Play CDN file (`dns-cli/static/tailwindcss.js`) — no Node/Tailwind build step
4. Open a PR; keep secrets out of the diff

```powershell
git checkout -b feat/my-idea
# edits...
.\scripts\quality.ps1
git push -u origin HEAD
```

Useful areas: docs clarity, safer web defaults, tests under `dns-cli/tests/`, smoke scripts.

## Rules

- `rustfmt` + `clippy -D warnings` (CI enforces this)
- Don’t commit `.env`, real `SLIPNET_CONFIG`, or live passwords
- Web stays localhost-first — see `docs/SECURITY_WEB.md`
- Smaller PRs are easier to review

## License

Contributions land under the same non-commercial [LICENSE](LICENSE) as the rest of the repo.
