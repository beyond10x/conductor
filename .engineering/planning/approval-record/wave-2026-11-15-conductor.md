---
format: aep.planning-md/3
id: approval-record:wave-2026-11-15-conductor
kind: approval-record
status: approved
title: 'Conductor wave 2026-11-15: a boxed instance runs its agents on a subscription token'
relations:
- decides: story:box-subscription-token
revision: 3
transitions:
- {from: "draft", to: "approved", at: "2026-11-15T07:51:40Z", actor: "human:agent", revision: 3, executor: "human:timo"}
---
# Conductor wave 2026-11-15: a boxed instance runs its agents on a subscription token

## Approval

The operator approved this wave on 2026-11-15, in the session that built it: commit and push the
change on an integration branch with a planning item for every change, without a release.

## Units

| unit | artifact | note |
|---|---|---|
| U1 | `story:box-subscription-token` | the runtime in a box reads its model-token slot and closes the descriptor; a `subscription_token` there reaches the harness as `CLAUDE_CODE_OAUTH_TOKEN`; validate no longer refuses the kind at a box placement; `api_key` and the token pool unchanged |

## Terms

- **Gate:** the pull request's checks; nothing merges red and no check is overridden.
- **Release:** none in this wave; the release decision is separate.
- **Evidence:** `tests/config.rs`, `tests/placement.rs` and `tests/token_pool.rs`; the local run
  recorded on the story.
