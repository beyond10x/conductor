---
format: aep.planning-md/3
id: story:ask-direct
kind: story
status: draft
title: conductor ask --to or @instance reaches one director, and the answer is labelled
relations:
- decomposes: epic:host-bridge
- depends_on: story:host-instances-list
- depends_on: story:mailbox
- depends_on: story:operator-relay-verified
revision: 2
---
# Story: ask-direct

## Outcome

`conductor ask --to <instance> "<text>"`, or `@<instance>` as the first word, reaches that instance's director with no routing, and its answer comes back labelled with the instance.

## Context

docs/design/roles-and-surfaces.md § 4 ("Direct: `conductor ask --to <instance> "<text>"`, or `@<instance>` as the first word, goes to that director with no routing. Every answer comes back through the same channel, labelled with its instance"). The operator chose a session with `--to` always available (§ 8, question 2). The request travels as the operator's verified message through the mailbox (story:mailbox, story:operator-relay-verified), so a director on any harness receives it and records it in its own store.

## Domain relations

- Ask → Instance, many-to-one; the delivered message is owned by the instance's store; an ask cannot name an instance that is not registered — inferable (docs/design/roles-and-surfaces.md § 4, operator-approved 2026-10-16; no ess/1 document declares it: this story's spec step adds `conductor.bridge.Ask`, with an `UNMAPPED:` marker for any relation it cannot type).
- Delivered message → Instance, many-to-one, owned by the instance's store — inferable (spec/domains/dispatch.yaml, `Message` with transport `Mailbox`, line 39; the instance is implied by the store, not a `relations:` entry).

## Acceptance

1. Spec first: `spec/domains/bridge.yaml` declares `conductor.bridge.Ask` (text, optional `to`) and `conductor.bridge.Answer` (instance, text); `spec/components.yaml` places `ask` at the top level; `ess specify validate --path spec` exits 0, the model crate is regenerated and `crates/conductor/tests/cli_tree.rs` lists `ask`.
2. `conductor ask --to alpha "status of repo-a"` writes one verified operator message into `alpha`'s director inbox and none into `beta`'s; `crates/conductor/tests/ask.rs` holds it.
3. `conductor ask "@alpha status of repo-a"` does the same, with `@alpha` stripped from the text; the same test holds it.
4. An answer the director posts back prints as `[alpha] <text>`, and with `--format json` carries `instance: alpha`; the same test holds it.
5. An unregistered instance is refused with exit 2, naming the registered ones; the same test holds it.

## Scope

`spec/domains/bridge.yaml`, `spec/components.yaml`, `crates/conductor/cli.yaml`, `crates/conductor-model`, `crates/conductor/src/ask.rs`, `crates/conductor/src/lib.rs`, `crates/conductor/src/cli.rs`, `crates/conductor/tests/ask.rs`, `crates/conductor/tests/cli_tree.rs`, `.agents/conductor.md` (the director answers an ask through the mailbox).

## Out of Scope

Routing without `--to` (story:ask-bridge-route). The dashboard input (story:dashboard-ask-input). The mailbox and the verified relay themselves.

## Ambiguities

None.
