---
format: aep.planning-md/3
id: epic:binary-only-operator
kind: epic
status: draft
title: An operator runs conductor from the released binary alone; no checkout, no task
relations:
- decomposes: initiative:adoption
- informed_by: epic:quick-start
- informed_by: epic:session-attach
- informed_by: epic:role-names
revision: 2
---
## Why

The operator, 2026-10-17: "do you expect an operator of conductor to clone the repo? that's not what I want, they download the binary and then can start using it."

Today an operator needs a checkout and `task` for most of daily use. `Taskfile.yml` holds the operator's verbs: `sessions` (:14), `conductor:start`, `conductor:attach`, `conductor:restart` (:31-153), `dev:start`, `dev:attach` (:154-198), `trust` (:199), `agents:link` (:204), `dashboard` (:253), `dashboard:stop` (:291), `status` (:318), `profiles` (:329) and `measure` (:334); the broker is the same kind of verb once it ships (docs/design/roles-and-surfaces.md section 6.2). `profiles` copies `.agents/*.md` out of the checkout into the config's profiles directory (Taskfile.yml:329-332), so a binary alone has no profile to start a session from. The guides tell the reader to run these tasks (`website/docs/guides/start-conductor.md`, `read-the-dashboard.md`, `first-task-existing-repo.md`, `troubleshoot.md`, and others), and `website/docs/getting-started.md:35` offers `task install`.

Part of this is done: `conductor session start|stop` (crates/conductor/cli.yaml:306-314), `conductor trust` (:268), `conductor status` (:354), `conductor upgrade` (:336), `conductor dashboard serve` (:184) and `conductor init` (:274) exist as commands. The rest lives in Taskfile shell, which cannot ship in a release archive.

## Outcome

An operator runs conductor from the released binary alone. With the archive unpacked on `PATH` and no checkout, no `task` and no Rust toolchain, every operator verb has a `conductor` command: install or refresh the profiles, start, restart, attach to and list sessions, start and stop the dashboard and the broker as systemd user units, trust the workspaces, read the status. No operator page in `website/docs` tells the reader to run `task ...` or to clone the repository, and a CI job proves the path from an empty home.

## Decision: a new epic, beside `epic:quick-start`

`epic:quick-start` is the first five minutes on a demo and ends at `conductor init`. This epic is the rest of the operator's life with the binary: running the real roles, the dashboard and the broker over time. It consumes `story:install-one-liner` and `story:init-defaults`, adds nothing they own, and its CI job (`story:binary-only-ci`) is the long-path counterpart of `story:quick-start-docs-test`. `epic:session-attach` owns `conductor session attach` and `session list`; this epic uses them and does not model them again (see `story:session-commands-complete`).

## Decision: Taskfile verbs become wrappers, not deletions

A contributor and conductor-dev keep typing `task conductor:restart`. Each operator task stays as a one-line call of the command that replaces it, so no session profile, doc or habit breaks in one release; `story:operator-tasks-retired` pins that the Taskfile holds no operator logic and the docs stop naming the tasks.

## Decision: the binary is the source of the profiles

The profiles are compiled in at build time, so the installed binary and its profiles can never disagree (the property `story:installed-profiles` wanted from the copy step). `conductor upgrade` refreshes them in the same step.

## Stories

`story:profiles-embedded`, `story:session-commands-complete`, `story:dashboard-broker-units`, `story:operator-tasks-retired`, `story:binary-only-ci`.

Order: `profiles-embedded` first; `session-commands-complete` and `dashboard-broker-units` after it; `operator-tasks-retired` and `binary-only-ci` last.

## Out of scope

Windows; package managers; an installer that edits shell rc files; developer tasks (`check`, `ui`, `regen`, `docs-generate`, `docs-check`, `install`); the five-minute demo (`epic:quick-start`); the contents of the profiles (`epic:role-names`).
