---
format: aep.planning-md/3
id: story:sessions-start-with-claude-bg
kind: story
status: draft
title: Role sessions start with claude --bg, without the systemd and tmux launcher
summary: attach is claude attach, delivery a message, no tmux pane
relations:
- informed_by: story:systemd-session-starts-unattended
revision: 1
---
## Why

The operator, 2026-11-07: "I never asked to run this in Tmux ... just run the conductor dev
session in a regular way. No Tmux."

`docs/design/conductor-run.md` § 4 (line 100) starts every session through the systemd provider
with the tmux launcher, so that `session attach` and `session deliver` (`type_text` into the pane,
line 77) work. On tmux 3.7c that launcher created no session until a client attached
(story:systemd-session-starts-unattended), and every controller spawn of `conductor run` failed.

Claude Code has its own background mode: `claude --bg` prints an id that `claude attach`,
`claude logs`, `claude stop` and `claude rm` take, and `claude agents` lists the session.

## Proposal

Start role sessions with `claude --bg` and drop the systemd + tmux launcher:
- `session attach` runs `claude attach <id>`;
- delivery goes through the harness's message route or the session's inbox, not keystrokes;
- the cgroup limits the systemd provider set (memory, CPU weight, tasks) are kept by another
  route, or their loss is recorded as the trade-off.

Known side effect: Claude Code holds a message between two sessions in different permission
modes for approval; conductor starts its roles in `bypassPermissions`, while a session that
messages them may run in another mode. The design says how delivery handles that.

## Acceptance

- `conductor run` starts a controller with no terminal attached, and `claude agents` lists it
  live within its bound.
- `conductor session attach <name>` attaches to it; a delivered message reaches it.
- `docs/design/conductor-run.md` § 4 describes the new launcher.
