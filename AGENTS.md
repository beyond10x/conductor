# AGENTS.md: conductor

Operating guide for agents in this repository. Humans start at [`README.md`](README.md). The
design is [`docs/design/conductor.md`](docs/design/conductor.md); the config fields are
[`docs/config.md`](docs/config.md).

## Purpose

Conductor makes the coordination of coding-agent sessions across one organization's repositories
a recorded, checkable process: every decision conductor takes or relays is a record with a class,
a reason and its evidence, and conductor-dev turns what went wrong into stories and delivers the
fixes in waves.

## Serves

The objectives of the collection this repository moves, by id from `atlas/ROADMAP.md`:

- **O2 — decisions as data, with evidence.** Every decision conductor takes or relays is a record
  with a class, a reason and its evidence.
- **O6 — self-improvement.** conductor-dev turns what went wrong into stories and delivers the
  fixes in waves.

## Who reads this

- **`conductor run`**, a process and no session, runs one instance's loop
  ([`docs/design/conductor-run.md`](docs/design/conductor-run.md)); the conductor profile is retired.
- **The conductor-dev session** (`task maintainer:start`). It develops conductor: code, specification,
  plan, design, profiles; it installs the binary and restarts the loop. Profile:
  [`crates/conductor/roles/maintainer.md`](crates/conductor/roles/maintainer.md), which the binary
  carries (`conductor role show --role maintainer`). Role: design § 12.
- **Any agent changing this repository.**

Repository controllers do not read this file. They follow
the controller profile the binary carries,
[`crates/conductor/roles/controller.md`](crates/conductor/roles/controller.md), and the charter conductor sends them.

Every session reads its profile first, then the instance's `rules.md` in the instance's records
directory (`records` in `conductor config show`), and follows that file's section for its role.
`rules.md` holds the rules of one organization, each with its source; this repository ships none.
A value a profile writes as `section.key`, such as `thresholds.build_slot`, is that field of
`conductor config show --format json`.

## Layout

| path | content |
|---|---|
| `spec/` | the ESS specification: domains, components, scenarios. The source of truth for every noun and command |
| `crates/conductor-model/` | Rust generated from `spec/` by `ess`. Never edited by hand |
| `crates/conductor/` | the `conductor` CLI: handlers, collectors, guard, watch, dashboard, store |
| `crates/conductor/cli.yaml` | the `ess-cli/1` binding: global flags and the local commands outside the `cli:` block |
| `crates/conductor/http.yaml` | the `ess-http/1` binding: the method and path conductord and the instance runtime serve each bound command and view at; `task regen` generates the servers in `crates/conductor-model/src/server/` from it, which nothing serves yet. `tests/api_parity.rs` holds them to the hand-written API against the baseline `tests/fixtures/api_parity.md` |
| `crates/conductor/tests/` | integration tests and fixtures |
| `crates/conductor-docs/` | the documentation generator: reference pages, status page, site provenance |
| `website/` | the documentation site (Docusaurus on the shared Docs System); `website/docs/reference/**`, `website/docs/status.md` and `website/data/status.json` are generated |
| `docs/waves/` | one page per wave: selection, units, outcome |
| `crates/conductor/roles/` | the role profiles, compiled into the binary (`include_str!`): `maintainer.md`, `controller.md`, and the host's `supervisor.md` and `teacher.md`; `conductor role show --role <role>` prints one |
| `crates/conductor/kinds/claude-code/` | the `claude-code` kind's packaging, compiled in: `conductor-settings.json` and `controller-settings.json`, the settings the director and each controller start with when their role names none; both wire the guard as a PreToolUse hook. At start the kind writes the settings and the role's profile into the agent's state (`<state>/sessions/<session name>.settings.json`, `.profile.md`) |
| `conductor.example.yaml` | an example config file |
| `Taskfile.yml` | every command below |

## Who writes what

| who | writes |
|---|---|
| conductor | its records, in the instance's records directory: `decisions/`, `dispatches/`, `charters/`, `docs/handoff/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md`, `REPORT.md`; `.engineering/workspace.yaml`, through `conductor config workspace` |
| conductor-dev | in this repository: `crates/`, `spec/`, `.engineering/`, `docs/` (`docs/analysis/` included), `website/` (the hand-written pages), `Taskfile.yml`, `Cargo.*`, `AGENTS.md`, `README.md`; and the instance's `rules.md` |
| the `conductor` CLI | the store under the state directory; and, from the commands that record an event (dispatch, report, message, resource answer, charter revision, decision, request), one line each in the records' `dispatches/` or `decisions/` log |

