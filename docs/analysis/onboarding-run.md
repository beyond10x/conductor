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
- **The activation waited for step 11** (the step that applies `conductor setup`, step 12 since
  the forge route step). The GitLab session stopped at step 9, which may need the
  person, and left `conductor setup --apply` for later. Step 8 now runs it after
  `conductor trust`: it asks nothing.

Not a gap: the sessions wrote their downloads and a clone of the repository into temporary
directories, some outside the scratch home; the host's own conductor config, `worktree` config
and Claude Code settings were not changed.

## Wave 50 live run

One live run of a boxed runtime on substrate-daemon 0.20.0, on 2026-11-11, with a conductord
built from `wave/50` at `59c8f653f` (`story:box-runtime-starts`,
`story:box-crash-loop-reads-failed`). A scratch home, one `local` source with one repository, one
instance at `placement: systemd`, the store at `<home>/.b10x/conductor/store/conductor.sqlite`;
the substrate daemon and conductord each in a transient user unit of their own, the substrate
daemon with a delegated cgroup subtree, started as SETUP.md steps 6 to 10 write them. Two runs:
in the first the state path was longer than a socket address holds, so the runtime's socket was
`conductor-api-<hash>.sock` in the temporary directory; the second used a shorter home.

What held:

- **The box.** `conductor config validate` exits 0. Before the substrate daemon runs, doctor
  prints `box h: placement.writable-root-missing: the writable root state-h is no directory`
  and ends `not ready`; after the four roots exist and the daemon serves them it prints
  `box h: systemd, 4 writable roots served by the substrate daemon`. The daemon's
  `/v1/machine` answers. conductord records `h: Start (Cadence): Succeeded`.
- **The mounts.** The runtime wrote `runtime.log` into `<state>/sessions`, and the sqlite store,
  its lock and its write-ahead log were written in the store root.
- **The failed start.** The runtime ended within 10 s of each start. conductord recorded
  `Start (LostRuntime): Refused: the runtime of h exited within 10 s of its start (failed start
  1 in a row, the next start in 10 s)` with the runtime's own error text, then failed starts 2
  and 3 with waits of 20 s and 40 s: three starts in 45 s where the loop of v1.1.0 started one
  every 2 s. `GET /v1/instances` answered `"state":"Unreachable"` at 10 s, 25 s and 45 s.

What did not hold:

- **The runtime does not serve its API in the box.** Its error is
  `conductor: instance serve: bind <state>/api.sock: Permission denied (os error 13)`; in the
  first run the same for the socket in the temporary directory. substrate-daemon 0.20.0 denies
  Unix sockets in a box by its seccomp profile, and the daemon has no flag for it. The cases of
  the story hold the box request against a fake substrate, which admits the bind.
  `story:box-runtime-starts` stays active behind `decision-blocker:box-api-unix-socket-denied`;
  the work is `story:box-runtime-api-without-unix-socket`.

Not part of the result: doctor's `substrate-daemon` row read the host's own older
`substrate-daemon` on `PATH`, not the 0.20.0 binary the run started by path, and the scratch
home's checkouts root was not activated as a `worktree` workspace.

## v1.2.0

**Full runs, on the release branch before its tag** (`release/1.2.0` at `d29b33e5d`), done as the
v1.0.1 runs were: a fresh Claude Code session (`claude -p`, resumed for the answer) in a scratch
home with no settings, plugins or forge logins, in a scratch repository whose `origin` names a
repository that does not exist, on GitHub or on a self-hosted GitLab (`gitlab.example.com`), with
a committed `aep.project/1` store. No forge repository was created. The release archive exists
only once the tag is pushed, so the binary built from the release branch
(`cargo build --release -p conductor-cli`) was put at `~/.local/bin/conductor`, and the prompt
said to read the repository's files from the release branch. The person's answer: the migration
approved, the login later, no daemon started on the host. Unlike v1.0.1 each session's temporary
directory was a scratch one, so its downloads stayed in the run's own directory.

The runs repeat because the release changes what onboarding runs: step 6's `daemon.store`, now
in a directory of its own, step 8's Check and step 9's writable roots in `SETUP.md`, the same in
the README and the getting-started page, doctor's `box <instance>:` row, and the store
`conductor setup --apply` writes.

| run | origin | `conductor setup` ended | `conductor doctor` ended |
|---|---|---|---|
| 1 | GitHub | not ready: `gh` not logged in to github.com | not ready: the same, and `box widgets (placement.writable-root-missing)` |
| 2 | self-hosted GitLab | not ready: `glab` not logged in to gitlab.example.com | not ready: the same, and `box widgets (placement.writable-root-missing)` |

