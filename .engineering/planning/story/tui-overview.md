---
format: aep.planning-md/3
id: story:tui-overview
kind: story
status: draft
title: conductor tui shows a read-only overview of sessions and attention lines from the dashboard's read model
relations:
- decomposes: epic:tui-frontend
scope:
- confidence: cited
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: inferred
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/dashboard.rs
- confidence: cited
  path: crates/conductor/src/dashboard/overview.rs
- confidence: inferred
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/tui/overview.rs
- confidence: cited
  path: crates/conductor/src/tui/screens.rs
- confidence: inferred
  path: crates/conductor/tests
- confidence: inferred
  path: crates/conductor/tests/
- confidence: cited
  path: spec/domains/observation.yaml
revision: 15
---
# Story: tui-overview

## Outcome

`conductor tui` opens a read-only overview screen, end to end: the instance's sessions and its attention lines, read from the same read model the web dashboard uses, refreshed on its own, left with `q`.

## Context

- The dashboard computes its overview in `crates/conductor/src/dashboard/overview.rs` (the `/api/board`, `/api/repositories`, `/api/unanswered` answers, :50-62) and the page's `/data.json` carries `sessions`, `watch`, `awaiting_operator`, `red_ci`, `context_handover` and `disk` (crates/conductor/src/dashboard.rs:1340-1380). Today those functions sit behind the HTTP server and its request `Sources` (dashboard.rs:53-75), so a second frontend cannot call them.
- The attention lines are the overview's "what needs you": unanswered class-O and class-H items with the oldest age (`/api/unanswered`, dashboard.rs:33-35), exited or silent sessions and red CI (`watch`, `red_ci` in `/data.json`).
- The command is presentation, not a record, as `dashboard serve` is: a `local` callable over an input modelled in `spec/domains/observation.yaml` (`DashboardServe` at spec/domains/observation.yaml:91; binding at crates/conductor/cli.yaml:14-18, :185-192).
- Sessions come from `claude agents --json` read on a thread of its own (dashboard.rs:57-60); the TUI reuses that reader, not a second one.

## Acceptance

1. Spec first: model `conductor.observation.TuiRun` (fields `instance`, `refresh_seconds`) in `spec/domains/observation.yaml` beside `DashboardServe`, add `[tui]` to `crates/conductor/cli.yaml`, validated with `ess specify validate --path spec`; `crates/conductor-model` is regenerated, not edited.
2. The read-model functions of all the TUI's screens (overview, decisions, dispatches, activity, waiting-on, blockers and disk) are extracted from the dashboard's HTTP server into one module that both frontends use: `dashboard`'s endpoints call it, the TUI calls it, and no file under `decisions/`, `dispatches/` or the store is opened by TUI code (a test greps the TUI module for those paths and for `File::open`). The siblings `story:tui-decisions-read`, `story:tui-dispatches-activity` and `story:tui-host-disk-builds` call that module and do not move any function themselves.
3. A snapshot test renders the overview from a fixture store and fixture session list into a fixed-size buffer and matches a stored frame: one row per session (name, repository, state, current dispatch), the attention lines (unanswered count with oldest age, red CI, exited sessions), and the data's age.
4. Its numbers equal the dashboard's: over the same fixture, `/api/unanswered` and the TUI's attention count are equal (one test asserts both).
5. The screen refreshes every `--refresh` seconds (default 5) without a keypress and keeps the selection; a read that fails shows the error and the last good data, not a blank screen.
6. `q` and `ctrl-c` leave the alternate screen and restore the terminal; on a non-tty stdin or stdout the command exits 2 saying it needs a terminal.
7. `conductor tui --help` and the generated `website/docs/reference/cli.md` name the command; `task docs-check` passes.
8. The TUI lives in the module `crates/conductor/src/tui/`, with a screen registry in `crates/conductor/src/tui/screens.rs` where each screen and its key register; the overview registers there, and `story:tui-decisions-read`, `story:tui-dispatches-activity`, `story:tui-host-disk-builds` and `story:tui-open-session` each add their screen and key there and nowhere else. The overview screen is `crates/conductor/src/tui/overview.rs` and has two extension points: `attention_lines()` collects its lines from registered providers, and the sessions table takes extra columns from registered providers. `story:tui-host-disk-builds` and `story:tui-open-session` register through those and do not edit `overview.rs`.

