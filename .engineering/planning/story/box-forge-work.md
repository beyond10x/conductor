---
format: aep.planning-md/3
id: story:box-forge-work
kind: story
status: implemented
title: A boxed runtime and its controllers do forge work
summary: Forge apertures per host, writable checkouts and trees, git config and persistent Claude state with trust in the box
relations:
- decomposes: epic:architecture-next
- informed_by: story:box-reaches-forge-and-daemon
scope:
- confidence: cited
  path: SETUP.md
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: cited
  path: crates/conductor/src/dashboard/usage.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: cited
  path: crates/conductor/src/spawn.rs
- confidence: cited
  path: crates/conductor/src/trust.rs
- confidence: cited
  path: crates/conductor/src/usage.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/adv_box_mounts_pass2.rs
- confidence: cited
  path: crates/conductor/tests/common/mod.rs
- confidence: cited
  path: crates/conductor/tests/isolated_home.rs
- confidence: cited
  path: crates/conductor/tests/placement.rs
- confidence: cited
  path: crates/conductor/tests/setup.rs
- confidence: cited
  path: crates/conductor/tests/trust.rs
- confidence: cited
  path: website/docs/concepts/the-host.md
- confidence: cited
  path: website/docs/getting-started.md
- confidence: cited
  path: website/docs/guides/configure-an-instance.md
- confidence: cited
  path: website/docs/guides/gitlab.md
- confidence: cited
  path: website/docs/guides/run-isolated.md
- confidence: cited
  path: website/docs/guides/troubleshoot.md
- confidence: cited
  path: website/docs/security/sessions.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-15T09:47:14Z", actor: "human:agent", revision: 2, executor: "human:timo"}
- {from: "proposed", to: "active", at: "2026-11-15T09:47:14Z", actor: "human:agent", revision: 3, executor: "human:timo"}
- {from: "active", to: "implemented", at: "2026-11-21T01:32:38Z", actor: "human:unknown", revision: 6, decided_on: {"recorded":{"test_result":2}}}
---
## Why

A runtime in a substrate box, and the controllers it starts as its children, could not do any
forge work. Read from a live instance at `placement: systemd` with one `gitlab` source and
`forge.route: direct`:

- The start named only the apertures `conductor` (the model host) and, with
  `daemon.dial_port`, `conductord`. `BoxRequest.egress` listed the forge host and nothing sent
  it, so the box's `/etc/hosts` held only the model host and every lookup of the forge failed:
  each snapshot since the box started failed at its `git fetch`. The pinned substrate SDK now
  takes a list of apertures (at most four per start), which cleared
  `decision-blocker:substrate-sdk-single-aperture`.
- The checkouts root was mounted read-only, and the managed trees root not at all: a controller
  could neither commit and push nor make a managed worktree. The `worktree` CLI also writes its
  registry under `~/.local/state/worktree`, which the box does not hold.
- `~/.gitconfig` was not in the box: `git push` over HTTPS had no credential helper and a commit
  had no identity.
- `~/.claude.json` and `~/.claude` in the box are the box's own, empty at each start: the trust
  `conductor trust` writes on the host was invisible, so each spawn was refused
  `UntrustedCheckout`, and the transcripts were lost at each start.

## Outcome

- One egress aperture per forge host of the sources, named `forge_` and the host with `_` for
  every byte but a letter or a digit (`forge_gitlab_example_com`), declared by the operator as
  `substrate-daemon --egress-aperture forge_<host>=<host>:443/tcp`; a `github` source needs
  `github.com` and `api.github.com`. The start names the dial-in aperture first (on
  `127.0.0.1`), then `conductor`, then the forge apertures; conductord names a forge aperture
  only where the daemon declares it (`exec.egress-apertures`), so the box still opens without
  it. An instance that needs more than four apertures is refused before substrate is asked and
  by `conductor config validate`, `placement.apertures-exceeded`.
- The checkouts root and the managed trees root are writable roots, `checkouts-<instance>` and
  `trees-<instance>`; conductord makes the trees root before it opens the box.
- The box reads `~/.gitconfig`, `~/.config/git/config` and `~/.config/worktree/config.toml`
  where they are there.
- The box's environment names `CLAUDE_CONFIG_DIR=<state>/claude` and
  `XDG_STATE_HOME=<state>/xdg-state`, in the state root it writes: Claude Code's
  `.claude.json` and transcripts and the `worktree` registry outlive a start. conductord marks
  each checkout and the records trusted in `<state>/claude/.claude.json` before it opens the
  box, and `conductor trust` marks it for a boxed instance too. Conductor reads the trust and the
  transcripts where `CLAUDE_CONFIG_DIR` names them.
- `conductor doctor` adds, for a box on the local daemon, `box <instance> forge` (a failure,
  `forge unreachable`, with the flag of each undeclared aperture), `box <instance> git`
  (identity and the forge host's credential helper; reported) and
  `box <instance> trust` (reported); a missing trees root is not a failure.

## Acceptance

- The box of an instance with a forge source names the forge's aperture after the dial-in and
  the model host's where the daemon declares it, and not where it does not
  (`tests/placement.rs`).
- More than four apertures are refused at validate, naming the placement (`tests/placement.rs`).
- The box writes the checkouts and the trees roots, reads the git and worktree configuration,
  carries `CLAUDE_CONFIG_DIR` and `XDG_STATE_HOME`, and conductord makes the trees root and marks
  the records trusted in the box's `.claude.json` (`tests/placement.rs`); `conductor trust`
  marks a boxed instance in its state and writes where `CLAUDE_CONFIG_DIR` names
  (`tests/trust.rs`).
- `conductor doctor` names the undeclared aperture with its flag, the missing git identity and
  helper, and the trust, and passes once they are there (`tests/placement.rs`).

## Boundaries

- Cloning the repositories of a boxed instance is not part of this story: the operator clones
  them on the host; the collector clones only at a pod placement.
- The box's `worktree` registry is its own: trees registered in the host's registry are not
  known to the box, and the box's are not known to the host's `worktree` commands.
- A forge CLI login kept in the system keyring does not reach the box.
