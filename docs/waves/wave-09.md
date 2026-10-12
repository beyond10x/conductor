# Wave 09

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Epic: `operating-loop-hardening`.
Approved by the operator's word on 2026-10-12 ("dispatch another 3 /aep:wave") within the standing
approval for waves 03–11. The approval covers one commit per unit, the closing store commit and the
merge into `main` once its pull request's `task check` and Gates jobs are green.

One managed tree, `conductor-wave-09`. Units ran in sequence (they share the specification, the
generated model and `cli.rs`), each written without a local build and gated by CI; the guard unit
had a security review.

## Units

| unit | story | commit |
|---|---|---|
| plan | the store review's moves, epic `disk-and-gate` | `1c9311a` |
| U1 | operator-decision-guarded (spec first) | `bd072ed` |
| U2 | single-conductor-lease (spec first: the `session` domain) | `ba96ec7`, `5ab6674` |
| U3 | lease-guard, with the review's F1 fix and the fixture exemption | `bd11208` |

## What happened on the way

- A store-fixture commit added two lease records whose digests the organization's secret scan
  flags on new lines; the branch was rebuilt without it (`wave/09b`, pull request 30, replacing
  pull request 28), and the lease view is exempt from the fixture, covered by `tests/session.rs`.
- The first cut of U3 read the lease from the store on every conductor write (about a second each);
  it reads a small file the claim writes instead.

## Review

The `aep:security-reviewer` pass on U3 found two gaps the change introduced: a second conductor
could disable the lease file with a shell command the guard does not count as a write (fixed), and
the holder is identified by session id only, so a second process resuming the holder's own session
writes freely (kept as an ignored test, owned by `story:lease-by-process`, stated on the limits
page). Its notes on the pid the hook records, a reused pid and a missing config are stated on the
limits page.

## Outcome

`task check` and the Gates job green in CI on `bd11208`
(https://github.com/beyond10x/conductor/actions/runs/38299247797). Stories moved to `implemented`
on recorded evidence.
