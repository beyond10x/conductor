---
title: Operators and credentials
sidebar_position: 4
description: Who may write through conductord and as whom; actors, grants, login and the TLS listener, dashboard writes, the token pool and secrets.
lede: Every write names its actor, only a person holds a grant, and no config file, record or log holds a credential's value.
source: crates/conductor/src/actor.rs, daemon/api.rs, dashboard.rs, dashboard/answers.rs, login.rs, token.rs, secret.rs, placement.rs, decide.rs; website/docs/reference/config.md
---

## Actors

Every command records an actor. `--actor` sets it as one JSON object:

| kind | `--actor` | is |
|---|---|---|
| `Human` | `{"kind": "Human", "operator": "<name>"}` | a person: the Unix user (`$USER`), or the operator the daemon admitted for an API write |
| `Agent` | `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` | a session of an instance |
| `Daemon` | `{"kind": "Daemon"}` | `conductor run` and each step it starts with `CONDUCTOR_DAEMON_STEP=1` |

Without `--actor` a process is the daemon under `conductor run`, else the agent `CONDUCTOR_SESSION`
names, else the daemon for a run step, else the Unix user. Where `CONDUCTOR_SESSION` is set,
`--actor` names an agent only.

A command reserved to a person refuses an agent actor with `ActorNotHuman`, exit 1:

| group | commands |
|---|---|
| `decision` | `record-operator-decision`, `answer-request`, `answer-escalated-request`, `answer-escalated-as-operator`, `reverse-operator-decision` |
| `goal` | `confirm-goal`, `drop-goal` |
| `metric` | `set-metric-target` |
| `secret` | `put` |

No v1 start path sets `CONDUCTOR_SESSION`, so a session's commands record as the Unix user and
pass this refusal.

## Operators and grants

`daemon.operators[]` entries hold a `name` (the Identity principal's email or subject, or on a
single host the Unix user), a `role` the config names, such as `owner`, and `grants`.

| write | grant |
|---|---|
| `POST /v1/instances/<name>/answers`, `POST …/plan` | `answer-decisions` |
| `POST /v1/instances/<name>/upgrade` (`conductor instance upgrade`) | `upgrade-instances` |
| `POST …/join`, `POST …/leave` | `join` |
| `PUT …/settings`, `PUT /v1/config` (`conductor config put`), `POST …/agents/<agent>/deliver` and `restart`, `POST /v1/ask`, `POST /v1/roles/<role>/start` | `change-settings` |

`start-instances`, `stop-instances` and `purge` have no route served yet. A principal that is not
a person gets `403` with `conductor.operator.ActorNotHuman`. An operator without the grant gets
`403` with `conductor.operator.GrantMissing`. While `daemon.operators` is empty the loopback
operator holds every grant. Once it lists anyone, the Unix user needs one too. The daemon forwards an
admitted write with `X-Conductor-Operator`, and the instance records it as that operator's.

```yaml
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/conductor.sqlite
  secrets: {source: keychain}
  tls:
    listen: 0.0.0.0:7443
    cert: ~/.b10x/conductor/tls/cert.pem
    key: ~/.b10x/conductor/tls/key.pem
    identity: https://identity.example-org.test
  operators:
    - {name: owner@example-org.test, role: owner, grants: [answer-decisions, change-settings, upgrade-instances, join]}
    - {name: reader@example-org.test, role: reader, grants: []}
instances:
  - name: example
    placement: local
    session_prefix: example
    sources: [{github: example-org}]
    checkouts: {root: ~/example-org, trees: ~/.local/state/worktree/trees/example-org}
    model_token: secret:example-model-token
```

## Listeners

| listener | reads | a write needs |
|---|---|---|
| loopback, `127.0.0.1:<daemon.port>` (7300) | open, from any request that names this listener's host or none | this listener's own origin and the daemon's token |
| TLS, `daemon.tls.listen` | an Identity bearer, `Authorization: Bearer`; app files and CORS preflights take none | a person's bearer with the grant |

On loopback, `/` carries the daemon's token in `<meta name="conductor-token">`. A local client
that is no browser reads it from `<daemon dir>/api.token`, mode 0600. The host check stops DNS
rebinding. On the TLS listener the page carries no token, and a request without a bearer gets
`401` naming the `identity` service. A browser logs in at `GET /login` for a session cookie. A
write with the cookie must name the listener's own origin. Bearer reads come from any origin
(`Access-Control-Allow-Origin: *`, no credentials). `daemon.loopback: false` without `tls` is
refused at validate.

