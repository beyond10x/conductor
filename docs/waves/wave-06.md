# Wave 06

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Epic: `operating-loop-hardening`.
Approved by the operator's word on 2026-10-12 ("decide on your own how to turn into planning
primitives and then dispatch /aep:wave on it to implement the changes") within the standing
approval for waves 03–11.

The approval covers: one commit per unit on `wave/06`, the closing store commit, the merge of
`wave/06` into `main` (pull request 24) once its `task check` job is green. The release is a
separate step after wave 08.

One managed tree, `conductor-wave-06`, branch `wave/06`.

## Selection

From the plan the critic panel reviewed twice (`review-result:ops-hardening-*-r1`, `-r2`): the four
stories with no dependency on another open story and no shared file among them. Scopes are cited.

## How the units were verified

The host had no disk for builds during the wave (`/` at 19–20G, under the build floor). U1 moved
`task check` into CI, so U2–U4 were written without a local cargo run, in parallel (disjoint
files), and CI's `task check` job was the gate. No separate red run per unit exists in CI: a newer
push cancels an older run. The red evidence is: U1's first CI run went red on a real defect; U2's
positive control was run with each rule disabled (outside cargo); U3's Taskfile logic was run by
hand red then green against fixture repositories.

## Units

| unit | story | commit |
|---|---|---|
| U1 | ci-full-gate | `53b332f`; first run red on a host-PATH test defect, fixed in `53837b2`; green |
| U2 | public-text-check | `e1f8a50` |
| U3 | restart-current-profile | `72ad19f` |
| U4 | guard-store-migrate | `aca8f02`; review cases `12fcea1` |

## Review

An `aep:security-reviewer` pass on U4 found no regression: every verdict the guard gave before is
unchanged except `store migrate`, now denied to a controller. It found one older gap: a `conductor`
run whose group or program word the shell expands, or whose leaf arrives through `xargs` or
`"$@"`, is allowed to a controller. Kept as an ignored test and filed as
`story:guard-expanded-conductor-words`; named on the website's limits page.

## Also in this wave

The conductor profile's class-H file moves to `$TMPDIR/conductor-h/<H-id>.sh`: the guard denied
the earlier path, which lies outside conductor's scratch (reported by conductor). Decided without
widening the guard.

## Outcome

`task check` green in CI at `12fcea1`
(https://github.com/beyond10x/conductor/actions/runs/38283906934). Stories moved to `implemented`
on recorded evidence.
