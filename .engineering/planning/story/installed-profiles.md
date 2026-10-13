---
format: aep.planning-md/3
id: story:installed-profiles
kind: story
status: implemented
title: A restart reads the installed profiles, not a checkout
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T13:47:17Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T13:47:18Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T14:20:52Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

`task conductor:restart` refuses while the checkout holding a role's profile is behind its
upstream (commit 72ad19f0), and the only way out it offers is a `git merge --ff-only` in that
checkout. After the v0.8.0 release conductor-dev could not run that merge, because its session's
worktree isolation refuses git in the shared checkout, so the operator had to type the restart.
A restart after every release needed a person.

## What

`task profiles` (which `task install` runs after `cargo install`) writes the three profiles of the checkout it installs from
(`.agents/conductor.md`, `.agents/conductor-dev.md`, `.agents/repo-controller.md`) into
`profiles/` beside the config file (`~/.b10x/conductor/profiles/` unless `CONDUCTOR_CONFIG` names
another). An instance's roles name those copies as their `profile`, so a profile always matches the
installed binary and a restart reads no checkout. The behind-upstream refusal stays for a profile
inside a git checkout; its message names the installed copy as the other way out.

## Acceptance

- `task profiles` prints `profiles installed in <dir>` and leaves the three files there, mode 0644,
  byte-identical to the checkout's `.agents/` files.
- With the roles pointing at those copies, `task conductor:restart` starts conductor while the
  product checkout is behind its upstream (observed 2026-10-13).
- README, `conductor.example.yaml`, the configure-an-instance and start-conductor guides and the
  troubleshooting table name `~/.b10x/conductor/profiles/`.

## Scope

`Taskfile.yml` (install, profiles, the two refusal messages), `AGENTS.md`, `.agents/conductor-dev.md`, `README.md`, `conductor.example.yaml`,
`website/docs/guides/configure-an-instance.md`, `website/docs/guides/start-conductor.md`,
`website/docs/guides/troubleshoot.md`.

## Not covered

A binary installed by `conductor upgrade` or from a release archive gets no profiles: the archive
carries none. Shipping them in the archive and writing them on upgrade is a follow-up story.
