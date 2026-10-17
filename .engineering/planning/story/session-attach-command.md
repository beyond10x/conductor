---
format: aep.planning-md/3
id: story:session-attach-command
kind: story
status: draft
title: conductor session attach connects the operator's terminal to a live session, with a detach key
relations:
- decomposes: epic:session-attach
- depends_on: story:session-pty-relay
revision: 1
---
# Story: session-attach-command

## Outcome

The operator runs `conductor session attach <name>` or `--role <r>` and gets the live session in his terminal: the screen as it is now, then live output, his keystrokes typed into it, until he presses the detach key; the session keeps running.

## Context

- `session start` and `session stop` take `--role` and are the commands `[session, start]` and `[session, stop]` of the CLI binding (crates/conductor/cli.yaml:306-314) over the callables at cli.yaml:113-129; their inputs are `conductor.session.StartSession` and `StopSession` (spec/domains/session.yaml:31-42).
- A controller's session is named after its repository, with the instance's session prefix (website/docs/reference/config.md, instance section; website/docs/guides/start-conductor.md:139-141), so the command takes a session name as well as a role.
- The relay and its socket come from `story:session-pty-relay`. A client joining a running session needs the current screen: the relay replays the tail of the typescript (bounded, for example 64 KiB) before live output.
- The client puts the operator's tty in raw mode and restores it on every exit path, a signal included.

## Acceptance

1. Spec first: model `conductor.session.AttachSession` (fields `name`, `role`, `watch`) and its refusals (`unknown-session`, `not-attachable`, `ambiguous`) in `spec/domains/session.yaml` beside `StopSession` (spec/domains/session.yaml:38), and add `[session, attach]` to `crates/conductor/cli.yaml`, validated with `ess specify validate --path spec`.
2. `conductor session attach --role conductor` on a live systemd-provider session shows the recent output, then live output; text typed in the terminal arrives in the harness's input (checked with an echoing stand-in harness).
3. The detach key (`ctrl-]` then `d`, printed on the first line of the attach and in `--help`) ends the client with exit 0 and restores the terminal mode; the session is still live and its unit active.
4. A terminal resize reaches the harness's pty (the stand-in prints its size after `SIGWINCH`).
5. `attach` on a name with no live session, or on one not under the systemd provider, exits 1 naming `unknown-session` or `not-attachable` and what to run instead (`claude attach <id>` for a background session).
6. `attach <name>` with `--role r` resolving to a different session, or a name two live sessions share, exits 1 naming `ambiguous` and each candidate; nothing is attached.
7. A killed client leaves the session running and the relay accepting a new client; a second read-write client while one is attached is refused with the first's age, not interleaved.
8. `website/docs/guides/start-conductor.md` and the generated `website/docs/reference/cli.md` name the command; `task docs-check` passes.

## Out of Scope

Read-only mode (`story:session-attach-watch`); the refusal inside a session (`story:session-attach-refuses-sessions`); the listing (`story:session-list-attachable`).

## Ambiguities

- Detach key: `ctrl-]` then `d` is proposed so `ctrl-p` and `ctrl-q` stay free for the harness; the operator may want another, and a `--detach-keys` option.
- Replay as the typescript tail or as a redraw forced by a resize: the tail is simpler but can start mid escape sequence.
