# Wave 53 (2026-11-15: a boxed instance runs its agents on a subscription token)

The operator approved this wave on 2026-11-15 (`approval-record:wave-2026-11-15-conductor`): one
story, committed on an integration branch, without a release. One pull request from
`wave/2026-11-15-conductor` into `main`, forked from `main` at `c34ce5a4` (release 1.10.0). The
wave before is [wave 52](wave-52.md).

## Units

| unit | story | waits for | stage |
|---|---|---|---|
| U1 | box-subscription-token | nothing | on the wave branch |

## Outcome

- `model_token_kind: subscription_token` validates at a placement that opens a box.
- The runtime in a box reads its model-token slot (`model_token`, or a pool slot
  `model_token_<i>`) once at its start and closes the descriptor, as it does the daemon's token
  slot; before, nothing read it and every process the runtime started inherited it.
- For a `subscription_token`, each agent the pool leases no token starts with the slot's value as
  `CLAUDE_CODE_OAUTH_TOKEN` and a settings copy with no `apiKeyHelper`; the `conductor session
  start` the runtime runs for the maintainer receives it in that variable. `api_key` is unchanged.
- The token pool is unchanged: a pool token reaches the harness through `apiKeyHelper` at every
  placement, not through a box's slot, and names no kind. A kind for pool tokens is left to a
  story of its own.
