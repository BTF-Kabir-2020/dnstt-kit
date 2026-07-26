# How to Update the GitHub Wiki

> GitHub Wiki is a **separate Git repo** — it is NOT part of the main `dnstt-kit` repo.

## Setup (one time)

Clone the wiki next to your project:

```powershell
cd ..
git clone https://github.com/BTF-Kabir-2020/dnstt-kit.wiki.git
```

This folder is already in `.gitignore` so it won't accidentally get committed into the main repo.

## Edit workflow

```powershell
# 1. Go to wiki folder
cd ..\dnstt-kit.wiki.git

# 2. Pull latest
git pull

# 3. Edit any .md file with your editor
#    - Home.md          = front page
#    - _Sidebar.md      = navigation sidebar
#    - Quick-Start.md   = download & first run
#    - CLI-Cheat-Sheet.md = common commands
#    - Sanitize.md      = sanitize command docs
#    - FAQ.md           = frequently asked questions
#    - Roadmap.md       = status & planned work
#    - Web-UI.md        = local web panel
#    - Contributing.md  = PRs & rules
#    - Docs-Index.md    = links to docs/ in main repo

# 4. Commit & push
git add -A
git commit -m "wiki: describe what changed"
git push origin master
```

## Rules

- **Never commit the wiki folder into the main repo** — it's in `.gitignore`
- **Encoding:** use plain ASCII or UTF-8 without BOM. Avoid fancy Unicode arrows (`→`) — use `->` instead. Avoid `·` (middle dot) — use ` - ` or ` / ` instead.
- **Links:** use GitHub wiki relative links like `[Page-Name](Page-Name)`, not full URLs
- **Code blocks:** use ` ```text ` or ` ```powershell ` fences
- **Version bumps:** update version in `Home.md` and `Roadmap.md` when a new release is tagged

## File structure

```
dnstt-kit.wiki.git/
├── Home.md              # front page
├── _Sidebar.md          # left sidebar navigation
├── Quick-Start.md       # download & first run
├── CLI-Cheat-Sheet.md   # common commands
├── Sanitize.md          # IP list cleanup
├── FAQ.md               # frequently asked questions
├── Roadmap.md           # status & planned work
├── Web-UI.md            # local web panel
├── Contributing.md      # PRs & rules
└── Docs-Index.md        # links to docs/ in main repo
```

## Quick reference

| Task | Command |
|------|---------|
| Clone wiki | `git clone https://github.com/BTF-Kabir-2020/dnstt-kit.wiki.git` |
| Pull latest | `git pull` |
| Push changes | `git push origin master` |
| Add new page | Create `NewPage.md`, add link to `Home.md` and `_Sidebar.md` |
| Delete page | Remove the `.md` file, remove links from `Home.md` and `_Sidebar.md` |

## Common mistakes

1. **Editing wiki inside the main repo** — won't push. Wiki is a separate `.wiki.git` repo.
2. **Using `git push` without `origin master`** — wiki uses `master` branch, not `main`.
3. **Unicode characters** — GitHub wiki rendering can break with certain Unicode. Stick to ASCII-safe alternatives.
4. **Forgetting to update `_Sidebar.md`** — new pages won't appear in navigation.
