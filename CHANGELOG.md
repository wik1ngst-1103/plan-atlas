# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `plan-atlas import <file|folder>... [--project <name>]`: import Markdown plans from anywhere. Folders are searched recursively for `*.md` (hidden folders, `node_modules` and `target` skipped, at most 2,000 files). Without `--project`, a plan goes to its git repository's project, else to the project of the Claude session that wrote it, else to `unsorted`. Re-imports are idempotent and files already in the store are skipped.
- Web **Import plans** button: upload `.md` files or a whole folder from the browser into an existing, new or unsorted project (`POST /api/import`; at most 200 files, 2 MB each, 20 MB in total).

### Fixed

- `plan-atlas import` now honours `CLAUDE_CONFIG_DIR`, like `setup` and the automatic import at startup.

## [0.1.0] - 2026-11-27

### Added

- Plan store: Markdown plans under `~/.plan-atlas/plans/<project>/` stay the source of truth; a SQLite (FTS5, accent-insensitive) index rebuilds from them, with a file watcher and import of `~/.claude/plans`.
- Local web UI (Svelte, English + Vietnamese): plan documents, task board, live activity feed, topic panel, knowledge map and global search.
- Claude Code integration: `plan-atlas setup` / `uninstall-claude`, MCP server, hook handler that records tool activity and links sessions to tasks, and the `task` CLI.
- Single binary with the web UI embedded (pre-compressed, served by `Accept-Encoding`); localhost-only server with Host and `X-Plan-Atlas` guards.
- Resource budget check (`scripts/bench-memory.sh`): idle RSS ≤ 40 MB with 1,000 plans, binary ≤ 20 MB.
- Release pipeline (cargo-dist): shell, PowerShell, Homebrew and npm installers for macOS, Linux and Windows.
- Project scaffold: `plan-atlas` CLI skeleton (`serve`, `mcp`, `hook`, `setup`, `uninstall-claude`, `import`, `task`), CI workflow and GitHub issue/PR templates.
