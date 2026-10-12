# Wave 07

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Epic: `operating-loop-hardening`.
Approved by the operator's word on 2026-10-12 within the standing approval for waves 03–11, as
wave 06. The approval covers one commit per unit on `wave/07`, the closing store commit and the
merge into `main` (pull request 25) once its `task check` job is green.

One managed tree, `conductor-wave-07`, branch `wave/07`. Units ran in parallel where their files
were disjoint, without a local build (the host stayed under its build floor); CI's `task check`
was the gate, as in wave 06.

## Units

| unit | story | commit |
|---|---|---|
| U1 | doctor-refusal-wording | `49b5541` |
| U2 | (status page, held from the docs round) | `4e39e7a` |
| U3 | session-tasks-follow-prefix (the dashboard's threshold) | `10f27b8` |
| U4 | init-records-without-origin | `c355b53`, narrowed in `d68cdd7` |
| U5 | instance-scoped-tasks (spec first: `dashboard_port`) | `06ccafc`, docs `d567048`, quoting fix `d38e34c` |
| U6 | agents-link-user-scope | `951984a` |

The two generated pages that a spec or list change touched (`website/docs/status.md` with its JSON,
`website/docs/reference/config.md`) were written by hand from the generator's code, since the host
could not build it; CI's `docs-check` confirmed both byte for byte.

## What CI caught

- U4's first cut skipped the fetch for every member without an origin, which turned ten collector
  tests' expected failures into completed snapshots; it was narrowed to the records member.
- U5's two new Taskfile values reached the shell outside the quoting the Taskfile's own test holds.

## Outcome

`task check` green in CI at `d38e34c`
(https://github.com/beyond10x/conductor/actions/runs/38287160377). Stories moved to `implemented`
on recorded evidence.
