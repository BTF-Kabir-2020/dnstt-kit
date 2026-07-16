# Contributing / مشارکت

Thanks for helping grow **dnstt-kit**. PRs are welcome.

**Author / مالک:** [BTF Kabir](https://github.com/BTF-Kabir-2020) (`@BTF-Kabir-2020`)

---

## Quick PR path / مسیر سریع PR

1. Fork → clone your fork  
2. `cargo test --workspace` and `.\scripts\quality.ps1` (Windows) or `./scripts/quality.sh`  
3. Web UI uses offline Tailwind (`dns-cli/static/tailwindcss.js`). No CSS rebuild step.  
4. Open a Pull Request with the template — keep secrets out of diffs  

```powershell
git checkout -b feat/my-idea
# ... edits ...
.\scripts\quality.ps1
git push -u origin HEAD
# then "Compare & pull request" on GitHub
```

## Good first issues

- Docs typos / FA+EN clarity  
- More web presets / safer defaults  
- Extra tests in `dns-cli/tests/`  
- Cross-platform smoke scripts  

## Code rules

- `rustfmt` + `clippy -D warnings` must pass (CI)  
- No real `.env` / `SLIPNET_CONFIG` / passwords in PRs  
- Web stays **localhost-first**; see `docs/SECURITY_WEB.md`  
- Prefer small PRs over mega-diffs  

## License

By contributing you agree your changes are under the same **Non-Commercial** [LICENSE](LICENSE) as the project (same model as [CS-1.6-Tool-v2](https://github.com/BTF-Kabir-2020/CS-1.6-Tool-v2)).

## Community

- Issues: bug / question templates  
- Be respectful — we build this together  
