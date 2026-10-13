---
format: aep.planning-md/3
id: story:agents-link-local
kind: story
status: draft
title: agents:link links into the checkout, not ~/.claude/agents
revision: 1
---
## Why

The founder's first run (2026-10-08, FIRST-4 in brain's case list) found that `task agents:link`
links the product's agent definitions, `repo-controller` among them, into `~/.claude/agents/`, so
every Claude Code session on the machine sees a controller agent it should not start. He suggested
linking into the checkout's own `.claude/agents/` and excluding the link through
`.git/info/exclude`.

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
