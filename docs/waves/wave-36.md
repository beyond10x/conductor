# Wave 36 (disk, secrets, usage, red checks)

Coordinator: conductor-dev. Approved up front (rules item 99). Integration branch `wave/36`. Target:
v0.14.0. Source: faults seen on this instance: a disk floor answered by waiting on the operator, a
forge secret that lagged its committed policy, sessions stopped on a usage limit nobody saw coming,
and pull requests reported red without the names of their failing checks.

## Units

| unit | story |
|---|---|
| U1 | hand-item-dry-run |
| U2 | pr-check-names |
| U3 | reclaim-scratch-class-c |
| U4 | policy-secret-drift |
| U5 | usage-probe |

## Notes

- U4 and U5 added the secret-syncs and usage-readings views, which no build before them could
  record, so the store-migration fixture held no rows for them. The committed fixture store is
  inside the Gates baseline and a new line in it trips the secret scan, so the test appends one
  observation of each to a copy of it at run time instead.

## Outcome

