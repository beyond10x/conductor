---
format: aep.planning-md/3
id: story:install-one-liner
kind: story
status: draft
title: One pasted command installs the newest release binary with its checksum verified
relations:
- decomposes: epic:quick-start
revision: 2
---
# Story: install-one-liner

## Outcome

One pasted command downloads the newest release's archive for the machine's target, verifies it against `SHA256SUMS`, and installs `conductor` into `~/.local/bin`, with no `install.sh` to trust, no `gh`, and no Rust toolchain.

## Context

- Releases carry `conductor-<version>-<target>.tar.gz` per target (`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`) and one `SHA256SUMS`; each archive holds `conductor-<version>-<target>/conductor`, `LICENSE`, `README.md` (.github/workflows/release.yml:1-4, :148-158).
- The documented download needs the version typed by hand and `gh release download` (website/docs/getting-started.md:41-58): `gh` is not installed on the machines this page is for. The asset names carry the version, so a fixed `releases/latest/download/<name>` URL cannot name the archive.
- `conductor upgrade` already does these steps for an installed binary, in Rust: archive for the build's target, `SHA256SUMS`, the sum, the new binary's `--version`, replace keeping `conductor.previous` (getting-started.md:60-65); the one-liner is the bootstrap for the machine that has no binary yet.
- No new product code is wanted if the shell can do it. The one-liner is a documented command, not a hosted script: what runs is on the page.

## Acceptance

1. The command (a short pipeline or a function of a few lines, shown on the page) resolves the newest version from the `releases/latest` redirect of `https://github.com/beyond10x/conductor` without the GitHub API (no token, no rate limit), picks the target from `uname -s` and `uname -m`, downloads archive and `SHA256SUMS` with `curl`, and installs only after `sha256sum --check --strict` (`shasum -a 256 --check` on macOS) passes for that archive's line.
2. A tampered archive (a byte changed) and an archive missing from `SHA256SUMS` each stop the command before anything is installed, exit non-zero and print the mismatch; `~/.local/bin/conductor` is unchanged. The command honours the environment variable `CONDUCTOR_INSTALL_BASE` (default `https://github.com/beyond10x/conductor/releases`), the base under which it resolves `latest` and downloads; a test sets it to a local file server that serves a release layout with one tampered case.
3. On an unsupported `uname` pair the command exits 1 naming the four supported targets and `cargo install` as the other way.
4. After the command `conductor --version` prints the release's version and, when `~/.local/bin` is not on `PATH`, the command prints the one line that adds it, and does not edit a shell profile.
5. The command uses only `curl`, `tar`, `sed`, `uname`, `install` and a sha256 tool; nothing else is required (a test runs it in a minimal container image).
6. The chosen branch is no fixed-name asset: the release publishes no `latest` marker or version-free alias, the one-liner reads the version from the `releases/latest` redirect, and there is no spec change and no change to `.github/workflows/release.yml`.

## Out of Scope

A hosted `install.sh`; Windows; package managers; verifying a signature beyond `SHA256SUMS` (the release publishes none).

## Ambiguities

- A checksum file served from the same release proves the download was not corrupted, not that the release is genuine; signatures or provenance attestations are a separate decision for the operator.
- Installing into `~/.local/bin` versus `/usr/local/bin`: the user directory is proposed so no `sudo` is in the first command.
