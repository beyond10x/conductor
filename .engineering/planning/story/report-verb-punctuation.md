---
format: aep.planning-md/3
id: story:report-verb-punctuation
kind: story
status: implemented
title: A report's verb may be followed by a colon, semicolon or comma
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T19:21:12Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T19:21:12Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T19:44:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Conductor reported on 2026-10-13: two freshly spawned controllers opened with `[REPORT <repo> <id>]
started; …`, and another wrote `started:`; each first line was refused as unparsable and the
controller got the format back. v0.10.2 accepted a colon only.

## What

A report's verb may be followed by `:`, `;` or `,`; other words in the verb's place still do not
parse.

## Acceptance

- `crates/conductor/tests/message_read.rs`: `started; …` and `done, …` parse; `maybe: …` and a
  dispatch id that is neither `DSP-…` nor `-` still do not.

## Scope

`crates/conductor/src/message.rs`, `crates/conductor/tests/message_read.rs`.
