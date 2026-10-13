# Wave 16

Coordinator: conductor-dev, `aep:implementing` 0.22.8, wave mode. Approved up front (rules item 99).
One managed tree, `conductor-wave-16`, branch `wave/16`; implementors write without a local build
and CI gates the round. Epic: `epic:conductor-autonomy`.

## Units

| unit | stories | agent | stage |
|---|---|---|---|
| U1 | prebuilt-binaries | `aep:implementor` | dispatched |
| U2 | status-board | `aep:implementor` | dispatched |
| U3 | resumable-gates, work-beside-long-gate, claims-cite-source, operator-items-age, gc-before-pause | `aep:implementor`, in that order | dispatched |
| U4 | self-arming-restart | `aep:implementor`, after U2 and U3 | waiting |

U1 and U2 and U3 touch disjoint files (`aep plan artifact waves`); U3's stories share
`.agents/conductor.md` and `crates/conductor/tests/profiles.rs` and run one after another; U4 shares
`Taskfile.yml` and `crates/conductor/tests/taskfile.rs` with U2 and the profile files with U3.

Commits the approval covers: one commit per unit on `wave/16`, the closing store commit, and the
fast-forward of `main` once CI is green on the round.

## Outcome

Pending.
