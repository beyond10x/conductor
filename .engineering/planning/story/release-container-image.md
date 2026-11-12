---
format: aep.planning-md/3
id: story:release-container-image
kind: story
status: implemented
title: Each release publishes a container image of conductor
relations:
- decomposes: epic:deployable-in-kubernetes
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Dockerfile
- confidence: inferred
  path: README.md
- confidence: inferred
  path: crates/conductor/tests/release_image.rs
- confidence: inferred
  path: website/docs/getting-started.md
- confidence: inferred
  path: website/docs/guides/run-in-a-container.md
- confidence: inferred
  path: website/sidebars.ts
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-11T11:12:32Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-11T11:12:32Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-12T00:05:03Z", actor: "human:timo", revision: 14, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Why

A consumer control plane that wants to run one conductor instance in a local cluster asked first for a published image (2026-11-10). The release workflow attaches binary archives only; the repository holds no image build.

## Acceptance

- The release workflow builds a container image for linux/amd64 from the release's own binary and pushes it to a public registry under the release's version tag.
- The Release notes name the image by tag and digest, and a documented `docker run <image> conductor --version` prints the version.
- The image runs as a non-root user and holds the programs `conductor doctor` needs for an instance with a forge source.

## Scope

Derived 2026-11-11 by `story-scoper` at 67b0fb096. Every line is **cited** (read from the story or
the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `.github/workflows/release.yml` — cited, the story's own scope entry
- **Files:** `.github/workflows/release.yml:85-165` (`package` job, builds the linux/amd64 binary the
  image must reuse) and `:167-235` (`release` job, holds `contents: write`, creates or uploads to the
  Release) — cited
- **Image build file:** a new `Dockerfile` at the root — inferred; the tree holds no image build file
- **Symbols:** `PROGRAMS` and `forges` in `crates/conductor/src/doctor.rs:69-80,293` (the programs the
  image must hold), `b10x.toml` (their pins), `--version` at `crates/conductor/src/cli.rs:83` — cited,
  read-only inputs, not edited
- **Documents:** `AGENTS.md` § Releases (`:182` describes the workflow, `:206` step 5 writes the
  Release body, `:215` step 6 counts five assets) — inferred that it changes
- **Documents:** `README.md:85` § Install and `website/docs/getting-started.md:22` § 1 Install, the
  two pages that name the install — inferred that the image line lands there
- **Also likely:** a new page under `website/docs/guides/` with its entry in `website/sidebars.ts` —
  inferred, where a run-in-a-container page would go
- **Tests:** none covers the workflow today: `release.yml` is named only in a comment
  (`crates/conductor/src/upgrade.rs:8`) and a fixture string
  (`crates/conductor/tests/guard_rules.rs:1459`) — cited
- **Tests:** existing checks that bind the touched pages: `crates/conductor/tests/docs_forge.rs`
  (README requirements table, doctor sample in getting-started), `docs_quickstart.rs` and
  `docs_setup.rs` (README top 12 lines), `public_text.rs` (whole tree) — cited
- **Also likely:** a new `crates/conductor/tests/release_image.rs` that holds the workflow and the
  image file to the acceptance — inferred
- **Confidence:** medium — the workflow is cited and read in full; the image file, the docs page and
  any check are new files whose paths are a guess
- **Would collide with:** any unit touching `.github/workflows/release.yml`, `AGENTS.md` § Releases,
  the Install sections of `README.md` and `website/docs/getting-started.md`, `website/sidebars.ts`,
  or a root image build file
- **Shares with `story:container-start-without-prompt`:** the image build file (user, entrypoint,
  credential mount paths), the container guide page and `website/sidebars.ts`; that story edits
  `crates/conductor/src/doctor.rs`, which this one only reads — inferred
- **Shares with `story:daemon-probes-and-metrics`:** no file found; it lands in `spec/`,
  `crates/conductor/http.yaml`, `crates/conductor/src/daemon/` and generated reference pages. Only a
  port line in the image file could touch — inferred
- **Safety fact:** `conductor upgrade` picks release assets by exact name
  (`crates/conductor/src/upgrade.rs:177,184`), and an image pushed to a registry adds no asset, so
  upgrade and the five-asset check are untouched — step 2, unproven

## Scope as merged (wave 51)

The files the unit's merge into `wave/51` (`61eb01e39`) changed, from `git diff --name-only`
of the merge against its first parent. Where this list and the Scope section above differ,
this list is what the unit touched.

- `.github/workflows/release.yml`
- `AGENTS.md`
- `Dockerfile`
- `README.md`
- `crates/conductor/tests/release_image.rs`
- `website/docs/getting-started.md`
- `website/docs/guides/run-in-a-container.md`
- `website/sidebars.ts`
