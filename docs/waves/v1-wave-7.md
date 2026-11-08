# v1 wave 7: every placement through substrate

Wave 7 follows the v1 plan ([`v1-plan.md`](v1-plan.md)): it closes the gaps between conductor and
substrate 0.20.0 that wave 5 and wave 6 left, under the rule that every placement goes through
substrate, `systemd` included. `wave/v1-7` is cut from `wave/v1-6` after its last commit; wave 6
merged into `v1` as pull request 114.

## Units

| unit | work | files (scope) | branch |
|---|---|---|---|
| S | substrate 0.20.0: adapters by facts, a slot pool, the 24 h bound; the `systemd` provider removed | `src/placement.rs`, `daemon/reconcile.rs`, `src/systemd.rs`, `src/confine/`, `spec/domains/{placement,config}.yaml`, `Cargo.*`, `b10x.toml` | `impl/substrate-020` |
| R | a shown remote placement loads back | `src/config.rs`, `tests/placement.rs` | `fix/v1-remote-roundtrip` |

## Decisions

- An adapter is the confinement facts it requires, never a name on the wire (substrate ADR 0048).
  The 0.20.0 SDK sends no `sandbox.requires`, so conductor checks the facts against the daemon's
  capability snapshot itself and the start names that snapshot.
- The `systemd` adapter is substrate's: `systemd` requires the `limits` family and
  `whole-tree-kill` on top of `linux`'s facts. conductor's own `systemd-run` provider is deleted
  and resolves to an unserved provider that still stops the units an earlier conductor started.
- The model token reaches a box through a pool of daemon slot files (`daemon.substrate_slots`),
  written just before the box opens and freed when it closes.
- conductord renews a box's leases hourly and restarts its runtime in the same box before
  substrate's 24 h run bound.

## Outcome

| unit | result |
|---|---|
| S substrate-020 | pin 0.20.0; facts refused by substrate's names before anything starts; slot pool; renewal and `RunBound` restart; `confine/systemd.rs` removed |
| R remote round-trip | the YAML `config show` prints for a remote placement loads back and shows the same |

Deviations and gaps:

- Agent sessions inside a box do not start on substrate 0.20.0: a run in a box reaches no daemon,
  and the in-process confinement API has no terminal. Under a substrate placement, `agent attach`
  and `deliver`, the host roles and the maintainer session have no tmux session until substrate
  serves them. Filed in substrate as draft stories; conductor keeps no fallback.
- Writable roots in the SDK are a substrate change; the box request carries its seam.
