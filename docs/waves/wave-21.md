# Wave 21

Coordinator: conductor-dev. Approved by the operator ("dispatch this with multiple agents (up to
5) … merge all into integration branch, then 1 PR remotely + cut release"). One managed tree,
`conductor-wave-21`, integration branch `wave/21`. Target: v0.9.0. One pull request.

Source: users' reports of 2026-10-13 (an evaluation run's findings V8-1 to V8-6; review as the
bottleneck; starting on an existing repository).

## Units

| unit | stories | files |
|---|---|---|
| A | records-from-commands | `spec/domains/{dispatch,decision}.yaml`, the recording commands' sources, `records.rs`, `tests/records_log.rs`, `tests/spawn.rs`, design § 10 |
| B | guard-write-targets, report-md-in-records | `guard/rules.rs`, `guard/shell.rs`, `tests/guard_rules.rs`, design § 7, README security section, the guard's page |
| C | review-limit | `spec/domains/config.yaml`, `config.rs`, `status.rs`, `watch.rs`, `dashboard.rs`, the dashboard app and `dist/`, their tests |
| D | worktree-activation (code) | `doctor.rs`, `setup.rs`, their tests |
| E | merge-human, controller-without-store, quickstart-existing-repo, and every story's profile sentence | `.agents/*.md`, `tests/profiles.rs`, `tests/docs_quickstart.rs`, the guides, README first screen, `SETUP.md`, roles page |

The coordinator alone runs `task regen` and `task docs-generate`, once, after A's and C's spec
edits, then compiles and tests the integrated head in one build slot before the one push.

## Plan review

One round of the four plan critics; every finding was answered in the stories (the guard half of
`merge: human` became `story:merge-human-guard`, a draft for a later wave).

## Deviations

- The integrated head was regenerated, compiled, linted and tested once in a build slot before the
  push (rules item 112): clippy clean; one guard test failed (`sed -ni` on a records file was not
  seen as an in-place edit) and was fixed; the reference sample config gained the new key.
- The slot waited for disk; that wait started `epic:disk-protocol` (filed in this wave, built in
  the next).

## Outcome

`task check`, documentation and the Gates job green in CI on `b529bbd74`
(https://github.com/beyond10x/conductor/actions/runs/38436644034), first run; eight stories moved to
`implemented` on that record. Released as v0.9.0.
