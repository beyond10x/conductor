# Wave 58 (2026-11-18: limit-aware token reassignment, a box's disk path)

The third wave of the goal that fixes the friction the agent box filed. Forked from wave 57's branch
`wave/2026-11-18-conductor-57` at `da5369da`; it goes to `main` after waves 56 and 57. The wave
before is [wave 57](wave-57.md).

Width: two units, run one after the other. The host ran out of memory and restarted; the operator
allows one Rust test build at a time until further notice. The other drafted stories share
`run.rs`, `tests/run.rs` or `token.rs` with U1.

## Units

| unit | story | issue | waits for | stage |
|---|---|---|---|---|
| U1 | limit-aware-token-reassignment | #206 | wave 57 U1, U2 of this wave (build slot) | merged into the wave branch (78b90c40); its two `tests/run.rs` acceptance tests need a socket and wait for CI |
| U2 | box-disk-path-default | #201 | nothing | merged into the wave branch (3efc0491) |

## Blocked

- `decision-blocker:wave-delivery-identity`: the push, the pull request and the merge wait on the
  bot identity `main` requires.

## Outcome

To be written when the wave merges.
