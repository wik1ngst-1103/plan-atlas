# Security Policy

## Supported versions

Only the latest minor release receives security fixes during the 0.x series.

## Reporting a vulnerability

Please **do not open a public issue**. Report privately through GitHub's **"Report a vulnerability"** button (Security → Advisories) on this repository.

Include:

- the affected version and OS,
- what an attacker can do and under which conditions,
- steps or a proof of concept to reproduce.

We aim to acknowledge reports within 3 working days and to ship a fix or mitigation within 30 days. You will be credited in the release notes unless you prefer otherwise.

## Threat model in brief

Plan Atlas runs locally and handles data that may be sensitive, such as plans, file names and commands run by Claude Code. Issues we especially care about:

- any way for a web page, another local user or a remote host to read or change data through the local server (the server should accept only `127.0.0.1` with a localhost `Host` header and a custom header on writes),
- `plan-atlas setup` corrupting or leaking `~/.claude/settings.json`,
- the hook handler blocking, crashing or altering Claude Code's behaviour,
- path traversal when reading or writing plan files.
