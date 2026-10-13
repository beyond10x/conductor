---
format: aep.planning-md/3
id: story:gate-run
kind: story
status: draft
title: conductor gate run runs a gate with a disk watchdog, du cap and resumable steps
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: .agents/repo-controller.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/gate.rs
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/tests/cli_tree.rs
- confidence: cited
  path: crates/conductor/tests/gate.rs
- confidence: cited
  path: crates/conductor/tests/profiles.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/gate.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 2
---
## Why

On 2026-10-13 controllers in eight repositories wrote fourteen gate and watchdog scripts of their
own, each carrying the profile's process-group, STOP/CONT disk watchdog and du cap in its own way.
One worker tree reached 23G against a 10G cap before any watcher existed, free disk fell from 63G
to 33G in two hours, and conductor pointed one controller at another repository's script. A
probe-only fix then restarted a seven-hour conformance suite from its first step, and its
controller sat idle beside it for 158 minutes.

## Acceptance

- Spec first: a gate run (tree, log directory, du limit, steps file, process group, state
  `running|stopped|passed|failed`, failed step) with `StartGateRun`, `EndGateRun` and a view
  `GateRuns` is declared in `spec/domains/gate.yaml` (new) and placed in a `gate` group in
  `spec/components.yaml`; `ess specify validate --path spec` exits 0.
- `conductor gate run --tree <t> --log <dir> [--du-limit <G>] [--steps <file>] -- <cmd…>` runs the
  command in its own process group, stops it at `thresholds.gate_stop` free disk or at the du limit
  and resumes it at `thresholds.gate_resume`, checking every `thresholds.watchdog_every`, and
  writes `pgid`, `steps.tsv` and an `EXIT=<code>` line; tests in `crates/conductor/tests/gate.rs`
  with a fake command and a fake disk reading read a stop, a resume and the exit line.
- `--from-step <n>` with a steps file reruns from that step and keeps earlier passes; a test in
  `crates/conductor/tests/gate.rs` fails step 2 of 3, reruns from 2 and reads steps 1-3 passed.
- A gate run registers its wait (`story:watch-waits`), so the watch reports its end; the same test
  file reads the `wait ended` line.
- `.agents/repo-controller.md`'s Boundaries paragraph names `conductor gate run` instead of
  describing the watchdog; `crates/conductor/tests/profiles.rs` holds it.

## Notes

- After `story:watch-waits`.
- Pairs with `story:resumable-gates` (implemented), whose wording this command makes executable;
  that story's acceptance gains a line for harness-only fixes (rerun the step and its dependants,
  keep the passes of unchanged inputs) in the same profile edit.
- Adds a CLI group: edits `crates/conductor/src/cli.rs` and `crates/conductor/tests/cli_tree.rs`
  alone in its wave on those files.

## Scope

`spec/domains/gate.yaml` (new), `spec/components.yaml`, `crates/conductor-model`,
`crates/conductor/src/gate.rs` (new), `crates/conductor/src/lib.rs`,
`crates/conductor/src/cli.rs`, `crates/conductor/tests/cli_tree.rs`,
`crates/conductor/tests/gate.rs` (new), `.agents/repo-controller.md`,
`crates/conductor/tests/profiles.rs`, `docs/design/conductor.md`,
`website/docs/reference/cli.md`.
