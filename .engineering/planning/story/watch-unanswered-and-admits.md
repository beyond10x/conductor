---
format: aep.planning-md/3
id: story:watch-unanswered-and-admits
kind: story
status: active
title: The watch names an unanswered report and disk coming back
relations:
- decomposes: epic:loop-without-handwork
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:13:52Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T13:13:52Z", actor: "human:timo", revision: 4}
---
## Why

Conductor's history review (2026-10-06 to 14): 23 controller reports and NEEDs waited more than 30
minutes for conductor's next event, 53 h in all, at most 6 h. A blocked controller also sat at 27G
free while `/` rose to 34G, released only by a message.

## Acceptance

- The watch prints `report unanswered: <repo> <dispatch> <m>m` once when a received report or NEED
  has had no conductor event on its dispatch for 30 minutes, and again each further 30; a test
  holds it.
- The watch prints `disk admits <n>G` once when free disk rises back over a level a refusal or a
  `blocked` report named; a test holds it.

## Scope

`crates/conductor/src/watch.rs`, `crates/conductor/tests/watch.rs`.

## Delivery (added 2026-11-07)

From the host teacher's brief, 2026-11-07, wave A, as written: a `[DECISION-REQUEST]` or `[NEED]`
older than one watch tick with no answer is named in the journal AND delivered to the supervisor
session by the broker (`claude agents` name `conductor-supervisor`). On 2026-11-07, 6 requests
waited about 1 h.

- The delivery target is a config key (the host supervisor's session name), not a name in the
  code (AGENTS.md rule 4); with no key set, the journal line alone is written.
- A test over the watch with one unanswered request older than `cadence.watch` reads one journal
  line and one delivery to the configured session; an answered one reads neither.

## Open requests list (added 2026-11-07)

From the host supervisor, 2026-11-07, relaying the operator: a NEED waited 2.5 h unanswered on a second instance (mailbox 12:06Z), and 25 requests with no answer were found at 14:37Z, by a host script outside any repository.

- `conductor dispatch requests --open` lists every `[NEED]` and `[DECISION-REQUEST]` with no answer, with its age, its repository and its mailbox file; a test with one answered and one open request lists only the open one.
- The loop writes that list to its journal each watch tick while it is not empty; it replaces the host script and its hand-kept answered file.
