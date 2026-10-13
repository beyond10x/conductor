# Wave 12

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Approved by the operator on
2026-10-13 ("I approve it", answering the proposal of `lease-by-process` and
`session-envelope-model` after the standing approval for waves 03–11 was used up). The approval
covers one commit per unit, the closing store commit and the merge into `main` once its pull
request's `task check` and Gates jobs are green.

One managed tree, `conductor-wave-12`, branch `wave/12`. The two units touch disjoint files and ran
in parallel without a local build; CI gated them; the guard unit had a security review.

## Units

| unit | story | commit |
|---|---|---|
| U0 | the newest tooling pins (the operator, 2026-10-13: "upgrade to latest tooling"): aep 0.71, ess 0.59, worktree 0.17 in `b10x.toml` and CI | `6521f28` |
| U1 | lease-by-process | `dcc7335` |
| U2 | session-envelope-model (spec first) | `26f0ed5`, reference page `51af5cc` |
| CI | a failed `task check` prints what `conductor-docs` would generate | `7edd9cb` |

## What happened on the way

The config reference page, edited by hand without a local build, differed from the generator in the
order of the role defaults. CI now prints the generator's diff when the gate fails; the page was
taken from that output and checked by its blob hash.

## Review

The `aep:security-reviewer` pass on U1 found no defect: nothing denied before is allowed, the
holder's sub-agents and Monitor share its process, and the pid comes from the hook's own shell.

## Outcome

`task check` and the Gates job green in CI at `51af5cc`
(https://github.com/beyond10x/conductor/actions/runs/38333594459). Stories moved to `implemented`
on recorded evidence; `epic:operating-loop-hardening` and `epic:disk-and-gate` are implemented.

## Rollout

After the merge: install the new binary, then put `--pid "$PPID"` into the instance's own copy of
the conductor settings (the guard command), then restart conductor. In that order: the old binary
refuses the new flag, and the guard command would then deny every tool call.
