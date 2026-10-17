---
format: aep.planning-md/3
id: story:binary-only-ci
kind: story
status: draft
title: CI runs init, session start, dashboard start and status from the release archive in an empty home
relations:
- decomposes: epic:binary-only-operator
- depends_on: story:profiles-embedded
- depends_on: story:session-commands-complete
- depends_on: story:dashboard-broker-units
- informed_by: story:quick-start-docs-test
revision: 2
---
# Story: binary-only-ci

## Outcome

A CI job installs the release archive into an empty home with no checkout and runs `conductor init`, a session start on a fake harness, `conductor dashboard start` and `conductor status`, and fails when any step needs a file of the repository. The binary-only path cannot rot.

## Context

- The workflows are in `.github/workflows/`; the checks run in `check.yml`, every action is pinned by commit, and the release job holds the only write authority (.github/workflows/release.yml:1-17).
- `story:quick-start-docs-test` runs the quick start page's commands against the newest published release; this job covers the longer operator path and runs on the archive a change would release, so it can fail before a release rather than after.
- The release archive is `conductor-<version>-<target>.tar.gz` with `SHA256SUMS` (website/docs/getting-started.md:35-58).
- A fake harness already exists for the Taskfile tests: a `claude` stand-in that logs its working directory and arguments (crates/conductor/tests/taskfile.rs:1-14).
- Shared surface: `.github/workflows/check.yml` with `story:quick-start-docs-test`; one job each, no ordering edge, the merge cost is accepted.

## Acceptance

1. A script `scripts/binary-only-check.sh` takes the path of an archive and an empty directory, unpacks the archive into it, sets `HOME` to a second empty directory, puts the unpacked binary and a stand-in `claude` first on `PATH`, and runs, from a working directory outside the repository: `conductor init`, `conductor profiles install --check`, `conductor session start --role director`, `conductor dashboard start`, a request to the dashboard's `/api/board`, `conductor status`, `conductor dashboard stop`, `conductor session stop --role director`; it exits 0 only when every step does.
2. A negative case runs the script against an archive built with its embedded profiles removed and sees it fail at `profiles install --check`; the case is a test of the script, run in the job.
3. `.github/workflows/check.yml` has a job `binary-only` that builds the release archive for the runner's target from the commit under test (the packaging step of release.yml, not a download) and runs the script; the workflow's final gate requires the job.
4. The job is green on the commit that adds it, and the pull request records the red run with the embedding removed (item 2) as evidence.
5. The job prints each step's command and exit code, and uploads the dashboard's log on failure.
6. `website/docs/develop/` says what the job covers and how to run the script locally.

## Out of Scope

A real model call or a real forge; macOS runners (Linux target first); testing the install one-liner (`story:install-one-liner`) or the quick start page (`story:quick-start-docs-test`); the broker (a step is added once `story:dashboard-broker-units` ships it).

## Ambiguities

- Decided: the job builds the archive from the commit rather than downloading the newest release, because the point is to fail before the release; `story:quick-start-docs-test` covers the published one.
- Open: whether the fake harness is the `claude` stand-in of the Taskfile tests, shared as a script, or a second one; as written it is shared so the two cannot disagree.
