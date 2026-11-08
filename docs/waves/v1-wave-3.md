# v1 wave 3: the instance API, the config levels, upgrade with rollback

Wave 3 of the v1 plan ([`v1-plan.md`](v1-plan.md)). Three stories, each on its own branch from
`wave/v1-3`, which is cut from `v1` after wave 2 (e65918700).

## Units

| unit | story | files (scope) | branch |
|---|---|---|---|
| A | `story:instance-api` | a new `src/instance.rs`, `daemon/api.rs` (router, proxy, operator admission), the dashboard app's route module and `dist/`, `spec/domains/operator.yaml`, `observation.yaml`, `config.yaml` | `impl/instance-api` |
| C | `story:config-globals` | `config.rs`, a new `src/config/{convert,effective,put}.rs`, `store/config_revision.rs`, `spec/domains/config.yaml`, `docs/config.md` | `impl/config-globals` |
| U | `story:instance-upgrade` | a new `daemon/upgrade.rs`, `daemon/reconcile.rs`, `daemon/units.rs`, `upgrade.rs`, `spec/domains/instance.yaml` | `impl/instance-upgrade` |

## How the collisions were carried

All three ran at once. `daemon/api.rs`, `config.rs`, `cli.rs`, `cli.yaml` and the config and
instance domains were shared. Merge order was the order the units finished: C, then U, then A.
A merged the other two and moved U's upgrade route and C's `put_config` onto its router, behind
its operator admission.

## Gate

Each unit ran only the checks its change touched (the plan's rule since this wave): the
specification, the regeneration, the test files it added or changed and the unit tests of its
modules. The full gate runs once, in CI, at the end of the plan.

## Outcome

| unit | result |
|---|---|
| C config-globals | role over instance over global over default; `config show --effective`; `config migrate` to `conductor.config/2`; `config put` validates, writes, commits and records a ConfigRevision; secret-bearing keys take only `secret:<name>` |
| U instance-upgrade | `POST /v1/instances/{name}/upgrade` and `conductor instance upgrade`: fetch, install, doctor, restart, rollback unless the runtime answers within `daemon.upgrade_timeout` |
| A instance-api | `conductor instance serve` answers `/v1/` on the instance's socket; conductord proxies it, serves the app on loopback and TLS, and admits every write by operator and grant |

Deviations from the plan:

- Merge order C, U, A: the order the units finished.
- The spec inputs did not list U's rollback scenario; one commit on `wave/v1-3` lists it.
- `instance_names_unique`'s migrate case expected v0's line-preserving migrate; it now expects
  the `conductor.config/2` conversion, which rewrites the file whole and keeps the old one as
  `<file>.config-1.bak`.
- `host-roles-by-daemon` moved from wave 4 to wave 5, after the local substrate placement, which
  design § 14 orders before host roles.
