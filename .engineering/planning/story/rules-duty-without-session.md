---
format: aep.planning-md/3
id: story:rules-duty-without-session
kind: story
status: draft
title: A rules.md duty given to a role no session holds is named, or the loop can carry it
summary: a channel-post rule went to the director, which the model-free loop cannot do
revision: 2
---
## Why

Reported 2026-11-07 from a running instance: a line of the instance's `rules.md` gives posting to
a team chat channel to the director role. The director is now the model-free loop, `conductor
run` (`docs/design/conductor-run.md` § 2), and the loop has no step that posts to a channel:
verified, no `slack`, `channel` or post step in `crates/conductor/src/run.rs` or
`crates/conductor/src/run/`. So the rule is given to nobody who can carry it out, and nothing
says so.

## Acceptance

- A duty that `rules.md` gives to a role no session holds (the director when it runs as
  `conductor run`) is named, with its line, by a check an operator runs (`conductor config
  validate`, or the records check of the cycle tick); or
- the loop gains a channel-post step configured per instance (a config key naming the channel and
  the events posted), and a rule that names it is carried out by that step, one post per event,
  recorded in the dispatch log.
- The chosen form is written in `docs/design/conductor-run.md`.
