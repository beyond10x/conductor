---
format: aep.planning-md/3
id: story:systemd-session-starts-unattended
kind: story
status: active
title: A systemd-provider session starts without anyone attaching
summary: The start connects one tmux client so tmux -D creates the session; long prompts load from a file
relations:
- informed_by: story:pinned-settings-keep-missing-hooks
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T03:17:45Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T03:17:45Z", actor: "human:timo", revision: 3}
---
## Why

Observed 2026-11-07: `conductor run` (0.19.1, `systemd` provider, tmux 3.7c) spawned the
controllers cortex and loom; each spawn ended `LaunchFailed: ... the systemd provider's start the
session ... was not listed live within 120 s`, and each unit kept running a tmux server with no
pane process. A session that `conductor session start` launched started only after someone ran
`conductor session attach`.

Probed with tmux 3.7c in a scratch directory: `tmux -D -S <socket> -f <conf>` runs the
configuration, whose `new-session` creates the session, only when the first client connects, and
the pane starts in that client's directory unless `new-session -c` names one. The provider's
start connected no client before it waited for `claude agents` to list the session.

Two more faults of the same path, found in an earlier uncommitted tree:
- a prompt over tmux's 16 KB command limit ("command too long") is refused by `set-buffer`; the
  charters of cortex and loom are 43,450 and 30,472 bytes;
- a start that times out left its unit running.

## Acceptance

- After `systemd-run`, the start waits for the server's socket (10 s) and runs one
  `tmux has-session -t =<name>`, so the session exists before the liveness wait.
- The pane starts in the request's working directory (`new-session -c "$CONDUCTOR_CWD"`).
- The prompt is typed through `load-buffer` from a file, so a prompt over 16 KB is typed.
- A start not listed live within its bound stops its unit and says so.