One round ran. Each session downloaded the release branch and read `SETUP.md` whole, installed
`aep` 0.79.1, `ess` 0.90.0, `worktree` 0.18.1, `substrate-daemon` 0.20.0 and `b10x-gates` from
their release archives, each checked against its `SHA256SUMS`, and ran `conductor setup --apply`,
which wrote the config file with `daemon.store: sqlite:~/.b10x/conductor/store/conductor.sqlite`
(the GitLab source with its `host`), the records and the trust, and activated the worktree
workspace with the profile it wrote. Each asked only for the forge login and the migration, and
named steps 9 and 10 as not started. After the answer each ran
`aep plan store migrate git --verify` and started nothing. The store is `aep.project/5`, left
uncommitted in the scratch repository; the scratch home's worktree config holds the one profile
`conductor-widgets`; no process of a run was left; the host's own conductor config, `worktree`
config and Claude Code settings were not changed.

`conductor setup` ends `not ready` on the forge login alone, as in v1.0.1, and that is the
verdict both sessions reported. `conductor doctor` names a second reason, new in this release:
`box widgets: placement.writable-root-missing: the writable root state-widgets is no directory`,
the row step 8's Check describes and step 9 clears. The person's answer left step 9 undone, so
the row stays. The two verdicts in the table were read in each scratch home after the sessions
ended: neither session ran `conductor doctor` once the config file existed, since
`conductor setup --apply` printed the rows step 8 checks for, so neither read the box row.

Gaps found: none. No session asked, skipped or got wrong anything because of this release's
documents or binary, so nothing was fixed and no second round ran.

Not a gap: the GitHub session installed four programs at step 4 and `substrate-daemon` only
once `conductor setup --apply` printed its `to install:` line. It asked nothing about it, and
this release does not change step 4.

Not run: steps 9 and 10, which start the substrate daemon and conductord host-wide. The roots
step 9 now writes were read by both sessions and run by neither; the Wave 50 live run above
started both daemons as steps 6 to 10 write them.

Open, not fixed in this release:

- a run that starts no daemon cannot end `conductor doctor` on the logins alone any more: the
  box row needs the substrate daemon serving the instance's roots. The `ready` that `AGENTS.md`
  asks of both runs needs steps 9 and 10 run, which a run on a shared host does not do;
- `conductor setup` prints doctor's rows without the box row, and its verdict does not name the
  box, while step 12 of `SETUP.md` says it prints doctor's rows. Step 13 runs doctor, which does.

## v1.3.0

**Full runs, on the release branch before its tag** (`release/1.3.0` at `e1c704a96` for the first
round, at `4f48aab54`, the fix below, for the second), done as the v1.2.0 runs were: a fresh
Claude Code session (`claude -p`, resumed for the answer) in a scratch home with no settings,
plugins or forge logins, in a scratch repository whose `origin` names a repository that does not
exist, on GitHub or on a self-hosted GitLab (`gitlab.example.com`), with a committed
`aep.project/1` store, and a scratch temporary directory. No forge repository was created. The
release archive exists only once the tag is pushed, so the binary built from the release branch
(`cargo build --release -p conductor-cli`, from an empty build target) was put at
`~/.local/bin/conductor`, and the prompt said to read the repository's files from the release
branch. Both rounds ran the same binary: the fix between them changed `SETUP.md` and a test only.
The person's answer: the migration approved, the login later, no daemon started on the host.

The runs repeat because the release changes what onboarding runs: the forge route, written by
`conductor setup --apply` from the origins of the checkouts under the checkouts root and read in
a step of its own (`SETUP.md` steps 6 and 11), step 9's secret slot, setup ending with doctor's
lines of the config file and its verdict (step 12), doctor's `forge route <instance>:` and
`guard <role>:` lines, the same in the README and the getting-started page, and `conductor init`.

| run | origin | `conductor setup` ended | `conductor doctor` ended |
|---|---|---|---|
| 1 | GitHub | not ready: `gh` not logged in to github.com, and `box widgets (placement.writable-root-missing)` | the same |
| 2 | self-hosted GitLab | not ready: `glab` not logged in to gitlab.example.com, and `box widgets (placement.writable-root-missing)` | the same |

Two rounds ran. In the second, each session read `SETUP.md` whole from the release branch,
installed `aep` 0.79.1, `ess` 0.90.0, `worktree` 0.18.1, `substrate-daemon` 0.20.0 and
`b10x-gates` from their release archives, each checked against its `SHA256SUMS`, and ran
`conductor setup --apply` before it asked anything. Setup wrote the config file, the records, the
trust and the worktree workspace with the profile it wrote, and the instance's route:

