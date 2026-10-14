# Wave 35 (trial findings)

Coordinator: conductor-dev. Approved up front (rules item 99). Integration branch `wave/35`. Target:
v0.13.0. Source: an outside user's trial of 0.12.0 (macOS, a local source, four goals, hands-off),
relayed by conductor, and the same faults seen on this instance.

## Units

| unit | story |
|---|---|
| U1 | status-command |
| U2 | dispatch-done-from-blocked |
| U3 | rules-section-names, controller-trees-by-path |
| U4 | marked-by-session |

## Notes

- Linked worktrees share one `git stash` list: two units popped each other's entries and put them
  back. Implementor briefs now say not to use `git stash`.
- `conductor status` prints waits as none until `story:watch-waits` gives the store a wait record.

## Outcome

Pending.
