---
format: aep.planning-md/3
id: story:hand-item-dry-run
kind: story
status: active
title: A class-H item carries its dry-run result
relations:
- decomposes: epic:loop-without-handwork
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T17:07:17Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-14T17:07:17Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-10-14: "Shouldn't the conductor system be re-triggered periodically and figure
this out itself? ... I think it's a defect of the system." A class-H script was handed to the
operator untested: its first step failed with HTTP 422 "Value is too large", which a size check
needing no credentials would have shown.

## Acceptance

- Spec first: a class-H escalation (`decision escalate-request` with class H) carries either a
  dry-run result (`--dry-run <file>`: the output of every step that needs no credential) or
  `--no-dry-run <reason>`; without one it is refused naming both flags; `ess specify validate`
  exits 0 and a test in `crates/conductor/tests/decision.rs` holds the refusal and both forms.
- `conductor decision hands-todo` prints each item's dry-run result or reason;
  `.agents/conductor.md` says an operator script is dry-run before it is handed over; a profile
  test holds the sentence.

## Scope

`spec/domains/decision.yaml`, `crates/conductor-model`, `crates/conductor/src/decide.rs`,
`crates/conductor/src/cli.rs`, `crates/conductor/tests/decision.rs`, `.agents/conductor.md`,
`crates/conductor/tests/profiles.rs`.
