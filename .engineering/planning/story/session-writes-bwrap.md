---
format: aep.planning-md/3
id: story:session-writes-bwrap
kind: story
status: draft
title: A session can write only its own places
relations:
- decomposes: epic:session-confinement
- depends_on: story:session-limits-systemd
- supersedes: story:guard-bash-write-targets
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/conductor/src/confine/bwrap.rs
- confidence: cited
  path: crates/conductor/src/confine/mod.rs
- confidence: cited
  path: crates/conductor/src/confine/systemd.rs
- confidence: cited
  path: crates/conductor/tests/confine_bwrap.rs
- confidence: cited
  path: crates/conductor/tests/docs_forge.rs
- confidence: cited
  path: docs/analysis/session-writes-run.md
- confidence: cited
  path: docs/config.md
- confidence: cited
  path: docs/design/conductor.md
- confidence: cited
  path: website/docs/security/limits.md
- confidence: cited
  path: website/docs/security/overview.md
- confidence: cited
  path: website/docs/security/the-guard.md
revision: 20
---
## Why

The guard does not check where a Bash command writes, and a write hidden in another language
(`python3 -c …`) is invisible to it (`website/docs/security/the-guard.md`); `README.md:42` says
"There is no operating-system sandbox yet". `architecture-decision-record:external-confinement`
puts that boundary outside the harness: bubblewrap, in the kernel. On this host `bwrap` 0.12.0
runs unprivileged (`unprivileged_userns_clone` 1, no AppArmor userns restriction). Supersedes
`story:guard-bash-write-targets`.

## Acceptance

- The `systemd` provider's `start` runs the session, inside its unit, under one bwrap argument list
  built in `crates/conductor/src/confine/bwrap.rs`, in this order: `--ro-bind / /`, `--dev-bind /dev
  /dev`, `--tmpfs /run/user/<uid>` (hides the session bus and the SSH agent), `--bind` back
  `/run/user/<uid>/cc-socks`, `--bind` each resolved writable path of the request, `--bind` the
  session's `TMPDIR`, for conductor `--bind` the instance's `state` and `cache`, `--die-with-parent`;
  and the unit's environment sets `CLAUDE_CONFIG_DIR` to the session's harness-state directory, so
  Claude Code's `.claude.json` lives in a bound place and no path directly under `~` is writable.
  Tests in `crates/conductor/tests/confine_bwrap.rs` read the list for a controller and for
  conductor, and find no `--bind` of `~` itself.
- `measure` records the writable paths applied (`envelope_writable`); a test reads them. Under
  `strict`, `bwrap` missing or a requested place that does not exist is a shortfall and `start`
  refuses (one test each, with a `PATH` without `bwrap` and with a place path removed); under
  `report` a missing `bwrap` starts the session in its unit without the fence and records the
  partial box `story:sandbox-spec` declares (a test reads it).
- The provider's `available` also runs `bwrap --ro-bind / / true`; a test with a fake `bwrap` holds
  both answers.
- One recorded live run (`docs/analysis/session-writes-run.md`) of a boxed controller:
  `touch ~/outside-probe` and `python3 -c 'open("/home/<user>/outside-probe-2","w")'` fail with a
  read-only filesystem error, and a write in its checkout succeeds; the session completes a model
  turn, receives a SendMessage, appears in `claude agents --json` and resumes with `claude
  --resume`; the run records whether an OAuth refresh happened.
- `README.md:42`'s "There is no operating-system sandbox yet" and the `bash-sandbox` pointer at
  `docs/design/conductor.md:284` are gone; the README, design § 7, `website/docs/security/limits.md`,
  `the-guard.md` and `overview.md` say the box bounds file writes and does not bound forge or network
  side effects; the `writable` rows of `docs/config.md` name the provider as their reader. A test in
  `crates/conductor/tests/docs_forge.rs` reads the two removed sentences.

## Notes

- After `story:session-limits-systemd` (it extends `confine/systemd.rs` and `docs/config.md`
  after it) and, through the seam, after `story:gitlab-guide` (README, design document and
  `the-guard.md` are shared with it).
- `story:session-forge-credentials` adds its masks to this list, after it.

## Scope

`crates/conductor/src/confine/bwrap.rs` (new), `crates/conductor/src/confine/systemd.rs`,
`crates/conductor/src/confine/mod.rs`, `crates/conductor/tests/confine_bwrap.rs` (new),
`crates/conductor/tests/docs_forge.rs`, `docs/analysis/session-writes-run.md` (new), `README.md`,
`docs/design/conductor.md`, `website/docs/security/limits.md`, `website/docs/security/the-guard.md`,
`website/docs/security/overview.md`, `docs/config.md`.
