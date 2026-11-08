# Config reference

The config file names conductor's instances. Its specification is `spec/domains/config.yaml`
(`conductor.config/2`, design `docs/design/architecture-next.md` § 9); its defaults and checks
are in `crates/conductor/src/config.rs`. A full example is
[`../conductor.example.yaml`](../conductor.example.yaml).

Two versions are read. `conductor.config/2` adds the `daemon` and `globals` sections, each
instance's `placement` and `limits`, and each role's `kind`, and removes `dashboard_port` and
`confinement` (Removed in `conductor.config/2`, below). A `conductor.config/1` file is still read
for one release, with those two keys and without the new ones, so a running instance keeps
working until the file is moved to `/2`. Each version refuses the other's keys by their YAML path.

## Where it is read from

| what | order |
|---|---|
| file | `--config <FILE>`, else `$CONDUCTOR_CONFIG`, else `~/.b10x/conductor/conductor.yaml` |
| instance | `--instance <NAME>` (`config validate`, `config show`, `config workspace`), else `$CONDUCTOR_INSTANCE`, else the file's `default`, else its only instance; else an error naming the instances |

A file named by `--config` or `$CONDUCTOR_CONFIG` must exist. When the default file does not
exist, a built-in instance applies (below). Every command resolves the instance first; a file that
does not validate stops the command.

`conductor config validate` checks the file and names each problem by its YAML path, such as
`instances[0].cadence.watch`. `conductor config show --format text|json|yaml` prints the effective
instance after defaults, with every path absolute. It writes an absent optional value (`operator`,
`catalog`, `gates_policy`, `usage.probe`, `usage.watcher`, `session_prefix`, a secret sync's `dry_run` and a secret's `repository`, a role's `agent`, `settings` and `profile`, the controller role's
`session_name`, `conductor.served_by` of an instance with its own conductor, a limit not set) as
`null`.

No two instances of the file share a session prefix (or one nesting with it: one followed by `-`
begins the other), a state directory or a records directory, and in a `conductor.config/1` file a
dashboard port. `conductor config validate` refuses, by the instance's YAML path, an instance
without a `session_prefix`. The host registry (`instances.json`) that compared the instances of
every config file on the host is gone in v1: one daemon reads one file.

`conductor config migrate` reads a `conductor.config/1` file once and writes it as
`conductor.config/2`, one note per change on standard output: `version`; each instance's
`dashboard_port`, `confinement.mode`, `confinement.provider` and `store` dropped;
`confinement.limits` moved to the instance's `limits`; the instance's `placement` written as the
name the `/1` file's provider reads as (`systemd`, `none`); and the roles of the old host file
(`~/.b10x/conductor/host/host.yaml`) moved to `globals.roles`, each without its `session_name`.
It writes no `daemon` section: the daemon starts only on a `daemon.store` the file writes. The
converted file is written whole, so its comments are not kept; the old one is kept beside it as
`<file>.config-1.bak`. A converted file that would not validate is refused, naming each problem,
and nothing is written; `--check` prints the notes and writes nothing. The daemon refuses a
`conductor.config/1` file on one line naming `conductor config migrate`.

`conductor config migrate` also writes a `session_prefix` into each instance that has none: its name in
letters, digits and `-`, with 2, 3, … appended when another instance of the file holds it. It
rewrites an instance's `session_prefix` line, or adds one after its
`- name:` line, keeps every other line and comment, and writes nothing when the edited file would
not read back as the old one with those prefixes; `--check` prints them and writes nothing. A
prefix renames the instance's sessions, its slice and its dashboard's unit, so stop them first
(`conductor session stop --role <role>` and `task dashboard:stop`), migrate, and start them again. For one release a file without a prefix still loads and its instance keeps running
under the bare names; only `config validate` refuses it.

`conductor config workspace` writes the instance's `<records>/.engineering/workspace.yaml`
(`aep.workspace/1`) from the config, replacing the file, and reads no network: one member per git
checkout directly under `checkouts.root` for a `github` source, one per git checkout directly
under its path, but its `exclude`, for a `local` source, one per git checkout under
`checkouts.root` whose `origin` is on a `gitlab` source's `host` and under its group, but its
`exclude`, and the records directory itself. A member's `name` is its directory's name and its `source` that
directory relative to `<records>/.engineering/`. Two sources that give one name to two directories
are refused by both YAML paths. `--check` writes nothing and exits 1 naming each member that
differs from the file. `conductor init` writes the same file into the skeleton it creates.

The role profiles the binary carries (`crates/conductor/roles/`, `conductor role show --role
<role>`) refer to config values as `section.key`, such as
`thresholds.disk_low`: that is the field of `conductor config show --format json` for the
instance the session runs.

## Rules for every value

- A key the specification does not declare is an error.
- A path is absolute or starts with `~/`, which is the home directory.
- A duration is a whole number above 0 with `s`, `m`, `h` or `d`: `180s`, `2h`.
- A size is a whole number of GiB with `G`: `15G`.
- A token count is a whole number, or thousands with `k`: `300k`.
- A time of day is `HH:MM`, UTC.
- A count (`controllers.*`, `retention.snapshots`, a `tasks_max`) is a whole number of 1 or more,
  except `controllers.max_open_reviews`, which is 0 or more (0: no limit).
- No value may look like a token (a word starting `ghp_`, `github_pat_`, `glpat-` or
  `xox<letter>-`). Such a value is refused by its path and never printed.

## Top level

| key | type | default | meaning |
|---|---|---|---|
| `version` | string | required | `conductor.config/2`, or `conductor.config/1` for one release |
| `daemon` | daemon | each key its own (below) | conductord's own section; `conductor.config/2` only |
| `globals` | globals | each key its own (below) | the defaults every instance starts from; `conductor.config/2` only |
| `default` | instance name | none | the instance a command runs when none is named; must name an instance of the file |
| `instances` | list of instance | required | instance names must be distinct; no two share a `session_prefix`, a `state` or a `records` directory, at most one has none, no prefix starts with another followed by `-`, and no prefix followed by `-` begins a name an instance without a prefix keeps (`conductor-dev`); in a `conductor.config/1` file no two share a `dashboard_port`, counting the port filled for an instance that writes none |

