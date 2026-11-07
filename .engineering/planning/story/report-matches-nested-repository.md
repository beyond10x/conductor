---
format: aep.planning-md/3
id: story:report-matches-nested-repository
kind: story
status: draft
title: A report naming a nested repository matches its dispatch
summary: reports naming the session suffix match no dispatch
revision: 3
---
## Why

Reported 2026-11-07 on an instance with nested repositories: the loop's `no open dispatch of <x>`
lines named `<group>-<repo>`, the controller's session suffix, while dispatches carry the
repository `<group>/<repo>`. The mailbox matches a `[REPORT <repo> -]` to a dispatch by comparing
the `<repo>` the message names with the dispatch's repository exactly (`open_dispatch`,
`crates/conductor/src/run/mailbox.rs:196`). So a controller that names its repository by its
session suffix never matches. Verified the same day: a report headed `[REPORT <group>/<repo> <dispatch>]` matched and moved its
dispatch to `Started`; one headed `<group>-<repo>` was refused with `no open dispatch of
<group>-<repo>`. Not verified: whether the charter or profile tells controllers which form to use.

## Acceptance

- A report naming `<group>-<repo>` (the session name's form) or `<group>/<repo>` matches the open
  dispatch of `<group>/<repo>`; a name that matches no repository is refused to the sender naming
  the repository names it may use.
- The charter a controller starts with names its repository in the form reports use.