The daemon proxies `/v1/instances/<name>/<route>` to the runtime's socket, `<state>/api.sock`,
mode 0600, with the instance's token (`CONDUCTOR_API_TOKEN`), which no browser holds.

## conductor login

`conductor login` logs this client in to the daemon `--daemon <https://…>` or `CONDUCTOR_DAEMON`
names, through the Identity service its `401` names: the authorization-code flow with PKCE (S256),
an address to open in a browser, and up to 300 s of waiting on `http://127.0.0.1:<port>/callback`.
The Identity session goes into this client's keychain, never a file, variable, argument or
log. Each later command mints a short-lived bearer from it (`POST /v1/access-token`, scope
`conductor.operate`), which the daemon resolves at `GET /v1/access-authority`. `login`, `join`,
`leave`, `config put` and `instance upgrade` take `--daemon`. Any other command refuses it. Setup
is the [remote daemon guide](/docs/guides/remote-daemon).

## Dashboard and API writes

The instance API only reads, but for three writes, each with the instance's token in
`X-Conductor-Token`:

| page | write | records |
|---|---|---|
| Decisions | `POST /v1/answers`, answers to open escalated requests | each as the operator's decision, `recorded_by` `dashboard`, the operator's words or "chosen on the Decisions page"; a refused row records nothing |
| Configuration | `PUT /v1/settings` | the file, validated as `config validate` does and refused by YAML path, the old one kept as `<file>.<UTC time>.bak`; one class-C decision by the operator with the diff |
| Plan | `POST /v1/plan` | one `aep plan artifact` verb on a story of a checkout that holds `.engineering/planning`; one class-C decision by the operator, or nothing when the store refuses |

A batch needs `Content-Type: application/json` (else `415`), a `Content-Length` of at most 64 KiB
and no `Transfer-Encoding` (else `411` or `413`), and at most 50 rows (else `413`). The token is
compared in constant time. A wrong one is `403`.

Anything that runs as the operator's user and reads the loopback page holds the daemon's token,
and its writes are recorded as the operator's. The guard's
[check](/docs/security/the-guard#dashboard-answers) is a substring test. Find such answers with
`conductor decision decisions --format json` by `recorded_by`, and reverse one with
`conductor decision reverse-operator-decision`.

## The token pool

An operator who joins an instance contributes a Claude token to a pool its sessions lease from.
`join` and `leave` need the `join` grant.

- `conductor join [--instance <name>] [--seat <seat>]` puts the token from standard input into the
  daemon's secrets source, and records owner, seat (default: the operator name)
  and state.
- `conductor leave` withdraws the caller's tokens. Only an owner withdraws. Each session on one is
  named in `<state>/run/tokens-withdrawn.json`, and the loop resumes it on another token.
- `GET /v1/instances/<name>/tokens` lists owners, seats and states, never a value or a secret's
  name.
- `spawn` and `session start` lease one token per start: the Available one leased fewest times,
  ties by `token_id`. A controller role's `token_owner` takes that owner's tokens only, and starts
  with `--remote-control <session name>`. With none Available it refuses `NoTokenAvailable`.
- The session's settings copy names `conductor token reveal --token <id>` as its `apiKeyHelper`.
  It prints the value of a `Leased` token and refuses any other with `TokenStateConflict`.
- A usage-limit stop marks the token Limited for 5 hours. With an empty pool a start is unchanged.

## Secrets

The config file holds no secret. A value that looks like a token (`ghp_`, `github_pat_`,
`glpat-`, `xox<letter>-`) is refused by its YAML path and never printed. A key whose name ends in
`token`, `password`, `secret` or `bearer`, and the password of a `daemon.store` URL, takes
`secret:<name>` and nothing else.

`daemon.secrets.source` is `keychain`, the OS keychain of the control host, or `remote`, the
Secrets service, which answers `SourceUnreachable` for now.
`conductor secret put --name <name>` reads the value on standard input (one line with echo off from
a terminal) and records the name and source in the daemon's store. A name on record is
`SecretExists` unless `--replace`. `GET /v1/secrets` lists names, sources and instances, never a
value.

Before a box opens, every `secret:` the instance's effective config names and its `model_token`
must be in the source, or the box is refused (`SecretMissing`). The box gets one slot, the model
token. A local daemon takes it from a pool of `daemon.substrate_slots` slots, each declared as
`--secret-slot model_token_<i>=<daemon dir>/slots/model_token_<i>`. The daemon writes the token
into a free slot's file (mode 0600) and frees it when the box closes. No forge token is sealed into
a box: forge writes take the bot route.
