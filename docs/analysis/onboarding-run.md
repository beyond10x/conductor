# Onboarding runs

The record of the release onboarding runs (`AGENTS.md`, Releases, step 3). Each release that
changes anything onboarding runs repeats both runs on its release commit: a fresh Claude Code
session, given only the repository URL, in a scratch repository with a GitHub `origin` and in one
with a self-hosted GitLab `origin`, each with a planning store one format old. Both end `ready`,
with questions only for logins and the store migration. Each run is appended here, newest last:
the release, what ran, where each run ended, and the gaps it found with what became of them.

## v0.15.0

**Reduced check (no fresh Claude session: claude -p is not authenticated on the release host).**
The setup steps were followed by hand from the documentation, in both scratch repositories, on the
release candidate.

| run | origin | ended |
|---|---|---|
| 1 | GitHub | not ready |
| 2 | self-hosted GitLab | not ready |

Both runs end not ready for the same reasons only, none of them a gap in the release:

- the local `ess` is 0.59, and `b10x.toml` pins 0.60;
- the forge logins are not done on the host;
- the worktree workspace is not activated.

Gaps found:

- **The documented role settings were refused (fixed on the release branch).** The guides point
  the `conductor` and `controller` roles at the shipped `.claude/conductor-settings.json` and
  `.claude/controller-settings.json`, which pin no instance. Under `story:instance-isolation`
  `conductor config validate` exited 1 (`pins no instance`), and `conductor session start` and
  `conductor spawn` refused the roles (`InstanceNotPinned`). Now a role whose own settings file
  pins no instance starts with a copy pinned to its instance, written to
  `<state>/sessions/<session name>.settings.json`, and `config validate` reports such a file as a
  note. A file pinned to another instance is still refused.

Fixed on the same branch:

- the first-task guide's `roles` write `harness: claude` and `model: opus`, as the
  start-conductor guide does;
- `conductor trust` reads a missing `~/.claude.json` as `{}` instead of failing.

Open, older than this release and not fixed in it:

- the workspace-activation step in `SETUP.md`;
- the retry of the records commit in `conductor setup`.
