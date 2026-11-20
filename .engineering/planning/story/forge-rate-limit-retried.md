---
format: aep.planning-md/3
id: story:forge-rate-limit-retried
kind: story
status: draft
title: A rate-limited forge read is retried, and a commit's diff is read once
summary: One GitLab HTTP 429 failed the snapshot three times in an hour; the collectors re-read every commit's diff each cycle
tags:
- from-finding
refs:
- provider: github
  reference: beyond10x/conductor#241
relations:
- decomposes: epic:architecture-next
- supersedes: story:collector-retries-rate-limit
scope:
- confidence: cited
  path: crates/conductor/src/collect/gitlab.rs
- confidence: cited
  path: crates/conductor/src/collect/repositories.rs
- confidence: cited
  path: crates/conductor/tests/collect_gitlab.rs
- confidence: inferred
  path: docs/design/conductor.md
revision: 4
---
## Why

Issue #241. One HTTP 429 from a GitLab forge fails the whole snapshot: the cycle's line reads
`snapshot exited 1: ... repositories: the gitlab group <group> on <host>: glab api ... ` three times in
one hour on one host with two instances on one forge, while the forge rate-limited every client
of the account for about six minutes. The collectors spend that budget themselves: each snapshot reads
the diff of every non-maintenance commit of the last 7 days of every project again, every
`cadence.cycle` (10 min there).

On main (release 1.19.2 plus waves 65-66):

- A forge command that fails is run once more at once, and a second failure is the collector's
  error, whatever the forge answered (`crates/conductor/src/collect/repositories.rs:1864-1878`,
  `answer`); a 429 and its `Retry-After` are not read. Every `glab api` read goes through it
  (`crates/conductor/src/collect/gitlab.rs:544-546`, `Forge::get`), and so does every `gh api`.
- An error of one project's read fails the repositories collector, and with it the snapshot
  (`crates/conductor/src/collect/repositories.rs:1904-1946`, `each`: the first error in order).
- `read` asks `projects/<id>/repository/commits/<sha>/diff` for each listed commit on every
  snapshot, with nothing kept between snapshots (`crates/conductor/src/collect/gitlab.rs:1046-1063`),
  although a commit's paths never change.
- The draft `story:collector-retries-rate-limit` asked for the retry alone; this story supersedes it.

Already holds, and stays: a failed snapshot does not stop dispatch. The cycle runs dispatch v1
after a failed `snapshot` step (`crates/conductor/src/run/dispatch.rs:96-122`), and `repository
activity` reads the newest `Complete` snapshot (`crates/conductor/src/repository.rs:70-72`,
`:167-171`), so the issue's third expectation is met; its `spawned none` came from the repositories
waiting for other reasons.

## Outcome

- A forge read (`glab api`, `gh api`) whose failure names HTTP 429 (or 502, 503) is retried up to 3
  times, waiting the `Retry-After` the forge named when the program shows one, else 5 s, 15 s, 45 s;
  each wait capped at 60 s and never past the collector's deadline (`collect::deadline`). Any other
  failure keeps today's one immediate retry.
- A rate-limited read that still fails names the forge, the path and `rate limited (HTTP 429)` on its
  line, so the cycle line says why.
- The paths of a commit read from `repository/commits/<sha>/diff` are kept in the instance's cache
  directory, keyed by forge host, project id and sha; a later snapshot reads a kept commit's paths
  from there and asks the forge only for commits it has not read. Kept entries for commits no longer
  in the 7-day window are dropped when the file is written. A cache that does not read is ignored and
  written anew.
- Design: one paragraph in `docs/design/conductor.md` beside the collectors' bound.

## Acceptance (tests that fail on the current code)

- `tests/collect_gitlab.rs` `a_project_read_answered_429_once_is_retried_and_the_snapshot_completes`.
- `tests/collect_gitlab.rs` `a_429_naming_retry_after_waits_that_long_capped_at_the_deadline`.
- `tests/collect_gitlab.rs` `a_read_still_rate_limited_after_its_retries_names_http_429`.
- `tests/collect_gitlab.rs` `a_second_snapshot_asks_no_diff_of_a_commit_the_first_read`.
- `tests/collect_gitlab.rs` `a_cache_that_does_not_read_is_written_anew_and_stops_nothing`.

## Boundaries

- No new config key: the retry schedule and the cap are constants.
- A collector that fails still fails the snapshot; recording the other collectors' rows from a
  partial snapshot is a separate story.
- The GitHub collector's per-commit reads are not cached here; only the retry applies to `gh api`.
- Forge rate budgets shared with the controllers' and the supervisor's own `glab` calls are out of
  scope.
