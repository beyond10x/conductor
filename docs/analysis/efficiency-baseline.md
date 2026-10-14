# Efficiency baseline, 2026-10-14

The first reading of the controller-health probe (`epic:controller-health`), 11:31Z, over the
preceding three hours, on one instance: twelve background sessions (conductor, conductor-dev and
ten repository controllers). Aggregates only; the per-session report stays in the instance's
state directory.

| measure | value |
|---|---|
| input tokens, all sessions | 164M (cache reads 99 %) |
| output tokens | 0.36M |
| pull requests merged | 3 |
| first-parent commits on `main` | 9 |
| merged pull requests per 100M input tokens (K14) | 1.8 |
| controllers with no commit and no merge in the window | 8 of 10 |
| builds queued / running on the host | 25 / 2 |
| build-queue wait, median / oldest (K15) | 15 min / 28 min |
| disk reserved over measured growth, median of 24 h | ~88x |
| sessions over the 300k context hand-over threshold at their last turn (K16) | 1, two more within 20k |
| cache writes with the 5-minute TTL (K17) | 70 % |
| tokens rewritten by cold wakes (> 5 min idle) | 0.5M |
| red `main` / red pull requests | 1 / 2 |

Reading: the sessions are cheap per turn (almost all input is cached) but deliver little, because
most controllers were waiting on the host build queue; the queue was slow because reservations
were far above what builds write and because each waiter at its head ran a disk walk under the
ledger lock (`story:reclaim-scratch-class-c`, `story:reserve-from-peak`,
`story:build-run-priority`).

Targets (the instance goal holds them): K14 at least 10 within a week; K15 under 5 minutes; K16 at
zero; K17 under 20 %; every red `main` and red pull request owned within one watch cycle.
