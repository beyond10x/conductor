---
format: aep.planning-md/3
id: epic:session-attach
kind: epic
status: draft
title: The operator attaches to a conductor or controller session and types into it, inside the systemd box
relations:
- decomposes: initiative:adoption
- informed_by: epic:session-confinement
revision: 1
---
## Why

The operator, 2026-10-17: sessions stay in the systemd box (`confinement.provider: systemd`), but he wants to open the conductor and controller sessions like Claude background sessions: watch a running session and type into it. Today nothing attaches. The unit runs `script`, which holds the pty and keeps a typescript; the only way in is the session's FIFO, which conductor itself types prompts into (crates/conductor/src/confine/systemd.rs:12-23, :72-74). `task conductor:attach` uses `claude attach <id>` (Taskfile.yml:81-99), which a session started under the systemd provider does not serve; website/docs/guides/start-conductor.md:139-141 sends the operator to `claude attach <id>` for the same reason.

Shape decided by conductor-dev: `conductor session attach <name|--role r>` connects the operator's terminal to the session's pty, which conductor holds through a small relay: a detach key, a read-only `--watch` mode, and a refusal inside a session.

## Outcome

With the systemd provider, the operator runs `conductor session attach` on any live conductor or controller session, sees what it shows now and what it prints next, types into it, and detaches with a key without stopping it; `--watch` shows without being able to type; a session cannot attach to a session; `conductor session list` and `task sessions` say which sessions can be attached. The box is unchanged: same unit, same slice, same limits.

## Decision: the mechanism

A conductor-owned pty relay, started by the unit in place of `script`: a hidden subcommand of the `conductor` binary (`conductor session relay`) opens a pty, runs `claude <arguments>` on its slave side, keeps the master, appends output to the existing typescript, and serves a unix socket at `<state>/sessions/<name>.sock` (mode 0600, in the directory that already holds `<name>.fifo` and `<name>.typescript`). `attach` is the client.

Why this and not tmux or dtach:
- No new host dependency. `script` (util-linux) is already used; tmux and dtach are not, and the Requirements table (README.md "Requirements") would grow by one program for every user, macOS included. The relay is code in the binary conductor already ships.
- The unit stays the supervisor. A tmux server forks away from the unit's main process and owns its own server and socket, so systemd's main-pid tracking, `Provider::stop` and the cgroup measurement (systemd.rs:24-32) would follow a daemon conductor does not own. dtach avoids the daemon but still adds the dependency, and gives conductor no hook for a read-only mode, a peer check or a bounded replay.
- Read-only mode and access policy live in the relay: it can refuse a second writer, mark a connection read-only and check the peer, none of which tmux or dtach let conductor enforce.
- Cost: conductor writes and owns a small pty multiplexer (about the size of dtach). That is why the first story proves the relay replaces `script` with no behaviour change before any attach exists.

## Decision: where it lives

A new epic, not stories under `epic:session-confinement`. That epic's outcome is the box (limits, writes, credentials, usage) and its done condition is seven confinement checks. Attach is an operator surface on top of the box: a different outcome and done condition, and it should neither hold confinement open nor wait for it. It stays related: it depends on the systemd provider (`story:session-limits-systemd`, implemented) and must not weaken the box. The one confinement-relevant risk, a session typing into another session, is a story of this epic (`story:session-attach-refuses-sessions`).

## Stories

`story:session-pty-relay`, `story:session-attach-command`, `story:session-attach-watch`, `story:session-attach-refuses-sessions`, `story:session-list-attachable`.

## Out of scope

Attach for sessions under another provider (background sessions already have `claude attach`); Codex sessions; attach over a network; recording or replaying keystrokes beyond the typescript that exists.
