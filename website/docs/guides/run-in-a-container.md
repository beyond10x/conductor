---
title: Run conductor in a container
sidebar_position: 10
description: Pull the container image each conductor release publishes for linux/amd64, run conductor from it, start one instance in it from a config file and mounted credentials with no prompt, and read which credential comes from where.
lede: Each release pushes one image with the release's own binary and the programs conductor starts, and it runs as a user that is not root. One instance starts in it with no terminal and nothing typed.
source: Dockerfile; .github/workflows/release.yml; .github/workflows/check.yml; crates/conductor/src/doctor.rs, daemon/mod.rs, daemon/unattended.rs, daemon/units.rs, secret.rs, token.rs; crates/conductor/tests/container_start.rs; b10x.toml
---

## The image

Each release after 1.2.0 pushes a container image for linux/amd64 to
`ghcr.io/beyond10x/conductor`. Pulling it needs no login.

| name | is |
|---|---|
| `ghcr.io/beyond10x/conductor:<version>` | the image of one release, by the version without the `v` of its tag |
| `ghcr.io/beyond10x/conductor:latest` | the image of the newest release |
| `ghcr.io/beyond10x/conductor@sha256:<digest>` | exactly one image, whatever a tag names later |

```console
docker run --rm ghcr.io/beyond10x/conductor:latest conductor --version
```

It prints `conductor <version>`, the version of the release the image belongs to.

The notes of each GitHub Release end with a section `## Container image`. It names the release's
tag, `ghcr.io/beyond10x/conductor:<version>`, and the digest of the image behind it.

## What the image holds

The `conductor` binary is the one in the release's `x86_64-unknown-linux-gnu` archive. The image
is built around that file and compiles nothing.

Beside it are the programs `conductor doctor` checks for an instance with a source on a forge:

| program | from |
|---|---|
| `git`, `jq` | the packages of the image's distribution, Debian 13 |
| `gh`, `glab` | a release of each, checked against that release's checksums |
| `claude` | the harness's own installer |
| `aep`, `ess`, `worktree` | their release archives at the pins of `b10x.toml`, checked against `SHA256SUMS` |
| `b10x-gates` | its newest release when the image is built, checked against `SHA256SUMS` |

`substrate-daemon` is not in the image. Only an instance whose `placement` opens a box needs it.

## The user

Every command runs as the user `conductor`, user id 10001, which is not root. Its home
directory is `/var/lib/conductor`, and the image starts there. The image has no entrypoint, so
the words after the image's name are the command, and nothing in it needs a terminal:

```console
docker run --rm ghcr.io/beyond10x/conductor:latest conductor doctor
```

The image holds no config file and no login. Without a config file, `conductor doctor` lists
`gh` and `glab` as `not needed` and every other program above as `present`, and ends `ready`.

## Start one instance with no prompt

A config file and two mounts start one instance. Nothing asks a question: no `conductor init`,
`conductor trust`, `gh auth login` or harness login runs by hand. This config file names one
instance on a GitHub owner:

```yaml
version: conductor.config/2
daemon:
  port: 7300
  address: 0.0.0.0
  unattended: true
  store: sqlite:/var/lib/conductor/conductor.sqlite
  secrets: {source: files, dir: /run/secrets/conductor}
instances:
  - name: example-org
    placement: none
    session_prefix: example-org
    sources: [{github: example-org}]
    forge: {route: direct}
    model_token: secret:model-token
    checkouts: {root: /var/lib/conductor/example-org/checkouts, trees: /var/lib/conductor/example-org/trees}
    records: /var/lib/conductor/example-org/records
    state: /var/lib/conductor/example-org/state
    cache: /var/lib/conductor/example-org/cache
```

Put the model token into a file named `model-token` in a directory of its own, then start the
container. It has no terminal and no standard input:

```console
docker run --detach --init --name conductor \
  --publish 7300:7300 \
  --env CONDUCTOR_CONFIG=/var/lib/conductor/conductor.yaml \
  --env GH_TOKEN \
  --env GIT_AUTHOR_NAME=conductor --env GIT_AUTHOR_EMAIL=conductor@example.com \
  --env GIT_COMMITTER_NAME=conductor --env GIT_COMMITTER_EMAIL=conductor@example.com \
  --volume "$PWD/conductor.yaml:/var/lib/conductor/conductor.yaml:ro" \
  --volume "$PWD/secrets:/run/secrets/conductor:ro" \
  ghcr.io/beyond10x/conductor:latest conductor daemon serve
```

