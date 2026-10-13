# Wave 12

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Approved by the operator on
2026-10-13 ("I approve it", answering the proposal of `lease-by-process` and
`session-envelope-model` after the standing approval for waves 03–11 was used up). The approval
covers one commit per unit, the closing store commit and the merge into `main` once its pull
request's `task check` and Gates jobs are green.

One managed tree, `conductor-wave-12`, branch `wave/12`. The two units touch disjoint files and run
in parallel without a local build; CI gates them; the guard unit gets a security review.

## Units

| unit | story | stage | commit |
|---|---|---|---|
| U1 | lease-by-process | running | — |
| U2 | session-envelope-model (spec first) | running | — |
