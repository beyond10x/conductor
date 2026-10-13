---
format: aep.planning-md/3
id: story:prebuilt-binaries
kind: story
status: draft
title: Each release attaches pre-built conductor binaries for macOS and Linux
relations:
- decomposes: epic:conductor-autonomy
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: README.md
- confidence: cited
  path: website/docs/getting-started.md
revision: 2
---
## Why

Conductor is installed only by building it from source (`cargo install --git … --tag`), which
takes a 1.5G build on the host and, on this host, a build slot from conductor itself; the sibling
tools (`ess`, `aep`, `worktree`) ship pre-built archives per platform with a `SHA256SUMS` on each
GitHub Release, which `b10x install` and the CI of other repositories download. The operator asked
for pre-built conductor binaries for macOS and Linux (2026-10-13).

## Acceptance

- Pushing a `v<version>` tag runs a release workflow that builds `conductor` for
  `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and
  `aarch64-apple-darwin` and attaches `conductor-<version>-<target>.tar.gz` for each plus one
  `SHA256SUMS` to the GitHub Release, in the same naming and layout as `ess` and `aep` (their
  `release.yml`); the next release's page lists the five assets, and an archive downloaded from it
  verifies against `SHA256SUMS` and runs `conductor --help` (recorded in the wave page).

## Notes

- Follow `ess`'s `.github/workflows/release.yml`: pinned actions by commit, `--locked`, native
  macOS runners for the darwin targets, `cross` or a linux arm runner for aarch64-linux.
- AGENTS.md § Releases and the README's install section then offer the archive download beside
  `cargo install`; registering conductor as a `b10x install` catalog binary is a follow-up in the
  agentplugins repository, not here.

## Scope

`.github/workflows/release.yml` (new), `AGENTS.md`, `README.md`, `website/docs/getting-started.md`.
