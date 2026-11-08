---
format: aep.planning-md/3
id: story:host-roles-by-daemon
kind: story
status: active
title: the daemon starts the supervisor and teacher as configured roles and the maintainer retires into an instance
summary: globals.roles; conductor_host.slice; no claude --bg and no ~/.claude/agents adapters
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:daemon-reconcile
- depends_on: story:config-globals
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:28Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:28Z", actor: "human:timo", revision: 3}
---
## Why

Design §8; operator, 2026-11-07: the teacher is its own role above the supervisor; no separate maintainer, conductor's repository is a repository of an instance.

## Acceptance

- `globals.roles` carries `supervisor` and `teacher`; the daemon starts them through `conductor session start --role <role>` in `conductor_host.slice` at the `host` or local `substrate` placement; the teacher only when `teacher.enabled` or the operator asks; one session per name.
- `task supervisor:start` and `task teacher:start` call the daemon; the adapters under `~/.claude/agents/` and `claude --bg` are no longer used by any task (a test greps the Taskfile).
- The `conductor-dev` role is accepted for one more release; the conductor repository listed in an instance's `sources` gets its work from that instance's controller like any repository.
