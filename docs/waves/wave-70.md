# Wave 70 (2026-11-20: a waiting unit gets the freed place; a secret refresh is a credential request; JSON input for two decision commands)

The fifteenth wave of the goal that fixes the friction the agent box filed: issues #245 and #246.
It forks from `main` after release 1.20.1 and builds while wave 69 builds. The critic panel's
result: `review-result:w70-critic-panel`.

Width: three units, disjoint in scope (the loop's spawn order; the decision classifier; the two
decision commands' JSON input). Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | blocked-repository-yields-place-to-waiting-unit | #245 | building |
| U2 | secret-refresh-classified-credential | #246 | building |
| U3 | decision-request-input-json | #246 | building |

## Outcome

To be written when the units merge.
