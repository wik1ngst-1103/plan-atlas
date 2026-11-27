# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-27

### Added

- Plan store: Markdown plans under `~/.plan-atlas/plans/<project>/` stay the source of truth; a SQLite (FTS5, accent-insensitive) index rebuilds from them, with a file watcher and import of `~/.claude/plans`.
- Local web UI (Svelte, English + Vietnamese): plan documents, task board, live activity feed, topic panel, knowledge map and global search.
- Claude Code integration: `plan-atlas setup` / `uninstall-claude`, MCP server, hook handler that records tool activity and links sessions to tasks, and the `task` CLI.
- Single binary with the web UI embedded (pre-compressed, served by `Accept-Encoding`); localhost-only server with Host and `X-Plan-Atlas` guards.
- Resource budget check (`scripts/bench-memory.sh`): idle RSS ≤ 40 MB with 1,000 plans, binary ≤ 20 MB.
- Release pipeline (cargo-dist): shell, PowerShell, Homebrew and npm installers for macOS, Linux and Windows.
- Project scaffold: `plan-atlas` CLI skeleton (`serve`, `mcp`, `hook`, `setup`, `uninstall-claude`, `import`, `task`), CI workflow and GitHub issue/PR templates.
