# Wave 13

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Approved up front (rules item 99,
the operator 2026-10-12: waves need no per-wave approval). The approval covers one commit per unit,
the closing store commit and the merge into `main` once its pull request's `task check` and Gates
jobs are green.

One managed tree, `conductor-wave-13`, branch `wave/13`. Two units in parallel (disjoint files),
without a local build; CI gates them.

## Selection

- `watch-silent-controller`, broadened: the operator asked on 2026-10-13 why conductor does not keep
  an eye on a working controller; the watch learns to report a live controller whose transcripts
  stay silent.
- `decision-id-free-in-records`: conductor's report of 2026-10-13 that a new decision id repeated a
  hand-written one.

## Units

| unit | story | stage | commit |
|---|---|---|---|
| U1 | watch-silent-controller (spec first: `thresholds.silent_session`) | running | — |
| U2 | decision-id-free-in-records | running | — |
