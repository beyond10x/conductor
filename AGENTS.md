# AGENTS.md: conductor

Operating guide for agents in this repository. Humans start at [`README.md`](README.md). The
design is [`docs/design/conductor.md`](docs/design/conductor.md); the config fields are
[`docs/config.md`](docs/config.md).

## Who reads this

- **The conductor session** (`task conductor:start`). It runs one instance. Profile:
  [`.agents/conductor.md`](.agents/conductor.md).
- **The conductor-dev session** (`task dev:start`). It develops conductor: code, specification,
  plan, design, profiles; it installs the binary and restarts conductor. Profile:
  [`.agents/conductor-dev.md`](.agents/conductor-dev.md). Role: design § 12.
- **Any agent changing this repository.**

Repository controllers do not read this file. They follow
[`.agents/repo-controller.md`](.agents/repo-controller.md) and the charter conductor sends them.

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
| `crates/conductor/tests/` | integration tests and fixtures |
| `.agents/` | the generic profiles: `conductor.md`, `conductor-dev.md`, `repo-controller.md` |
| `.claude/agents/` | Claude Code adapters, one per profile; each keeps the agent frontmatter and points at its `.agents/` file |
| `.claude/conductor-settings.json`, `.claude/controller-settings.json` | the settings conductor and each controller start with; both wire the guard as a PreToolUse hook |
| `conductor.example.yaml` | an example config file |
| `Taskfile.yml` | every command below |

## Who writes what

| who | writes |
|---|---|
| conductor | its records, in the instance's records directory: `decisions/`, `dispatches/`, `charters/`, `docs/handoff/`, `goals.jsonl`, `STATUS.md`, `NORTHSTAR.md` |
| conductor-dev | in this repository: `crates/`, `spec/`, `.engineering/`, `docs/`, `.agents/`, `.claude/`, `Taskfile.yml`, `Cargo.*`, `AGENTS.md`, `README.md`; and the instance's `rules.md` |
| the `conductor` CLI | the store under the state directory |

Each session commits only the files it writes, with a pathspec.

## Commands

