# 🗺 Plan Atlas

**English** · [Tiếng Việt](README.vi.md)

**One home for every plan Claude Code writes — readable by developers and non-developers alike.**

Plan Atlas is a small, local, open-source app that collects the implementation plans Claude Code creates across all your projects, shows them as clean documents with a task board, and lets you watch what Claude and its subagents are doing — in plain language, in real time.

> Status: **pre-alpha**. The UI prototype lives in [`docs/prototype/index.html`](docs/prototype/index.html); the v1 implementation plan is in [`docs/superpowers/plans/`](docs/superpowers/plans/).

---

## Why

Claude Code writes plans into `~/.claude/plans/` with random names like `jiggly-popping-popcorn.md`. After a few weeks you have dozens of them, no idea which project each belongs to, and no way to see progress. Plan Atlas fixes that:

| | |
|---|---|
| 📚 **One store, per project** | Every plan lives in `~/.plan-atlas/plans/<project>/`. Existing Claude Code plans are imported automatically (originals are never touched). |
| 📄 **Plans as documents** | A fixed structure — Overview, Requirements & Scope, Technical Analysis, Task Breakdown, Risks — with an "On this page" outline. |
| ✅ **Task tracking** | Each task has a status, a complexity score (1–5) and the recommended model + effort (Haiku / Sonnet / Opus). Drag cards on the board to update. |
| 🤖 **Live activity** | See "Coding assistant is editing the token store" nested under the right plan and task. Raw tool calls are one click away for developers. |
| 🧭 **Knowledge map** | Topics (tags) from a shared taxonomy, plus a graph of how plans relate (`depends_on`, `extends`, `relates_to`, shared topics). |
| 🔍 **Search** | Plans, tasks and topics. Diacritic-insensitive (type `dang nhap`, find `đăng nhập`). |

## How it works

```
Claude Code ──(MCP, stdio)── plan-atlas mcp ──┐
Claude Code ──(hooks)─────── plan-atlas hook ─┼──► SQLite index ◄── plan-atlas (web, 127.0.0.1)
~/.plan-atlas/plans/*.md ◄──── file watcher ──┘                         ▲
                                                                  your browser
```

- **One binary**, written in Rust: web server, MCP server, hook handler and installer in a single ~9 MB file. Idle memory target: **≤ 40 MB** with 1,000 plans.
- **Markdown files are the source of truth.** The SQLite database is only an index — delete it and it rebuilds.
- **Local only.** The server listens on `127.0.0.1`, nothing is sent anywhere.

## Install

> The first release (v0.1.0) is pending: these commands start working once it is published.

**macOS / Linux**
```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/WiK1ngST/plan-atlas/releases/latest/download/plan-atlas-installer.sh | sh
```

**Windows (PowerShell)**
```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/WiK1ngST/plan-atlas/releases/latest/download/plan-atlas-installer.ps1 | iex"
```

**Other options**
```sh
brew install WiK1ngST/tap/plan-atlas   # Homebrew
npm i -g plan-atlas                    # npm (global install; `npx` cannot run setup)
```

From source (needs Rust 1.89+ and Node 22): `cd web && npm ci && npm run build && cd .. && cargo build --release` — the web UI is embedded into the binary, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Use

```sh
plan-atlas
```

That's it. On first run Plan Atlas asks whether to connect to Claude Code, imports your existing plans and opens your browser at `http://127.0.0.1:7420`. Running it again while it is already running just opens a new tab.

| Command | What it does |
|---|---|
| `plan-atlas` | Start the app and open the browser (`--port <n>`, `--no-open`) |
| `plan-atlas setup` | Install the Skill, hooks and MCP server into Claude Code |
| `plan-atlas uninstall-claude` | Remove everything `setup` added (your other settings are kept) |
| `plan-atlas import` | Re-import plans from `~/.claude/plans/` |
| `plan-atlas mcp` | MCP server over stdio (started by Claude Code, not by you) |
| `plan-atlas hook` | Hook handler (called by Claude Code, not by you) |

### What `setup` changes

- Adds `~/.claude/skills/plan-atlas/SKILL.md`, which tells Claude where and how to write plans.
- Adds hook entries for `PreToolUse`, `SubagentStart`, `SubagentStop` and `SessionEnd` to `~/.claude/settings.json`. A timestamped backup is written first, and existing hooks are preserved.
- Registers the MCP server: `claude mcp add --scope user plan-atlas -- plan-atlas mcp`.

### Configuration

| | Default | Override |
|---|---|---|
| Data directory | `~/.plan-atlas` | `PLAN_ATLAS_HOME` |
| Port | `7420` (falls back up to `7430`) | `--port`, `PLAN_ATLAS_PORT` |
| UI language | Follows your browser (English or Tiếng Việt) | language switch in the top bar, remembered per browser |
| Topics | `~/.plan-atlas/taxonomy.toml` | edit the file, then run `plan-atlas setup` to refresh the Skill |

## Plan format

```markdown
---
id: PLAN-auth-gateway-001
project: auth-gateway
title: Migrate sessions to JWT + RBAC
tags: [type/feature, domain/security, tech/jwt]
related:
  - { to: PLAN-auth-gateway-000, type: depends_on }
created: 2026-11-20
---
## 01. Overview
## 02. Requirements & Scope
## 03. Technical Analysis
### Current Architecture
### Existing Problems
### Proposed Architecture
## 04. Task Breakdown
### TASK-001: Authentication Service
- complexity: 4
- model: opus
- effort: medium
- status: todo
- tags: [tech/jwt]

Description and acceptance criteria…
## 05. Risks & Rollback
```

You rarely write this by hand: Claude creates it through the MCP `create_plan` tool. Hand edits are fine, though. The file is re-indexed on save, and problems show up as warnings in the UI instead of errors.

## Privacy & security

- No telemetry, no network calls except to `127.0.0.1`.
- Write requests require a custom header and a localhost `Host` header (DNS-rebinding protection).
- The hook handler never blocks Claude Code: it exits within 200 ms and always with code 0.

See [SECURITY.md](SECURITY.md) to report a vulnerability.

## Contributing

Contributions are welcome, including code, translations, taxonomy topics, docs and bug reports. Start with [CONTRIBUTING.md](CONTRIBUTING.md). Everyone taking part agrees to the [Code of Conduct](CODE_OF_CONDUCT.md).

## Roadmap

- [ ] v0.1 — store, import, documents, board, live activity, knowledge map, search, installers
- [ ] Run as a background service (launchd / systemd / Windows service)
- [ ] Per-task cost & token usage
- [ ] Export a plan as a shareable static page
- [ ] More UI languages

## License

[MIT](LICENSE) © Plan Atlas contributors
