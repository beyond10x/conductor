# Wave 22

Coordinator: conductor-dev. Approved by the operator ("we need some sort of protocol", "the level
of autonomy is not enough, you need to fix conductor"). One managed tree, `conductor-wave-22`,
integration branch `wave/22`. Target: v0.10.0. One pull request.

Source: work across the instance stopped twice on free disk on 2026-10-13, because build slots were
granted by message and nothing freed space under a floor.

## Units

| unit | stories | files |
|---|---|---|
| spec | disk-protocol-spec | `spec/domains/host.yaml` (from the draft), `spec/{system,ess-inputs,components}.yaml`, `crates/conductor/cli.yaml` |
| A | build-run, disk-escalation (the operator item) | `host.rs`, `build.rs`, `store/host.rs`, `cli.rs`, `tests/build_run.rs` |
| B | disk-reclaim, disk-escalation (the watch) | `gc/reclaim.rs`, `store/reclaim.rs`, `watch.rs`, `tests/reclaim.rs` |
| C | guard-requires-build-run, disk-escalation (profiles) | `guard/rules.rs`, `.agents/*.md`, `concepts/disk.md`, design §§ 6, 7, 9 |

Also: `records check` judges only lines from the first one a command wrote; `message
receive-message` accepts `-` as a report's dispatch id, the `ready` verb and a verb followed by a
colon; the macOS provenance troubleshooting row names its cause (an `ess` older than 0.54.0).

## Deviations

- The spec takes `RequestDisk` as a separate step (ess refuses a transition no command takes) and
  binds reclaim as `conductor host reclaim` (`conductor gc` is a leaf).
- The ledger lives in the instance's state directory: two instances on one host do not yet see
  each other's reservations (`story:host-grants`).
- First local run: clippy refused one `sort_by`; the store fixture test needed the two new views
  exempted. Fixed before the push.

## Outcome

`task check`, documentation and the Gates job green in CI on `3a6a16fcc`
(https://github.com/beyond10x/conductor/actions/runs/38443915639), first run; five stories moved to
`implemented` on that record. The branch was rebuilt on `main` as one bot commit
(`wave/22-release`) because a rebase had left a commit with a person as committer. Released as
v0.10.0.
