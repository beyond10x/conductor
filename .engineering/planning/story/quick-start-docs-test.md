---
format: aep.planning-md/3
id: story:quick-start-docs-test
kind: story
status: draft
title: CI runs the quick start's commands against the release binary
relations:
- decomposes: epic:quick-start
- depends_on: story:quick-start-page
revision: 2
---
# Story: quick-start-docs-test

## Outcome

A CI job runs the quick start's shell blocks, as written, from a clean account against the newest release binary, and fails when a command, a printed line or the dashboard's answer differs from the page.

## Context

- The existing docs tests read a guide and resolve the `conductor <group> <leaf>` commands it names against the CLI tree (`crates/conductor/tests/docs_quickstart.rs`, `docs_forge.rs`, `docs_setup.rs`); they do not run anything, so a wrong flag, a changed output or a missing release asset passes them.
- Workflows are pinned by commit for every action, and the release job holds the only write authority (.github/workflows/release.yml:1-17); the checks run in `.github/workflows/check.yml`, other workflows in `.github/workflows/`.
- The page and its commands: `story:quick-start-page`, `story:install-one-liner`, `story:demo-mode`. The one-liner installs the newest published release, so the job tests the page against what a visitor gets today, which moves on every release.
- `task docs-check` fails a derived page that is stale (Taskfile.yml:380-383); this check is its runtime counterpart.

## Acceptance

1. A script `scripts/quick-start-check.sh` (or a Rust integration test, the implementor's choice, stated in the PR) extracts the fenced `console` and `sh` blocks of `website/docs/quick-start.md` marked as commands, runs them in order in an empty `HOME` in a clean directory, with `PATH` holding only the programs the page names as required, and compares each block's output to the lines the page shows after it (version and timing lines masked by a stated pattern).
2. After the last command the check asks the demo's dashboard, `GET /api/board` and `/api/events` on `127.0.0.1`, and fails unless the sample repository and the `demo` dispatch are in them; it then runs `conductor demo --clean` and fails if the home directory differs from its baseline, which is the home directory as it stands right after the install step (the installed binary is part of the baseline).
3. The job runs on Linux (`x86_64-unknown-linux-gnu`) and macOS (`aarch64-apple-darwin`) runners: on every pull request that changes `website/docs/quick-start.md`, `README.md`'s first section, `crates/conductor/src/init.rs`, the demo's module or `.github/workflows/release.yml`; on a schedule (weekly); and on every tag push. It does not run on `release: published`, which fires after the tag's assets are public. It is a separate workflow file whose actions are pinned by commit.
4. The pull-request run installs the one-liner's archive from the newest release; a second pull-request run sets `CONDUCTOR_INSTALL_BASE`, the variable defined by `story:install-one-liner`, to a local file server holding the binary built from the branch in a release layout, and runs the same page's `conductor` commands against the branch, so a command change is caught before it is released.
5. A negative test: a copy of the page with one command's flag changed, and one with a printed line changed, each make the check fail with the block and the diff in the log (run in the job as a self-test).
6. The job reports the elapsed time of the page's commands and fails over five minutes in total, the promise of the page.
7. The check runs on tag push in `.github/workflows/release.yml`, before the job that publishes the release assets, and a red check fails that workflow. `AGENTS.md`'s release steps (updated by the coordinator) create the Release only after that check is green.

## Out of Scope

Running a live controller in CI (needs `claude` and a login); checking the website build (`.github/workflows/pages.yml` does).

## Ambiguities

- A job that tests the newest release fails when a release breaks the page, but also when GitHub is down; the weekly schedule and the pull-request run share one retry rule, stated in the workflow.
- Whether the macOS runner minutes are worth it for every page change; the Linux run is the gate and macOS runs on schedule and tag push only, if the operator prefers.
