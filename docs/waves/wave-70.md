# Wave 70 (2026-11-20: a waiting unit gets the freed place; a secret refresh is a credential request; JSON input for two decision commands)

The fifteenth wave of the goal that fixes the friction the agent box filed: issues #245 and #246.
It forks from `main` after release 1.20.1 and builds while wave 69 builds. The critic panel's
result: `review-result:w70-critic-panel`.

Width: three units, disjoint in scope (the loop's spawn order; the decision classifier; the two
decision commands' JSON input). Rust builds run one at a time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | blocked-repository-yields-place-to-waiting-unit | #245 | merged |
| U2 | secret-refresh-classified-credential | #246 | merged |
| U3 | decision-request-input-json | #246 | merged |

## Outcome

U1 (#245): when places under `controllers.max_working` are short, a repository whose every open
dispatch is `Blocked` gives its place to a waiting unit of work: always to the unit its own
`[NEED]` opened, and on a respawn to any unit that waits. A unit waits only while the controller
limit refused it and its repository is not stopped at a usage limit, so a unit refused for a
lasting reason keeps no place; with a place to spare, the repository restarts as before. A restart
that yields writes one line and the tick goes on.

U2 (#246): a request to refresh, update, replace, reissue or upload a secret is class H under the
rule `credential`, and `decision handle` escalates it instead of letting a decider answer.

U3 (#246): `decision raise-decision-request` and `decision escalate-request` read their input
from one JSON object with `--input-json -`, each value checked by its flag's own check.

An adversary review of U1 found that a yield could strand a repository while places were free,
and that a unit refused for a lasting reason kept its requester waiting for ever; both are fixed,
each with a test. In the box: clippy and the decision suites; the run suite binds a socket, and
CI proves it.
