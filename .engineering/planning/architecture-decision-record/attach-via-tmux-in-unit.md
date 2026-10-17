---
format: aep.planning-md/3
id: architecture-decision-record:attach-via-tmux-in-unit
kind: architecture-decision-record
status: accepted
title: Operator sessions are attached through tmux inside their systemd unit
relations:
- decides: story:session-attach-command
revision: 2
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-17T19:20:16Z", actor: "human:timo", revision: 2}
---
## Decision

The operator decided on 2026-10-17. Sessions stay in their systemd unit, which keeps the confinement box. Each unit runs its harness in a foreground tmux server, `tmux -D` with a config file that creates the session. The pane is moved back into the unit's cgroup. `conductor session attach <name>` attaches to the live CLI; `--watch` is read-only. Detach is `ctrl-]` then `d`. With no name, attach opens a picker across every instance, and it works from any directory. Nothing in it is harness-specific: the operator said "we don't want to rely on claude in the future at all".

## Rejected

- The background (`claude --bg`) provider: it is openable in Claude's agent view, but it leaves the box and ties sessions to one harness.
- A transcript follower: it is not a live CLI.

## Consequence

Each session restarts once into the new launcher, with the operator's OK.
