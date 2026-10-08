---
format: aep.planning-md/3
id: story:session-box-spike
kind: story
status: draft
title: 'Spike: how a controller session lives in a box conductor controls'
relations:
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: docs/analysis
revision: 2
---
## Why

`claude --bg` sessions belong to Claude Code's daemon, not to the command that starts them (docs/analysis/2026-10-09-substrate-benefits.md
§ 2), so conductor cannot box them by wrapping the start command. Two ways are open and neither is
verified: (a) conductor starts `claude` itself in the foreground, in a systemd user unit with a PTY
(for example under `script`), and keeps it there; (b) Claude Code's daemon runs inside a delegated
user unit, and each session's `bg-pty-host` is moved into a child cgroup.

## Acceptance

- A page in `docs/analysis/` with, for each way, the commands run in a scratch checkout and their
  output: does the session run in the intended cgroup (`/proc/<pid>/cgroup`), does it appear in
  `claude agents --json`, does it receive a SendMessage from another session, does it survive the
  starting shell ending, can it be resumed (`claude --resume`) and stopped cleanly, and does a
  `bwrap` wrapper around it keep it working (credentials under `~/.claude`, network, its own child
  processes).
- A go/no-go per way and the one conductor's other stories build on.

## Scope

`docs/analysis/` (new page); no product code.
