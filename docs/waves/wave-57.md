# Wave 57 (2026-11-18: the host token pool, the merge pipeline relay, a box's memory)

The second wave of the goal that fixes the friction the agent box filed. Forked from wave 56's branch
`wave/2026-11-18-conductor` at `4627d05e`, whose stories #206's pool builds on; it goes to `main`
after wave 56. The wave before is [wave 56](wave-56.md).

Width: three units. The other drafted stories share files with these: `tests/run.rs` and `run.rs`
(dispatch-waiting-reason, idle-controller-yields-place, dispatch-v1-once-per-goal,
resumed-controller-done-held), `watch.rs` and `doctor.rs` (box-disk-path-default), `token.rs`
(limit-aware-token-reassignment and status-token-pool, which wait for U1), and box-tool-roots and
box-registry-egress wait on substrate's mount and route limits.

## Units

| unit | story | issue | waits for | stage |
|---|---|---|---|---|
| U1 | host-token-pool | #206 | wave 56 U1 | building |
| U2 | default-branch-pipeline-wakes-controller | #193 | nothing | building |
| U3 | box-memory-default | #195 | nothing | building |

An adversary review of U1's design and of its token handling runs beside the units.

## Blocked

- `decision-blocker:wave-delivery-identity`: the push, the pull request and the merge of this wave
  and of wave 56 wait on how a boxed controller delivers under the bot identity `main` requires.

## Outcome

To be written when the wave merges.
