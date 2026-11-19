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
| `POST /v1/join`, `POST /v1/leave`, `POST …/join`, `POST …/leave` | `join` |
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
    forge: {route: direct}
```

`forge: {route: direct}` lets a controller write its pull requests with the forge login of the
account it runs as; the default route, `gates`, writes through the bot route under a
`gates_policy` ([forge calls](./the-guard.md#forge-calls)).

## Listeners

| listener | reads | a write needs |
|---|---|---|
| loopback, `127.0.0.1:<daemon.port>` (7300) | open, from any request that names this listener's host or none | this listener's own origin and the daemon's token |
| TLS, `daemon.tls.listen` | an Identity bearer, `Authorization: Bearer`; app files, CORS preflights and the probes take none | a person's bearer with the grant |

On loopback, `/` carries the daemon's token in `<meta name="conductor-token">`. A local client
that is no browser reads it from `<daemon dir>/api.token`, mode 0600. The host check stops DNS
rebinding. On the TLS listener the page carries no token, and a request without a bearer gets
`401` naming the `identity` service. A browser logs in at `GET /login` for a session cookie. A
write with the cookie must name the listener's own origin. Bearer reads come from any origin
(`Access-Control-Allow-Origin: *`, no credentials). `daemon.loopback: false` without `tls` is
refused at validate.

`GET /healthz`, `/readyz` and `/metrics` answer on both listeners without a bearer
([the host](/docs/concepts/the-host)). Their bodies hold states, counts, ages and a fixed reason:
no name, path, token or store URL.

The daemon proxies `/v1/instances/<name>/<route>` to the runtime's socket, `<state>/api.sock`,
mode 0600, with the instance's token, which no browser holds. A runtime started as a unit gets
it in `CONDUCTOR_API_TOKEN`; a runtime in a box of the local substrate daemon gets it through
the sealed slot `api_token_<instance>`, where that daemon declares the slot, and none
otherwise ([the host](/docs/concepts/the-host#placement)). The token lies in two files of the
daemon's directory: `runtimes.json` while the runtime runs, written at the mode the daemon's
umask gives, and the slot's file `slots/api_token_<instance>`, mode 0600, which keeps the last
one after the box closes, when no runtime takes it any more.

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

The token pool is the host's: conductord's store holds one pool, and every instance on the host
leases from it. A token joined once serves every instance the daemon runs, and adding or removing
one edits no config file and restarts nothing. `join` and `leave` need the `join` grant.

- `conductor join [--instance <name>] [--seat <seat>] [--kind <kind>]` puts the token from
  standard input into the daemon's secrets source, and records owner, seat (default: the operator
  name), kind (`SubscriptionToken` by default, or `ApiKey`) and state. It prints the instances the
  token serves: every instance of the daemon's config file. `--instance` also makes the caller an
  operator of that instance with the grants `daemon.operators` gives. Under the `files` source the
  value is a file of `daemon.secrets.dir`, mode 0600, which `leave` removes.
- `conductor leave [--instance <name>]` withdraws every token the caller contributed to the
  host's pool. Only an owner withdraws. Each lease on a withdrawn token is released, its session
  named in its instance's `<state>/run/tokens-withdrawn.json`, and that instance's loop resumes it
  on another token.
- `GET /v1/tokens` (also at `GET /v1/instances/<name>/tokens`) lists owners, seats, states, kinds
  and live leases, never a value or a secret's name. `GET /v1/tokens/leases` lists which session
  of which instance holds which token.
- `spawn` and `session start` of any instance lease one token per session. A token serves several
  sessions at once, each with a lease of its own: the pick takes the token with headroom with the
  fewest live leases, ties by `token_id`. A token never serves more sessions at once than its
  kind's cap, `daemon.subscription_token_sessions` (4 by default) or `daemon.api_key_sessions` (no
  cap by default). A controller role's `token_owner` takes that owner's tokens only, and starts
  with `--remote-control <session name>`. With no token with headroom the start is refused
  `NoTokenAvailable`, as with an empty pool.
- A session that reaches its usage limit marks the token it holds `Limited` until the reset its
  limit line names. The loop resumes the session from its transcript in the same tick on another
  token with headroom; with none, at the first instant a token of the pool resets. A `Limited`
  token is never leased before its reset, and no usage probe is asked.
- The value reaches the harness once, at its start, in the harness process's own environment:
  `CLAUDE_CODE_OAUTH_TOKEN` for a `SubscriptionToken`, `ANTHROPIC_API_KEY` for an `ApiKey`. The
  session's settings copy names no `apiKeyHelper`. No command prints a pool token, and no log,
  record, argument, settings file or box spec carries it. A token recorded before the kind
  existed is an `ApiKey`.
- A runtime in a box mounts no daemon database. It leases, releases and marks limited through
  conductord, over the `conductord` aperture it already dials (`daemon.dial_port`), on a
  connection opened with its instance's name and the token the daemon minted for its start.
  conductord acts on that instance's leases only, whatever the request names. The lease answer
  carries the token's value once; the runtime keeps it in memory for that one start. Nothing of it
  is in the box's spec or the environment the box starts with.
- Each lease names the runtime start that took it. Once an instance runs on a new start,
  conductord releases the leases of the one that ended.
- A usage-limit stop marks the token Limited until the reset the limit names, else for 5 hours,
  never longer than a week and an hour, and releases that session's lease. With an empty pool a
  start is unchanged: an instance that names a `model_token` runs on it.
- `conductor store migrate` keeps the tokens of an older store: each token of a per-instance pool
  joins the host's pool as it is.

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
must be in the source, or the box is refused (`SecretMissing`). The box gets one slot from the
source, the model token. A local daemon takes it from a pool of `daemon.substrate_slots` slots, each declared as
`--secret-slot model_token_<i>=<daemon dir>/slots/model_token_<i>`. The daemon writes the token
into a free slot's file (mode 0600) and frees it when the box closes. The runtime in the box reads
the slot once at its start and closes its descriptor, so no process it starts inherits it. With
`model_token_kind: subscription_token` (the token `claude setup-token` prints) the runtime starts
each agent whose pool leases it no token with that value as `CLAUDE_CODE_OAUTH_TOKEN` in the
harness's process environment, and a settings copy that names no `apiKeyHelper`; with `api_key`
the launch is left as it is. A new value reaches the agents with the next start of the box. No
forge token is sealed into a box: forge writes take the bot route.
