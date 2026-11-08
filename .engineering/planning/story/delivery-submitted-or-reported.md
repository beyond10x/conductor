---
format: aep.planning-md/3
id: story:delivery-submitted-or-reported
kind: story
status: active
title: A delivery into a session's pane is submitted, or reported unsubmitted
summary: A watch relay sat typed but unsubmitted in a controller's input for 33 min
relations:
- decomposes: epic:loop-without-handwork
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T12:18:01Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-08T12:18:07Z", actor: "human:timo", revision: 4}
---
## Why

Host supervisor, 2026-11-08: the watch's `PR checks done` relay to a controller sat typed but
unsubmitted in the controller's input. At 13:16 local the pane's input line held `[REPORT <repo>
-] watch: PR checks done: <repo>#176 green`; the session had been idle since 12:43 (33 min),
waiting for that relay to merge the request by hand. The loop delivered again at 13:17:01 into the
same input; only a second carriage return submitted both. `type_text`
(`crates/conductor/src/confine/systemd.rs`) pastes the text, waits `PASTE_SETTLE`, pastes one
carriage return and returns `Ok` without checking that the harness took the input; nothing in the
run journal shows a delivery that was not submitted.

## Acceptance

1. After the carriage return, the delivery reads the pane (`capture-pane -p`); while its input line
   still holds the start of the delivered text, it pastes another carriage return, at most 3 times
   with `PASTE_SETTLE` between. A test with a fixture pane that takes the second return.
   The pane is read with `capture-pane -p -e`, and text in SGR 2 (dim) is ignored: the harness
   shows a prompt suggestion as dim ghost text on the input line (`❯ ESC[2m<text>ESC[0m`, host
   supervisor 2026-11-08), which a return does not submit. A test where the input line holds only
   dim text counts the delivery submitted; a test reads a real typed delivery and finds it not dim.
2. Every delivery logs one journal line with a delivery id, the session name and `submitted` or
   `unsubmitted after <n> returns`. Test.
3. An unsubmitted delivery is returned as an error to the caller, which reports it like a failed
   delivery. Test.

## Out of Scope

The `-` dispatch id in a watch relay's header: a relay with no dispatch carries `-` by the
controller profile's format.

## Ambiguities

None.
