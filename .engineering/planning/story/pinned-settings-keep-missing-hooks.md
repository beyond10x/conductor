---
format: aep.planning-md/3
id: story:pinned-settings-keep-missing-hooks
kind: story
status: active
title: Pinned session settings keep a missing hooks key missing
summary: 'config::pinned reads hooks with get_mut, so no "hooks": null reaches Claude Code'
refs:
- provider: github
  reference: beyond10x/conductor#85
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T03:09:02Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T03:09:02Z", actor: "human:timo", revision: 3}
---
## Why

Reported in https://github.com/beyond10x/conductor/issues/85 (Conductor 0.19.1, Claude Code
2.1.319, 2026-11-07): `conductor session start --role maintainer` wrote
`state/sessions/beyond10x-conductor-dev.settings.json` with `"hooks": null`. Claude Code stopped
on a "Settings Warning" dialog (`hooks` must be an object), the session was not listed live within
120 s, and `session start` answered `LaunchFailed` while its unit kept running.

`config::pinned` read `settings["hooks"].as_object_mut()`; serde_json's `IndexMut` inserts `null`
for a missing key. `matcher["hooks"]` did the same for a matcher without a `hooks` list. Both
`session start` and the controller spawn path write through `pinned`.

## Acceptance

- A settings file with no `hooks` key pins to a file with no `hooks` key.
- A matcher with no `hooks` list keeps no `hooks` key after pinning.
- Hook commands that run `conductor` are still pinned to the instance.