`--env GH_TOKEN` hands the container the variable of the shell that runs `docker`, without its
value on the command line. Mount the config file into the home directory, not under
`~/.b10x/conductor/`, so that directory stays the image user's and the start can write in it.
`--init` puts a process at the container's root that reaps what the agents leave behind.

### Where each credential is read from

| credential | read from | read by |
|---|---|---|
| GitHub token | the variable `GH_TOKEN` of the container | `gh` itself, on every call; conductor reads and stores none |
| GitLab token | the variable `GITLAB_TOKEN` of the container | `glab` itself, on every call; conductor reads and stores none |
| model token | the file `model-token` of the directory `daemon.secrets.dir` names, a read-only mount; the instance's `model_token: secret:model-token` names it | conductor: each agent's settings name `conductor token reveal --model-token <instance>` as their `apiKeyHelper`, which the harness runs when it needs the token |
| git identity of the records' commit | `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL` | `git` |

None of them is written to the image or to the config file. The image holds no token and no
login, and the config file names the model token only as `secret:model-token`. Conductor never
writes into the secrets directory: `conductor secret put` refuses it in one line,
`SourceReadOnly`, and writes nothing. A token in the pool of the instance, contributed with
`conductor join`, is leased first; the model token is what an agent runs on when the pool has
none.

### What the start does

With `daemon.unattended: true`, `conductor daemon serve` does at its start, for each instance of
the file, what `conductor setup --apply` does, each only where it is missing:

1. It writes the records skeleton and commits it, as `conductor init` does. Records that are there
   are left as they are.
2. It creates the checkouts root, then trusts each checkout under it and the records directory,
   as `conductor trust` does. Paths that are trusted already are not written again.
3. It activates the checkouts root as a `worktree` workspace once a checkout is under it, writing
   the profile template `worktree/<instance>.toml` beside the config file when it is not there. A
   root that is activated is left as it is.

A second start of the same container writes nothing of the three. Without the key, the start
creates none of them, as before.

The instance's runtime then starts. `placement: none` opens no box. Where no user systemd manager
runs, as in this image, the runtime is a child process of the daemon. It ends when the daemon
ends, and the daemon starts it again when it stops. A runtime that ends within moments of its
start counts as a failed start, and the next one waits a little longer each time. The instance's
`confinement.limits` are set on a systemd slice, so a child runtime runs without them. A placement
that opens a box, such as `systemd` or `linux`, needs `substrate-daemon`, which the image does not
hold, so a container takes `placement: none`.

`daemon.address: 0.0.0.0` binds the listener on `daemon.port` to every address of the container,
so the published port and a cluster's probes reach it. Its default is `127.0.0.1`. On an address
that is not loopback, `/healthz`, `/readyz` and `/metrics` answer without a bearer, and every
`/v1/` request carries an Identity bearer of `daemon.tls.identity` or is refused `401`. The
commands that reach the daemon with its own token on `127.0.0.1`, `conductor instance upgrade`
and `conductor daemon start-role`, are refused there too.

### What doctor says in the container

`docker exec conductor conductor doctor` ends `ready` when each row below is:

| row | ready when |
|---|---|
| each program | the image holds it at its pin |
| `gh: logged in to github.com as <account>` | `GH_TOKEN` holds a token GitHub accepts: `gh auth status` reads the variable and nothing else. Without it, or with a token GitHub refuses, the row is `gh: not logged in to github.com` and the verdict is `not ready: gh (not logged in to github.com)` |
| `worktree workspace: <root> activated` | the start activated the root. With no checkout under the root yet the row is `not checked: no git checkout in it`, which changes no verdict |
| `forge route example-org: direct` | the instance writes `forge: {route: direct}`; the route `gates` needs a `gates_policy` the container can read |

With a token that is not valid, `gh auth status --hostname github.com` in the image answers
`X Failed to log in to github.com using token (GH_TOKEN)` and `The token in GH_TOKEN is
invalid.`, and `glab auth status --hostname gitlab.com` answers `401 Unauthorized` and
`Token is from environment variable GITLAB_TOKEN`. Both exit 1. The image holds no stored login for either.

The `secrets: files /run/secrets/conductor namespace default (answers)`, `store:` and
`daemon unit: conductord.service not running (systemctl not found)` lines change no verdict.
