---
format: aep.planning-md/3
id: story:tui-open-session
kind: story
status: draft
title: From the TUI the operator attaches to or watches a session with one key
relations:
- decomposes: epic:tui-frontend
- depends_on: story:tui-overview
- depends_on: story:session-attach-watch
- depends_on: story:session-list-attachable
- depends_on: story:session-attach-refuses-sessions
- depends_on: story:tui-decisions-answer
revision: 2
---
# Story: tui-open-session

## Outcome

From the TUI's session list the operator opens a session with one key: `enter` attaches to it (`conductor session attach`), `w` watches it, and the TUI comes back when he detaches.

## Context

- The attach command, its detach key and `--watch` come from `epic:session-attach` (`story:session-attach-command`, `story:session-attach-watch`); the attachable flag from `story:session-list-attachable`.
- The sessions rows are the overview's (`story:tui-overview`; crates/conductor/src/dashboard.rs:1343-1360).
- A TUI in raw mode on the alternate screen must hand the terminal over, then take it back.
- The screen registers in the screen registry `crates/conductor/src/tui/screens.rs` (`story:tui-overview`) with its key.
- Keeping sessions out of `conductor tui` is owned by `story:tui-decisions-answer`'s guard rule; this story adds no client-side refusal of its own and depends on it. It depends on `story:session-attach-refuses-sessions` so the attach it calls never ships without that refusal.

## Acceptance

1. Spec first: the keys are presentation; the attach model it calls is `conductor.session.AttachSession` (`spec/domains/session.yaml`, `story:session-attach-command`), validated with `ess specify validate --path spec`.
2. The sessions table shows the attachable column of `session list`; `enter` on an attachable session leaves the alternate screen, runs the attach in the same terminal, and on detach restores the TUI with its selection and refreshed data.
3. `w` does the same with `--watch`.
4. `enter` on a session that is not attachable shows the refusal the command gives (`not-attachable`, with `claude attach <id>`) and stays in the TUI.
5. A terminal resize while attached reaches the session, and after detach the TUI redraws at the new size.

## Out of Scope

Attaching to non-systemd sessions; opening a pull request or a repository page.

## Ambiguities

- Whether `claude attach <id>` for background sessions is offered from the same key; proposed: no, one clear refusal.
