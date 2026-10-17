---
format: aep.planning-md/3
id: story:demo-mode
kind: story
status: draft
title: conductor demo shows a sample repository and a first dispatch on the dashboard with no model and no forge
relations:
- decomposes: epic:quick-start
revision: 2
---
# Story: demo-mode

## Outcome

`conductor demo` sets up a sandbox instance on a sample repository it creates, serves the dashboard, opens it in the browser, and shows a first dispatch, with no model call, no forge and nothing of the person's own, so the quick start shows conductor working in seconds.

## Context

- A sample needs no private data: the `local` source of the getting-started page watches two tiny local repositories created with `git init` (website/docs/getting-started.md, "Two repositories to watch", from :129); a snapshot of them completes without a forge (the page, "One snapshot").
- The dashboard serves a live page and answers `/api/board`, `/api/repositories`, `/api/events` and the session rows from the newest complete snapshot and the records' `dispatches/*.jsonl` (crates/conductor/src/dashboard.rs:25-60); it listens on `127.0.0.1` only.
- A dispatch is a line in `dispatches/` written by `dispatch send-dispatch` and shown as a session's current dispatch (docs/design/conductor.md § 10 table, :404; crates/conductor/src/dashboard/repo.rs:55-100). A real controller is started by `conductor spawn` and needs `claude` (crates/conductor/cli.yaml:94-112).
- The demo instance must be separate from a real one: a different state and records directory under the instance's default root (website/docs/reference/config.md, instance section), a port that does not collide (`dashboard_port` distinct, config.md:25).
- Honesty: a demo dispatch is a recorded sample, not a model's work; every place that shows it labels it.
- Shared surface: this story shares `crates/conductor/cli.yaml`, `crates/conductor-model` and `website/docs/reference/cli.md` with `story:tui-overview`, `story:session-attach-command` and `story:session-list-attachable`; the merge cost is accepted and the generated files are regenerated after each merge.

## Acceptance

1. Spec first: model `conductor.config.RunDemo` (fields `port`, `no_open`, `clean`) and its refusals (`port-taken`, `demo-exists`) in `spec/domains/config.yaml` beside `InitRecords` (:538), add `[demo]` to `crates/conductor/cli.yaml`, validated with `ess specify validate --path spec`; `crates/conductor-model` is regenerated.
2. `conductor demo` on a machine with only the release binary and `git`: creates the demo instance (a `demo` config instance in a config file of its own, the person's real config untouched), a sample repository with two commits and a small planning store the sample's board can show, runs one snapshot, records one sample goal and one dispatch for it marked `demo`, serves the dashboard on `127.0.0.1` and opens the browser (`xdg-open`/`open`; without a browser it prints the URL), all in under 30 seconds on the release binary.
3. The dashboard shows the sample repository on the overview, one session row or dispatch row for the sample dispatch, and an activity-feed event for it; the session row, the dispatch row and the activity-feed event each show the label `demo`, and `/api/events` returns the event with the label.
4. With `--no-open` nothing is launched; with the port busy the command exits 1 naming `port-taken` and the `--port` flag; a second run attaches to the existing demo (prints the URL) and does not create a second one.
5. `conductor demo --clean` stops the server it started and removes the demo instance's directories and config, and nothing else (a test lists the tree before and after).
6. No file the demo writes, no line it prints and no dashboard string names a person, an organization or a path outside the demo's directories; `doctor` is not run and no program other than `git` is required (a test with a `PATH` holding only `git`).
7. `conductor demo` is refused inside a conductor, controller or worker session by the guard, as a command that writes (docs/design/conductor.md § 7).

## Out of Scope

A live controller on the sample (`conductor spawn` needs `claude`; the page's "Next" names it); hosting a shared demo.

## Ambiguities

- Whether the sample dispatch should be a real, tiny model-free step (a deterministic script that "does" the sample task, see `epic:deterministic-steps`) rather than a recorded line; a recorded sample is proposed for speed and honesty, a real step is the alternative.
- Opening a browser from a command is unusual for a server; `--no-open` and a printed URL cover headless hosts.
