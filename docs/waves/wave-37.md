# Wave 37 (the operator's surface first)

Coordinator: conductor-dev. Approved up front (rules item 99). Integration branch `wave/37`. Target:
v0.15.0. Source: the operator, 2026-10-14 ~23:0xZ: the dashboard is slow and he cannot decide on
one page. Measured on the live dashboard:
- `/api/board`: 18.2 s;
- `/api/health`: 15.7 s;
- `/api/repositories`: 26.0 s.

Re-ordered that night: the dashboard and decisions stories come first, and the rest of session
confinement moves to the next wave.

## Units

| unit | story | note |
|---|---|---|
| U1 | sandbox-spec | done |
| U2 | session-envelope-seam | done |
| U3 | session-limits-systemd | done; two live runs on this host, the second all pass (`docs/` live-run record) |
| U7 | decisions-notify | done |
| U8 | dashboard-fast | done; merged with U9 (`/api/decisions` and `/api/unanswered` served from the shared memo) |
| U9 | decisions-page | done |
| U10 | decisions-batch-answer | in work; adversarial review of its write path before the gate |
| U11 | guard bot release route | in work on `wave/37-guard`. Operator decision 2026-10-15, option A of conductor's NEED N-20261014-28: release and CI re-run commands through the bot route, for the session's own repository only |

Moved to wave 38: session-writes-bwrap, session-forge-credentials, confined-tutorial.

## Notes

- U3 release gate: the instance's config names `confinement.provider: systemd`, so 0.15.0 is
  installed only after a live run on this host passes; the second run passed. Still open from U3:
  - the first prompt is sent as a bracketed paste (unverified for a multi-line charter);
  - `set-property --runtime` on an inactive slice;
  - an empty `CPUWeight=`;
  - `stop` leaves the session's FIFO and typescript;
  - `strict` stops every systemd session until a write fence exists (U4).
- U10's design: one composite spec command that records the operator decision and answers the
  request in one store open, checking the choice against the request's kind and options. The page
  writes with a per-process token, JSON only and from the dashboard's own origin. The guard refuses
  a session's post to the answer route, which is a speed bump, not a boundary, and the security
  pages say so.

## Outcome
