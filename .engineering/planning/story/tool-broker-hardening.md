---
format: aep.planning-md/3
id: story:tool-broker-hardening
kind: story
status: draft
title: The broker runs unattended, checks every grant rule and can be read
relations:
- decomposes: epic:agent-tools
- depends_on: story:tool-broker-read
revision: 2
---
# Story: tool-broker-hardening

## Outcome

The broker of story:tool-broker-read runs without a hand step, and its grants and records are complete. Every session reaches the broker at one fixed socket path, a stopped controller leaves no registration behind, and the recorded calls can be read.

## Context

story:tool-broker-read shipped the broker, `tool`, the read verbs and `tool mcp` (53d0e1b4, wave/39-tool). Left open:
- **Decided by conductor-dev:** `tool` is modelled as a command binding, `crates/tool/cli.yaml`, not a component. ess refuses a component that owns nothing (`empty_declaration`), and `spec/components.yaml` says so.
- The fixed socket path inside a boxed session's workspace is missing. The systemd box has no file-system fence yet.
- `controller stop-controller` does not unregister the session; only `session stop` does.
- The grant's `repositories` rule is stored but not checked.
- `search` matches literal text only.
- `conductor` has no view of the recorded tool calls.
- Nothing starts the broker automatically, and `task broker` and `task broker:stop` are untested.

## Acceptance

1. The broker starts with the instance, as a systemd user unit `<prefix>-conductor-broker` beside the dashboard, and `task broker` / `task broker:stop` are tested.
2. `stop-controller` and `pause-controller` unregister the session's grant, with a test.
3. A call whose repository is outside the grant's `repositories` is refused `NotGranted: repository`, with a test.
4. `conductor broker calls [--session s] [--format …]` lists the recorded calls (spec view `ToolCalls`).
5. Each session sees the socket at one fixed path, `$TOOL_SOCKET` set to `<state>/broker/<session>.sock`, and the box makes only that path writable. The mount waits for the box's file-system fence: refer to it, and do it once the fence exists.
6. `search` takes `--regex`.

## Out of Scope

The build, commit and report verbs (story:tool-build-commit-report).

## Ambiguities

None.
