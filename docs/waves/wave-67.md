# Wave 67 (2026-11-20: a rate-limited forge read waits; the box's records commit finds its bot key)

The twelfth wave of the goal that fixes the friction the agent box filed: #241 and #242. It forks
from wave 66's head. The wave before is [wave 66](wave-66.md). The critic panel's result:
`review-result:w67-critic-panel`.

Width: two units, disjoint in scope (collectors and their design note; the records commit, doctor
and the box guide). `collector-failure-stays-its-own`, which takes the half of the superseded
retry story that is not a retry, is drafted for a later wave. Rust builds run one at a time (one
lock, one build per box). Both units change the planning store, so the generated status page is
regenerated after the second merge rather than merged by hand.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | forge-rate-limit-retried | #241 | building |
| U2 | box-records-commit-config-home | #242 | building |

## Outcome

Written when the wave merges.
