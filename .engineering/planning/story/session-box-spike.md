---
format: aep.planning-md/3
id: story:session-box-spike
kind: story
status: implemented
title: 'Spike: how a controller session lives in a box conductor controls'
relations:
- decomposes: epic:session-confinement
scope:
- confidence: cited
  path: docs/analysis/session-box-spike.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T23:21:55Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-08T23:21:55Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-10-09T03:29:30Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

`claude --bg` sessions belong to Claude Code's daemon, not to the command that starts them (docs/analysis/2026-10-09-substrate-benefits.md
§ 2), so conductor cannot box them by wrapping the start command. Two ways are open and neither is
verified: (a) conductor starts `claude` itself in the foreground, in a systemd user unit with a PTY
(for example under `script`), and keeps it there; (b) Claude Code's daemon runs inside a delegated
user unit, and each session's `bg-pty-host` is moved into a child cgroup.

## Acceptance

- A page `docs/analysis/session-box-spike.md` with, for each way, the commands run in a scratch checkout and their
  output: does the session run in the intended cgroup (`/proc/<pid>/cgroup`), does it appear in
  `claude agents --json`, does it receive a SendMessage from another session, does it survive the
  starting shell ending, can it be resumed (`claude --resume`) and stopped cleanly, and does a
  `bwrap` wrapper around it keep it working (credentials under `~/.claude`, network, its own child
  processes).
- A go/no-go per way and the one conductor's other stories build on.

## Safety on a shared host

This host runs conductor and its controllers under the same Claude Code daemon. The spike never
restarts, stops or moves that daemon, and never stops a session it did not start. Every session it
starts runs in a scratch directory under `~/.cache/conductor-spikes/`, and is stopped and removed
(`claude stop`, `claude rm`) before the page is written; the page lists their ids. A way that can
only be tried by touching the running daemon is tried with a separate daemon (its own
`CLAUDE_CONFIG_DIR`), or recorded as not tried and why.

## Scope

`docs/analysis/session-box-spike.md` (new); no product code.
