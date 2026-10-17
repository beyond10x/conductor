---
format: aep.planning-md/3
id: story:tui-overview
kind: story
status: draft
title: conductor tui shows a read-only overview of sessions and attention lines from the dashboard's read model
relations:
- decomposes: epic:tui-frontend
revision: 2
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
8. The TUI lives in the module `crates/conductor/src/tui/`, with a screen registry in `crates/conductor/src/tui/screens.rs` where each screen and its key register; the overview registers there, and `story:tui-decisions-read`, `story:tui-dispatches-activity`, `story:tui-host-disk-builds` and `story:tui-open-session` each add their screen and key there and nowhere else.

## Out of Scope

Decisions (`story:tui-decisions-read`), dispatches and activity (`story:tui-dispatches-activity`), disk and builds (`story:tui-host-disk-builds`), any write.

## Ambiguities

- Terminal-UI crate: `ratatui` with `crossterm` is proposed; the implementor confirms it builds on the four release targets (.github/workflows/release.yml targets) before adopting it.
- The extraction of the read model from the HTTP server is part of this story to keep the slice vertical; if it proves larger than one story, split it first and say so.
