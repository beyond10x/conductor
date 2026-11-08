# v1 wave 4: federated views, agent kinds, secrets as slots

Wave 4 of the v1 plan ([`v1-plan.md`](v1-plan.md)). Three stories, and the fixes from the
adversary pass on wave 3. `wave/v1-4` is cut from `wave/v1-3` after its last merge; wave 3 merged
into `v1` as pull request 111.

## Units

| unit | story | files (scope) | branch |
|---|---|---|---|
| K | `story:agent-kinds` | a new `src/agent/`, `session.rs`, `doctor.rs`, the agent verbs in `cli.rs` and `cli.yaml`, `spec/domains/agent.yaml` | `impl/agent-kinds` |
| S | `story:secrets-source` | a new `src/secret.rs`, `store/secret.rs`, `daemon/mod.rs`, `doctor.rs`, the secrets crates in `Cargo.toml`, `spec/domains/secret.yaml` | `impl/secrets-source` |
| F | `story:federated-views` | `daemon/api.rs`, a new `daemon/federate.rs`, `dashboard/agents.rs`, `status/federated.rs`, `spec/domains/config.yaml`, `instance.yaml` | `impl/federated-views` |
| X | adversary findings on wave 3 | `daemon/upgrade.rs`, `upgrade.rs`, `config.rs`, `config/{put,convert,effective}.rs`, `daemon/reconcile.rs` | `impl/w3-fixes` |

K and S started from `wave/v1-3` before its last unit (instance-api) finished; F and X started
from `wave/v1-4` after K and S merged. Merge order: K, S, F, X.

## Decisions

The coordinator's stage-1 decisions with safe defaults are in the stage-1 notes of this wave; the
ones that changed what a story's acceptance says:

- The keychain stays the default secrets source; the daemon refuses to start only on a source it
  cannot use.
- `secret put` on an existing name is refused unless `--replace` is given.
- `ask` is refused `SupervisorNotRunning` until the host roles run (wave 5).
- An unreachable instance reports its last-seen time from the daemon, not a mirror.
- Agent attach over HTTP answers a named 501; `conductor agent attach` attaches on the host.
- `placement` stays a key per instance, not at every level.
- `host-roles-by-daemon` moved to wave 5, after the local substrate placement.

## Outcome

| unit | result |
|---|---|
| K agent-kinds | the `Kind` trait, `claude-code` wrapping today's launcher with the same argv, a run-to-exit kind in the tests, `conductor agent` verbs, the `Starting` state |
| S secrets-source | `daemon.secrets` through the secrets crates; resolution at box open with one slot; `secret put`; the secrets view without values |
| F federated-views | the union views with unreachable instances as rows, the events stream, `GET /v1/secrets`, the agent routes deliver and restart, federated `conductor status` |
| X wave 3 fixes | upgrade tags checked before any path; passwords refused in every URL; `config put` under a lock; migrated host roles reach their instance role; first start pins a copy of the daemon's binary |

Deviations from the plan:

- After S merged beside instance-api, `no_organization`'s allowance array was one entry short and
  a secrets test waited for the old daemon log line; one commit fixes both.
- The documentation build failed on pull request 111: a status entry linked the removed
  `dashboard serve` section. It links `instance serve` now.
- `config put`'s lock file `.<file>.lock` stays beside the config file.
