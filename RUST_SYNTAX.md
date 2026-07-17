# Rust syntax cheat-sheet for contributors (dnstt-kit)
# Mirror of the teaching style used in related BTF Kabir Rust repos.

## Ownership & borrowing

| Form | Meaning |
|------|---------|
| `T` | Owned value |
| `&T` | Shared borrow |
| `&mut T` | Exclusive borrow |
| `PathBuf` | Owned path |
| `&Path` | Borrowed path |

## Errors

This crate mostly uses `Result<(), String>` (`AppResult`) at CLI edges. Prefer `?` and `.map_err(|e| e.to_string())`.

## Async

`scanner_core` / `scan_cmd` use Tokio. CLI entry builds a runtime only when needed so idle `serve` stays light.

## UTF-8

Source files and HTTP are UTF-8. On Windows the binary sets console CP 65001 at startup.

## Low memory / large lists

Use `--preset low` or `--stream` (line-by-line + disk). See [docs/MEMORY.md](docs/MEMORY.md).

## Layout

```
dnstt-kit/
  dns-cli/          # binary + web UI static/
  scanner-core/     # library + optional cdylib
  docs/             # user docs
  .github/          # CI / issues
```
