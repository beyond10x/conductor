# Wave 05

Coordinator: conductor-dev. Approved by the operator's standing word for waves 03–11 (rules item
31). Build slot: conductor's DEC-20261012-06 (4G du limit, start at 24G free on `/`, wait before a
step that would leave it under 21G).

One managed tree, `conductor-wave-05`, holds every change of the round; units ran one after the
other in it, each by an `aep:implementor`.

## Selection

From the hand-over `handoff-conductor-dev-2026-10-12b.md`, items 2 and 3, and
`story:class-h-handover`, filed this session from conductor's transcript (the org secret refresh of
2026-10-12, 11:41–11:44Z).

## Units

| unit | story | commit |
|---|---|---|
| U1 | workspace-snapshot-end-to-end | `40a1b9d` |
| U2 | agent-needs-user-setting-source; class-h-handover's profile part | `77410f9` |
| fix | `conductor.example.yaml` validates (found by U2) | `e7312392` |
| U3 | class-h-handover, the guard's text | `5874a37` |

U1's test fails on each of `f2e7959`, `4840a9d` and `b6d6508` reverted, each with the error the live
snapshot gave.

## Decisions

- U2's refusal is the story's rule: a `controller` role that names an `agent` and no `profile`. A
  first build refused any controller role without a profile; the built-in default roles name
  neither, so `config show` no longer round-tripped. Narrowed rather than inventing a default
  profile path (principle 3).
- U3 changes only the text of a denial the guard already gave, so it had no security review.

## Filed

`story:agents-link-user-scope`: `task agents:link` serves no session started with
`--setting-sources project,local`.

## Rollout

`.agents/conductor.md` changed: conductor restarts after `task install`.
