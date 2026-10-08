---
format: aep.planning-md/3
id: review-result:conf-scope-round-2
kind: review-result
status: active
title: Scope critic, session-confinement, round 2
relations:
- reviews: story:session-writes-bwrap
- reviews: story:session-envelope-model
- reviews: story:instance-usage-metrics
- reviews: story:substrate-gate-provider
- reviews: upstream-blocker:substrate-long-sessions
- reviews: epic:session-confinement
revision: 1
---
needs-revision

Round 1 findings 1–12 are resolved or recorded as decisions, except finding 9, whose title residue remains (finding 3 below), and finding 11, which is half-resolved (finding 4 below). I found no gap: every epic promise has a claiming item.

Findings:

1. `story:session-writes-bwrap` — It supersedes `story:guard-bash-write-targets`, but that story's second promise ("Conductor may write `<records>/docs/analysis/**` with file tools and these Bash forms") has no owner now. The guard's file-tool rule still allows only four record dirs (`RECORD_DIRS`: decisions, dispatches, charters, docs/handoff), so it still denies conductor's Write to `docs/analysis`. A bwrap box does not change that Write denial. The body should either carry the guard change or say it is dropped. — `.engineering/planning/story/session-writes-bwrap.md:10`, `crates/conductor/src/guard/rules.rs:90` (origin: introduced)
2. `story:session-envelope-model` — The epic and ADR ask for `strict` (refuse) or `report` (start, reported as unboxed). The story adds `mode: off`, which "starts and records no measurement", and `provider: none`. `off` contradicts the ADR's "started only when config says unconfined is acceptable, and is then reported as unconfined", and nothing in the parent asks for it. Drop `off`, or have it record `unboxed`. — `.engineering/planning/story/session-envelope-model.md:35` (origin: introduced)
3. `story:instance-usage-metrics` — The title still reads "The dashboard and the watch show CPU, memory and io", while the body says "Not here: a threshold line in the watch". The epic asks only for the dashboard to show usage. This is the residue of round 1 finding 9: the body was fixed and the title was not. — `.engineering/planning/story/instance-usage-metrics.md:6` (origin: carried)
4. `docs/analysis/2026-10-09-substrate-benefits.md` — § 5 now answers the "same way as metaharness" ask. § 4 "Recommendation" was not updated, so its "Adopt from substrate, later" lists only gates. It says to wait for "execs … in an adopted directory", while § 5.2 says the in-process host adopts one now. The recommendation a reader acts on omits the tool route the epic and ADR commit to. This is the unfixed half of round 1 finding 11. — `docs/analysis/2026-10-09-substrate-benefits.md:148` against `:240` (origin: carried)
5. `story:substrate-gate-provider` — The epic promises "builds and tests run as substrate execs". The story's acceptance is one new `gate run` command and one hand-run record. No item routes controllers' gates (`task check`, `cargo test`) through it. The round 1 version routed `Taskfile.yml` through substrate, and the revision dropped that scope silently. The story should name how a session's gate reaches `gate run`, or say that adoption is not claimed. — `.engineering/planning/story/substrate-gate-provider.md:25` (origin: introduced)
6. `upstream-blocker:substrate-long-sessions` — Its only `blocks` edge is to `story:substrate-session-provider`, which is archived. The epic no longer promises Claude Code inside substrate, so this blocker waits on an outcome nobody asks for. Retire it, or re-point it at a live item. — `.engineering/planning/upstream-blocker/substrate-long-sessions.md:8` (origin: introduced)

What I read: 17 artifacts and the benefits page.
- Artifacts: the epic, the ADR, the 14 stories that decompose the epic, the 3 blockers, `story:spawn`, `story:guard-bash-write-targets` and `review-result:conf-scope-round-1`.
- Commands: `aep plan artifact show` on each, `aep plan artifact graph` and `aep plan artifact list`, plus a grep of the guard rules. Round 1 was read only for finding 12 and for the round-1 version of the gate story. The other three conf-*-round-1 reviews I did not read.
- Count: I extracted 16 promises (9 outcome lines, 5 acceptance lines, the "substrate items close later" line, and the operator's benefits ask). 16 trace to an item. Two are narrowed (findings 1 and 5).

What I could not establish:
- Whether workers are in the box. Design § 3 says a worker is a sub-agent of a controller, so it inherits the controller's box. I counted it covered; no item says so.
- Out of my lane:
  - `upstream-blocker:substrate-exec-in-checkout` blocks the gate story, but benefits § 5.2 says the in-process host avoids that blocker (design).
  - The gate story records usage "in the observation store" without scoping `spec/domains/observation.yaml` (design/acceptance).

```findings
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 10
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it supersedes story:guard-bash-write-targets, whose second promise (conductor may write records/docs/analysis with file tools) no item carries; the guard still denies that Write (crates/conductor/src/guard/rules.rs:90 RECORD_DIRS lacks docs/analysis) and bwrap does not change it; the body must carry the guard change or say it is dropped"
- file: .engineering/planning/story/session-envelope-model.md
  line: 35
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic and ADR ask for strict or report only; mode off (starts, records no measurement) and provider none are not asked for, and off contradicts the ADR's 'then reported as unconfined'"
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 6
  category: scope
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "the title still promises 'the dashboard and the watch' while the body excludes the watch and the epic asks only for the dashboard; the body was fixed for round 1 finding 9, the title was not"
- file: docs/analysis/2026-10-09-substrate-benefits.md
  line: 148
  category: scope
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "section 4 Recommendation still lists only gates under 'Adopt from substrate, later' and says to wait for exec in an adopted directory, while section 5.2 (line 240) says the in-process host adopts now and 5.4 names the tools route as the better substrate step; the operator's benefits ask is answered in section 5 but the recommendation was not updated (round 1 finding 11, second half)"
- file: .engineering/planning/story/substrate-gate-provider.md
  line: 25
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic promises builds and tests run as substrate execs; the story delivers one gate run command and one hand-run record, and no item routes a session's task check or cargo test through it (the round 1 Taskfile.yml scope was dropped without a note); name how gates reach the command or say adoption is not claimed"
- file: .engineering/planning/upstream-blocker/substrate-long-sessions.md
  line: 8
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the only blocks edge points at the archived story:substrate-session-provider, and the epic no longer promises Claude Code inside substrate, so the blocker waits on an outcome nobody asks for; retire it or re-point it at a live item"
```
