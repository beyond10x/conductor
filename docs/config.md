# Config reference

The config file names conductor's instances. Its specification is `spec/domains/config.yaml`
(`conductor.config/1`); its defaults and checks are in `crates/conductor/src/config.rs`. A full
example is [`../conductor.example.yaml`](../conductor.example.yaml).

## Where it is read from

| what | order |
|---|---|
| file | `--config <FILE>`, else `$CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml` |
| instance | `--instance <NAME>` (`config validate`, `config show`), else `$CONDUCTOR_INSTANCE`, else the file's `default`, else its only instance; else an error naming the instances |

A file named by `--config` or `$CONDUCTOR_CONFIG` must exist. When the default file does not
exist, a built-in instance applies (below). Every command resolves the instance first; a file that
does not validate stops the command.

`conductor config validate` checks the file and names each problem by its YAML path, such as
`instances[0].cadence.watch`. `conductor config show --format text|json|yaml` prints the effective
instance after defaults, with every path absolute. It writes an absent optional value (`operator`,
`catalog`, a role's `agent` and `settings`) as `null`.

The profiles in `.agents/` refer to config values as `section.key`, such as
`thresholds.disk_low`: that is the field of `conductor config show --format json` for the
instance the session runs.

## Rules for every value

- A key the specification does not declare is an error.
- A path is absolute or starts with `~/`, which is the home directory.
- A duration is a whole number above 0 with `s`, `m`, `h` or `d`: `180s`, `2h`.
- A size is a whole number of GiB with `G`: `15G`.
- A token count is a whole number, or thousands with `k`: `300k`.
- A time of day is `HH:MM`, UTC.
- A count (`controllers.*`, `retention.snapshots`) is a whole number of 1 or more.
- No value may look like a token (a word starting `ghp_`, `github_pat_`, `glpat-` or
  `xox<letter>-`). Such a value is refused by its path and never printed.

## Top level

| key | type | default | meaning |
|---|---|---|---|
| `version` | string | required | must be `conductor.config/1` |
| `default` | instance name | none | the instance a command runs when none is named; must name an instance of the file |
| `instances` | list of instance | required | instance names must be distinct |

## Instance

"Read by" names what uses a field today: the binary's commands, the tasks in `Taskfile.yml`, or a
session profile in `.agents/` (conductor's or the controller's). A field read by nothing is
accepted, defaulted and shown by `config show`.

| key | type | default | read by | meaning |
|---|---|---|---|---|
| `name` | string | required | all | the instance's name: letters, digits, `.`, `_` and `-`, starting with a letter or digit |
| `sources` | list of source | required | collectors, watch, guard | where the repositories come from (below); at least one |
| `checkouts.root` | path | required | collectors, watch, guard, dashboard, `resource usage`, `task trust` | the directory holding the checkouts |
| `checkouts.trees` | path | required | watch, guard, dashboard, `resource usage` | the directory holding their managed worktrees |
| `records` | path | `~/.b10x/conductor/<name>/records` | guard, watch, dashboard, blockers collector, every session | conductor's records: logs, charters, hand-overs, and the instance's `rules.md`. The conductor session runs here |
| `state` | path | `~/.b10x/conductor/<name>/state` | every store command, guard, watch, dashboard | the state directory; `--state-dir` overrides it |
| `cache` | path | `~/.cache/b10x/conductor/<name>` | blockers and specifications collectors, dashboard | exports of `origin/main` and other rebuildable data |
| `roles` | list of role | `conductor`, `conductor-dev`, `controller`, each `claude` / `opus`, no agent, no settings | start tasks, watch, conductor profile | one entry per session role; a list given replaces the whole default |
| `roles[].role` | string | required | | the role name; the tasks, the watch and the profiles know `conductor`, `conductor-dev` and `controller` |
| `roles[].harness` | `claude` \| `codex` | required | | the harness the role runs on; the start tasks start `claude` only |
| `roles[].model` | string | required | | the model name passed to the harness |
| `roles[].agent` | string | none | conductor profile | the harness agent the role starts with, such as `repo-controller` |
| `roles[].settings` | path | none | conductor profile | the settings file the role starts with, such as the repository's `.claude/controller-settings.json`, which wires the guard |
| `controllers.max_working` | integer ≥ 1 | `5` | conductor profile | most controllers working at once; an idle one does not count |
| `controllers.max_subagents` | integer ≥ 1 | `4` | controller profile | most sub-agents one controller runs at once |
| `repositories` | list of rule | `[]` | nothing yet | activity overrides by repository name |
| `repositories[].match` | string | required | | the repository name the rule applies to |
| `repositories[].activity` | `active` \| `inactive` | required | | the activity the rule sets |
| `cadence.cycle` | duration | `2h` | conductor profile | time between two cycles |
| `cadence.daily` | time of day | `08:07` | conductor profile | start of the daily cycle, UTC |
| `cadence.watch` | duration | `180s` | watch | time between two watch passes; `--every` overrides it |
| `cadence.ci` | duration | `900s` | watch | time between two reads of `main` CI; `--ci-every` overrides it |
| `thresholds.context_handover` | tokens | `300k` | watch, profiles | a session's context above which the watch reports it and the session is handed over |
| `thresholds.disk_path` | path | `/` | profiles | the filesystem whose free space the disk thresholds read. The watch and the dashboard read `/` |
| `thresholds.disk_low` | GiB | `100G` | watch, conductor profile | free disk under which the watch reports it and conductor dispatches cleanup |
| `thresholds.disk_clear` | GiB | `110G` | watch | free disk from which a further fall is reported again |
| `thresholds.disk_admit` | GiB | `20G` | conductor profile | free disk under which no session starts and nothing is granted |
| `thresholds.build_slot` | GiB | `60G` | profiles | free disk under which a large build asks for a build slot |
| `thresholds.build_size` | GiB | `10G` | profiles | the expected size above which a build is large |
| `thresholds.gate_stop` | GiB | `15G` | controller profile | free disk under which the full-gate watchdog stops the gate's process group |
| `thresholds.gate_resume` | GiB | `17G` | controller profile | free disk from which the watchdog resumes a stopped gate; must be above `gate_stop` |
| `thresholds.watchdog_every` | duration | `10s` | controller profile | time between two reads of free disk by the watchdog |
| `retention.snapshots` | integer ≥ 1 | `12` | store | complete snapshots whose observations are kept |
| `reports` | list of report | `[]` | nothing yet | where reports go |
| `reports[].channel` | string | required | | the destination |
| `reports[].as` | string | required | | the identity a report is posted as |
| `reports[].when` | list of string | required | | the occasions a report is sent on |
| `authority.class_c` | `conductor` \| `operator` | `conductor` | conductor profile | who decides class-C decisions |
| `authority.class_o` | `conductor` \| `operator` | `operator` | nothing yet | who decides class-O decisions |
| `operator` | string | none | nothing yet | how sessions name the person who decides class O |
| `catalog` | catalog | none | repositories collector | the instance's catalog of repositories (below) |

An instance may leave out every key but `name`, `sources` and `checkouts`. Inside `controllers`,
`cadence`, `thresholds`, `retention` and `authority`, each absent key takes its own default.

## Source

A source is a mapping with exactly one of these keys.

| form | type | meaning |
|---|---|---|
| `github: <owner>` | string | the owner's repositories on GitHub, listed with `gh`; takes no `exclude` |
| `local: <path>` | path | the repositories under a directory, read as they are |
| `gitlab: <group>` | string | a GitLab group or root; the binary does not read it, the session records its rows |
| `exclude: [<path>]` | list of relative path | `local` and `gitlab` only: repositories at or under these paths are not listed and not placed; no `.` or `..` steps; default `[]` |

A repository is the directory holding `.git` at depth 1 or 2 under a root: `<root>/<repo>` or
`<root>/<group>/<repo>`, named by its path under the root. The root is a `local` source's path, and
`checkouts.root` for the other sources.

## Catalog

An instance may name a catalog: a directory in one of its repositories that lists the repositories
the organization tracks, one `.json` file each. Without one, the repositories collector records no
catalog membership.

| key | type | default | meaning |
|---|---|---|---|
| `catalog.repository` | relative path | required | the repository holding the catalog, written as its path under `checkouts.root` (`registry`, `group/registry`); no `.` or `..` steps |
| `catalog.path` | relative path | required | the directory of the catalog in that repository, relative to its top (`catalog/entries`); no `.` or `..` steps |
| `catalog.names` | `plain` \| `hex` | `plain` | how a file's stem spells a repository name: `plain` is the name itself, `hex` the lower-case hex of the name's UTF-8 bytes |

The collector reads the directory on the catalog repository's `origin/main`, never its working
tree. Each recorded repository then carries `in_catalog`: `true` when an entry names it, `false`
when none does. When the instance names no catalog, or the catalog repository has no checkout under
`checkouts.root`, `in_catalog` is absent for every repository (`null` in JSON).

## Roles, agents and settings

The start tasks read the `conductor` and `conductor-dev` roles' `harness` and `model`, and start
the sessions with the repository's own adapters (`--agent conductor`, `--agent conductor-dev`) and,
for conductor, `.claude/conductor-settings.json`.

Conductor starts each controller with the `controller` role's `model`, `agent` and `settings`. The
repository ships the adapter `.claude/agents/repo-controller.md` and the settings file
`.claude/controller-settings.json`, which wires the guard. A controller starts in another
repository's checkout, so the adapter must be reachable from there, for example linked into
`~/.claude/agents/`, and `settings` must be an absolute or `~/` path. A start command passes
`model`, `agent` and `settings` to the harness as single shell words, so each holds only letters,
digits and `.`, `_`, `:`, `/`, `-` (`conductor.config.CommandWord`). The guard denies every session a file-tool write to any
role's `settings` file, and a Bash write form that names it; it does not check where other Bash
commands write (README, Security model).

## Built-in instance

Without a config file, one instance applies, with placeholder values: name and GitHub owner
`example-org`, `checkouts.root` `~/example-org`, `checkouts.trees`
`~/.local/state/worktree/trees/example-org`, `records` the working directory, `state` `state/`
under the working directory, `cache` `~/.cache/conductor-watch`, no catalog, and every other key
at its default. Write a config file before you run conductor for real.
