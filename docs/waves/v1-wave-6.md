# v1 wave 6: the remote substrate daemon

Wave 6 of the v1 plan ([`v1-plan.md`](v1-plan.md)), its last. One story. `wave/v1-6` is cut from
`wave/v1-5` after its last merge; wave 5 merged into `v1` as pull request 113.

## Units

| unit | story | files (scope) | branch |
|---|---|---|---|
| Q | `story:placement-substrate-remote` | `src/placement.rs`, `daemon/{reconcile,api,federate}.rs`, `src/instance.rs` (the relay), `config.rs`, `spec/domains/placement.yaml`, `config.yaml` | `impl/placement-substrate-remote` |
| C | CI failures on `v1` after wave 5 | `tests/adv_cli.rs`, `tests/store_tree.rs` and its fixture | `fix/v1-ci-2` |

## Decisions

- Substrate 0.16.1's SDK reaches a remote daemon over TLS 1.3 with configured trust roots and a
  bearer; the placement names both, the bearer as `secret:<name>`.
- The SDK forwards no socket into a box, so conductord reaches a remote runtime through `conductor
  instance relay`, one pipe session per request; the remote host carries conductor, the harness
  and tmux at the control host's paths.
- A silent remote makes the instance `Unreachable`, a row and no failed pass; no second box opens.
- No synced copy and no `instance restore` (design § 8 and § 13).

## Outcome

| unit | result |
|---|---|
| Q placement-substrate-remote | a box on a remote substrate daemon; the relay for the proxy and the federated views; `Unreachable` for a silent remote |
| C CI fixes | the tuner view among the implemented leaves and in the store fixture |

Deviations from the plan:

- The `pod` adapter in a cluster and a real remote substrate daemon are runs by hand, not part of
  the gate; attach over HTTP answers a named 501 for remote and local instances alike.
