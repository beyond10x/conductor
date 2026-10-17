---
format: aep.planning-md/3
id: epic:quick-start
kind: epic
status: draft
title: Anyone installs the release binary and sees conductor work in under five minutes
relations:
- decomposes: initiative:adoption
- informed_by: epic:onboarding-from-url
revision: 1
---
## Why

The operator, 2026-10-17: the public docs need a quick start that makes people's jaw drop: download the pre-built binary from the GitHub Release (`conductor-<version>-<target>.tar.gz` and `SHA256SUMS`), two or three commands, a dashboard open, the first dispatch visible, in under five minutes.

Today the first page is long. `website/docs/getting-started.md` (237 lines) installs with `cargo install` first and lists the pre-built download as a second way of seven shell lines (:22-58), then checks programs, writes a config by hand, validates it and takes a snapshot of two local repositories; it "starts no session" (:13-14). The README opens with a pointer for agents and four paragraphs before any command (README.md:1-11). A person who wants to see conductor work needs a Rust toolchain or a `gh`-assisted download, a hand-written config, a repository of his own with a planning store, and a logged-in forge CLI.

## Outcome

A person on Linux or macOS with no Rust toolchain, no config and no repository of conductor's kind pastes at most three commands from the front of the README or of `website/docs/quick-start.md`, and within five minutes has: the release binary installed with its checksum verified, a demo instance on a sample repository, the dashboard open in his browser showing a first dispatch, and the next step (his own repository) named. A CI job runs those same commands on every change to the page and before every release, so the page cannot rot.

## Decision: a new epic, not an extension of `epic:onboarding-from-url`

`epic:onboarding-from-url` is for an agent that sets conductor up in the person's own repository from the URL, with the prerequisites that needs (a forge CLI logged in, a current planning store), and its done condition is two recorded runs of that. The quick start is the other path: a person, binary first, nothing of his own needed, finished in five minutes on a sample, with a CI check as its done condition. Different reader, different outcome, different gate. It is informed by the older epic: the quick start ends by pointing at `SETUP.md` and `story:quickstart-existing-repo`'s guide for the person's own repository, and it must not duplicate their checks.

## Decision: the demo needs no model call and no forge

A demo that waits on a Claude login, a forge account or a model reply cannot be "under five minutes" or run in CI. The demo instance therefore runs on a `local` source (website/docs/getting-started.md:110-140, the source the page already uses) over a sample repository conductor writes, and records its first dispatch from a recorded sample, labelled as a demo in every place it shows. A real controller (`conductor spawn`) is the next step, not part of the five minutes. The operator can overrule this in `story:demo-mode`'s ambiguities.

## Stories

`story:quick-start-page`, `story:install-one-liner`, `story:init-defaults`, `story:demo-mode`, `story:quick-start-docs-test`.

## Out of scope

Windows; package managers (brew, apt); a hosted playground; replacing `SETUP.md` or the existing guides.
