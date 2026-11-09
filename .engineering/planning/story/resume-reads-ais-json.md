---
format: aep.planning-md/3
id: story:resume-reads-ais-json
kind: story
status: active
title: The loop resumes after a usage limit while one ais account is exhausted
summary: Read the headroom from the JSON ais prints whatever its exit; only missing or unparsable output is an error
relations:
- decomposes: epic:deterministic-steps
scope:
- confidence: cited
  path: crates/conductor/src/run.rs
- confidence: inferred
  path: crates/conductor/src/run/handlers.rs
- confidence: inferred
  path: crates/conductor/tests/run.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T01:00:37Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T01:00:37Z", actor: "human:timo", revision: 4}
---
## Why

`ais status --json --provider claude` (ais 0.5.1) exits 3 while any account of the pool is exhausted, and still prints the whole status (reproduced 2026-11-08 01:59 CET: 21 accounts, an active one, exit 3). `Ais::headroom` in `crates/conductor/src/run.rs` bailed on the non-zero exit before reading the JSON, so `run/handlers.rs` `pending` noted `resume after a usage limit` each tick and resumed nothing. Journals, 2026-11-08: three instances hit a usage limit between 01:47:53 and 01:50:10 and logged `exited 3` from 01:51:47 on; no `resumed` line on any of them (conductor-teacher, relayed by the host supervisor).

## Acceptance

- The headroom is read from the JSON whenever `ais` prints a JSON object, whatever its exit; only missing or unparsable output is an error, and its journal line names the exit. `headroom_is_read_from_printed_json_whatever_the_exit` in `crates/conductor/src/run.rs` holds both paths.
- `.agents/conductor-supervisor.md` has the supervisor resume the named sessions by hand once on an `exited <n>` journal line and file the defect.

## Scope

`crates/conductor/src/run.rs` (`Ais::headroom`, `headroom_read`), `.agents/conductor-supervisor.md`.
