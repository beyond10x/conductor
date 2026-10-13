---
format: aep.planning-md/3
id: story:conductor-upgrade
kind: story
status: draft
title: conductor upgrade checks for, downloads and verifies a newer release
relations:
- depends_on: story:setup-command
scope:
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: SETUP.md
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/build.rs
- confidence: cited
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/src/cli.rs
- confidence: cited
  path: crates/conductor/src/lib.rs
- confidence: cited
  path: crates/conductor/src/upgrade.rs
- confidence: cited
  path: crates/conductor/tests/upgrade.rs
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 5
---
## Why

The operator, 2026-10-13: "conductor upgrade -> checks newer version and download from gh + check
sha, etc...". Since v0.6.0 each release carries `conductor-<version>-<target>.tar.gz` for four
targets (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`aarch64-apple-darwin`) and one `SHA256SUMS` (`.github/workflows/release.yml`). Upgrading today
means finding the release, downloading the right archive, checking its sum and replacing the binary
by hand (README, getting started). Installing a named older release (a rollback) is the same steps
with another version, the "etc." of the request.

## Acceptance

- Spec first: `conductor upgrade` (`[upgrade]` in `crates/conductor/cli.yaml`, inputs `--check` and
  `--version <x.y.z>`) and its outcomes — upgrade available, none available, upgraded from/to,
  already current, no archive for this target, checksum mismatch, new binary does not run, binary
  not writable — are declared in `spec/domains/config.yaml` and validated (`ess specify validate`)
  before any code.
- The releases are read from the repository the binary was built from:
  `env!("CARGO_PKG_REPOSITORY")`, set by `repository` in `[workspace.package]` of the root
  `Cargo.toml` and `repository.workspace = true` in `crates/conductor/Cargo.toml`; no
  organization's name appears under `crates/conductor/src`, `crates/conductor/tests` or `spec`, and
  `crates/conductor/tests/no_organization.rs` passes unchanged. The archive's target is the one the
  binary was built for (`crates/conductor/build.rs` passes Cargo's `TARGET` as a `rustc-env`).
- The network is `curl`, the sum is `sha256sum` (or `shasum -a 256` where `sha256sum` is missing,
  as on macOS), the unpacking is `tar -xzf` into a temporary directory; no crate is added
  (`Cargo.lock` is unchanged).
- `conductor upgrade` downloads the archive and `SHA256SUMS`, verifies the sum, unpacks, runs the
  unpacked binary's `--version`, and replaces the running binary's file by a rename in its own
  directory, keeping the old file as `conductor.previous`. A refusal changes no file.
- Tests in `crates/conductor/tests/upgrade.rs`, with a fake `curl` on `PATH` serving a fixture
  release (the API's JSON, a small archive, `SHA256SUMS`) and the binary copied into a temporary
  directory, one per declared outcome:
  - `--check` with a newer release prints the installed and newest versions and `upgrade
    available`, exit 0;
  - `--check` at the newest release prints `none available`, exit 0;
  - `upgrade` replaces the copy, which then prints the new version, and keeps
    `conductor.previous` with the old bytes;
  - `upgrade` at the newest release prints `already current` and leaves the copy byte-identical;
  - a release without this target's archive refuses with `no archive for this target`, copy
    unchanged;
  - a `SHA256SUMS` line that does not match refuses with `checksum mismatch`, copy unchanged;
  - an archive whose binary exits non-zero on `--version` (a script that does) refuses with
    `new binary does not run`, copy byte-identical and no `conductor.previous` written;
  - a copy in a read-only directory refuses with `binary not writable`, copy unchanged;
  - `--version 0.6.0` downloads `conductor-0.6.0-<target>.tar.gz` (the fake `curl` records the
    URL it was asked for) and installs it.
- README, `website/docs/getting-started.md` and `SETUP.md` name `conductor upgrade` as the way to
  upgrade.

## Notes

- Decided: `curl`, `sha256sum`/`shasum` and `tar` rather than a TLS, hash or archive crate — no new
  dependency, and all three ship on macOS and the Linux hosts `SETUP.md` covers; the GitHub API's
  unauthenticated limit is enough for one check.
- Dropped: a `conductor doctor` note about a newer release. Nothing in the request asks for it, and
  it would need a cache and an edit to `doctor.rs`.
- After `story:setup-command` (shares `cli.yaml`, `cli.rs`, `lib.rs`, `crates/conductor-model`,
  `website/docs/reference/cli.md`, `SETUP.md`). `story:snapshot-member-failures` comes after it on
  `cli.rs`, the model and `reference/cli.md`; `story:dashboard-spa` comes after it on
  `crates/conductor/Cargo.toml` (this story adds `repository.workspace`, that one `include_dir`).

## Scope

`spec/domains/config.yaml`, `crates/conductor/cli.yaml`, `crates/conductor-model`,
`crates/conductor/src/cli.rs`, `crates/conductor/src/upgrade.rs` (new),
`crates/conductor/src/lib.rs`, `crates/conductor/build.rs`, `Cargo.toml`,
`crates/conductor/Cargo.toml`, `crates/conductor/tests/upgrade.rs` (new),
`website/docs/reference/cli.md`, `README.md`, `website/docs/getting-started.md`, `SETUP.md`.