## Daemon

The `daemon` section (`conductor.config.Daemon`) configures conductord, the one process per
deployment that reconciles the instances, serves the global API and the dashboard, and holds the
store and the secrets. Nothing reads it yet but `config validate` and `config show`, which writes
it for a `conductor.config/2` file.

| key | type | default | meaning |
|---|---|---|---|
| `daemon.port` | port | `7300` | the loopback port of the global API on 127.0.0.1: a whole number from 1 to 65535 |
| `daemon.loopback` | `true` or `false` | `true` | whether the daemon serves the loopback listener on `daemon.port`. A box with no local operator writes `false`: its API is the TLS listener alone, so `false` without `daemon.tls` is refused by this key's path, and `conductor daemon serve --install` writes the system unit `/etc/systemd/system/conductord.service` (running as the installing user, with its home) rather than the user unit, refused while the config file lies in no git checkout or while `daemon.secrets.source` is `keychain`. A workstation reaches it with `conductor login --daemon <url>`, then `--daemon <url>` (or `CONDUCTOR_DAEMON`) on `config put` and `instance upgrade` |
| `daemon.tls.listen` | `<host>:<port>` | none (no TLS listener) | the TLS listener, for a dashboard deployed elsewhere and for every client of a daemon on a remote box. It serves the same app and `/v1/` as the loopback listener, both at once; every request but the app's files and a CORS preflight carries `Authorization: Bearer`, which the Identity service resolves, and any origin may read with one |
| `daemon.tls.cert` | path | required with `listen` | the PEM file of the listener's certificate chain |
| `daemon.tls.key` | path | required with `listen` | the PEM file of the listener's private key |
| `daemon.tls.identity` | URL | required with `listen` | the Identity service that resolves a bearer (`GET <identity>/v1/access-authority`, audience `conductor`): `https://`, or `http://` on `127.0.0.1`, `localhost` or a `*.svc.cluster.local` name. A principal that is not a person is refused `ActorNotHuman` on a write; an operator is named by the principal's email, else its subject |
| `daemon.cadence.reconcile` | duration | `60s` | how often the daemon reconciles when nothing else starts a pass |
| `daemon.store` | store URL | `sqlite:~/.b10x/conductor/conductor.sqlite` | the one store of the deployment, `sqlite:<path>` or `postgres://…` (its password a `secret:<name>`), which the daemon hands to every instance runtime in `$CONDUCTOR_STORE`. A command of the instance on its own `state` opens that URL, else the `daemon.store` the file writes, else `<state>/store.sqlite`; on any other state directory, `<state>/store.sqlite`. `conductor doctor` names the backend and whether it answers |
| `daemon.secrets.source` | `keychain` \| `remote` | `keychain` | where the `secret:<name>` values resolve: the OS keychain, or the Secrets service |
| `daemon.secrets.namespace` | string | none | the secrets namespace the names live in |
| `daemon.secrets.url` | string | none | the Secrets service's address; `remote` only |
| `daemon.dashboard` | string | `/` | the path the daemon serves the dashboard at: on loopback the page carries the daemon's token, the one token a browser holds; the instances' routes are `/v1/instances/<name>/<route>`, proxied to each runtime's socket (`conductor instance serve`, `api.sock` in its state directory, with a token the daemon minted at its start). A write on the global API needs an operator holding its grant (`AnswerDecisions` for answers and the Plan page, `UpgradeInstances` for an upgrade, `ChangeSettings` for settings and `PUT /v1/config`), else it is refused `GrantMissing` naming the operator and the grant; with no `daemon.operators`, the operator the Unix user names holds every grant on loopback |
| `daemon.operators` | list of operator | `[]` | the deployment's operators, each named once |
| `daemon.operators[].name` | string | required | the operator's name: the Identity principal, or the Unix user on a single host |
| `daemon.operators[].role` | string | required | the operator's role in the config, such as `owner` |
| `daemon.operators[].grants` | list | `[]` | what the operator may do, each once: `answer-decisions`, `change-settings`, `start-instances`, `stop-instances`, `upgrade-instances`, `purge`, `join` |
| `daemon.upgrade_timeout` | duration | `120s` | how long an upgraded instance runtime (`conductor instance upgrade`, `POST /v1/instances/{name}/upgrade`) has to answer before the daemon rolls it back to its previous binary |
| `daemon.timeout` | duration | `5s` | how long an instance has to answer one federated read of the global API (`GET /v1/sessions`, `decisions`, `unanswered`, `kpis`, `events`) before it is a row with `state: unreachable` |

## Globals

The `globals` section (`conductor.config.Globals`) holds the defaults every instance starts from.
`roles`, `cadence`, `thresholds` and `limits` take the keys an instance's keys of the same name
take (Instance, below); `tuning` is the tuning grant. The host's roles, the supervisor and the
teacher, which the daemon starts, are written here only; a global role's session is named after
the role, with no instance prefix, and the file may write only that name.

| key | type | default | meaning |
|---|---|---|---|
| `globals.roles` | list of role | `[]` | the built-in and custom roles every instance starts from, each named once |
| `globals.cadence`, `globals.thresholds`, `globals.limits` | as for an instance | the built-in defaults | the global value of each key |
| `globals.tuning.keys` | list | `[]` | the keys a tuning may change, each a `path` with the `min` and `max` it may set, `min` at or under `max` |

Overrides: a key an instance writes overrides the global value, key by key, and a role's own key
overrides both (role over instance over global); a key none of them writes is conductor's
built-in default. An instance's `cadence`, `thresholds` and `limits` start from the global ones,
key by key. An instance role takes each key it leaves out from the global role of the same name
(so it may write only `role` and the keys it changes); an instance that writes no `roles` takes
the built-in roles, each with every key its global role writes. A global role that no instance
names, such as `supervisor`, stays the daemon's. Every command reads the effective value.