- GitHub: `forge route widgets: direct, which --apply wrote: B10X_GATES_POLICY names no readable
  policy file, and the bot route writes under a policy only`;
- GitLab: `forge route widgets: direct, which --apply wrote: the origin is on gitlab.example.com,
  and the bot route writes to github.com only`.

Each asked only for the forge login and the migration, and named steps 9 and 10 as not started.
After the answer each ran `aep plan store migrate git --verify` in its checkout and started
nothing. The store is `aep.project/5`, left uncommitted in the scratch repository; the scratch
home's worktree config holds the one profile `conductor-widgets`; no process of a run was left;
the host's own conductor config, `worktree` config and Claude Code settings were not changed.

What setup and doctor print, read in each scratch home after the sessions ended:

- **The same verdict.** Setup's last line is doctor's verdict, the store's reason added while it
  is old: `not ready: gh (not logged in to github.com), box widgets
  (placement.writable-root-missing)` in the GitHub home, the same with `glab` and
  gitlab.example.com in the GitLab one. Setup now prints the box row, which v1.2.0 left open.
- **The route and the guard.** Both print `forge route widgets: direct`, then
  `guard conductor: not checked (the origin of <records> names no repository)` and
  `guard controller: allowed`. The records `conductor init` writes have no `origin`, so the
  guard has no forge write of the conductor to judge; the line adds nothing to the verdict, and
  no session asked about it.
- **Another forge under the root**, read without a session in a scratch home whose checkouts
  root holds a GitHub checkout and a GitLab one, with `B10X_GATES_POLICY` naming a readable file:
  `conductor setup` in the GitHub checkout prints `forge route widgets: direct, which --apply
  writes: checkouts under <root> not on github.com: gadgets on gitlab.example.com; the bot route
  writes to github.com only`; with the GitLab checkout removed, `forge route widgets: gates,
  policy <file>, which --apply writes: the origin is on github.com and B10X_GATES_POLICY names
  this policy file`.

Gaps found in the first round (fixed on the release branch, and both runs repeated):

- **The migration ran in a worktree.** The GitLab session read conductor's own `AGENTS.md`,
  took its rule for changes to conductor's repository, a managed worktree for each change, for
  the repository it set up, and ran `aep plan store migrate git --verify` in a worktree it
  created. The checkout stayed on `aep.project/1`, and setup in it ended on `store (old)` as
  well. Step 5 now says the migration runs in the repository's own checkout, not in a worktree
  of it, since doctor and setup read the store there.
- **The steps that ask nothing waited for the answer.** The GitHub session asked for the
  migration at step 5 and stopped, leaving steps 6 to 8 for the person's answer. Step 5 now says
  that steps 6 to 8 need no migrated store and go on while the person decides. In the second
  round both sessions ran `conductor setup --apply` before they asked.

`tests/docs_setup.rs` holds both sentences of step 5.

Not a gap: the GitHub session of the second round wrote its install script and downloads into
a scratch directory above its run's own, outside the scratch home, while installing into the
scratch home's `~/.local/bin`; the files were moved into the run's directory afterwards.

Not run: steps 9 and 10, as in v1.2.0.

Open, not fixed in this release:

- `AGENTS.md`, Releases, step 3 still asks that both runs end `ready`. A run that starts no
  daemon ends on the box row as well as the logins, in setup now as in doctor;
- three of the four sessions installed `substrate-daemon` only once `conductor setup --apply`
  printed its `to install:` line, not at step 4: doctor prints `substrate-daemon: not needed`
  before any config names `placement: systemd`, and step 4's sentence to install it even then
  was passed over. None asked about it, and each run ended with it installed.

## v1.4.0

**Full runs, on the release branch before its tag** (`release/1.4.0` at `1f968c4c1` for the first
round, at `712ceae50`, the fix below, for the second), done as the v1.3.0 runs were: a fresh
Claude Code session (`claude -p`, resumed for the answer) in a scratch home with no settings,
plugins or forge logins, in a scratch repository whose `origin` names a repository that does not
exist, on GitHub or on a self-hosted GitLab (`gitlab.example.com`), with a committed
`aep.project/1` store, and a scratch temporary directory. No forge repository was created. The
release archive exists only once the tag is pushed, so the binary built from the release branch
(`cargo build --release -p conductor-cli`, on a cold build target) was put at
`~/.local/bin/conductor`, and the prompt said to read the repository's files from the release
branch. Both rounds ran the same binary: the fix between them changed `SETUP.md` and a test only.
The person's answer: the migration approved, the login later, no daemon started on the host.

