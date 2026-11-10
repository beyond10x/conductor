---
format: aep.planning-md/3
id: story:documented-commands-parse
kind: story
status: implemented
title: Every conductor line in start-conductor.md parses as written
scope:
- confidence: inferred
  path: crates/conductor/tests/docs_quickstart.rs
- confidence: inferred
  path: crates/conductor/tests/docs_start_conductor.rs
- confidence: inferred
  path: website/docs/guides/first-task-existing-repo.md
- confidence: cited
  path: website/docs/guides/start-conductor.md
revision: 16
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:03Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:03Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-11-10T07:51:27Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Why

Reported by the operator session of a trial instance on 2026-11-09 with conductor 1.0.1: `conductor controller
stop-controller --controller-id <id>` answers "--reason is required", while
website/docs/guides/start-conductor.md shows the command without `--reason`.

## Acceptance

- Every command line in website/docs/guides/start-conductor.md runs as written against the CLI's
  argument parser; a test parses each documented `conductor` line with the command tree and fails
  on a missing required flag.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `website/docs/guides/start-conductor.md` and one doc test under `crates/conductor/tests` — cited
- **Files:** `website/docs/guides/start-conductor.md:85` — cited, the `stop-controller` line without `--reason`
- **Files:** `crates/conductor/tests/docs_start_conductor.rs` — inferred, a new file; each guide test is its own `docs_*.rs` and none reads this guide
- **Symbols:** `StopControllerArgs`, `required`, `conductor_problem`, `command_texts` — cited
- **Also likely:** `crates/conductor/tests/docs_quickstart.rs` — inferred, only if the implementor extends its resolver instead of adding a file; it checks `first-task-existing-repo.md`
- **Also likely:** `website/docs/guides/first-task-existing-repo.md` — inferred, line 171 shows the same `stop-controller --controller-id <id>` without `--reason`; outside the acceptance as written
- **Documents:** `website/docs/guides/start-conductor.md` — cited
- **Confidence:** medium — the guide and the defect line are read from the tree, but the test's file and its source of "required" are open: no flag is clap-required (`crates/conductor/src/cli.rs:29`), so the handler decides
- **Would collide with:** any unit editing `website/docs/guides/start-conductor.md`; any unit editing `crates/conductor/tests/docs_quickstart.rs`; not with units on `crates/conductor-docs` — inferred
- **Safety fact:** the story needs no change under `crates/conductor/src`: `--reason` is refused by `required(args.reason, WHAT, "--reason")` at `crates/conductor/src/controller.rs:266`, not by clap, and `crates/conductor/tests/cli_tree.rs:970` holds that no flag is clap-required, so marking the flag required in `cli.rs` would break that test — cited, step 2 (pointed at `file:line`), unproven
