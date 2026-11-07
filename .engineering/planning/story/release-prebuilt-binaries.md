---
format: aep.planning-md/3
id: story:release-prebuilt-binaries
kind: story
status: active
title: Every release carries pre-built binaries for Linux, macOS and Windows
summary: Add the Windows archive to the four Linux and macOS ones, and a download-and-verify install line per platform
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T20:27:06Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T20:27:06Z", actor: "human:timo", revision: 4}
---
## Why

The operator, 2026-11-07: "dev work: conductor should have pre-built binaries for linux, win + mac". Since 0.23.0, `.github/workflows/release.yml` already attaches `conductor-<version>-<target>.tar.gz` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin`, plus one `SHA256SUMS` (`AGENTS.md`, Releases). Windows is missing, and the install pages lead with `cargo install`.

## Acceptance

- The release workflow also builds `x86_64-pc-windows-msvc` and attaches `conductor-<version>-x86_64-pc-windows-msvc.zip`; `SHA256SUMS` covers all six archives.
- What does not build or run on Windows (the systemd units, tmux sessions, bubblewrap confinement, `/proc` reads) is excluded by `cfg` and named in the story's evidence and on the install page; the command that needs it refuses on Windows with one line naming the platform, exit 1.
- The README and `website/docs/getting-started.md` give, per platform, a download-and-verify install line (`curl` plus `sha256sum --check` on Linux, `shasum -a 256 -c` on macOS, `Get-FileHash` on Windows) beside `cargo install`.
- `AGENTS.md` Releases step 6 checks six assets and verifies one archive per operating system.
- The first release that carries the Windows archive is announced (rules item 128).
