---
format: aep.planning-md/3
id: story:quick-start-page
kind: story
status: draft
title: The quick start page and the README's top section show three commands to a first dispatch
relations:
- decomposes: epic:quick-start
- depends_on: story:install-one-liner
- depends_on: story:demo-mode
- depends_on: story:init-defaults
revision: 2
---
# Story: quick-start-page

## Outcome

`website/docs/quick-start.md` is the first page of the docs and the top section of the README: three commands, what each prints, the dashboard with a first dispatch, and the next step.

## Context

- The current front door: the README opens with an agent pointer and prose (README.md:1-11); the docs' first page is `getting-started.md`, whose install section leads with `cargo install` and lists the release download second (website/docs/getting-started.md:22-58). The release archives and checksum file are named in .github/workflows/release.yml:1-4 and :148-158; four targets are listed in getting-started.md:41-44.
- The three commands the page shows are the ones delivered by `story:install-one-liner` (install) and `story:demo-mode` (the demo instance and its dashboard); the step after the demo, `conductor init` for the person's own repositories, is `story:init-defaults`. Until those land the page cannot be true; this story is last in order and is written against their acceptance, not their hope.
- Guides are passive Markdown checked by `task docs-check` (Taskfile.yml:380-383), and `crates/conductor/tests/docs_quickstart.rs` already checks the existing guide by resolving every `conductor <group> <leaf>` it names against the CLI tree.
- The page order of the site is set in `website/sidebars.ts`.

## Acceptance

1. The page is `website/docs/quick-start.md` and is first in `website/sidebars.ts`; `getting-started.md` links it in its first lines as "the five-minute version", and keeps its own content.
2. The page has, in order: what you will have in five minutes (one sentence and one screenshot or terminal capture of the dashboard with the demo dispatch), the requirements in one line (Linux or macOS, a browser; nothing else), at most three shell blocks that each hold one command (install, `conductor demo`, and its dashboard line if the demo does not open one), what each prints, and "Next": your own repository (`SETUP.md`, `website/docs/guides/first-task-existing-repo.md`), the confined start (`website/docs/security/confinement.md`), the TUI once delivered; and, as the step after the demo, `conductor init` with no config (`story:init-defaults`), which is not on the five-minute path.
3. `README.md` carries the same three commands in a section within its first 15 lines, above the agent pointer's prose or directly under it, and links the page; the agent pointer to `SETUP.md` that `crates/conductor/tests/docs_setup.rs` holds stays.
4. Every command the page names resolves against the CLI tree (`docs_quickstart.rs`' check, extended to this page). The page checks its own printed lines locally with a test that runs the page's commands' `--help` and `--version` output and compares it to the lines the page shows. The run of the whole page in CI is `story:quick-start-docs-test`'s, not this story's.
5. The page states the demo's honesty in one line: the demo dispatch is a recorded sample; a real controller needs `claude` and a forge login, and the page says where that starts.
6. `task docs-check` passes; a person timing the page from a clean account on each target reaches the dashboard in under five minutes (recorded once per target in the story's evidence).
7. The install command text is the same on `website/docs/quick-start.md`, in `README.md` and in `website/docs/getting-started.md`, and one test compares the three.
8. The page shows the demo's real first lines of output, as printed by `conductor demo` at the release the page names.

## Out of Scope

The commands themselves (the stories named in Context); translating the page.

## Ambiguities

- Whether the screenshot is a committed image or a captured terminal block; a terminal block is diffable and is proposed.
- Version pinning in the page: the page names the newest release by the `latest` form of the one-liner, not a number, so it does not go stale at each release (`story:install-one-liner`).
