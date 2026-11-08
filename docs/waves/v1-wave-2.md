# v1 wave 2: the daemon, the shared store, the terminal log

Wave 2 of 6 of the v1 plan ([`v1-plan.md`](v1-plan.md)). Three stories, each on its own branch from
`wave/v1-2`, which is cut from `v1` after wave 1 (551eb2eef).

## Units

| unit | story | files (scope) | branch |
|---|---|---|---|
| D | `story:daemon-reconcile` | a new `crates/conductor/src/daemon/`, `registry.rs` removed, `run/unit.rs`, the `daemon` command in `cli.rs` and `cli.yaml`, `spec/domains/instance.yaml`, `spec/domains/config.yaml` | `impl/daemon-reconcile` |
| S | `story:store-shared` | `crates/conductor/src/store*`, `store/migrate.rs`, the eventlog dependency lines in `crates/conductor/Cargo.toml` (0.8.4 to 0.9.2, which ships `eventlog-sqlite` and `eventlog-postgres`), `spec/domains/store.yaml` | `impl/store-shared` |
| T | `story:terminal-log-name` | `confine/systemd.rs`, `attach.rs`, two test files, help text | `impl/terminal-log-name` |

## The verb's answer

`aep plan artifact waves --kind story --status active`, 2026-11-08, after the scopes were written:
one story per wave (daemon-reconcile, then onboarding-run, then store-shared, then
terminal-log-name), on these collisions:

```text
daemon-reconcile × store-shared on crates/conductor-model, cli.yaml, cli.rs, spec/domains/config.yaml, spec/domains/instance.yaml
daemon-reconcile × terminal-log-name on cli.yaml, cli.rs, confine/systemd.rs
```

`story:onboarding-run` is the v0 line's, delivered on `main`, and not a candidate here.

## How the collisions are carried

All three run at once. Each collision is an addition in a file the others also add to, and none
changes a line another unit changes:

- `cli.rs` and `cli.yaml`: D adds the `daemon` group; S changes the `store migrate` handler's
  target; T changes one help line. Resolved at merge.
- `spec/domains/config.yaml` and `instance.yaml`: D owns them; S reads `daemon.store`, which wave 1
  already declared, and changes no spec file outside `store.yaml`.
- `crates/conductor-model`: generated. Each unit regenerates it; the merge regenerates it once more.
- `confine/systemd.rs`: T renames the recording file; D only calls `start_service` and
  `stop_unit`.

Merge order: T, then S, then D, each followed by `task regen` and the package tests.

## Commits this wave makes

One commit per unit on its branch, their merges into `wave/v1-2`, the closing commit on
`wave/v1-2`, and the pull request into `v1`.

## Outcome

(filled at the close)
