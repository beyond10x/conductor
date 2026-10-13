---
format: aep.planning-md/3
id: story:review-limit
kind: story
status: implemented
title: The board, watch and dashboard show what waits for review, and a full queue holds dispatch
relations:
- decomposes: epic:review-first
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T16:33:56Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T16:33:57Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T17:17:45Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

See `epic:review-first`: review, not output, limits what ships. Nothing in conductor says how many
pull requests wait for a person, and conductor dispatches to a repository whatever its queue.

## What

A pull request **waits for review** when the newest complete snapshot holds it open, not draft,
with no failing check. Per repository:
- the config key `controllers.max_open_reviews` (default 3, `0` = no limit), declared first in
  `spec/domains/config.yaml` on `conductor.config.Controllers` with its invariant (`>= 0`);
- the board (`STATUS.md`, `conductor snapshot publish-board`) opens with a **Waiting for review**
  section: one line per such pull request, oldest first, `<repo> #<n> <title> — <age>`, and per
  repository at or over the limit the line `<repo>: <k> wait for review (limit <n>): no new dispatch`;
- the watch prints `review full: <repo> <k>/<n>` once when a repository reaches its limit, and
  `review open: <repo>` once when it falls under it;
- the dashboard overview's first panel lists the same pull requests with their links, oldest first;
- `.agents/conductor.md`: conductor sends no new dispatch to a repository the board marks review
  full; it reports it in the cycle instead.

## Acceptance

- `ess specify validate --path spec` exits 0 with the new key; `conductor config validate` refuses
  `max_open_reviews: -1` naming the key.
- Over a snapshot fixture with four open pull requests of `alpha` (one draft, one with a failing
  check, two waiting, opened on different days) and the limit 2, `publish-board` writes the section
  with the two in age order and the `alpha: 2 wait for review (limit 2): no new dispatch` line;
  with the limit 3, no limit line.
- A watch test: a snapshot where `alpha` reaches 2/2 prints `review full: alpha 2/2` once; the next
  snapshot at 1/2 prints `review open: alpha`.
- A dashboard test (Vitest) renders the waiting list in age order from the overview endpoint, and
  the endpoint test reads the rows.
- With no `max_open_reviews` in the config, the fixture with three waiting pull requests prints the
  limit line (default 3); with `max_open_reviews: 0`, no limit line and no watch line however many
  wait.
- A profile test holds the conductor.md sentence.

## Notes

- "Holds dispatch" is conductor's rule in `.agents/conductor.md`, fed by the board; no CLI refusal
  in this story.
- The coordinator alone regenerates `crates/conductor-model` and the reference pages, once, after
  every spec edit of the wave; this story patches the `Controllers` literals in `config.rs` then.

## Scope

`spec/domains/config.yaml`, `crates/conductor/src/config.rs`, `crates/conductor/src/status.rs`,
`crates/conductor/src/watch.rs`, `crates/conductor/src/dashboard.rs`,
`crates/conductor/dashboard/src/` (the overview), `crates/conductor/dashboard/dist/`,
`crates/conductor/tests/status.rs`, `crates/conductor/tests/watch.rs`,
`crates/conductor/tests/config.rs`, `crates/conductor/tests/profiles.rs`, `.agents/conductor.md`,
`website/docs/reference/config.md` (generated), `crates/conductor-model` (generated).