| command | does |
|---|---|
| `task check` | the gate: a Gates scan of every tracked file when `B10X_GATES_POLICY` names a policy (`b10x-gates scan-text`), `ess specify validate --path spec`, the conformance run, `crates/conductor-model` compared byte for byte with a fresh generation, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` |
| `task regen` | regenerates `crates/conductor-model` from `spec/` |
| `task install` | `cargo install --path crates/conductor --locked` |

Run `task check` on the exact commit before you push or merge. For a quick loop,
`cargo test -p conductor-cli --test <name>` runs one test file.

## Rules

1. **Spec first.** A new noun, field, command, view or error is modelled in `spec/` first. Then:
   - `ess specify validate --path spec`;
   - `task regen`;
   - implement the handler in `crates/conductor` against the generated types.

   Never edit `crates/conductor-model` by hand; `task check` fails on any byte that differs from a
   fresh generation. A change the specification cannot express is reported, not hand-written beside
   it. The `ess` version the specification needs is `requires:` in `spec/ess-inputs.yaml`.
2. **clap derive only.** The CLI parses arguments with clap's derive API. No hand-rolled argument
   parsing.
3. **Rust only.** Running code is Rust. Do not commit Python or shell scripts. Hooks are
   `conductor` subcommands.
4. **No organization in the product.** The code, the tests, the specification and the tasks name no
   organization: an organization is a config file and its `rules.md`.
   `crates/conductor/tests/no_organization.rs` scans `crates/conductor/src`,
   `crates/conductor/tests`, `crates/conductor-model/src`, `spec/`, `Taskfile.yml` and
   `crates/conductor/Cargo.toml`, and allows only the default paths `~/.b10x/conductor` and
   `~/.cache/b10x/conductor`, the Gates scanner's names and the store crates' source. Use
   `example-org` in code, tests and fixtures. No absolute path into a user's home directory:
   write `~/`.
5. **Tests never read the operator's real config or state.** A test that spawns `conductor` sets
   `HOME` to a temporary directory and removes `CONDUCTOR_CONFIG` and `CONDUCTOR_INSTANCE` (or
   clears the environment), and passes `--state-dir` or `--config` under that directory. A test
   that touches `~/.b10x/conductor` or a real checkout is a defect.
6. **The guard answers fast.** `conductor guard record-guard-decision --from-pre-tool-use` runs on
   every tool call of conductor and of every controller. An allowed call must not open the store;
   a denial is recorded within a bound (`RECORD_DEADLINE` in `src/guard.rs`).
   `tests/guard_latency.rs` measures it; it is ignored by default:
   `cargo test --release -p conductor-cli --test guard_latency -- --ignored --nocapture`.
7. **Views create nothing.** A view opens only an existing store. A command creates the store.
8. **Every problem names where it is.** A config problem names its YAML path; a refused command
   names the specification's error on one line of standard error and exits 1. A handler that is not
   written yet exits 2.
9. **Profiles stay generic.** `.agents/` and `.claude/` name no organization, repository list,
   person or grant. What is true for one organization goes in its `rules.md`; a value that differs
   per instance is a config key, written `section.key` in the profile. A change to a profile, an
   adapter, a settings file or a guard rule is reported with its diff.
10. **Decision classes and message formats are the design's** (§ 5, § 6). A message whose first
    line does not parse is answered with the format, not acted on. An unclear class is O.
11. **Facts live with their owner.** Work is in each repository's AEP store; issues, pull requests,
    CI and releases are on the forge. Conductor's records hold decisions, dispatches, charters and
    hand-overs only; a copied fact there is a defect.

## Worktrees and builds

- Work in a managed worktree, not in the primary checkout: `worktree create --purpose <what>`,
  and end with `worktree finish --discard-cache --archive <tree>`.
- Build into the tree's own `target/`. Never set `CARGO_TARGET_DIR`: a shared target can run
  another tree's test binaries and report them green.
- Run at most one full `task check` per repository at a time; prefer `cargo test -p conductor-cli`
  on what you touched.

## Sessions

Two background sessions belong to an instance: `conductor` (runs the instance, in its records
directory) and `conductor-dev` (develops conductor, in this repository). Both run in permission
mode `bypassPermissions`, so messages between sessions are not held for approval.

| task | does |
|---|---|
| `task conductor:start`, `task dev:start` | start the session with its adapter (`--agent conductor`, `--agent conductor-dev`) on the model of its role in `conductor config show`; refuse unless the role's harness is `claude`, and while a session of that name runs. Conductor starts with `.claude/conductor-settings.json` |
| `task conductor:attach`, `task dev:attach` | open a session in this terminal |
| `task conductor:restart` | stop conductor and start a new one that reads the newest hand-over; conductor-dev runs it after `[RESTART conductor]` |
| `task sessions` | list both sessions |
| `task trust` | mark every checkout under `checkouts.root` as a trusted Claude Code workspace, so a controller can start there |
| `task dashboard`, `task dashboard:stop` | the read-only page on 127.0.0.1:7313 |

## CLI quick reference

```console
conductor config validate                       # exit 1 naming each problem by its YAML path
conductor config show --format json             # the effective instance after defaults; profiles read section.key here
conductor snapshot start-snapshot               # run every collector; git fetch in every checkout
conductor snapshot <view> --format json         # repositories, pull-requests, workflow-runs, blockers, specifications, sessions, releases, merged-pull-requests, snapshots
conductor repository activity                   # each repository Active or Inactive
conductor repository shipped --since <RFC 3339> # releases and merged pull requests since an instant
conductor resource usage --since <RFC 3339>     # token spend per repository and session
conductor watch run --follow                    # one line per change
conductor guard guard-decisions                 # the guard's recorded denials
conductor decision show --decision-id <id>
conductor store migrate                         # move an older store into the tree layout
```

The views list every kept snapshot's rows; filter on the newest `snapshot_id`.
`conductor <group> --help` lists every command of a group.
