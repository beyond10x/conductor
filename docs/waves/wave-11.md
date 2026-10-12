# Wave 11

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Approved by the operator's word on
2026-10-12 ("dispatch another 3 /aep:wave") within the standing approval for waves 03–11, its last
wave. The approval covers one commit per unit, the merge of `main` into the branch, the closing
store commit and the merge into `main` once its pull request's `task check` and Gates jobs are
green.

One managed tree, `conductor-wave-11`. No local build; CI gated every unit; `spawn` had a security
review.

## Selection

From today's store review: `spawn` (rewritten first), `watch-disk-reading` (narrowed first) and
`session-envelope-model`. The envelope model shares the specification and the generated model with
`spawn` and would have waited for its review; it moves to the next wave.

## Units

| unit | story | commit |
|---|---|---|
| U1 | watch-disk-reading (the watch reads free disk again after a jump of more than 20G) | `0d3031e` |
| U2 | spawn (spec first: `conductor spawn` starts a controller session from code) | `35ac23c`, review fixes `f4825aa` |

## Review

The `aep:security-reviewer` pass on `spawn` found five gaps the change introduced, all fixed: a
relative config path reached the controller unresolved; a records directory under the checkouts
root could be started as a controller; a checkout linking out of the root was launched; a failed
launch could leave a live session the store called stopped; the profile's manual fallback was wider
than `spawn`'s own checks.

## Outcome

`task check` and the Gates job green in CI at `f4825aa` (https://github.com/beyond10x/conductor/actions/runs/38306408811). Stories moved to `implemented` on recorded evidence.
