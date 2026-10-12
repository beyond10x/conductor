---
format: aep.planning-md/3
id: review-result:ops-hardening-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, epic operating-loop-hardening
relations:
- reviews: story:single-conductor-lease
- reviews: story:guard-store-migrate
revision: 1
---
needs-revision

- story:single-conductor-lease — no acceptance says the conductor role's settings template runs `conductor session claim` at `SessionStart`, so the story can close with a lease no real session ever takes; Design promises the hook and `.claude/conductor-settings.json` is in scope. This repeats round 1, and the revision moved the hook out of the acceptance rather than adding a check — .engineering/planning/story/single-conductor-lease.md:56
- story:guard-store-migrate — "each one is either a read leaf the guard allows or denied to a controller" is true of any command the guard decides, so it cannot fail. The existing `leaves()` in `crates/conductor/tests/guard_rules.rs:1204` calls a leaf a view whenever its only argument is `--format`, which is how `store migrate` passed as a read. The acceptance names no classification independent of that argument shape, so "a new write command cannot pass as a read" is still unchecked. Only "`store migrate` is denied" is checkable — .engineering/planning/story/guard-store-migrate.md:31

Round 1 findings that now hold no longer:
- **Single statement:** ci-full-gate, operator-decision-guarded, session-tasks-follow-prefix, init-records-without-origin and the lease split are each one checkable statement.
- **Website text:** the unnamed "website" clause is gone from operator-decision-guarded.
- **agents-link-user-scope:** it now names the exact phrase to be dropped, "so a session started in another directory finds them", which is still at `Taskfile.yml:164`.

I found nothing to change in the other ten stories, including the new lease-guard, watch-two-conductors and doctor-refusal-wording. Each has one observable outcome with a before and after.

What I read: all 12 ids given, 12 read, with `aep plan artifact show` on each. I also read round 1 and ran `git grep` and `sed` on `guard/rules.rs`, `tests/guard_rules.rs`, `dashboard.rs`, `doctor.rs`, `Taskfile.yml` and `AGENTS.md`.

What I could not establish: scope completeness and cross-story ordering (scope and parallel-safety lanes). The website quotes in doctor-refusal-wording's scope have no acceptance check; that is the drafter's call, so it did not set my verdict.

```findings
- file: .engineering/planning/story/single-conductor-lease.md
  line: 56
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "no acceptance says the conductor role's settings template runs `conductor session claim` at SessionStart, which is what makes the lease get taken in a real session; Design line 48 promises it and the file is in scope"
- file: .engineering/planning/story/guard-store-migrate.md
  line: 31
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'each one is either a read leaf the guard allows or denied' is true of any command, and leaves() in tests/guard_rules.rs:1204 classes a leaf as a view by its --format-only shape, which is how store migrate passed; the acceptance names no independent read classification, so 'a new write command cannot pass as a read' is unchecked"
```
