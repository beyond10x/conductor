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

## v1.0.0

**Full runs, on the release branch before its tag.** Each run is a fresh Claude Code session
(`claude -p`, resumed for each answer) in a scratch home with no settings, plugins or forge logins,
in a scratch repository whose `origin` names a repository that does not exist, on GitHub or on a
self-hosted GitLab, with a committed `aep.project/1` store. No forge repository was created. The
release archive exists only once the tag is pushed, so the binary built from the release branch
(`cargo build --release -p conductor-cli`, again after the binary's fix below) was put at
`~/.local/bin/conductor` before each run; from the second round on, the prompt also said to read
the repository's files from the release branch, since `main` did not have the fixes yet. The
person's answers: the logins later, the migration approved after its dry run, and no daemon
started on the host. Steps 9 and 10 start the substrate daemon in a delegated cgroup and a systemd
user unit, host-wide, and were not run.

| run | origin | ended |
|---|---|---|
| 1 | GitHub | not ready |
| 2 | self-hosted GitLab | not ready |

Five rounds ran; the last ended with the store migrated, the config, records and trust written,
and every pinned program installed from its release archive, asking only for the forge login and
the migration. Both runs end not ready for the same reasons only:

- the forge login is not done (the person's);
- the worktree workspace is not activated (open, below).

Gaps found (fixed on the release branch, and both runs repeated):

- **The pinned programs were installed through the page doctor names.** Doctor's and setup's
  `install:` line for `aep`, `ess` and `worktree` names the plugin marketplace's setup page, which
  installs the plugins as well and asks the person first; both sessions of the first round
  followed it instead of step 4's Fix and asked. Step 4 now says to install with its Fix, not that
  page.
- **`SETUP.md` was read as a summary.** A web fetch tool returned a summary of `SETUP.md` without
  step 4's rule, and the session went to the marketplace page again. The README's pointer now says
  to download `SETUP.md` whole with `curl`.
- **`substrate-daemon` was skipped.** Step 4 runs before any config exists, when doctor prints
  `substrate-daemon: not needed`; the session took that as done, and the config step 6 writes
  needs it. Step 4 now says to install it even then.
- **`conductor config show` without a config file read as an old file.** It printed the built-in
  defaults as a whole `conductor.config/1` document and said nothing else; the session planned
  `conductor config migrate` and asked whether to keep the instance. It now says on standard
  error that there is no config file and these are the built-in defaults.

Not a gap: in the first round the GitLab session ran the README's install lines before checking
`conductor --version`, found no `v1.0.0` archive (there is none before the tag) and installed the
newest published release over the build; the build was put back.

Open, not fixed in this release:

- the workspace-activation step in `SETUP.md`: `conductor setup` prints
  `worktree activate --profile <profile> --workspace <root>`, and a fresh person has no profile
  template; every session asked for one;
- doctor's and setup's `install:` line for `aep`, `ess` and `worktree` still names the
  marketplace's setup page; only `SETUP.md` step 4 sends an agent to the release archives;
- the Gates CLI is needed for commits and pushes under the default `forge.route: gates`, and
  neither `SETUP.md` nor doctor names it.

## v1.0.1

**Full runs, on the release branch before its tag**, done as the v1.0.0 runs were: a fresh Claude
Code session (`claude -p`, resumed for the answer) in a scratch home with no settings, plugins or
forge logins, in a scratch repository whose `origin` names a repository that does not exist, on
GitHub or on a self-hosted GitLab, with a committed `aep.project/1` store. No forge repository was
created. The binary built from the release branch (`cargo build --release -p conductor-cli`) was
put at `~/.local/bin/conductor`, and the prompt said to read the repository's files from the
release branch. The person's answer: the migration approved, the login later, no daemon started
on the host. Unlike v1.0.0 the answer said nothing of a worktree profile.

| run | origin | ended |
|---|---|---|
| 1 | GitHub | not ready: `gh` not logged in to github.com |
| 2 | self-hosted GitLab | not ready: `glab` not logged in to gitlab.example.com |

Two rounds ran. In the last, each session installed `aep`, `ess`, `worktree`, `substrate-daemon`
and `b10x-gates` from their release archives, wrote the config, ran `conductor init`,
`conductor trust` and `conductor setup --apply`, which activated the worktree workspace with the
profile it wrote, and asked only for the forge login and the migration. After the migration
`conductor doctor` and `conductor setup` end `not ready` on the forge login alone: a login the
scratch run cannot do, and the only reason left. The store is `aep.project/5`, the scratch home's
worktree config holds the one profile `conductor-widgets`, and the daemon unit not running
changes no verdict.

The three gaps v1.0.0 left open are closed on the release branch:

- **Workspace activation.** `conductor setup --apply` activates the instance's checkouts root
  when it is not activated: it writes a profile template beside the config file,
  `worktree/<instance>.toml`, named `conductor-<instance>` with the instance's `checkouts.trees`
  as its managed root, unless one is there, and runs `worktree activate` with it. Doctor prints
  `conductor setup --apply --repository <checkout>` under a root not activated; a refused
  activation is printed as the exact `worktree activate` command.
- **Install lines.** Doctor's and setup's `install:` line for `aep`, `ess`, `worktree` and
  `substrate-daemon` is the download of the release archive at the pin, checked against
  `SHA256SUMS`, as step 4 writes it; no line names the marketplace page.
- **Gates CLI.** Doctor checks `b10x-gates`, needed unless every instance takes
  `forge.route: direct`, with the direct install of its newest release; step 4 installs it.

Gaps found in the first round (fixed on the release branch, and both runs repeated):

- **Step 4 counted the programs.** Its Check and Fix said "each of the four" once `b10x-gates`
  joined the step; the GitLab session read the four as `aep`, `ess`, `worktree` and
  `b10x-gates`, skipped `substrate-daemon` and asked the person about it. The Check and Fix now
  name each program.
- **The activation waited for step 11.** The GitLab session stopped at step 9, which may need the
  person, and left `conductor setup --apply` for later. Step 8 now runs it after
  `conductor trust`: it asks nothing.

Not a gap: the sessions wrote their downloads and a clone of the repository into temporary
directories, some outside the scratch home; the host's own conductor config, `worktree` config
and Claude Code settings were not changed.
