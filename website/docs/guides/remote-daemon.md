---
title: Run the daemon on another box
sidebar_position: 6
description: Serve conductord on a box with no local operator behind a TLS listener, log in to it from a workstation, send the commands that take --daemon, join an instance's token pool, and place an instance on a remote substrate daemon.
lede: A daemon on another box serves its API on a TLS listener only, every request carries an Identity bearer, and a workstation reaches it with conductor login and --daemon.
source: crates/conductor/src/daemon/mod.rs, daemon/api.rs, daemon/browser.rs, daemon/install.rs; crates/conductor/src/login.rs, remote.rs, token.rs, secret.rs, placement.rs, config.rs; conductor login --help, conductor join --help, conductor daemon serve --help
---

In sections 1 to 4, conductord runs on a server and engineers reach it from their workstations.
In section 5, conductord runs on your workstation and opens one instance's box on another host.

## 1. Serve the daemon behind TLS

On the server, the config file lives in a git checkout of the config repository:

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/store/conductor.sqlite
  loopback: false
  tls:
    listen: 0.0.0.0:7443
    cert: /etc/conductor/tls/chain.pem
    key: /etc/conductor/tls/key.pem
    identity: https://identity.example.com
  secrets:
    source: remote
    url: https://secrets.example.com
  operators:
    - name: engineer@example.com
      role: owner
      grants: [answer-decisions, change-settings, upgrade-instances, join]
instances:
  - name: platform
    placement: linux
    session_prefix: platform
    sources:
      - gitlab: example-group
        host: gitlab.example.com
    checkouts:
      root: ~/example-group
      trees: ~/.local/state/worktree/trees/example-group
```

| key | what it sets |
|---|---|
| `daemon.loopback` | `false`: no listener on `127.0.0.1:<daemon.port>`. Validate refuses it without `daemon.tls` |
| `daemon.tls.listen` | `<host>:<port>` |
| `daemon.tls.cert`, `daemon.tls.key` | the PEM files of the certificate chain and its key |
| `daemon.tls.identity` | the Identity service that resolves each bearer, audience `conductor`. `daemon serve` refuses to start without it, `cert` or `key` |
| `daemon.secrets` | where `secret:<name>` values resolve: `remote`, the Secrets service at `url` |
| `daemon.operators` | each operator by the principal's email or subject, with a role and the grants of [the dashboard's writes](./dashboard.md#4-what-it-writes) |

```console
$ conductor daemon serve --install
```

With `daemon.loopback: false`, `--install` writes the system unit
`/etc/systemd/system/conductord.service` instead of the user unit. It refuses while
`daemon.secrets.source` is `keychain`, while `daemon.tls` is absent, and while the config file is
in no git checkout, or in one that ignores it.

In this release the `remote` secrets source answers `SourceUnreachable`. On such a box no
`secret:<name>` resolves, so an instance there writes no secret and no `model_token`.

## 2. Log in from a workstation

```console
$ conductor login --daemon https://conductor.example.com:7443
conductor login: open this address in a browser and sign in: https://identity.example.com/…
logged in to https://conductor.example.com:7443 as engineer@example.com; the session is kept in this client's keychain
```

`login` runs the Identity service's authorization-code flow with PKCE and keeps the Identity
session in the OS keychain. Each later command mints a short-lived bearer from it.

A browser logs in at `https://conductor.example.com:7443/login` and gets a session cookie, held in
the daemon's memory, so a daemon restart logs every browser out. The TLS listener serves the same
app and `/v1/` as the loopback one, and any origin may read with a bearer.

A cluster probes the daemon at `GET /healthz` and `GET /readyz` and a Prometheus scrapes
`GET /metrics`, none with a bearer ([the host](/docs/concepts/the-host)).

## 3. Send commands to it

`--daemon <url>`, or `CONDUCTOR_DAEMON`, sends these commands to that daemon:

| command | route |
|---|---|
| `conductor login` | the Identity flow above |
| `conductor tui` | the global API, read-only ([the dashboard and the TUI](./dashboard.md#5-the-terminal-ui)) |
| `conductor config put` | `PUT /v1/config` ([configure an instance](./configure-an-instance.md#4-change-one-key)) |
| `conductor instance upgrade` | `POST /v1/instances/<name>/upgrade` ([upgrade and roll back](./upgrade-and-rollback.md)) |
| `conductor join`, `conductor leave` | `POST /v1/instances/<name>/join`, `leave` |

```console
$ export CONDUCTOR_DAEMON=https://conductor.example.com:7443
$ conductor config put --level instance --instance platform --path cadence.cycle --value 3h --reason "fewer snapshots at night"
```

Only those commands read `CONDUCTOR_DAEMON`. Any other command given `--daemon` exits 1:

```text
snapshot start-snapshot: --daemon https://conductor.example.com:7443: this command has no route on the daemon's global API; run it on the daemon's box, or leave --daemon out (the commands that take it: login, join, leave, config put, instance upgrade, tui)
```

A refused write is one line naming the error: `conductor.operator.GrantMissing` with the grant,
or `ActorNotHuman` for a principal that is not a person.

## 4. Join an instance

```console
$ conductor join --daemon https://conductor.example.com:7443 --instance platform < ~/claude-token
joined platform as engineer@example.com (grants AnswerDecisions,ChangeSettings,UpgradeInstances,Join): token <id> in its pool, seat engineer@example.com, Available
```

`join` makes you an operator of the instance with the grants `daemon.operators` gives you, and
puts your Claude token, read from standard input, into the instance's pool. The value goes into
the daemon's secrets source. The pool records its owner, seat and state, never the value.
`--seat` names the Claude login the token draws on, your operator name by default. It needs the
`join` grant, and an agent's session is refused `ActorNotHuman`.

The daemon of section 1 cannot store the value yet, and `join` exits 1:
`conductor: join: refused: SourceUnreachable: the Remote source did not answer; nothing was written`.
A daemon that keeps `loopback: true` beside `daemon.tls`, with the `keychain` source, stores it.

`conductor leave --daemon <url> --instance platform` withdraws every token you contributed.

## 5. Place an instance on a remote substrate daemon

Conductord opens the box through a `substrate-daemon` started there with `--tcp-listen` and `--tcp-bearer-file`:

```yaml
    placement:
      daemon: https://substrate.example.com:7600
      adapter: oci
      bearer: secret:substrate-bearer
      trust_roots: ~/.config/conductor/substrate-roots.pem
      server_identity: substrate.example.com
```

| key | what it sets |
|---|---|
| `daemon` | `local`, the default, or the remote daemon's `https://` URL |
| `adapter` | `systemd`, `linux`, `oci` or `pod`. A local daemon refuses `oci` and `pod` |
| `bearer` | the Identity bearer, written `secret:<name>` only. Required for a remote daemon |
| `trust_roots` | the PEM file the remote certificate is verified against, and no other roots. Required |
| `server_identity` | the DNS name the certificate carries. Absent, the URL's host |

Put the bearer into the daemon's secrets source, the value on standard input:

```console
$ conductor secret put --name substrate-bearer < bearer.txt
```

`conductor config show` writes this placement as `oci@https://substrate.example.com:7600`, with a
derived `remote` holding `url`, `bearer`, `trust_roots` and `server_identity`. A remote daemon
that does not answer within 10 s is `placement.substrate-unreachable`, and the instance shows
`Unreachable`. The daemon reaches the instance API in the box through one
`conductor instance relay` pipe session per request.
