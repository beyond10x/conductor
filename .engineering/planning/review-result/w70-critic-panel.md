---
format: aep.planning-md/3
id: review-result:w70-critic-panel
kind: review-result
status: active
title: Critic panel, wave 70 (blocked-repository-yields-place-to-waiting-unit, secret-refresh-classified-credential, decision-request-input-json)
summary: 'Four lenses and a round 2: the #245 story rewritten (need-link yield, respawn-only rule, no tick abort), the two #246 stories approved with edits; all edits taken in; scopes disjoint'
relations:
- reviews: story:blocked-repository-yields-place-to-waiting-unit
- reviews: story:secret-refresh-classified-credential
- reviews: story:decision-request-input-json
revision: 1
---
# Critic panel, wave 70 (blocked-repository-yields-place-to-waiting-unit, secret-refresh-classified-credential, decision-request-input-json)

Four lenses, one round over the three drafts, then a combined design and acceptance round 2 over
the rewritten first story. Every required edit is taken into the stories.

| story | scope | design | acceptance | parallel-safety | round 2 |
|---|---|---|---|---|---|
| blocked-repository-yields-place-to-waiting-unit (#245) | PASS-WITH-CHANGES | FAIL | FAIL | PASS-WITH-CHANGES | PASS-WITH-CHANGES, edits taken in |
| secret-refresh-classified-credential (#246) | PASS | PASS-WITH-CHANGES | PASS-WITH-CHANGES | PASS-WITH-CHANGES | PASS |
| decision-request-input-json (#246) | PASS-WITH-CHANGES | PASS-WITH-CHANGES | PASS-WITH-CHANGES | PASS-WITH-CHANGES | PASS |

Parallel safety: the three stories' recorded scopes are pairwise disjoint and disjoint from
`story:supervisor-watch-command` (spec/domains/config.yaml and host.yaml, src/cli.rs, config.rs,
lib.rs, watch.rs, src/supervisor_watch/, roles/supervisor.md, cli.yaml, http.yaml, docs/config.md).
None needs `spec/` or a regeneration; the input-json story needs neither cli.rs nor lib.rs; the
first story uses `crate::watch` as it is. `story:stopped-controller-stays-stopped` (draft) shares
`run/dispatch.rs` with the first story and stays out of the wave.

Already fixed or never broken: the `--help` of `decision escalate-request --lettered-options` names
the JSON array of `{letter, label, cost}` on main and on 1.19.2 (`crates/conductor/src/cli.rs:4219-4222`).

```findings
[
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"judgement","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"design r1: yielding every Blocked-only repository would take the place of every routine context hand-over, since [HANDOVER] marks the dispatch Blocked (run/mailbox.rs:149-151); taken in: a held controller's restart yields only to the unit its own need opened"},
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"integrity","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"design r1: restart_with turns a refusal into Err and handovers propagates it, aborting the tick (run.rs:1176-1179, run/handlers.rs:344); taken in: a yield is one journal line and the session forgotten, other refusals unchanged"},
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"judgement","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"design r1: restarting only Running/Paused records on session gone strands loop-stopped repositories; taken in: the rule is dropped, the operator-stop question left to story:stopped-controller-stays-stopped"},
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"design r1: the yield check must run inside the STARTS lock before make_room (run.rs:834-837); taken in"},
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"round 2: by try_start the held record is already Stopped in every path (run.rs:1153-1172); taken in: the caller passes whether it held a controller; the need link is read from the unit's brief first line, not a raw reference prefix"},
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"acceptance r1: a test accepted two outcomes, one test named no trigger, one had no fixture; taken in: one shared fixture, five tests, one outcome each"},
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"scope r1: the docs entries were inferred and the third outcome bullet contradicted the first; taken in: docs cited, contradiction gone with the dropped rule; spec stays unchanged since only dispatch v1 writes waiting"},
{"file":".engineering/planning/story/blocked-repository-yields-place-to-waiting-unit.md","line":1,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"parallel r1: story:stopped-controller-stays-stopped (draft) also scopes run/dispatch.rs and must not share the wave; crate::watch is used as is; taken in as Boundaries"},
{"file":".engineering/planning/story/secret-refresh-classified-credential.md","line":1,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"design r1: a new verb on a credential name that is no credential over-escalates (update the deploy key docs), as change already does; recorded in Boundaries; regenerate and recreate dropped; exact word forms listed; the verb test names one object"},
{"file":".engineering/planning/story/decision-request-input-json.md","line":1,"category":"judgement","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"design and scope r1: actor is a spec input field; clap's parsers do not run on JSON values; taken in: actor accepted as a key, --actor beside --input-json refused, values pass the flags' checks, lettered_options re-encoded"},
{"file":".engineering/planning/story/decision-request-input-json.md","line":1,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"acceptance r1: the same request id cannot be raised twice in one store; the comparison now uses a second store, and a test covers no object, a wrong type and a missing key"}
]
```
