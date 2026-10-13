# Wave 20

Coordinator: conductor-dev. Approved up front (rules item 99). One managed tree,
`conductor-wave-20`, branch `wave/20`. Target: v0.8.2. One pull request. The stories come from the
engineering team's cases of 2026-10-13 and conductor's request about gc under load.

## Units

| unit | story |
|---|---|
| A | install-hints |
| B | guard-git-c-outside |
| C | gc-archive-measure |
| D | run-isolated-guide, why-and-how-walkthrough, agents-link-local |

Also: the `worktree` pin moves to 0.18.1 (the dependency upgrade of 2026-10-13).

## Deviations

- The integrated head was compiled, linted and tested locally in a build slot before the push
  (rules item 112): clippy clean; five tests failed and were fixed (three `agents:link` tests of
  the old behaviour, the shell-quoting check on `REPO`), and one guard-hook test timed out under
  load and passed on the rerun.
- `adv_guard.rs` Finding 7 (`GIT_DIR`, `GIT_WORK_TREE`) moved from an accepted limit to a denial.
- The gc archive bound is the environment variable `CONDUCTOR_GC_ARCHIVE_BOUND` (seconds, default
  1800), not a flag.

## Outcome

`task check`, documentation and the Gates job green in CI on `c9cb767c4`
(https://github.com/beyond10x/conductor/actions/runs/38423065030); six stories moved to
`implemented` on that record. The first CI run failed only on example ids in two new pages, which
are now placeholders. Released as v0.8.2.
