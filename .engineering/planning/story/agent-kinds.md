---
format: aep.planning-md/3
id: story:agent-kinds
kind: story
status: implemented
title: an agent kind fills any role at any placement; claude-code is the first, with today's behaviour kept exactly
summary: the agent trait (start, deliver, attach, reports, stop, capabilities); roles.<role>.kind; the agents/* routes; today's session scenarios and tests pass unchanged
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:instance-api
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:26Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:26Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-08T19:45:58Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-08: "currently we use ClaudeCode agents. but at some point a controller could also be a takt protocol, and the workers could also be various different 'executions' - we just need to abstract it. any combination possible - so abstract it - but keep exact behaviour now (claudecode-based)". Design §7.

## Acceptance

- `crates/conductor/src/agent/` holds the trait from `conductor.agent`: `start(slot) -> Handle`, `deliver(handle, text)`, `attach(handle) -> PtyStream` (interactive kinds), `reports(handle)`, `stop(handle)`, `capabilities()`; a role slot is `(role, kind, placement, profile, limits)` read from the effective settings; `roles.<role>.kind` defaults to `claude-code`.
- The `claude-code` kind is today's launcher moved behind the trait, byte for byte in what it writes: the settings file, `tmux.conf`, one tmux server, the profile appended, the envelope, the terminal log, the hand-over, the guard hook. `conductor.session`'s scenarios and the tests `confine_systemd`, `session_attach` and the session start tests pass unchanged against it (no test file is edited except to rename the entry point).
- `conductor agent start|deliver|attach|stop --instance <i> --role <r>` replace the `session` verbs, which stay as aliases for one release; the instance API serves `agents/{name}/attach`, `deliver`, `restart`.
- A test registers a second, run-to-exit kind (a fake program that exits with a code and writes an events file) in a worker slot: the runtime writes one `[REPORT]` line from the exit code and the file, a non-zero exit of the stopped class becomes a new generation, and no interactive route is offered for it (`attach` answers the capability refusal by name).
- `conductor doctor` names each configured kind's binary and version.
