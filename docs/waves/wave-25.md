# Wave 25

Coordinator: conductor-dev. Approved up front (rules item 99). Branch `wave/25`. Target: v0.10.3.
Source: conductor's report of controllers whose first report line was refused.

## Units

| unit | story |
|---|---|
| A | report-verb-punctuation |

Also: a reservation short only of live reservations waits for them (up to an hour) and raises no
operator item; the shortfall item writes the command with `~` paths. That change was compiled and
tested in CI only: the local build was refused for disk by the installed v0.10.2.

## Outcome

`task check`, documentation and the Gates job green in CI on `a2fa02d08`
(https://github.com/beyond10x/conductor/actions/runs/38456081103). Released as v0.10.3.
