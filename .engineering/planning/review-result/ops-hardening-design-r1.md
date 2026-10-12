---
format: aep.planning-md/3
id: review-result:ops-hardening-design-r1
kind: review-result
status: active
title: Design critic, round 1, epic operating-loop-hardening
relations:
- reviews: epic:operating-loop-hardening
revision: 1
---
approve

The nine stories form a sound set. Each decomposes the epic, none depends on another, and no acceptance line reads state that a sibling creates.

Three things I checked that could have been defects:
- **Guard stories.** `story:guard-store-migrate` and `story:operator-decision-guarded` both touch `crates/conductor/src/guard/rules.rs` and `crates/conductor/tests/guard_rules.rs`. They close different holes. The first adds the missing `store` group to the guard's command groups (`GROUPS`, with `READ_LEAVES` as the allowlist). The second changes how conductor's own `record-operator-decision` call is judged. The controller denial for that command already exists at `crates/conductor/tests/guard_rules.rs:1727`. Neither is half of the other's abstraction. Whether they can run at once is the parallel-safety critic's question.
- **Taskfile stories.** `story:restart-current-profile`, `story:session-tasks-follow-prefix` and `story:agents-link-user-scope` share `Taskfile.yml` and `crates/conductor/tests/taskfile.rs` but edit separate tasks. That is the same parallel-safety question.
- **`story:public-text-check` against the other stories.** It scans files that `story:ci-full-gate`, `story:single-conductor-lease`, `story:operator-decision-guarded`, `story:init-records-without-origin` and `story:agents-link-user-scope` edit. The only coupling is that those edits must not add a forbidden pattern, and none of them plans to. Today no tracked `handoff-*` file exists under `docs/`, and the only `DEC-` ids are in `website/docs/guides/read-the-dashboard.md` (lines 51–80), the one place the story allows them in a fenced block.

**What I read:** 9 stories and the epic with `aep plan artifact show`, plus `aep plan artifact relations`, `aep plan artifact graph` and `aep plan artifact validate`. Validate reported "valid". Its only notes were five unrelated stories closed on an assertion. I walked the whole store graph, not only the nine ids. The nine have one `decomposes` edge each to the epic and no `depends_on` edge among themselves, so there is no cycle and no chain. I traced 3 acceptance reads of state that siblings write, and no edge is needed for any of them: the `public-text-check` scan over files siblings edit, the lease claim state read by the guard and watch inside `story:single-conductor-lease`, and the decision evidence read inside `story:operator-decision-guarded`.

**What I could not establish:**
- **Out of my lane (scope):** `story:operator-decision-guarded` needs changes to the decision command and its record, `decision show`, the dashboard and `tests/decision.rs`. Its Scope section lists only the guard rules, the guard test and one website page.
- **Out of my lane (acceptance):** `story:public-text-check` says "a path under `/home/`". A plain substring match would hit `docs/analysis/2026-10-09-substrate-sessions.md:57`, which contains `/workspace/home/`.
- **Out of my lane (acceptance):** `story:agents-link-user-scope` asks `AGENTS.md` to say what the task still serves, but `AGENTS.md:177` already marks it legacy and says it serves none of the sessions.
- **Unease, not a finding:** `story:session-tasks-follow-prefix` bundles three independent fixes: session-name matching in the tasks, the dashboard threshold, and one message wording. It would also cost nothing to split.
- **Unease, not a finding:** `story:single-conductor-lease` does not say how a legitimate `task conductor:restart` interacts with a claim while the old pid is still exiting.
- **Unease, not a finding:** `story:ci-full-gate` installs tools at the `b10x.toml` pins, while `story:single-conductor-lease` validates with the "newest `ess`". I did not check whether the two can differ.

```findings
[]
```
