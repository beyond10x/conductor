---
title: Getting started
sidebar_position: 2
description: Take a fresh host to one running instance. Install the CLI, check the programs, write a conductor.config/2 file, write the records, start the substrate daemon and conductord, and see the instance's loop take its first snapshot.
lede: From a fresh host to one instance whose runtime runs, with its loop, under conductord.
source: crates/conductor/src/doctor.rs, config.rs, init.rs, trust.rs, setup.rs, placement.rs, instance.rs, run.rs, daemon/reconcile.rs, daemon/install.rs; b10x.toml; conductor --help
---

New to conductor? Read [why conductor](./concepts/why-conductor.md) first.

This page takes a fresh host to one running instance. To give that instance its first task in a
repository, go on with [one task in an existing repository](./guides/first-task-existing-repo.md).

An agent that sets conductor up in a repository follows
[`SETUP.md`](https://github.com/beyond10x/conductor/blob/main/SETUP.md) instead. Give a Claude
Code session in the repository its raw URL,
`https://raw.githubusercontent.com/beyond10x/conductor/main/SETUP.md`, and ask it to set
conductor up.

The output here writes the home directory as `~`. Versions, ids and times differ on your machine.

## 1. Install

You need `git`, `curl` and `tar`. The
[README](https://github.com/beyond10x/conductor#requirements) lists every program conductor starts.

Each GitHub Release carries `conductor-<version>-<target>.tar.gz` for `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin`, and one
`SHA256SUMS`. Download the archive for your machine, check it, and put the binary on `PATH`:

```console
version=1.1.0 target=x86_64-unknown-linux-gnu
archive="conductor-$version-$target.tar.gz"
url="https://github.com/beyond10x/conductor/releases/download/v$version"
curl -fsSLO "$url/$archive"
curl -fsSLO "$url/SHA256SUMS"
grep " $archive\$" SHA256SUMS | sha256sum --check --strict
tar -xzf "$archive"
mkdir -p ~/.local/bin
install -m 755 "conductor-$version-$target/conductor" ~/.local/bin/
conductor --version
```

On macOS, `shasum -a 256 --check` stands in for `sha256sum --check --strict`. `~/.local/bin`
must be on `PATH`. To build from source instead, see
[developer onboarding](./develop/onboarding.md#build-from-source).

Later, `conductor upgrade` replaces the binary with the newest release, keeping the old one as
`conductor.previous` ([upgrade and roll back](./guides/upgrade-and-rollback.md)).

## 2. Check the programs

```console
$ conductor doctor
host: workstation
system: Linux 6.18.0
architecture: x86_64
conductor 1.1.0
git: present, git version 2.56.0
gh: not needed
glab: not needed
claude: present, 2.1.319 (Claude Code)
jq: present, jq-1.8.2
aep: present, aep 0.79.0
worktree: present, b10x-worktree-cli 0.19.4
ess: present, ess 0.90.0
substrate-daemon: not needed
b10x-gates: present, b10x-gates 0.1.27
pins: none
aep: 0.79.0 meets the build's pin 0.79
worktree: 0.19.4 meets the build's pin 0.18.1
ess: 0.90.0 meets the build's pin 0.90
secrets: keychain namespace default (answers)
store: sqlite ~/.b10x/conductor/conductor.sqlite (does not answer: no database at ~/.b10x/conductor/conductor.sqlite)
daemon unit: conductord.service not running (inactive)
agent kind claude-code: claude 2.1.319 (Claude Code)
ready
```

- The first four lines name the machine and the conductor version. Quote them when you ask for
  help.
- A program is needed only once the config file asks for it: `gh` for a `github` source, `glab`
  for a `gitlab` source, `substrate-daemon` for a `placement` that opens a box. `b10x-gates` is
  needed unless every instance takes `forge.route: direct`: the default route commits and pushes
  through it. With a forge source, doctor also prints the account each forge CLI is logged in as.
- `pins: none` means no `b10x.toml` was found at or above the working directory, so doctor checks
  against the pins the binary was built with.
- Under a program that is missing or too old, doctor prints the command that installs it on this
  machine, such as `  install: brew install jq`; for `aep`, `ess`, `worktree`, `substrate-daemon`
  and `b10x-gates` it downloads the release archive and checks it against the release's
  `SHA256SUMS`.
- The store and the daemon unit do not exist yet. conductord creates both in step 6.

The last line is `ready`, or `not ready:` and what failed, such as
`not ready: substrate-daemon (too old)`. Doctor then exits 1. It writes nothing.

## 3. Write the config

Write `~/.b10x/conductor/conductor.yaml`. `--config` or `CONDUCTOR_CONFIG` names another file.
This instance watches the repositories under `~/src`:

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/store/conductor.sqlite
instances:
  - name: home
    placement: systemd
    session_prefix: home
    sources:
      - local: ~/src
    checkouts:
      root: ~/src
      trees: ~/.local/state/worktree/trees/home
```

- `daemon.store` is the one database every instance writes its records to. conductord refuses a
  file without it. It is in a directory of its own because a box writes the store's whole
  directory: conductord refuses a box whose store lies beside the config file or
  `~/.b10x/conductor/daemon` (`placement.writable-root-shared`), and `conductor config validate`
  names it. To every other command a file without the key still means
  `sqlite:~/.b10x/conductor/conductor.sqlite`, so
  an existing deployment keeps its store.
- `placement: systemd` runs the instance's runtime in a substrate box (step 5).
- `session_prefix` starts the name of every session of the instance, such as `home-alpha`.

In a checkout, `conductor setup --apply` writes such a file for that repository when there is
none ([one task in an existing repository](./guides/first-task-existing-repo.md#1-set-the-checkout-up)).
Where a database is already at `~/.b10x/conductor/conductor.sqlite`, the file it writes names that
one, and `--apply` never changes a `daemon.store` that is written or moves a store.
For a forge source, see [configure an instance](./guides/configure-an-instance.md). Every key is in
the [config reference](/docs/reference/config).

```console
$ conductor config validate
~/.b10x/conductor/conductor.yaml: valid, 1 instance(s): home
$ conductor config show | grep -E '^instances\[0\]\.(records|state|cache):'
instances[0].records: ~/.b10x/conductor/home/records
instances[0].state: ~/.b10x/conductor/home/state
instances[0].cache: ~/.cache/b10x/conductor/home
```

Run `conductor doctor` again: it now needs `substrate-daemon`, at its pin 0.20.0.

## 4. Write the records and trust the checkouts

```console
conductor init
conductor trust
```

`conductor init` writes and commits the instance's records directory. `conductor trust` marks each
checkout under `~/src` and the records directory as trusted Claude Code workspaces in
`~/.claude.json`, so a session can start there. [Initialise the records](./guides/initialise-the-records.md)
says what to write into `rules.md`.

## 5. Start the substrate daemon

With `placement: systemd`, conductord asks the local `substrate-daemon` for a box and runs the
runtime in it. Install it from its own release at the pin doctor names. Substrate's tags carry no
`v`, and its releases carry `x86_64-unknown-linux-gnu` only:

```console
v=0.20.0 target=x86_64-unknown-linux-gnu
archive="substrate-daemon-$v-$target.tar.gz"
url="https://github.com/beyond10x/substrate/releases/download/$v"
curl -fsSLO "$url/$archive"
curl -fsSLO "$url/SHA256SUMS"
grep " $archive\$" SHA256SUMS | sha256sum --check --strict
tar -xzf "$archive"
install -m 755 "substrate-daemon-$v-$target/substrate-daemon" ~/.local/bin/
```

conductord does not start `substrate-daemon`, and doctor checks only its version. You start it
and keep it running:

- inside a delegated cgroup subtree carrying `cpu`, `memory` and `pids`, the subtree passed as
  `--cgroup-root`. The `systemd` adapter needs it, and substrate's
  [Serving exec](https://github.com/beyond10x/substrate#serving-exec) says how to set one up;
- on the socket conductord opens, `$XDG_RUNTIME_DIR/substrate/daemon.sock`, unless
  `CONDUCTOR_SUBSTRATE_SOCKET` in conductord's environment names another;
- with a writable root for each directory the runtime writes: `state-<instance>` for the state
  directory, `records-<instance>` for the records, `cache-<instance>` for the cache and `store`
  for the directory of the sqlite store. Each must exist before the daemon starts;
- with the aperture `conductor`, which the runtime starts with.

```console
state=$(conductor config show --format json | jq -r '.instances[0].state')
install -d -m 700 "$state/sessions" "$XDG_RUNTIME_DIR/substrate" ~/.local/state/substrate/workspaces
install -d -m 700 ~/.b10x/conductor/store ~/.cache/b10x/conductor/home
substrate-daemon \
  --socket "$XDG_RUNTIME_DIR/substrate/daemon.sock" \
  --state ~/.local/state/substrate/state.db \
  --workspaces ~/.local/state/substrate/workspaces \
  --deployment personal \
  --allow-uid "$(id -u)" \
  --cgroup-root <delegated cgroup root> \
  --writable-root "state-home=$state" \
  --writable-root "records-home=$HOME/.b10x/conductor/home/records" \
  --writable-root "cache-home=$HOME/.cache/b10x/conductor/home" \
  --writable-root "store=$HOME/.b10x/conductor/store" \
  --egress-aperture conductor=api.anthropic.com:443/tcp \
  --ca-bundle /etc/ssl/certs/ca-certificates.crt
```

From another terminal, read the daemon's facts. `exec.writable-roots.host` lists the four roots:

```console
curl -s --unix-socket "$XDG_RUNTIME_DIR/substrate/daemon.sock" http://localhost/v1/machine
```

Each further instance with a box adds its own `state-<instance>`, `records-<instance>` and
`cache-<instance>`; `store` is declared once. Directories that lie inside one another are one
root, the outer one. A box that names a root the daemon does not serve is refused, and the
refusal prints the flag to add. The box writes nothing else of the host: not the config file,
and not `~/.b10x/conductor/daemon`. Besides its writable roots it gets conductord's `HOME` and,
read-only, the config file, the directories of `conductor` and `claude`, the checkouts root,
and the CLI and login of each forge among the sources. Where the records lie under the
checkouts root, the box reads the root's other entries one by one, and one box reads at most
four directories, those of `conductor` and `claude` among them: with more, conductord refuses
the box (`placement.mounts-exceeded`) until the
records are outside the checkouts root. An
instance that names a `model_token` also needs `daemon.substrate_slots` and one `--secret-slot`
per slot ([secrets](/docs/security/operators-and-credentials#secrets)).

## 6. Start conductord

```console
conductor daemon serve --install
```

This installs and starts the user unit `conductord.service`. It creates the store, then opens the
instance's box and starts `conductor instance serve` in it. It does so again when the config file
changes, when a runtime stops, and every 60 s ([the host](./concepts/the-host.md)). Check it:

```console
curl -s http://127.0.0.1:7300/v1/instances
journalctl --user -u conductord.service
cat "$state/sessions/runtime.log"
```

`/v1/instances` lists each instance with its placement, state, binary version and when it was
last seen. The journal has one line per action, `<instance>: <action> (<reason>): <result>`. A box
substrate refuses is recorded `PlacementRefused` with substrate's name for the reason, and nothing
falls back ([a runtime is refused](./guides/troubleshoot.md#a-runtime-is-refused)).

The runtime's log says `conductor instance home: the loop runs, holding <state>/run/lock`. The
runtime serves the instance API and runs the instance's loop beside it. The loop's first cycle
tick takes a snapshot at once:

```console
conductor snapshot snapshots
conductor repository activity
```

A repository with a real commit in the last 7 days is Active. The loop starts a controller only
for an Active repository that has a charter, and no repository has one yet.

## Next

- [One task in an existing repository](./guides/first-task-existing-repo.md): a charter, a
  dispatch, a controller and its pull request.
- [Run conductor](./guides/start-conductor.md): the daemon, the loop and the sessions, day to day.
- The [security overview](/docs/security/overview) before a controller runs unattended.
