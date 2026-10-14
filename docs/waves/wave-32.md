# Wave 32 (sweep transition)

Coordinator: conductor-dev. Approved up front (rules item 99). Branch `wave/32-sweep`. Target: v0.10.7.
Source: conductor's report of live 0.10.5 waiters refused by the 0.10.6 sweep.

## Units

| unit | story |
|---|---|
| A | sweep-legacy-waiters |
| B | the conductor-dev start prompt in `Taskfile.yml` names the hand-over directory the profile names |

## Outcome

`task check`, documentation and the Gates job green in CI on `d15d940a`
(https://github.com/beyond10x/conductor/actions/runs/38493934449), first run. Released as v0.10.7.
