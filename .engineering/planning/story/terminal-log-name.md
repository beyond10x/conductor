---
format: aep.planning-md/3
id: story:terminal-log-name
kind: story
status: active
title: the session recording is the terminal log, not the typescript
summary: rename <session>.typescript to <session>.terminal.log and say terminal log everywhere; the old name read for one release
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:29Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:29Z", actor: "human:timo", revision: 3}
---
## Why

Operator, 2026-11-07: "I have seen typescript multiple times ... I am not really using typescript and find this confusing." The file `<session>.typescript` is the recording the `script` command writes (`crates/conductor/src/confine/systemd.rs:22,84,376`); the name is `script`'s, and every reader takes it for the language.

## Acceptance

- The session recording is written as `<state>/sessions/<session>.terminal.log`; `conductor session attach --watch` and `conductor session list` read that name, and read the old `.typescript` name for one release when the new one is absent.
- The environment variable the launcher passes becomes `CONDUCTOR_TERMINAL_LOG`; `CONDUCTOR_TYPESCRIPT` is accepted for one release.
- No page under `website/docs/` and no `--help` text says "typescript" except once, to name the old file for the migration (a test greps).
