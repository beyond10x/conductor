---
format: aep.planning-md/3
id: story:class-o-repos-adr
kind: story
status: implemented
title: Repositories and architecture decisions are class O; a bare report verb parses
relations:
- decomposes: epic:conductor-autonomy
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T21:38:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T21:38:27Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T21:47:03Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The operator decided on 2026-10-13 that new, renamed or retired repositories and ADR-level changes
stay class O; design § 5 put "architecture and repository changes" in class C, and conductor's profile
said class O is only what cannot be undone, so conductor classed an ADR-level language route as C.
Conductor also reported that a bare `[REPORT <repo> <id>] started` was refused.

## Acceptance

- Design § 5, `.agents/conductor.md` (Decide, the class-O bullet), the decisions concept page and the
  confinement page list a new, renamed or retired repository and an architecture decision (with the
  example of a product's language architecture) under class O, and no longer put them in class C;
  `rg 'architecture and repository changes included'` finds nothing.
- `crates/conductor/tests/message_read.rs`: `[REPORT gates DSP-1] started` and `[REPORT conductor -]
  ready` parse.

## Scope

`docs/design/conductor.md`, `.agents/conductor.md`, `website/docs/concepts/decisions-and-messages.md`,
`website/docs/security/confinement.md`, `crates/conductor/src/message.rs`,
`crates/conductor/tests/message_read.rs`.
