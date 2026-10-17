---
format: aep.planning-md/3
id: story:tui-host-disk-builds
kind: story
status: draft
title: The TUI shows disk, host grants and the build queue
relations:
- decomposes: epic:tui-frontend
- depends_on: story:tui-overview
- depends_on: story:watch-stalled-sessions
revision: 2
---
# Story: tui-host-disk-builds

## Outcome

A Host screen in `conductor tui` shows free disk against the floor and the reserve, the host grants, and the build slots and queue, so the operator sees why a build waits without reading the ledger.

## Context

- The page's `disk` object (with the reclaimable waste folded in) and `build_slot` are in `/data.json` (crates/conductor/src/dashboard.rs:1352-1368); `build_slot` reads "not recorded" today.
- Host grants are listed by `conductor host grants` (crates/conductor/cli.yaml:168-172, :360-365; crates/conductor/src/host.rs:13-14), each with `requested_at`, `started_at`, `released_at`, `bytes`, `peak_bytes`, repository and command.
- The build queue figures (queue now, per repository per day, slots) are `story:dashboard-build-queue`'s `/api/build-queue`; slots wait on `story:build-run-cpu-slots`. This story shows what exists when it is delivered and adds the queue the day that story lands, from the same function.
- `conductor host reclaim` removes what needs no decision (cli.yaml:372-377); the TUI only shows what it would free and prints the command, it does not run it.
- The screen registers in the screen registry `crates/conductor/src/tui/screens.rs` (`story:tui-overview`) with its key; the threshold `thresholds.build_queued` comes from `story:watch-stalled-sessions`.

## Acceptance

1. Spec first: no new noun; the `tui` command's model is unchanged (`conductor.observation.TuiRun`, `story:tui-overview`), validated with `ess specify validate --path spec`.
2. The Host screen shows free disk, the floor, bytes reserved by live grants and reclaimable waste with the command `conductor host reclaim` to run; the figures equal the page's `disk` object over the same fixture (one test).
3. Grants list newest first with state (queued, running, released), bytes against peak, repository and command; equal in count and order to `conductor host grants --format json` over the same ledger.
4. A queued grant shows its wait so far; the longest wait is in the screen's header and in the overview's attention lines when it is over `thresholds.build_queued` (default 30m), the threshold introduced by `story:watch-stalled-sessions`.
5. Where `/api/build-queue` exists (`story:dashboard-build-queue`) the screen shows its queue length, oldest wait and slots in use from the same function; before it exists the screen says `not recorded`, as the page does.
6. Nothing on this screen writes: no key runs `host reclaim` or releases a grant (a test sends every key and compares the ledger before and after).

## Out of Scope

Running reclaim; per-day build charts.

## Ambiguities

- Whether the floor and the reserve are the instance's or the host's (`epic:disk-headroom`, `epic:multi-instance-host`); the screen shows the figure the dashboard shows.
