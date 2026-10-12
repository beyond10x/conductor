# Wave 04

Coordinator: conductor-dev. Approved by the operator's standing word for waves 03–11 (rules item
31). Build slot: conductor's DEC-20261011-22 (10G hard cap, a hold under 20G free on `/`, a stop
under 15G).

One managed tree, `wt-8d21329a67c8`, holds every change of the round; units ran one after the
other in it, each by an `aep:implementor`, the target pruned between units.

## Selection

From `aep plan artifact waves --kind story --status draft`: wave 1 (`guard-all-tools`,
`watch-tests-shared-isolation`), plus `workspace-from-config`, unassessed until it was scoped at
the round's start. The scope showed it carried two new spec nouns (doctor's member report, a
per-member snapshot failure), so it was split: `snapshot-absent-member` and the narrowed
`workspace-from-config` went in; `doctor-workspace-members` and `snapshot-member-failures` stay
draft. `guard-read-flag-paths` was filed from conductor's transcript at the round's start and
went in because it lands on the same files as `guard-all-tools`.

## Units

| unit | story | commit on `main` |
|---|---|---|
| U1 | watch-tests-shared-isolation | `bf6022f` |
| U2 | snapshot-absent-member | `2c821c3` |
| U3 | guard-all-tools | `a533ab1` |
| U4 | guard-read-flag-paths | `fce341b` |
| U5 | workspace-from-config | `441b455` |
| review fixes | guard-all-tools, guard-read-flag-paths | `1eecb2f` |

## Review

An `aep:security-reviewer` pass on U3 and U4 found two gaps, both fixed in `1eecb2f` with the
reviewer's cases kept in `tests/guard_rules.rs`:
- blocker, introduced by U4: the read-flag exemption let the shell reach a flag's value without
  naming it again (`$_`, `set --`, `sh -c`, a `bash -s` here-document, `xargs`, `python -c`); 19 of
  20 such writes to a settings or profile file were allowed;
- warning, U3's allowlist: `Agent` and `Task` can set `cwd` or `isolation: "remote"`, moving the
  sub-agent outside the caller's rule or outside the hook; they now have a rule.

## Outcome

Closing gate on `wave/04` at `919c09e` (the tree `main` now carries): `task check` exit 0
(spec valid, conformance 282 of 282, 634 passed, 0 failed, 3 ignored, 5 generated files current);
the Gates scan with the instance's policy, run separately because the session's environment did
not name it: 556 files, exit 0.

The unit commits were first made under the personal identity, against rules item 42. They were
replayed through the bot route onto `wave/04-bot` with the same messages; its tree is identical to
`wave/04` (`git diff wave/04 wave/04-bot` empty), and `wave/04-bot` is what merged.

Not measured: the guard's allow latency on a quiet host (U3 ran under load average 22–30), filed
as `story:guard-latency-quiet-host`.

## Rollout

The instance's settings files (`~/.b10x/conductor/<instance>/settings/`) take the matcher `*` after
`task install`; conductor restarts because its settings changed.