## Out of Scope

Decisions (`story:tui-decisions-read`), dispatches and activity (`story:tui-dispatches-activity`), disk and builds (`story:tui-host-disk-builds`), any write.

## Ambiguities

- Terminal-UI crate: `ratatui` with `crossterm` is proposed; the implementor confirms it builds on the four release targets (.github/workflows/release.yml targets) before adopting it.
- The extraction of the read model from the HTTP server is part of this story to keep the slice vertical; if it proves larger than one story, split it first and say so.

## Scope

## Scope

Derived 2026-10-17 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor/src/dashboard/` (the read model, extracted out of the HTTP server) and the new `crates/conductor/src/tui/` — cited
- **Files:** `crates/conductor/src/dashboard/overview.rs:50-62` (the `/api/board`, `/api/repositories` path lists) — cited
- **Files:** `crates/conductor/src/dashboard.rs:33-35` (`UNANSWERED_PATH`), `:53-75` (`Sources`, the sessions reader), `:1340-1380` (the `/data.json` assembly) — cited
- **Files:** `crates/conductor/src/tui/screens.rs` (the screen registry), `crates/conductor/src/tui/overview.rs` (`attention_lines()`, the sessions table), a new read-model module beside `dashboard.rs` — cited, new files named by acceptance 2 and 8
- **Files:** `spec/domains/observation.yaml:91` (new `TuiRun` beside `DashboardServe`), `crates/conductor/cli.yaml:14-18` and `:185-192` (new `[tui]` command and callable) — cited
- **Files:** `crates/conductor/Cargo.toml` (a TUI crate dependency; `ratatui` with `crossterm` is proposed in the story) — cited
- **Symbols:** `DashboardServe`, `dashboard-serve`, `Sources`, `UNANSWERED_PATH`, `awaiting_operator`, `red_ci`, `watch`, `sessions` — cited
- **Also likely:** `crates/conductor/src/cli.rs` (`Group`, `DashboardCommand` at :506 and :1337, the dispatch at :410; a `Tui` variant and args struct) — inferred, from how `dashboard serve` is wired
- **Also likely:** `crates/conductor/src/lib.rs` (a `pub mod tui;`) — inferred
- **Also likely:** `crates/conductor/tests/` (a snapshot test, an equality test against `/api/unanswered`, a grep test over the TUI module) — inferred
- **Also likely:** `crates/conductor-model/` regenerated by `ess`, not edited by hand — cited, AGENTS.md layout table
- **Documents:** the generated `website/docs/reference/cli.md` and `website/data/status.json` (regenerated, not hand-edited) — cited
- **Confidence:** high — the story cites file:line and symbols, and I confirmed that they exist in the tree. The citations drift by a few lines.
- **Would collide with:** any unit touching `crates/conductor/src/dashboard.rs` or `dashboard/` (HTTP handlers or the read model), `crates/conductor/src/cli.rs`, `crates/conductor/cli.yaml` or `spec/domains/observation.yaml`, and the siblings `tui-decisions-read`, `tui-dispatches-activity`, `tui-host-disk-builds` and `tui-open-session` on `tui/screens.rs` and the extracted read-model module
- **Safety fact:** the dashboard's endpoints must give the same answers once the read-model functions are extracted, because acceptance 4 puts `/api/unanswered` and the TUI's attention count under one test over one fixture. Step 2, pointed at `dashboard.rs:33-35` and `:1340-1380`, unproven. I have not checked who else calls the `dashboard::` read-model functions (for example `status.rs` or `rollup.rs`), nor whether the extraction moves any `pub` path that is used elsewhere.
