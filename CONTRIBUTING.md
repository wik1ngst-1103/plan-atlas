# Contributing to Plan Atlas

Thanks for helping! This guide gets you from clone to merged pull request.

## Ways to contribute

- **Report a bug or request a feature.** Open an issue using the templates. Include your OS, `plan-atlas --version`, and steps to reproduce.
- **Translate the UI.** Add or improve a file in `web/src/locales/`. Translations only need a pull request, no code.
- **Improve the taxonomy.** Propose topics for `assets/taxonomy.default.toml`, each with a label and description in every supported language.
- **Write code or docs.** Look for issues labelled `good first issue` or `help wanted`. For anything larger than a bug fix, open an issue first so we can agree on the approach.

## Development setup

Requirements:

- **Rust 1.89+** via [rustup](https://rustup.rs). `rust-toolchain.toml` pins the exact version.
- **Node.js 22+**, needed only to build the web UI.
- Git.

```sh
git clone https://github.com/wik1ngst-1103/plan-atlas.git
cd plan-atlas

# 1. Build the web UI once (it is embedded into the binary)
cd web && npm ci && npm run build && cd ..

# 2. Run against a throw-away data directory
PLAN_ATLAS_HOME=$(mktemp -d) cargo run
```

To iterate on the UI with hot reload, run the backend and the Vite dev server side by side:

```sh
PLAN_ATLAS_HOME=/tmp/pa-dev cargo run -- --no-open   # API on 127.0.0.1:7420
cd web && npm run dev                                 # UI on :5173, proxies /api to 7420
```

> ⚠️ Never run `plan-atlas setup` against your real `~/.claude` while developing unless you mean to. Set `HOME` to a temporary directory to test the installer safely.

## Project layout

| Path | What lives there |
|---|---|
| `src/plan/` | Plan model, Markdown parser, status write-back |
| `src/store/` | SQLite schema, queries, full-text search |
| `src/events/` | Hook ingestion, plain-language step descriptions, live updates |
| `src/server/` | REST API, SSE stream, localhost guards, embedded UI |
| `src/mcp.rs` | MCP server used by Claude Code |
| `src/setup.rs` | Installs the Skill, hooks and MCP server into Claude Code |
| `assets/` | Skill template and default taxonomy |
| `web/` | Svelte 5 + Vite front end |
| `docs/` | Design spec, implementation plans, UI prototype |

## Before you open a pull request

Run the same checks CI runs:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test

cd web
npm run check   # svelte-check + TypeScript
npm test        # vitest
npm run build
npm run size    # JS bundle must stay ≤ 200 KB gzip
```

If you touched anything that runs while the app is idle (watcher, poller, SSE), also run the memory budget check on Linux:

```sh
./scripts/bench-memory.sh   # fails if idle RSS > 40 MB with 1,000 plans
```

## Guidelines

**Tests first.** Every bug fix comes with a test that fails without the fix. Every feature comes with tests for its main behaviour and its obvious failure modes.

**Resource use is a feature.** Plan Atlas runs on people's personal machines all day:

- Avoid busy loops and polling faster than 500 ms.
- Stream or paginate instead of loading everything into memory.
- Add dependencies sparingly. Explain in the PR why a new crate or npm package is worth its size.

**The hook must never break Claude Code.** Code under `src/events/hook.rs` must not panic outward, must not print to stdout, and must finish within 200 ms.

**Never modify the user's data unexpectedly.** Do not write to `~/.claude/plans/`, and back up `~/.claude/settings.json` before any change.

**UI copy is for non-developers too.** Prefer plain words ("Coding assistant is running tests") over jargon (`Bash npm test`). Every user-visible string goes through i18n.

**Style.** Use `rustfmt` and `clippy` defaults. Svelte/TS follow the repo's Prettier config. Prefer small, focused files.

## Commits and pull requests

- Use [Conventional Commits](https://www.conventionalcommits.org/): `feat(web): add topic panel`, `fix(store): handle CRLF files`, `docs: …`, `chore: …`.
- Keep each PR to one topic. Fill in the PR template, link the issue, and attach screenshots for UI changes (light and dark mode).
- A maintainer reviews within a few days. Two approvals are needed for changes to `setup.rs`, `hook.rs` or `security.rs`.
- By contributing you agree that your contribution is licensed under the [MIT License](LICENSE).

## Releasing (maintainers)

1. Update `CHANGELOG.md` and the version in `Cargo.toml`.
2. Tag `vX.Y.Z` and push. The `release` workflow (cargo-dist) builds every target and publishes the installers, Homebrew formula and npm package.

## Getting help

Open a GitHub Discussion or comment on the relevant issue. Please don't ask for support in security reports.
