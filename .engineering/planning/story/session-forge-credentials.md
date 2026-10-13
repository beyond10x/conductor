---
format: aep.planning-md/3
id: story:session-forge-credentials
kind: story
status: draft
title: A boxed session holds only read-only forge credentials
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-writes-bwrap
scope:
- confidence: cited
  path: crates/conductor/src/confine/bwrap.rs
- confidence: cited
  path: crates/conductor/src/confine/systemd.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/tests/confine_bwrap.rs
- confidence: cited
  path: crates/conductor/tests/confine_systemd.rs
- confidence: cited
  path: crates/conductor/tests/doctor.rs
- confidence: cited
  path: docs/analysis/session-forge-run.md
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: website/docs/security/limits.md
- confidence: cited
  path: website/docs/security/overview.md
- confidence: cited
  path: website/docs/security/the-guard.md
revision: 22
---
## Why

A write box does not stop a forge write: a boxed session keeps network, and `gh` and `glab` find
credentials in their config directories, the keyring and the SSH agent, so `python3 -c 'import os;
os.system("glab mr merge 1")'` passes both the guard and the box (scoping of
`story:session-writes-bwrap`, 2026-10-13). Forge writes are meant to go only through the bot route
(`b10x-gates bot`), from outside any session.

## Acceptance

- The bwrap argument list of `story:session-writes-bwrap` gains, after its binds, `--tmpfs` masks
  over `~/.config/gh`, `~/.config/glab-cli` and `~/.ssh`, and empty read-only binds over
  `~/.git-credentials` and `~/.netrc` where they exist; with `/run/user/<uid>` already hidden there,
  no forge credential of the host is reachable. A role's `forge_token_env` names
  (`story:sandbox-spec`) are passed into the unit with `systemd-run --setenv=<NAME>` (the value
  inherited from the provider's environment, never written into the unit's properties). Tests in
  `crates/conductor/tests/confine_bwrap.rs` read the masks and in
  `crates/conductor/tests/confine_systemd.rs` the `--setenv` arguments.
- `conductor doctor` prints, per role with an empty `forge_token_env`, a line that the role's boxed
  sessions have no forge access, before the verdict line, which stays last; a test in `crates/conductor/tests/doctor.rs` holds it.
- One recorded live run (`docs/analysis/session-forge-run.md`) of a boxed controller: with no token
  set, `gh api rate_limit` and `glab api user` fail to authenticate; with a read-only token,
  `gh api rate_limit` exits 0 and `python3 -c 'import os; os.system("gh pr merge 1")'` fails with an
  authorization error; the bot route still merges from outside the box.
- The `forge_token_env` row of `docs/config.md` and `website/docs/security/the-guard.md`,
  `limits.md` and `overview.md` say forge writes are bounded by the session's credentials.

## Notes

- The read-only tokens are the operator's to create (class H); the live run waits on
  `decision-blocker:sandbox-forge-tokens`, the code does not.
- After `story:session-writes-bwrap` (`confine/bwrap.rs`, `confine/systemd.rs`, `docs/config.md`
  and the security pages are shared and it lands first); `tests/doctor.rs` is shared with the seam.
  The former edge to `story:gitlab-workspace` is replaced by the seam's edge to
  `story:gitlab-guide`.

## Scope

`crates/conductor/src/confine/bwrap.rs`, `crates/conductor/src/confine/systemd.rs`,
`crates/conductor/src/doctor.rs`, `crates/conductor/tests/confine_bwrap.rs`,
`crates/conductor/tests/confine_systemd.rs`, `crates/conductor/tests/doctor.rs`,
`docs/analysis/session-forge-run.md` (new), `docs/config.md`, `website/docs/security/the-guard.md`,
`website/docs/security/limits.md`, `website/docs/security/overview.md`.
