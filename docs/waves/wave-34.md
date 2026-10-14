# Wave 34 (wide)

Coordinator: conductor-dev. Approved up front (rules item 99). Integration branch `wave/34`. Target:
v0.12.0. The first wave run as rules item 66 now asks: every ready unit with disjoint scope, five
implementors started in one message, each in its own tree.

## Units

| unit | story |
|---|---|
| U1 | build-run-queue, build-run-stderr-redacted |
| U2 | decision-reads-charter, profile-zsh-shell, conductor-no-poll |
| U3 | wave-width |
| U4 | dashboard-disk-series |
| U5 | test-fixtures-no-nested-git |
| — | the controller profile runs wide, concurrent waves; guard-latency-quiet-host measured |

## Notes

- Free disk on the host stayed near 35G while other repositories built; three units' own 8G gates
  were refused and ran as narrow lanes at 4G. The full gate runs once, on the integrated head.
- `disk_peak_vs_reserved` keeps the series naming of the other series, not the story's hyphens.

## Outcome

`task check`, documentation and the Gates job green in CI on the integration head
(https://github.com/beyond10x/conductor/actions/runs/38512130171), first run. Released as v0.12.0.
