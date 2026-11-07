---
format: aep.planning-md/3
id: story:host-scripts-into-conductor
kind: story
status: active
title: The host supervisor's status pass, watch and controller restart are conductor commands
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T16:52:25Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T16:52:26Z", actor: "human:timo", revision: 4}
---
## Why

The operator, 2026-11-07: "how can we prevent it creating control scripts basically outside of the install of conductor itself. they will just be lost when we want to use this outside. also shouldnt we instead create this stuff in rust code"; "all modifications need to be persistent when generic, anything else should not even exist". The host supervisor runs three shell scripts and a ledger file outside the product: a status pass over every instance, a watch that waits for a mailbox file or a run-journal line, and a controller restart from a hand-off. Rule 3 of `AGENTS.md` says running code is Rust.

## Acceptance

- `conductor status --full` prints, per registered instance: run and dashboard unit state, the last cycle tick, every open dispatch (id, state, repository), every controller (repository, state), the open controller requests, the records checkout's branch line and changed-file count; then free disk, and the conductor product checkout's unpushed commits (count, count older than 30 min) and open pull requests.
- `conductor dispatch requests --open` lists every `[NEED …]` and `[DECISION-REQUEST …]` mailbox file of the last 48 h whose request is neither Answered nor Withdrawn nor answered by an answer record, with its age; `conductor dispatch answer-request` writes that answer record, so no ledger file outside the records is needed. Shared with `story:watch-unanswered-and-admits`, which this story may take in.
- `conductor session wait --until mailbox,journal --timeout 20m` exits 0 with one line naming the first of: a new mailbox file whose first line is not a `progress`, `done` or `started` report (nor a blocked/unblocked mr-route, ci or review report); a run-journal line carrying `context:`, `LaunchFailed`, `no open dispatch`, `lists no lettered option`, `DispatchNotFound` or `snapshot exited`; else exits with the timeout named.
- `conductor controller restart --repository <r> --handoff <path>` waits up to 3 min for the session to stop being busy, stops its unit, records the controller stopped, spawns it, delivers the open dispatch id and the hand-off path, and moves a blocked dispatch to started; it prints the dispatch's state.
- `conductor session context <name>` prints the session's context tokens, read from its transcript by session id.
- Each command has a test over fixtures; the host supervisor's scripts are deleted once it runs these.
