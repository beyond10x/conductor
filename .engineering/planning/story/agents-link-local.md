---
format: aep.planning-md/3
id: story:agents-link-local
kind: story
status: implemented
title: agents:link links into the checkout, not ~/.claude/agents
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T15:22:37Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T15:22:38Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-13T15:40:46Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

An evaluation run on 2026-10-08 found that `task agents:link` links the product's agent definitions,
`repo-controller` among them, into `~/.claude/agents/`, so every Claude Code session on the machine
sees a controller agent it should not start. Linking into the repository's own `.claude/agents/`,
excluded through `.git/info/exclude`, was proposed instead.

## What

`task agents:link` links into `<checkout>/.claude/agents/` of the repository it is given (default:
the product checkout), adds each link's path to that repository's `.git/info/exclude`, and removes
links it made earlier under `~/.claude/agents/` (only symlinks that point into this checkout).
Nothing is written to `~/.claude/agents/`.

## Acceptance

- After `task agents:link` in a fake HOME, `~/.claude/agents/` holds no link into the checkout, and
  `<checkout>/.claude/agents/repo-controller.md` is a link that `git status` does not show.
- A link under `~/.claude/agents/` pointing into the checkout is removed; a file there that is not
  such a link is left alone.
- The start-conductor guide and README describe the new place.

## Scope

`Taskfile.yml` (`agents:link`), `README.md`, `website/docs/guides/start-conductor.md`.
