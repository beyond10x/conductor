---
format: aep.planning-md/3
id: story:box-crash-loop-reads-failed
kind: story
status: active
title: A crash-looping boxed runtime reads as failed, and doctor checks a boxed placement
scope:
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/instance.rs
- confidence: inferred
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: crates/conductor/tests/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/instance_api.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: website/docs/concepts/the-host.md
revision: 21
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:07Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:08Z", actor: "human:timo", revision: 8}
---
## Why

Reported by the operator session of a trial instance on 2026-11-10 from conductor 1.0.1 at `placement: systemd`
(the reporter's defect list items 19 and 20).

- The boxed runtime exited 1 within seconds, 91 times in 4 minutes; each restart was logged
  `Start (LostRuntime): Succeeded`, and `/v1/instances` showed `"state":"Running"`. The error is
  printed before `log_to` runs (instance.rs:189 against :201), so it is only in substrate's exec
  output; no `runtime.log` is written.
- `conductor doctor` answers `ready` for a `placement: systemd` config that cannot run, and the
  Placement table of concepts/the-host.md:93-98 names only the sessions directory as the box's
  need.

## Acceptance

- A runtime that exits within a bound of its start is recorded as a failed start with its
  error text, and `/v1/instances` answers it as not Running; the error is written to
  `runtime.log`; tests hold both.
- `conductor doctor` starts or dry-runs the box for a `placement: systemd` instance and answers
  not ready, naming the failure, when the runtime cannot serve; concepts/the-host.md lists what
  the box needs.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` daemon reconcile loop and doctor — cited
- **Files:** `crates/conductor/src/instance.rs` `crates/conductor/src/daemon/reconcile.rs` `crates/conductor/src/doctor.rs` `website/docs/concepts/the-host.md` — cited: the story names `instance.rs:189`/`:201` and `the-host.md:93-98`; the tree puts `Start (LostRuntime): Succeeded` and the `Running` row in `reconcile.rs` (`pass` :649, `observe` :485, `rows` :1075), and the `ready` verdict in `doctor.rs` (`read_config` :1061, `check_in` :1324, `verdict_of`)
- **Symbols:** `instance_serve` `log_to` `Daemon::pass` `Daemon::observe` `Daemon::rows` `Daemon::start_runtime` `doctor::check_in` `doctor::verdict_of` — cited, read in the tree
- **Also likely:** `crates/conductor/src/placement.rs` `crates/conductor/tests/placement.rs` `crates/conductor/tests/instance_api.rs` `crates/conductor/tests/doctor.rs` — inferred: the `Substrate` trait (`placement.rs:867`) has no dry-run and no way to read a session's output, so doctor's box check and the error text probably need it; `tests/placement.rs` holds the `Fake` substrate and the boxed `rows` state tests (:991-1056); `tests/instance_api.rs:310` holds the `runtime.log` test; `tests/doctor.rs` holds the verdict tests
- **Documents:** `website/docs/concepts/the-host.md` Placement table — cited
- **Confidence:** medium — the defect sites are read from the story and the tree, but how the error text reaches the daemon and how doctor dry-runs a box are design choices the story leaves open
- **Would collide with:** any unit touching the reconcile pass or the `rows` state mapping in `daemon/reconcile.rs`, doctor's check assembly and verdict in `doctor.rs`, the `Substrate` trait or the test `Fake` in `tests/placement.rs`, the head of `instance_serve`, or the Placement section of `the-host.md`
- **Safety fact:** `doctor::check_in` is also called on a repeating thread by `crates/conductor/src/dashboard/health.rs:208` and by `crates/conductor/src/setup.rs:645`, so a box check placed inside it would open a box on every dashboard health refresh; it has to sit behind the CLI path (`doctor_with`) or be side-effect free — step 2 (`git grep -n 'check_in(\|check_at('`), unproven