The runs repeat because the release changes the prerequisites: `b10x.toml` and the README's
table pin `ess` 0.93 (was 0.90), so step 4 installs `ess` 0.93.0. Doctor also gained the rows of
a daemon that runs its instances as pods (`kubernetes <namespace>:`), printed only where the
config holds `daemon.kubernetes`, which no run writes. `SETUP.md` and `conductor setup` did not
change.

| run | origin | `conductor setup` ended | `conductor doctor` ended |
|---|---|---|---|
| 1 | GitHub | not ready: `gh` not logged in to github.com, and `box widgets (placement.writable-root-missing)` | the same |
| 2 | self-hosted GitLab | not ready: `glab` not logged in to gitlab.example.com, and `box widgets (placement.writable-root-missing)` | the same |

Two rounds ran. In the second, each session read `SETUP.md` whole from the release branch, the
GitHub one from a clone, the GitLab one with `curl`, installed `aep` 0.79.1, `ess` 0.93.0,
`worktree` 0.18.1, `substrate-daemon` 0.20.0 and `b10x-gates` 0.1.27 from their release archives,
each checked against its `SHA256SUMS`, and ran `conductor setup --apply` before it asked
anything. Setup wrote the config file, the records, the trust, the worktree workspace with the
profile it wrote, and the route `direct`, for the same reasons as in v1.3.0. Each asked only for
the forge login and the migration, and named steps 9 and 10 as not started. After the answer
each ran `aep plan store migrate git --verify` in its checkout and started nothing. The store is
`aep.project/5`, left uncommitted in the scratch repository; the scratch home's worktree config
holds the one profile `conductor-widgets`; no process of a run was left; no session pushed or
wrote to a forge; the host's own conductor config, `worktree` config and Claude Code settings
were not changed.

What setup and doctor print, read in each scratch home after the sessions ended: the verdict of
v1.3.0, `not ready: gh (not logged in to github.com), box widgets
(placement.writable-root-missing)` in the GitHub home, the same with `glab` and
gitlab.example.com in the GitLab one; `ess: 0.93.0 meets the build's pin 0.93`; the route and
guard lines of v1.3.0; and no `kubernetes` row, since the config setup writes holds no
`daemon.kubernetes`. The runs end where the v1.3.0 runs ended.

Gaps found in the first round (fixed on the release branch, and both runs repeated):

- **`SETUP.md` was read as a summary.** The GitHub session read the README and `SETUP.md`
  through a web fetch tool, which returned a summary of each. The README's summary dropped its
  `curl`, the fix v1.0.0 made for the same failure, and the summary of `SETUP.md` dropped the
  rules of steps 4, 5 and 8. The session said so and followed doctor's and setup's lines instead:
  it installed four programs without `substrate-daemon`, asked for the migration and left
  `conductor setup --apply` for the answer, asked as well to confirm the source and the route,
  then migrated the store in a worktree, committed it and fast-forwarded the checkout to it. It
  ended `not ready: substrate-daemon (missing), gh (…), box widgets (…)`. `SETUP.md` now opens,
  before its steps, with the `curl` of its raw URL, why a summary is not enough and what one
  drops, so that a summary of the file carries that first. `tests/docs_setup.rs` holds the first
  paragraph to it. The GitLab session of the first round read `SETUP.md` with `curl` and ended
  as in v1.3.0.

In the second round neither session used a web fetch tool, so the new paragraph was not tried
against a summary.

Not a gap:

- the GitLab session of the second round said at its first answer that it would migrate in a
  worktree, then ran the migration in the checkout, citing step 5;
- the GitLab session of the first round read `glab auth status`, which found no token, as a
  login to gitlab.com that failed; the scratch home held no login;
- as in v1.3.0, the sessions wrote their install scripts and downloads into scratch directories
  above their run's own while installing into the scratch home's `~/.local/bin`; the files were
  moved into each run's directory afterwards.

Not run: steps 9 and 10, as in v1.2.0.

Open, not fixed in this release:

- `AGENTS.md`, Releases, step 3 still asks that both runs end `ready`. A run that starts no
  daemon ends on the box row as well as the login (`story:setup-prints-box-row`);
- in the second round the GitHub session again installed `substrate-daemon` only once
  `conductor setup --apply` printed its `to install:` line, not at step 4; the GitLab session
  installed it at step 4. None asked about it, and each run ended with it installed.