Each session commits only the files it writes, with a pathspec.

## Commands

| command | does |
|---|---|
| `task check` | the gate: a Gates scan of every tracked file when `B10X_GATES_POLICY` names a policy (`b10x-gates scan-text`), `ess specify validate --path spec`, the conformance run, `crates/conductor-model` compared byte for byte with a fresh generation, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `task docs-check`, and for the dashboard app (`crates/conductor/dashboard`, Node as `.nvmrc` pins) `npm ci`, Vitest, a check that the committed `dist/` equals a fresh `task ui` build, and the dashboard's UX gate on it (`task ui:ux-gate`) |
| `conductor doctor`, `conductor init`, `conductor trust` | the programs conductor starts and their pins (`b10x.toml`); a new instance's records skeleton; trust for the checkouts and the records in `~/.claude.json` (`task trust` calls it) |
| `task measure` | the wall-clock measurements, ignored by the gate (append cost against store size, guard latency, the dashboard's warm answers); run alone on a quiet host |
| `task regen` | regenerates `crates/conductor-model` from `spec/` and `crates/conductor/http.yaml`; a change to `spec/` re-pins the binding's `source_digest` (`ess specify http validate --path spec --binding crates/conductor/http.yaml` names the new one) |
| `task docs-generate`, `task docs-check` | `conductor-docs generate` writes `website/docs/reference/**`, `website/docs/status.md` and `website/data/status.json`; `--check` fails on any difference. Never edit those files by hand |
| `npm --prefix website ci && npm --prefix website run build` | builds the documentation site; `.github/workflows/pages.yml` runs it on every push |
| `task ux-build`, `task ux-gate`, `task ux-round -- R-nn` | the UX loop for `website/` (the `ui-iteration-loop` skill): `ux/gate.mjs` measures the built site at 1280x800 and 390x844 in both themes (layout contract, axe-core, screen-only bot) and fails on a value worse than `ux/baseline.json` or an improvement not taken into it (`node ux/gate.mjs --update`; hard-gate values above 0 need `--decision <id>`); `ux/regression/` holds one assertion per closed finding in `ux/findings/`; `ux/round.mjs` writes `ux/rounds/R-nn/`. Not part of `task check`; Playwright and axe-core are devDependencies of `website/` |
| `task ui:ux-gate`, `task ui:ux-round -- R-nn` | the UX loop for the dashboard app (the `ui-iteration-loop` skill, story:dashboard-ui-loop), in `crates/conductor/dashboard/ux/`: `adapter.ts` names the scenarios (one per page under `ux/scenarios/`, driven state by state from the store fixtures of `ux/fixtures.ts`), the gating viewports 1440x900 and 1280x800, the measured-only ones, both themes, the always-visible header, instance selector and navigation, and the completion state (an answered decision); `ux/gate.ts`, part of `task check` and run without keys or network, runs the contract's break-tests, then the layout contract, axe-core and the screen-only and keyboard-only users over every state from the committed `dist/`, and fails on a value worse than `ux/baseline.json` or an improvement not taken into it (`task ui:ux-gate -- --update`; a hard-gate value above 0 needs `--decision <id>`); `tests/ux-findings.spec.ts` (Vitest) fails on a closed S1 or S2 record in `ux/findings/` without a regression asset; `ux/round.ts` writes `ux/rounds/R-nn/` and a row of `ux/trend.md`. The TypeScript runs on the pinned Node with its type stripping; Playwright and `@axe-core/playwright` are devDependencies of the app, and the gate needs a Playwright Chromium (`crates/conductor/dashboard/node_modules/.bin/playwright install chromium`) |
| `task install` | `cargo install --path crates/conductor --locked`; the binary carries the role profiles and the `claude-code` settings, so nothing else is copied |

A pull request merges when its `task check` job (`.github/workflows/check.yml`) and its Gates job
(`common / Security and privacy`, which scans with the organization's policy; the `task check` job
runs without one) are green on its head commit. Run `task check` locally only where CI cannot run. For a quick loop,
`cargo test -p conductor-cli --test <name>` runs one test file.

## Rules

1. **Spec first.** A new noun, field, command, view or error is modelled in `spec/` first. Then:
   - `ess specify validate --path spec`;
   - `task regen`;
   - implement the handler in `crates/conductor` against the generated types.

   Never edit `crates/conductor-model` by hand, and format with `cargo fmt`, never `cargo fmt --all` (it reformats that crate); `task check` fails on any byte that differs from a
   fresh generation. A change the specification cannot express is reported, not hand-written beside
   it. The `ess` version the specification needs is `requires:` in `spec/ess-inputs.yaml`.
2. **clap derive only.** The CLI parses arguments with clap's derive API. No hand-rolled argument
   parsing.
3. **Rust only.** Running code is Rust. Do not commit Python or shell scripts. Hooks are
   `conductor` subcommands.
4. **No organization in the product.** The code, the tests, the specification and the tasks name no
   organization: an organization is a config file and its `rules.md`.
   `crates/conductor/tests/no_organization.rs` scans `crates/conductor/src`,
   `crates/conductor/tests`, `crates/conductor-model/src`, `spec/`, `crates/conductor/roles/`,
   `crates/conductor/kinds/`, `Taskfile.yml` and `crates/conductor/Cargo.toml`, and allows only the default paths `~/.b10x/conductor` and
   `~/.cache/b10x/conductor`, the Gates scanner's names and the store crates' source. Use
   `example-org` in code, tests and fixtures. No absolute path into a user's home directory:
   write `~/`.
5. **Tests never read the operator's real config or state.** A test that spawns `conductor` sets
   `HOME` to a temporary directory and removes `CONDUCTOR_CONFIG` and `CONDUCTOR_INSTANCE` (or
   clears the environment), and passes `--state-dir` or `--config` under that directory. A test
   that touches `~/.b10x/conductor` or a real checkout is a defect. The command comes from
   `common::conductor` (`crates/conductor/tests/common/mod.rs`); a test that calls the library
   fixes the instance of its process first with `active::isolate`
   (`crates/conductor/tests/active/mod.rs`). `crates/conductor/tests/isolated_home.rs` fails on
   a test file that does neither.
6. **The guard answers fast.** `conductor guard record-guard-decision --from-pre-tool-use` runs on
   every tool call of conductor and of every controller. An allowed call must not open the store;
   for a conductor-role write it reads one small file, the guard's copy of the conductor lease
   (`conductor.lease` in the state directory). A denial is recorded within a bound
   (`RECORD_DEADLINE` in `src/guard.rs`).
   `tests/guard_latency.rs` measures it; it is ignored by default:
   `cargo test --release -p conductor-cli --test guard_latency -- --ignored --nocapture`.
7. **Views create nothing.** A view opens only an existing store. A command creates the store.
8. **Every problem names where it is.** A config problem names its YAML path; a refused command
   names the specification's error on one line of standard error and exits 1. A handler that is not
   written yet exits 2.
9. **Profiles stay generic.** `crates/conductor/roles/` and `crates/conductor/kinds/` name no
   organization, repository list, person or grant. The repository has no `.claude/` directory
   (`tests/roles.rs`): what a harness reads is product data the binary carries. What is true for one organization goes in its `rules.md`; a value that differs
   per instance is a config key, written `section.key` in the profile. A change to a profile, a
   settings file or a guard rule is reported with its diff.
10. **Decision classes and message formats are the design's** (§ 5, § 6). A message whose first
    line does not parse is answered with the format, not acted on. An unclear class is C: the
    option that can be undone, recorded with why (design § 5).
11. **Facts live with their owner.** Work is in each repository's AEP store; issues, pull requests,
    CI and releases are on the forge. Conductor's records hold decisions, dispatches, charters and
    hand-overs only; a copied fact there is a defect.

## Worktrees and builds

- Work in a managed worktree, not in the primary checkout: `worktree create --purpose <what>`,
  and end with `worktree finish --discard-cache --archive <tree>`.
- `conductor build run` owns the target directory. It sets `CARGO_TARGET_DIR` for a cargo-based
  command to the repository's one target under the instance's cache, shared by all its trees, so
  the dependencies are built once; the tree's own `target/` stays empty. Never set
  `CARGO_TARGET_DIR` yourself: `build run` keeps a value you set and takes no lock for it. A
  shared target is safe only under `build run`'s lock: it holds the repository's lock for the
  whole command, not only cargo's build step, so no test runs a binary another tree uplifted
  meanwhile, and a run from another tree than the target's last marks its own files modified, so
  cargo rebuilds the repository's crates rather than reuse the other tree's. Runs of one
  repository queue behind each other; `conductor gc` removes a shared target no run holds.
- Run at most one local `task check` per repository at a time; prefer `cargo test -p conductor-cli`
  on what you touched.

## Commits, gates and public text

- Every commit, push, pull request and tag goes through the bot route:
  `b10x-gates bot -- <git args>`. A GitHub API write goes through `b10x-gates api`.
- A local `task check` that printed "the Gates scan is skipped" ran no secret scan and is no gate.
  Set `B10X_GATES_POLICY` and run it again.
- Every pull-request and release body passes `b10x-gates scan-text` before it is sent.
- This repository is public. Committed text (code, docs, wave pages, commit messages) names no
  private repository, session or decision id, host detail or person.
- End-user pages (`README.md`, `SETUP.md`, `website/docs/` but `develop/` and the generated
  pages) install from the release archive and use `conductor` commands only; a `task` or `cargo`
  install or build step of conductor belongs in `website/docs/develop/`.
- Record a story's gate on the story: `aep plan artifact evidence <id> --kind test_result`.
- Fast-forward the product checkout before a session starts from it: the roles read their profile
  from it.
- `cargo clean` a tree's own `target/` once its gate has run, when a build outside `build run`
  wrote one.

## Releases

A release is a tag `v<version>` on `main` and a GitHub Release of the same name; the binary is
installed from the tag (`cargo install --git https://github.com/beyond10x/conductor --tag
v<version> --locked conductor-cli`) or downloaded pre-built from the Release. Pushing the tag runs
`.github/workflows/release.yml`, which builds `conductor` for `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin` and attaches
`conductor-<version>-<target>.tar.gz` for each and one `SHA256SUMS` to the Release, in the naming
and layout of the `ess` and `aep` releases. Conductor-dev cuts one after a wave merges, about once
a day, without asking: several waves may go into one release.

1. On a branch from `main`, set `version` in the root `Cargo.toml` (semver; `0.x`: a minor bump for
   a new command, config key or record, a patch bump otherwise), refresh `Cargo.lock`
   (`cargo update -p conductor-cli --offline`), and point the install lines at the new version:
   the archive download (`version=`) of the README and `website/docs/getting-started.md`, and the
   `cargo install --tag` of `website/docs/develop/onboarding.md`.
2. The `task check` job (`.github/workflows/check.yml`) and the Gates job green on that exact
   commit; CI runs them in a fresh checkout by construction, on the pull request of step 4. Read
   each job's own conclusion.
   Only where CI cannot run: `task check` in a fresh clone at that commit with `B10X_GATES_POLICY`
   set, exit 0.
3. When the release changes anything onboarding runs — `SETUP.md`, the README pointer to it,
   `conductor setup`, `conductor doctor`, the install path (the release archives) or the
   prerequisites — repeat both onboarding runs of `docs/analysis/onboarding-run.md` on that
   commit (a fresh Claude Code session in a scratch repository with a GitHub `origin` and one
   with a self-hosted GitLab `origin`, each with a planning store one format old, given only the
   repository URL), and append them to that record. Both end `ready`, with questions only for
   logins and the store migration; a gap either hits is fixed on the branch and the runs repeated.
4. A pull request through the bot route, its checks green, then fast-forward `main` to it.
5. `b10x-gates bot -- tag -a v<version> -m <title> <commit>` and `b10x-gates bot -- push origin
   v<version>`; then the GitHub Release through `b10x-gates api` (`POST
   /repos/beyond10x/conductor/releases`, `tag_name`, `name`, `body`: the stories implemented since
   the last tag, one line each). The tag push has started the Release workflow; it attaches the
   archives to this Release when its builds finish. Should it finish first, it creates the Release
   with the tag's message as its notes, and the `body` goes on with `PATCH
   /repos/beyond10x/conductor/releases/<id>` instead. A failed run is re-run, or the archives of an
   existing tag backfilled, with `workflow_dispatch` and `tag: v<version>`.
6. Verify: the tag points at `main`'s commit, the tag's checks are green, the Release workflow's
   run on the tag is green, the Release exists and lists five assets (the four archives and
   `SHA256SUMS`), an archive downloaded from it passes `sha256sum --check` against `SHA256SUMS` and
   its `conductor --help` exits 0, and `cargo install` from the tag with `--root <scratch dir>`
   succeeds and `cargo install --list --root <scratch dir>` names `conductor-cli v<version>`.

## Sessions

The instance's loop is `conductor run`, no session; a conductor session (in its records
directory) is started only by hand, and conductor-dev develops conductor in this repository; they
are named `<session_prefix>-conductor` and `<session_prefix>-conductor-dev`. The prefix is required and
unique within the config file (`conductor config validate` checks it; v1 has no host registry,
design `docs/design/architecture-next.md` § 11); an instance written before that keeps `conductor` and
`conductor-dev` until `conductor config migrate` writes its prefix. Both run in permission mode `bypassPermissions`, so messages
between sessions are not held for approval. Every task below acts on the active instance only:
its sessions by their full names, its dashboard page by its name in `?instance=`.

| task | does |
|---|---|
| `conductor daemon serve --install`, `--uninstall` | install and start, or stop and remove, conductord as the user unit `conductord.service`; it starts each instance's runtime from the config and restarts one on a new binary or placement, with no hand-over (design architecture-next § 4) |
| `task maintainer:start` | start conductor-dev with `conductor session start --role maintainer`, in this checkout, under the maintainer's session name and on its model in `conductor config show` (`host.roles`), with `--permission-mode bypassPermissions --setting-sources project,local`; refuse unless its harness is `claude`, and while a session of that name runs. It appends the role's `profile` with `--append-system-prompt-file`; without one, the maintainer profile the binary carries, which `session start` writes to `<state>/sessions/<session name>.profile.md` |
| `task supervisor:start`, `task teacher:start` | ask the daemon to start the host role now (`conductor daemon start-role --role <role>`); the daemon starts each host role `globals.roles` lists through `conductor session start --role <role>` beside the config file, its session in `conductor_host.slice`, and keeps it running |
| `task conductor:attach`, `task maintainer:attach` | open the active instance's session of that role, by its full name, in this terminal; refuse when no live background session or more than one holds that name |
| `task sessions` | list the active instance's two sessions by their full names; exit 1 naming each name more than one live background session holds |
| `task trust` | mark every checkout under `checkouts.root` and the instance's `records` directory as trusted Claude Code workspaces, so conductor and each controller can start there |
| `task dashboard` | prints the active instance's page, `http://127.0.0.1:<daemon.port>/?instance=<name>`: conductord serves the app and proxies `/v1/instances/<name>/<route>` to the instance's runtime (`conductor instance serve`, on `api.sock` of its state directory); read-only except the answers and settings writes, which carry the daemon's token (same origin) and an operator holding the grant; with `daemon.tls` set, the same app is served on the TLS listener to Identity bearers. No per-instance dashboard process or unit runs |

## CLI quick reference

```console
conductor doctor                                # each program conductor starts, its version and pin
conductor config validate                       # exit 1 naming each problem by its YAML path
conductor config show --format json             # the effective instance after defaults; profiles read section.key here
conductor snapshot start-snapshot               # run every collector; git fetch in every checkout
conductor snapshot <view> --format json         # repositories, pull-requests, workflow-runs, blockers, sessions, ...
conductor repository activity                   # each repository Active or Inactive
conductor guard guard-decisions                 # the guard's recorded denials
conductor store migrate                         # copy an older store into the tree layout; the old one is kept beside it
conductor daemon serve --install                # conductord as a user unit; conductor run --once runs one watch and one cycle tick
conductor message send --to conductor --from <session name> --text -   # a controller's message, on standard input, into conductor's mailbox
```

The views list every kept snapshot's rows; filter on the newest `snapshot_id`. Every command and
flag is in the generated [`website/docs/reference/cli.md`](website/docs/reference/cli.md);
`conductor <group> --help` lists every command of a group.
