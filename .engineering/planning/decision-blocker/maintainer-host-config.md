---
format: aep.planning-md/3
id: decision-blocker:maintainer-host-config
kind: decision-blocker
status: cleared
title: Nobody has decided where the one maintainer per host is configured when the host has several config files
relations:
- blocks: epic:role-names
revision: 4
transitions:
- {from: "open", to: "cleared", at: "2026-10-16T19:25:53Z", actor: "human:timo", revision: 4}
---
## Question

Epic `role-names`, acceptance 2: the maintainer is configured per host, not per instance, and
`instances[].roles` no longer holds it. Which record holds the maintainer's role (harness, model,
settings, profile, session name) when one host runs several config files?

What is settled and what is not:
- Settled: one maintainer per host, outside every instance (`docs/design/roles-and-surfaces.md`
  § 2 and § 8 question 5, approved by the operator on 2026-10-16).
- Settled in `spec/domains/config.yaml` (domain summary): what the instances of one file share
  lives in the host directory, `host/` beside that file. That is per config file, not per host.
- Drafted, not decided: a host registry of every config file, `<host>/instances.json`
  (story `instance-names-unique`), and a `host:` section of the config file for the host's limits
  (story `host-grants`).
- Not settled anywhere: the cardinality of maintainer to config file. With a `maintainer:` key per
  file, two files can each name one, and "one per host" becomes "one per file" unless something
  refuses the second.

Options:
- (A) A top-level `maintainer:` key in `conductor.config/1`, beside `instances`; the host registry
  refuses a second registered file that names one, naming both files.
- (B) A host file of its own in the host directory (`~/.b10x/conductor/host/`), read by every
  config file's commands; config files hold no maintainer at all.
- (C) The `host:` section story `host-grants` adds to the config file gains the maintainer role,
  with the same refusal as (A) across files.

Recommendation: (B). It is the only option whose cardinality is one per host by shape rather than
by a check across files; it needs the host directory to be one per host, which the registry story
already assumes.

## What it stops

The story that moves the maintainer out of `instances[].roles` into the chosen record, with its
`config validate` note for a `conductor-dev` or `maintainer` role still under an instance, and
`task maintainer:start` reading it there. Every other story of the epic is drafted without it:
until it clears, the `maintainer` role stays an entry of `instances[].roles` under its new name.

## What clears it

The operator's choice, recorded as an `approval-record` that `decides` this blocker; then this
blocker moves to `cleared` and the maintainer-config story is drafted against it.

## Decision

(B), decided by conductor-dev on 2026-10-16 under the operator's approval of docs/design/roles-and-surfaces.md (§ 8 question 5: one maintainer per host). It is a detail inside that approved design and can be undone, so it was not escalated.

- The host has one file of its own, `host.yaml` in the host directory (`~/.b10x/conductor/host/`). Every config file's commands read it, and config files hold no maintainer.
- The file holds the per-host roles (maintainer, bridge: harness, model, settings, profile, session name). The bridge stories of epic:host-bridge read their role from it too.
- The host registry `instances.json` (story:instance-names-unique) sits beside it.

Reason: it is the only option that is one per host by its shape rather than by a check across files.