`conductor config show --effective [--instance <i>]` prints every key of the instance with its
effective value and the level it came from, `default`, `global`, `instance` or `role`: as text,
one `<path>: <value> (<level>)` line each, a role's key under `roles.<role>.`; with `--format
json` or `yaml`, the rows of `conductor.config.EffectiveSettings` (`setting_key`
`<instance>/<role or ->/<path>`, `instance`, `role`, `path`, `value`, `level`).

## Changing a key

`conductor config put --level global|instance|role [--instance <i>] [--role <r>] --path <key>
[--value <v>] --reason <why>` changes one key of a `conductor.config/2` file at one level
(`conductor.config.PutConfig`, the function `PUT /v1/config` calls): `--path` is the key under
`globals`, the instance or the instance's role, its words joined by `.`; `--value` is read as YAML,
and without it the key is removed at that level, so the level below applies again. A global
change names no instance and no role, an instance change an instance, a role change both;
otherwise it is refused (`LevelScopeMismatch`). A role change needs the role in the instance's
`roles`. The changed file is validated as `config validate` would; a problem refuses the change
(`ConfigInvalid`), naming its YAML path, and nothing is written. A change by an agent (a
session) is refused (`ActorNotHuman`) unless `globals.tuning.keys` names the path and the value
is a number inside its range. An accepted change is written (the whole file, as YAML reads it, so
its comments are not kept), committed alone to the git checkout the file is in, with the actor in
the subject and the reason in the body (a file in no checkout is written and not committed), and
recorded as a `ConfigRevision` (its `ConfigChanged`) in the daemon's store, the URL
`daemon.store` names, under the tenant `daemon`. The daemon reconciles on its next look at the
changed file. A commit that fails puts the old text back and records nothing. `PUT /v1/config` on
the daemon takes the same change as JSON (`level`, `instance`, `role`, `path`, `value`, `reason`)
from an operator holding `ChangeSettings`, as that operator; it answers the revision, or a refusal
`422` (`403` for `ActorNotHuman`) with its one line.

## Secrets

The file holds no secret. A value that looks like a token is refused wherever it is; a key whose
name ends in `token`, `password`, `secret` or `bearer` takes only `secret:<name>` (letters,
digits, `.`, `_` and `-`), the name the daemon's secrets source keeps the value under; and a
`daemon.store` URL's password is `secret:<name>`. Anything else is refused by its YAML path,
without echoing the value.

## Removed in `conductor.config/2`

| key | why | in a `conductor.config/1` file |
|---|---|---|
| `instances[].dashboard_port` | the daemon serves the one dashboard, at `daemon.dashboard` | read and not used: no per-instance dashboard runs (`conductor dashboard serve` is gone) |
| `instances[].confinement` (`mode`, `provider`, `limits`) | an instance's box is its `placement` (a Substrate box); its limits are the instance's `limits` | read as before (Confinement, below); the instance's `placement` is named after the provider and its `limits` are `confinement.limits` |
| `instances[].store` | the store is the deployment's one database, at `daemon.store` | ignored; `conductor config migrate` drops it with one note per instance |
| `host.yaml`, the host file | the host's roles are `globals.roles` | read once, by `conductor config migrate`, into `globals.roles`; otherwise not read: the maintainer is the instance's own maintainer role |
| `instances.json`, the host registry | one daemon reads one file | not read or written |

A `conductor.config/2` file that writes `dashboard_port`, `confinement` or `store` is refused by its YAML
path, naming why. In a `/2` file an instance's sessions start through the `systemd` provider in
`report` mode, inside the instance's `limits`, until placements are read; its dashboard is the
daemon's page with `?instance=<name>`. `conductor doctor` prints no confinement provider rows.

## Instance

"Read by" names what uses a field today: the binary's commands, the tasks in `Taskfile.yml`, or a
role profile the binary carries (`crates/conductor/roles/`). A field read by nothing is
accepted, defaulted and shown by `config show`.

| key | type | default | read by | meaning |
|---|---|---|---|---|
| `name` | string | required | all | the instance's name: letters, digits, `.`, `_` and `-`, starting with a letter or digit |
| `session_prefix` | string | required by `config validate` (`config migrate` writes one) | start tasks, `task dashboard`, `task dashboard:stop`, guard, sessions collector, `controller start-controller`, conductor profile | the prefix of every session the instance starts: conductor is `<prefix>-conductor`, conductor-dev `<prefix>-conductor-dev`, a controller `<prefix>-<repository>`; and of the dashboard's user service, `<prefix>-conductor-dashboard`. Letters, digits and `-`, starting and ending with a letter or digit. `config validate` refuses an instance without one; for one release such an instance still runs, its sessions under the bare names `conductor`, `conductor-dev` and `<repository>`, its slice `conductor.slice` and its service `conductor-dashboard`. No two instances of the file share one. A controller messages only its instance's conductor and conductor only its instance's sessions; an instance that names no `conductor` role is served by the `default` instance's conductor, which its controllers message and which messages its sessions. A session belongs to the instance whose name for the controller of the repository it works in it carries, else to the instance whose prefix it carries; the sessions collector, the watch and the dashboard keep another instance's session out |
| `placement` | `{daemon, adapter}`, or a string | required in `conductor.config/2`; in a `/1` file the confinement provider's name | `daemon: local` (`adapter: linux`): conductord opens one substrate box per instance through `substrate-daemon`'s socket and runs `conductor instance serve` in it; `adapter: systemd`: conductor's own `systemd-run` units, as before | where the instance's box is opened. `daemon` is `local` (the default) or an `https://` URL; `adapter` is `systemd`, `linux`, `oci` or `pod`. A remote daemon also writes `bearer: secret:<name>` (the Identity access credential the daemon's secrets source keeps), `trust_roots` (the PEM file its certificate is verified against, and no other roots) and optionally `server_identity` (the DNS name its certificate carries; the URL's host by default); a local daemon writes none of the three. The box writes the instance's `sessions` directory under its state directory through a writable host root the daemon declares by name: `substrate-daemon --writable-root sessions-<instance>=<state>/sessions`, each character of the instance name outside letters, digits, `_` and `-` written `_`; the directory exists and the daemon's user owns it; conductord creates it mode 0700 on its own host and refuses the box `exec.writable-roots-host-unserved` before anything starts when the daemon does not publish the root. The runtime in the box writes its output to `<state>/sessions/runtime.log`, since substrate ends a session whose output reaches its bound. A remote daemon that does not answer shows its instance `Unreachable`. Its runtime's instance API is reached through one `conductor instance relay` pipe session in the box per request; an agent's attach over the API answers 501 there as it does for a local instance. The box profile names the control host's paths for the conductor binary, the profiles and the harness, so the remote host carries conductor, the harness and tmux at those paths; running `conductor doctor` there is the operator's job. A string is `{daemon: local, adapter: <string>}`; one that names no adapter keeps the `systemd-run` units. `oci` and `pod` are refused on a local daemon |
| `dashboard_port` | port | 7313 plus the instance's position in the file (7313 for the first) | nothing | `conductor.config/1` only, removed in `/2`: read and checked (a whole number from 1 to 65535, no two instances sharing one) but not used, since the daemon serves the one dashboard |
| `sources` | list of source | required | collectors, watch, guard | where the repositories come from (below); at least one |
| `checkouts.root` | path | required | collectors, watch, guard, dashboard, `resource usage`, `task trust`, `config workspace` | the directory holding the checkouts |
| `checkouts.trees` | path | required | watch, guard, dashboard, `resource usage` | the directory holding their managed worktrees |
| `records` | path | `~/.b10x/conductor/<name>/records` | guard, watch, dashboard, blockers collector, `config workspace`, every session | conductor's records: logs, charters, hand-overs, and the instance's `rules.md`. The conductor session runs here |
| `state` | path | `~/.b10x/conductor/<name>/state` | every store command, guard, watch, dashboard, `task dashboard` | the state directory; `--state-dir` overrides it. Where systemctl is not on PATH, `task dashboard` keeps the dashboard's pid in `dashboard.pid` and its output in `dashboard.log` here |
| `cache` | path | `~/.cache/b10x/conductor/<name>` | blockers and specifications collectors, dashboard | exports of `origin/main` and other rebuildable data |
| `roles` | list of role | `conductor`, `conductor-dev`, `controller`, each `claude` / `opus`, no agent, no settings, no profile | start tasks, watch, conductor profile | one entry per session role; a list given replaces the whole default |
| `roles[].role` | string | required | | the role name; the tasks, the watch and the profiles know `conductor`, `conductor-dev` and `controller`; the maintainer, `conductor-dev`, is read from here (Host roles, below) |
| `roles[].kind` | `claude-code` | `claude-code` | | the kind of agent that fills the role's slot; `claude-code`, today's sessions, is the only kind in v1 |
| `roles[].harness` | `claude` \| `codex` | required | | the harness the role runs on; the start tasks start `claude` only |
| `roles[].model` | string | required | | the model name passed to the harness |
| `roles[].agent` | string | none | `task conductor:start` | the harness agent the role starts with when it names no `profile`, in place of the profile the binary carries, found only in the start directory's `.claude/agents/` (`--setting-sources project,local` loads no user-level agent); the `conductor` role, which the binary carries no profile for, defaults to `conductor`. The `controller` role starts from its `profile`, never with `--agent`: `conductor config validate` refuses a controller role that names an `agent` and no `profile` |
| `roles[].settings` | path | none | conductor profile, `task conductor:start`, `task dev:start` | the settings file the role starts with, in place of the one the binary carries (`crates/conductor/kinds/claude-code/conductor-settings.json` for `conductor`, `controller-settings.json` for a controller, each wiring the guard; none for the other roles), written to `<state>/sessions/<session name>.settings.json` at start. Sessions start without the user's settings, so this file carries everything else they need (below). Under a config file a file that pins no instance (no `env.CONDUCTOR_INSTANCE`) starts as a copy pinned to the instance, `<state>/sessions/<session name>.settings.json`, and `conductor config validate` notes it; one that pins another instance is refused (`InstanceNotPinned`) |
| `roles[].profile` | path | none; required for a `controller` role that names an `agent` | conductor profile, `task conductor:start`, `task dev:start` | a profile file that takes the place of the one the binary carries for the role (`crates/conductor/roles/<role>.md`, for `conductor-dev`, `controller`, `supervisor` and `teacher`), read by the harness at start through `--append-system-prompt-file`. Without it the session starts from the binary's text, which the `claude-code` kind writes to `<state>/sessions/<session name>.profile.md`; no adapter is needed either way. A `controller` role the file writes with an `agent` and without a profile is refused by this path |
| `roles[].session_name` | string | derived | `task conductor:start`, `task conductor:restart`, `task dev:start`, `task conductor:attach`, `task dev:attach`, `task sessions` | the name the role's session starts under: `<session_prefix>-<role>`, or `<role>` without a prefix; none for `controller`, whose sessions are named per repository. Derived, never chosen: a file may write only that value. The session tasks act on the active instance's sessions by this full name and refuse a name more than one live background session holds |
| `roles[].read_only_tools` | list of tool names | empty | guard | the tools, beyond the guard's built-in read-only allowlist (`Read`, `Glob`, `Grep`, `WebFetch`, the session's own task and schedule tools and the like; `READ_ONLY` in `crates/conductor/src/guard.rs`), that the guard allows this role's sessions, such as a read-only MCP tool `mcp__<server>__<tool>`. Read for the `conductor` and `controller` roles. Every other tool without a rule of its own is denied; `Edit`, `Write`, `NotebookEdit`, `SendMessage`, `Bash` and `Monitor` keep their rules whatever the list says |
| `roles[].limits.memory_max` | GiB ≥ 1G | none | `session start`, `spawn` (the request); the systemd provider, as the session's unit's `-p MemoryMax=` | the most memory one session of the role may use (its cgroup's `MemoryMax`); at or under `confinement.limits.memory_max`, where that is set |
| `roles[].limits.cpu_weight` | integer 1–10000 | none | `session start`, `spawn` (the request); the systemd provider, as the session's unit's `-p CPUWeight=` | the CPU weight of one session of the role (`CPUWeight`); at or under `confinement.limits.cpu_weight`, where that is set |
| `roles[].limits.tasks_max` | integer ≥ 1 | none | `session start`, `spawn` (the request); the systemd provider, as the session's unit's `-p TasksMax=` | the most processes and threads one session of the role runs at once (`TasksMax`); at or under `confinement.limits.tasks_max`, where that is set |
| `roles[].writable` | list of places and paths | `checkout`, `trees`, `build-cache`, `scratch`, `harness-state`; for `conductor` also `records` | `session start`, `spawn` (the request) | where a session of the role may write (below); a list given replaces the default |
| `roles[].network` | `open` | `open` | `session start`, `spawn` (the request) | the session's network: the host's, the only one there is |
| `roles[].forge_token_env` | list of variable names | empty | `session start`, `spawn` (the request) | the environment variables that carry the forge tokens a session of the role may use, such as `GH_TOKEN` or `GITLAB_TOKEN`: names only, capital letters, digits and `_`, not starting with a digit; the values are never part of the config |
| `roles[].grant` | mapping: `tools`, `repositories`, `paths`, `max_output_bytes`, `ttl` | `tools: [show, search, diff]`, no repository or path rule (the whole working directory), `max_output_bytes: 65536`, no `ttl` (the session's life) | `session start`, `spawn` (the session's registration with the broker); `conductor broker serve` (each call's check) | what each session of the role may call through `tool`: its verbs (`show`, `search`, `diff`), the paths under its working directory they read (relative, no `..`), the most bytes one answer carries (1 or more) and the grant's time to live from the session's start (`30m`, `8h`); `repositories` is kept and not checked yet, since a session reads its one working directory; a key left out keeps its default. A call outside it is refused `NotGranted: <rule>` and recorded |
| `roles[].token_owner` | an operator's name | none | `spawn`, `controller resume-controller` (`story:token-pool`) | a controller's key only: the operator whose Claude token from the instance's pool the controller leases, and it starts with `--remote-control`, so it appears in that owner's Claude app; with no Available token of that owner the start is refused (`NoTokenAvailable`), no other owner's token is taken. Absent, the agent leases the pool's token with the most headroom (the Available one leased the fewest times). With no token in the pool every start is today's |
| `confinement.mode` | `strict` \| `report` | `report` | `conductor session start`, `conductor spawn` | what happens when a session's box cannot be set up: `strict` starts no session, and stops one whose measured box is short of the request; `report` starts it unboxed and records why |
| `confinement.provider` | `systemd` \| `none` | `systemd` | `conductor session start`, `conductor spawn`, `conductor controller resume-controller`, `conductor doctor` | what sets up the box: `systemd`, a `systemd-run --user` unit `conductor_<session name>.service` per session in the instance's slice `conductor_<session_prefix>.slice` (`conductor.slice` without a prefix; a `-` of the prefix written `\x2d`, since systemd reads a `-` in a slice name as nesting, so every instance's slice is a sibling of every other's and no instance's limits cap another's), available where the user manager delegates the `cpu`, `memory` and `pids` controllers (its `cgroup.controllers`); it sets the limits and sets up no write fence yet, so its box is partial: `report` records it, `strict` stops it; `none`, a host with no provider, where `strict` starts no session and `report` starts every one unboxed |
| `confinement.limits.memory_max` | GiB ≥ 1G | none | the systemd provider (`session start`, `spawn`), as the slice's `MemoryMax` (`infinity` when unset) | the most memory the instance's sessions use together (the slice's `MemoryMax`) |
| `confinement.limits.cpu_weight` | integer 1–10000 | none | the systemd provider (`session start`, `spawn`), as the slice's `CPUWeight` (reset when unset) | the CPU weight of the instance's slice (`CPUWeight`) |
| `confinement.limits.tasks_max` | integer ≥ 1 | none | the systemd provider (`session start`, `spawn`), as the slice's `TasksMax` (`infinity` when unset) | the most processes and threads the instance's sessions run at once (`TasksMax`) |
| `conductor.served_by` | instance name | derived | conductor profile, controller profile | the `default` instance whose conductor serves this one, when this one names no `conductor` role; otherwise none. Derived, never chosen: a file may write only that value |
| `conductor.session_name` | string | derived | controller profile, guard | the session name of the conductor that serves the instance: its own `<session_prefix>-conductor` (or `conductor`), or the serving instance's; none when no conductor serves it. Derived, never chosen |
| `controllers.max_working` | integer ≥ 1 | `5` | conductor profile | most controllers working at once; an idle one does not count |
| `controllers.max_subagents` | integer ≥ 1 | `4` | controller profile | most sub-agents one controller runs at once |
| `controllers.max_open_reviews` | integer ≥ 0 | `3` | conductor profile, board, watch, dashboard | pull requests of one repository waiting for a person's review before conductor sends it no new dispatch; `0` is no limit |
| `repositories` | list of rule | `[]` | `metric compute` (`no_releases`) | what holds for a repository, by its name: an activity override, and whether it ships without releases. A rule writes `activity`, `no_releases` or both |
| `repositories[].match` | string | required | | the repository name the rule applies to |
| `repositories[].activity` | `active` \| `inactive` | none | nothing yet | the activity the rule sets |
| `repositories[].no_releases` | string | none | `metric compute` | the reason the repository ships without releases (docs published on merge, a specification consumed at `main`). The repository is left out of every K5 reading, `instance` included, and `metric compute` prints it once: `K5\|<repository>\|<day>: skipped: ships without releases: <reason>`. A repository that releases by semver tag (`v<major>.<minor>.<patch>`) without Release objects needs no rule: the `github` and `gitlab` collectors record such a tag as its release |
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
| `thresholds.silent_session` | duration | `30m` | watch, conductor profile | time a live controller (`working` or `waiting`) and every one of its sub-agents may write nothing to their transcripts before the watch reports `session silent:` |
| `thresholds.cpu_reserve` | cores | `2` | `conductor build run` | cores `build run` leaves to the desktop: its command gets `CARGO_BUILD_JOBS` and `MAKEFLAGS=-j` of the cores less this, unless it sets them, at nice 19 and idle I/O class |
| `thresholds.build_slots` | integer ≥ 0 | `3` | `conductor build run` | `build run` commands granted or running at once on the host, every instance's together: a request waits in the queue, in its order, while this many hold a grant, and is never refused for it; `0` for no bound |
| `thresholds.repo_cache` | `on` or `off` | `on` | `conductor build run` | whether a cargo-based command (`cargo …`, `task …` in a Cargo project, a shell script naming `cargo`) gets its repository's compile cache: `RUSTC_WRAPPER=sccache` and the repository's own `SCCACHE_DIR` (`~/.cache/sccache/<repository>`, `XDG_CACHE_HOME` when set), `SCCACHE_SERVER_PORT` (4300–5299, from the repository's name) and `SCCACHE_CACHE_SIZE`, each unless the environment sets it. Every worktree of a repository shares its cache and its one server; no server is shared between repositories. The server starts outside the command's process group and ends itself after 30 idle minutes. Incremental compilation stays on for the workspace's crates, and sccache does not cache an incremental unit: the cache serves the dependencies. Without `sccache` on `PATH` nothing is set and one line says so. Each grant records the cache's hits and misses over its run (`host grants`) |
| `thresholds.repo_cache_size` | GiB | `2G` | `conductor build run` | the size of each repository's compile cache (`SCCACHE_CACHE_SIZE`) |
| `thresholds.lean_debuginfo` | `true` or `false` | `true` | `conductor build run` | whether a cargo-based command gets `CARGO_PROFILE_DEV_DEBUG=line-tables-only` and `CARGO_PROFILE_TEST_DEBUG=line-tables-only`, each unless the environment sets it or the workspace manifest (the working directory's `Cargo.toml` or its repository root's) sets `debug` of `[profile.dev]`; the test variable also yields to `[profile.test]`. Line tables keep backtraces and shrink each `target/` |
| `thresholds.short_lane_slots` | integer ≥ 0 | `1` | `conductor build run` | build slots besides `build_slots` that admit only a command the ledger knows to be short: at least 3 earlier runs of its repository and command in 14 days, p90 run time at most `short_lane_max`. A short request that finds every regular slot taken takes a free one ahead of longer requests, never ahead of an earlier short one, with at most 4 jobs (`CARGO_BUILD_JOBS`, also an `env CARGO_BUILD_JOBS=<n>` of its command, and `MAKEFLAGS=-j`). The queued line and `host grants` name the lane; `0` turns the lane off |
| `thresholds.short_lane_max` | duration | `3m` | `conductor build run` | the longest p90 run time of a command the short lane admits. A short-lane run that outlives twice this is not stopped; its grant is marked (`lane_overrun`), and its command takes no short-lane slot for 24 hours |
| `thresholds.stalled_session` | duration | `45m` | watch, conductor profile | time a live controller a dispatch was sent to (`working` or `waiting`) may go with neither a report conductor received nor a commit in its checkout or any of its managed trees before the watch reports `session stalled:`, however its transcript moves |
| `thresholds.build_queued` | duration | `30m` | watch, conductor profile | time a host grant of the instance may wait `Requested` before the watch reports `build queued:` |
| `thresholds.tree_stale` | duration | `48h` | `conductor gc`, `conductor host reclaim`, `conductor build run` | time a managed tree that is dirty or unmerged may be untouched (the newest of its registry activity, its directory, its git `HEAD` and `index`, and each changed or untracked path) before reclaim removes its cargo `target/` and lists it `stale, kept`; the tree itself is never removed for it. A tree merged into its default branch on origin, clean apart from `target/` and `.check/`, in use by no process and held by no lease is finished instead, and removed through `worktree finish --discard-cache --archive` and `worktree gc --apply` |
| `retention.snapshots` | integer ≥ 1 | `12` | store | complete snapshots whose observations are kept |
| `reports` | list of report | `[]` | nothing yet | where reports go |
| `reports[].channel` | string | required | | the destination |
| `reports[].as` | string | required | | the identity a report is posted as |
| `reports[].when` | list of string | required | | the occasions a report is sent on |
| `authority.class_c` | `conductor` \| `operator` | `conductor` | conductor profile | who decides class-C decisions |
| `authority.class_o` | `conductor` \| `operator` | `operator` | nothing yet | who decides class-O decisions |
| `usage.probe` | string | none | `usage` collector, watch, `conductor status` | a command, run with `sh -c` on each snapshot, that prints the harness accounts' usage as JSON (below); without it no usage is read |
| `usage.watcher` | systemd unit name | none | `conductor doctor` | the systemd user unit that rotates the accounts; doctor reports it running or not (`systemctl --user is-active`) |
| `operator` | string | none | nothing yet | how sessions name the person who decides class O |
| `catalog` | catalog | none | repositories collector | the instance's catalog of repositories (below) |
| `secret_syncs` | list of secret sync | `[]` | secrets collector, board, conductor profile | the files the organization's own tool publishes into GitHub Actions secrets, whose drift the snapshot reads (below) |
| `gates_policy` | path | none | `conductor repository refresh-hooks --apply` | the Gates policy file each checkout's hooks are installed with (`b10x-gates install --policy`); without it `--apply` installs none |
| `forge.route` | `gates` \| `direct` | `gates` | guard, `conductor run` (records commit), broker `commit`, `conductor status` | how the instance writes to its forge (below) |
| `limits.memory_max`, `limits.cpu_weight`, `limits.tasks_max` | as `confinement.limits` | none | as `confinement.limits` | `conductor.config/2` only: the instance's box as a whole, every role's limits at or under it; a `/1` file writes it as `confinement.limits` |

An instance may leave out every key but `name`, `sources` and `checkouts`, and in a
`conductor.config/2` file `placement`. Inside `controllers`, `cadence`, `thresholds`,
`retention`, `authority`, `confinement`, `limits` and each role's `limits`, each absent key takes
its own default.

## Forge route

`forge.route` names how the instance's sessions and `conductor run` write to the forge.

| route | pull and merge requests | records commit and push | broker `commit` |
|---|---|---|---|
| `gates` (default) | `gh` and `glab` only read; writes go through the bot route `b10x-gates` | `b10x-gates bot -- commit`, `b10x-gates bot -- push origin main` | `b10x-gates bot`, with the policy in `B10X_GATES_POLICY` |
| `direct` | the guard admits `gh pr create\|edit\|comment\|merge\|ready` and `glab mr create\|update\|note\|merge` (auto-merge included) of the session's own repository, named with `-R`/`--repo` | `git commit`, `git push origin main` | `git commit` |

Under either route the guard denies a write to another repository, a URL naming a request, an
assignment of `GH_HOST`, `GH_REPO`, `GITLAB_HOST`, `GL_HOST`, `GITLAB_URI` or `GITLAB_REPO` on a
direct write, a credential write (class H), an API write, and every other `gh` or `glab` write. A
`git push` from the session's own checkout or managed trees is allowed under both. Gates hooks
are reported stale on the board only under `gates` or with a `gates_policy`. An instance without
`b10x-gates` on its host writes `forge: {route: direct}`.

## Usage

`usage.probe` is any command that prints the accounts of the instance's harness subscriptions
as one JSON object; conductor names no tool. Each snapshot runs it once and records one reading
per window of each account (`conductor snapshot usage-readings`). Only these keys are read:

```json
{"accounts": [
  {"id": "account-a", "active": true, "health": "ready",
   "usage": {"windows": [{"name": "seven_day", "used_percent": 40.0,
                          "resets_at": "2026-10-16T11:00:00Z"}]}}
]}
```

An account is its `id`: every other key of it (an e-mail address, a name, a plan) is never read
or kept. `active` is the account in use; `health` is `exhausted` for an account with nothing left;
an account whose `usage` is `null` is not recorded. A percent is kept as a whole percent, 0 to
100; `resets_at` is RFC 3339 or `null`. An id, a health or a window name holds letters, digits,
`.`, `_`, `:` and `-` only; a probe that prints another value there, exits unsuccessfully or prints
another shape records no reading but one member failure `usage`, whose line never repeats what the
probe printed.

From the newest two complete snapshots that hold readings, the watch and `conductor status` print
`usage: pool runs out ~<hour>, next reset <minute>`: per window, the headroom is the percent left
summed over the accounts that are not `exhausted`, the burn rate what every account used between
the two readings per hour; the pool runs out when its first window does. The watch prints the line
once per change of that hour. `usage.watcher` names the systemd user unit that rotates the
accounts; `conductor doctor` prints `usage watcher: <unit> running`, or `not running (<state>)`.

## Confinement

`confinement` is a key of a `conductor.config/1` file only (Removed in `conductor.config/2`,
above); a `/2` file writes the instance's box as `limits`.

Each session conductor starts is meant to run in a box set up and measured from outside: a
cgroup with the role's limits inside the instance's slice, and writes allowed only to the role's
`writable` places. `confinement` and each role's `limits`, `writable` and `network` are that
request. `conductor session start` and `conductor spawn` resolve them into the request a session's
provider is given, and record what the provider measured. `conductor config validate` refuses an unknown `mode` or `provider`, a limit
that is no limit (a size under 1G, a weight outside 1 to 10000, a `tasks_max` under 1), and a role
limit above the instance's limit of the same key, each by its YAML path.

A `writable` entry is a path, absolute or under `~`, or one of these places, resolved to absolute
paths for each session when it starts, whether or not they exist:

| place | where |
|---|---|
| `checkout` | the directory the session starts in: a controller's repository's checkout, conductor's records directory, conductor-dev's working directory |
| `trees` | the managed worktrees of that checkout, `<checkouts.trees>/<repository>` |
| `build-cache` | the build tools' caches under the home directory: `~/.cargo` and `~/.npm` |
| `scratch` | the session's scratch: `$TMPDIR` (else `/tmp`), and for a controller `<cache>/scratch/<repo>`, under the instance's own `cache` (`<repo>` its repository, `/` written `-`) |
| `harness-state` | the harness's own state: `~/.claude` and `~/.claude.json`, or `~/.codex` |
| `records` | the instance's `records` directory |

The request leaves out readable paths (the box binds the root read-only), the programs a session
may run (a program list belongs to the tool server), and a lifetime (sessions are long-lived and
conductor ends them). What a box actually applied is measured, not taken from this request: the
`envelope_*` columns of `conductor snapshot sessions` hold the provider, the cgroup, the limits read
back and the places applied, or the reason a session runs unboxed; they are empty for a session
nothing measured.

## Source

A source is a mapping with exactly one of these keys.

| form | type | meaning |
|---|---|---|
| `github: <owner>` | string | the owner's repositories on GitHub, listed with `gh`; takes no `exclude` |
| `local: <path>` | path | the repositories under a directory, read as they are |
| `gitlab: <group>` | string | a GitLab group or root, read through `glab api --hostname <host>` |
| `host: <name>` | string | `gitlab` only: the GitLab host, a name with an optional `:port`; default `gitlab.com` |
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

## Secret syncs

A secret sync is a file in one of the instance's repositories that the organization's own tool
publishes into GitHub Actions secrets, which its CI reads, such as a policy file. Conductor never
publishes one and never reads a secret's value: the `secrets` collector records, per sync, the
committer time of the last commit on `origin/<branch>` that touched the file (`git log -1
--format=%cI`, in the source's checkout under `checkouts.root`) and the oldest `updated_at` of its
secrets (`gh api orgs/<owner>/actions/secrets/<name>`, or `repos/<owner>/<repository>/…` for a
repository's secret), which `conductor snapshot secret-syncs` lists. The board names
`secret stale: <name>` on the source repository's row when the file was committed after the oldest
secret was written, and `secret unread: <name>` when either read failed. The conductor profile
runs `command` only where the instance's `rules.md` grants it; otherwise it hands the sync over as
class H with the output of `dry_run`.

| key | type | default | meaning |
|---|---|---|---|
| `secret_syncs[].name` | string | required | the sync's name on the board; no two syncs of an instance share one |
| `secret_syncs[].source.repository` | relative path | required | the repository holding the file, written as its path under `checkouts.root`; no `.` or `..` steps |
| `secret_syncs[].source.branch` | string | `main` | the branch whose `origin/<branch>` the last commit is read on |
| `secret_syncs[].source.path` | relative path | required | the file, relative to the repository's top (`ci/policy.json`); no `.` or `..` steps |
| `secret_syncs[].secrets` | list of secret | required | the secrets the file is published into, one or more |
| `secret_syncs[].secrets[].owner` | string | required | the GitHub organization or user that owns the secret |
| `secret_syncs[].secrets[].repository` | string | none | the owner's repository whose secret it is; absent, it is the owner's own secret |
| `secret_syncs[].secrets[].name` | string | required | the secret's name |
| `secret_syncs[].command` | string | required | the command line that publishes the file into the secrets: the organization's own tool |
| `secret_syncs[].dry_run` | string | none | the command line that shows what `command` would publish, without publishing |

## Roles, agents, profiles and settings

Every session start passes the role's `settings` file with `--settings`. A session the `systemd`
provider starts passes `--setting-sources project,local`, so the user's own settings
(`~/.claude/settings.json`) are not loaded. A session the `none` provider starts with `claude --bg`
passes `--setting-sources user,project,local`: Claude Code starts a background session in
`bypassPermissions` only when the disclaimer acceptance is in user or managed settings, and the
`--settings` file and local settings do not count (`story:bg-launch-reads-user-settings`). The
`--settings` file wins over user settings on every key it sets.
A role's session starts with `--append-system-prompt-file` and no `--agent`: its own `profile`, or
the one the binary carries, written into the state directory (`story:roles-in-the-binary`). With `project,local` Claude Code loads no user-level agent, so `--agent
<agent>` finds an agent only in the start directory's `.claude/agents/`, never in
`~/.claude/agents/`; a controller, which starts in another repository's checkout, therefore always
starts from a profile.

The start tasks read the `conductor` and `conductor-dev` roles' `harness`, `model`, `settings`
and `profile`. `task conductor:start` starts conductor in the instance's `records` directory with
the `conductor` role's `settings` (default the binary's `conductor-settings.json`) and
its `profile`, or without one its `agent` (default `conductor`). It refuses a `profile` that is
not a file and, without a profile, an agent that is not a file in `<records>/.claude/agents/`.
`task maintainer:start` (old name `task dev:start`, for this release) starts the maintainer,
conductor-dev, in this checkout from the instance's maintainer role (Host roles, below) with its
`settings` (default none) and its `profile`, or without one the maintainer profile the binary
carries; it refuses a `profile` that is not a file. `task maintainer:attach` (old name `task dev:attach`) opens it.

Conductor starts each controller with the `controller` role's `model`, `settings` and `profile`,
never with `--agent`: `conductor config validate` refuses a `controller` role that names an
`agent` and no `profile`. The binary carries the controller's profile, `crates/conductor/roles/controller.md`, and its
settings, `crates/conductor/kinds/claude-code/controller-settings.json`, which wire the guard; a
role that names neither starts from them, written into the state directory. A controller starts
in another repository's checkout, where a `profile` it names is read from its path. `settings` and `profile` must be absolute or
`~/` paths. A start command passes `model`, `agent`, `settings` and `profile` to the harness as
single shell words, so each holds only letters, digits and `.`, `_`, `:`, `/`, `-`
(`conductor.config.CommandWord`). The guard denies every session a file-tool write to any role's
`settings` file, and a Bash write form that names it; it does not check where other Bash commands
write (README, Security model).

## Host roles

The maintainer (conductor-dev), the supervisor and the teacher are roles of the host, not of an
instance: one per host, outliving every instance. In v1 they are `globals.roles` of a
`conductor.config/2` file (Globals, above); the host file `host.yaml` that held them before is
gone and is not read.

Until `globals.roles` is applied to the sessions, the maintainer is the instance's own: its
`maintainer` role under `instances[].roles`, or one under its old name `conductor-dev` (the
built-in roles of an instance that writes none hold it), with the session name it has there (such
as `<session_prefix>-conductor-dev`). `conductor session start --role maintainer` (or `--role
conductor-dev`, with a note) starts that role in the working directory, pinned to the instance the
command runs for, and refuses (`UnknownRole`) an instance that names none.
## Sessions without the user's config

With `--setting-sources project,local` (the `systemd` provider) a session reads no user settings, and the settings the
binary carries, `crates/conductor/kinds/claude-code/conductor-settings.json` and
`controller-settings.json`, carry only the guard
wiring, and conductor's also the SessionStart hook that claims the instance's conductor lease
(`conductor session claim`). An instance that needs more gives each role its own `settings` file,
outside this repository, that carries both hooks of the shipped file and everything its sessions
used to take from the user's settings:

| key | what |
|---|---|
| `env` | the environment the sessions need, such as `TMPDIR` and `TMPPREFIX`; each path absolute, since a settings file expands no `~` |
| `attribution` | what the harness adds to commits and pull requests |
| `enabledPlugins`, `extraKnownMarketplaces` | the plugins the sessions use and the marketplaces they come from |
| `skipDangerousModePermissionPrompt` | `true`: the sessions run in `bypassPermissions`, where no prompt is answered. A `claude --bg` session (the `none` provider) takes it only from user or managed settings, so for those it goes in the session user's `~/.claude/settings.json` |
| `claudeMdExcludes` | the absolute path of the user's `~/.claude/CLAUDE.md`, which is otherwise loaded into every session |
| `hooks` | the guard, as in the shipped settings files |

```json
{
  "env": {
    "TMPDIR": "<absolute path>",
    "TMPPREFIX": "<absolute path>"
  },
  "attribution": { "commit": "", "pr": "" },
  "extraKnownMarketplaces": {
    "example-marketplace": {
      "source": { "source": "github", "repo": "example-org/example-marketplace" }
    }
  },
  "enabledPlugins": { "example-plugin@example-marketplace": true },
  "skipDangerousModePermissionPrompt": true,
  "claudeMdExcludes": ["<absolute path of ~/.claude/CLAUDE.md>"],
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "conductor guard record-guard-decision --from-pre-tool-use || exit 2"
          }
        ]
      }
    ]
  }
}
```

The matcher is `*`, as in the shipped files: every tool call reaches the guard, which allows a tool
without a rule of its own only from its read-only allowlist and the role's `read_only_tools`. Its sessions need nothing in `~/.claude/agents/`:
the profile is the binary's, or the role's `profile`.

## Built-in instance

Without a config file, one instance applies, with placeholder values: name and GitHub owner
`example-org`, `checkouts.root` `~/example-org`, `checkouts.trees`
`~/.local/state/worktree/trees/example-org`, `records` the working directory, `state` `state/`
under the working directory, `cache` `~/.cache/conductor-watch`, no catalog, and every other key
at its default. Write a config file before you run conductor for real.
