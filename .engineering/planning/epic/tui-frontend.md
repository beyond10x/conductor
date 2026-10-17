---
format: aep.planning-md/3
id: epic:tui-frontend
kind: epic
status: draft
title: The operator runs conductor from a terminal client that shows what the dashboard shows
relations:
- decomposes: initiative:adoption
- informed_by: epic:dashboard-decisions
revision: 1
---
## Why

The operator, 2026-10-17: a lean terminal client that shows what the web dashboard shows: sessions, the decisions to answer (and answering the escalated ones with the operator commands, after a confirm step), dispatches, the activity feed, the disk and build queue, and the attention lines. Today the operator reads the dashboard in a browser (crates/conductor/src/dashboard.rs:1-25) and answers decisions on its Decisions page (`epic:dashboard-decisions`); on a host without a browser, over ssh, or inside a terminal session, he has `conductor status` (a text print, crates/conductor/cli.yaml status command) and nothing live.

## Outcome

`conductor tui` opens a full-screen terminal client over the instance's records and state. It shows the overview, the sessions, the open decisions with their options, the dispatches, the activity feed, and the disk and build queue, refreshes by itself, and lets the operator answer an escalated decision (class O or H) through the same recording path the dashboard's Decisions page uses, after a confirm step. It reads the same read model as the dashboard; it never reads the records files by hand.

## Decision: a subcommand, not a binary

`conductor tui`, a subcommand of the one `conductor` binary:
- The README, the release archives (`conductor-<version>-<target>.tar.gz`, .github/workflows/release.yml:1-4) and `conductor upgrade` all assume one binary; a second binary doubles the release matrix, the checksum list and the upgrade path.
- The read model lives in the `conductor` crate (crates/conductor/src/dashboard/*.rs: overview.rs, decisions.rs, repo.rs, series.rs, health.rs); a separate binary would link the same crate or duplicate it.
- The command follows the dashboard's own pattern: a `local` callable presented by the `ess-cli/1` binding over an input modelled in `spec/domains/observation.yaml` (`conductor.observation.DashboardServe`, spec/domains/observation.yaml:91; callable at crates/conductor/cli.yaml:14-18, command at :185-192).
- Cost: the terminal-UI crate (for example `ratatui` with `crossterm`) enters the one binary's build. That is a build dependency only; no host program is added.

## Decision: in-process read model, not HTTP

The TUI calls the dashboard's view functions (the ones behind `/api/board`, `/api/decisions`, `/api/events`, `/api/unanswered`, `/api/sessions`, `/api/health` and the `/data.json` the page reads, dashboard.rs:1-60, :492-507, :967, :1340-1380) in its own process, after they are moved out of the HTTP server into a read-model module both frontends use (story `tui-overview`). It does not require `conductor dashboard serve` to be running, and it needs no token. Answers go through `decide::answer_as_operator` (crates/conductor/src/decide.rs:607), the function `POST /api/answers` uses (crates/conductor/src/dashboard/answers.rs:41), recorded by `tui` instead of `dashboard`.

Why not talk to the running dashboard over HTTP: it adds a hard prerequisite (a server on a port) for a local read, and the page's token (dashboard.rs:20-26) is meant for a browser origin. The cost of in-process is two processes each holding a store handle; the store is already read by several short commands, and the dashboard's own caching (dashboard/cache.rs) is the dashboard's, not the model's.

## Stories

`story:tui-overview` (first, end to end), `story:tui-decisions-read`, `story:tui-decisions-answer`, `story:tui-dispatches-activity`, `story:tui-host-disk-builds`, `story:tui-open-session`.

## Out of scope

Charts and series pages (`epic:dashboard-charts`), the static export, the host dashboard across instances, mouse support, themes. Answering decisions of class C or R (conductor decides them).
