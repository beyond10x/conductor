# Wave 68 (2026-11-20: a failed collector keeps the snapshot's other rows; a dial test's race)

The thirteenth wave of the goal that fixes the friction the agent box filed: the second half of
#241, and a test that failed once on CI on the same code it passed on. It forks from `main` after
wave 67. The wave before is [wave 67](wave-67.md). The critic panel's result:
`review-result:w68-critic-panel`.

Width: two units, disjoint in scope (the snapshot, its collector runs and their readers; the token
pool's probe and its test). Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | collector-failure-stays-its-own | #241 | building |
| U2 | instance-api-refusal-said-once-flake | - | critic panel |

## Outcome

Written when the wave merges.
