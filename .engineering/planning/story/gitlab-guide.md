---
format: aep.planning-md/3
id: story:gitlab-guide
kind: story
status: implemented
title: A guide sets conductor up on GitLab
relations:
- decomposes: epic:forge-neutral
- depends_on: story:gh-optional
- depends_on: story:gitlab-source-host
- depends_on: story:gitlab-repositories
- depends_on: story:gitlab-merge-requests
- depends_on: story:gitlab-workspace
- depends_on: story:watch-gitlab-ci
- depends_on: story:guard-glab-writes
- depends_on: story:doctor-verdict
- depends_on: story:doctor-minimum-versions
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/conductor/tests/docs_forge.rs
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: website/docs/concepts/decisions-and-messages.md
- confidence: cited
  path: website/docs/concepts/instances-and-config.md
- confidence: cited
  path: website/docs/concepts/the-loop.md
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/guides/gitlab.md
- confidence: cited
  path: website/docs/guides/troubleshoot.md
- confidence: cited
  path: website/docs/security/the-guard.md
- confidence: cited
  path: website/sidebars.ts
revision: 17
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:43:06Z", actor: "human:timo", revision: 15, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "proposed", to: "active", at: "2026-10-13T10:43:07Z", actor: "human:timo", revision: 16, decided_on: {"recorded":{"review_outcome":6}}}
- {from: "active", to: "implemented", at: "2026-10-13T10:43:07Z", actor: "human:timo", revision: 17, decided_on: {"recorded":{"test_result":1,"review_outcome":6}}}
---
## Why

The README, the design document and the website describe a GitHub instance only ("not read by the
binary" for GitLab, `gh`-only CI and guard prose, `gh` among the required programs); a team on a
self-hosted GitLab has no page that says how to point conductor at it.

## Acceptance

- `website/docs/guides/gitlab.md` exists and is listed in `website/sidebars.ts` as
  `guides/gitlab`; the page contains `host: gitlab.example.com`, `glab auth login --hostname`, and
  the words repositories, merge requests, pipelines and releases in its list of what conductor
  reads. A test in `crates/conductor/tests/docs_forge.rs` holds the sidebar entry and the four
  content items; the Pages build job (`.github/workflows/pages.yml`) builds the site with the page.
- No hand-written page (`README.md`, `docs/design/conductor.md`, `website/docs/` but its generated
  `reference/`) contains "not read by the binary", "the binary does not read it" or "does not read
  that forge"; the same test holds it.
- The README's requirements table has a row starting `| \`gh\`` whose text contains "GitHub
  source" and a row starting `| \`glab\`` whose text contains "GitLab source"; the same test holds
  both.
- The sample `conductor doctor` output in `website/docs/getting-started.md` ends with a line
  starting `ready` or `not ready:` (`story:doctor-verdict`); the same test holds it.

## Notes

- Example host `gitlab.example.com`; no real host or company name (rules item 97).
- Also corrects, without its own check, the `gh`-only CI and guard prose in
  `website/docs/concepts/the-loop.md`, `decisions-and-messages.md`, `website/docs/security/
  the-guard.md` and `docs/design/conductor.md`, and the pin wording in `troubleshoot.md`.
- Lands last: it depends on every story whose behaviour it documents (`gitlab-source-host`,
  `gh-optional`, `gitlab-repositories`, `gitlab-merge-requests`, `gitlab-workspace`,
  `watch-gitlab-ci`, `guard-glab-writes`, `doctor-verdict`, `doctor-minimum-versions`), and is the
  only story on the hand-written pages below.

## Scope

`website/docs/guides/gitlab.md` (new), `website/sidebars.ts`, `README.md`,
`website/docs/getting-started.md`, `website/docs/concepts/the-loop.md`,
`website/docs/concepts/instances-and-config.md`, `website/docs/concepts/decisions-and-messages.md`,
`website/docs/security/the-guard.md`, `website/docs/guides/troubleshoot.md`,
`docs/design/conductor.md`, `crates/conductor/tests/docs_forge.rs` (new).
