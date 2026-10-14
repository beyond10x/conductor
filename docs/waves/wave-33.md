# Wave 33

Coordinator: conductor-dev. Approved up front (rules item 99). Integration branch `wave/33`, which
also carries wave 27. Target: v0.11.0. Units were written in parallel by sub-agents, units B and D
in trees of their own, and land here as local commits; one full gate runs on the integrated head.

## Units

| unit | story |
|---|---|
| A | rules-show, conductor-start-reading, conductor-dev-report-form |
| B | gates-hooks-fresh |
| C | guard-latency-quiet-host |
| D | disk-item-per-fall, disk-item-redacted |

## Notes

- Unit A's profile read `--section conductor` only; the integration made the start read the rules
  every session follows too (`--section "all sessions"`).
- `guard-latency-quiet-host` is measured by dispatching `.github/workflows/guard-latency.yml` after
  the merge; its numbers go in the story's evidence.
- `conductor-start-reading`'s measurement of a restart is recorded in
  `docs/analysis/conductor-start-reading.md` after the next restart.

## Outcome

`task check`, documentation and the Gates job green in CI on the integration head of waves 27 and
33 (https://github.com/beyond10x/conductor/actions/runs/38500142662), first run. Released as v0.11.0.
