# Contributing

**PRs welcome.** Human contributors are invited — fork, open issues, send pull requests. That is how this project grows.

Maintainer: [BTF Kabir](https://github.com/BTF-Kabir-2020) (`@BTF-Kabir-2020`)

## Quick path

1. Fork → branch off `main`
2. `cargo test --workspace` and `.\scripts\quality.ps1` (Windows) or `./scripts/quality.sh`
3. Web CSS is the vendored Play CDN file (`dns-cli/static/tailwindcss.js`) — no Node build
4. Open a PR with the template; keep secrets out of the diff

```powershell
git checkout -b feat/my-idea
# edits...
.\scripts\quality.ps1
git push -u origin HEAD
```

Good first areas: docs, safer web defaults, tests under `dns-cli/tests/`, FFI samples, smoke scripts.

## Rules

- `rustfmt` + `clippy -D warnings` (CI)
- Don’t commit `.env`, real `SLIPNET_CONFIG`, or live passwords
- Web stays localhost-first — see `docs/SECURITY_WEB.md`
- Prefer focused PRs over mega-diffs

## Commits & Contributors (important)

- GitHub **Contributors** should reflect **people** who actually contributed.
- Do **not** add `Co-authored-by: Cursor`, `cursoragent`, or any AI/tool trailer to commits.
- If you use an AI assistant while editing, keep the **git author** as yourself (or the human submitting the PR). Never attribute authorship to the tool.
- Maintainers may rewrite/reject commits that inject bot co-authors into history.

## License

By contributing you agree your changes are under the same non-commercial [LICENSE](LICENSE) as the project.
