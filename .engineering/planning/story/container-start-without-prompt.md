---
format: aep.planning-md/3
id: story:container-start-without-prompt
kind: story
status: implemented
title: One instance starts in a container from a config file and mounted credentials, with no prompt
relations:
- decomposes: epic:deployable-in-kubernetes
scope:
- confidence: inferred
  path: .github/workflows/check.yml
- confidence: inferred
  path: Dockerfile
- confidence: inferred
  path: conductor.example.yaml
- confidence: inferred
  path: crates/conductor-model
- confidence: inferred
  path: crates/conductor/http.yaml
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/daemon/mod.rs
- confidence: inferred
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: inferred
  path: crates/conductor/src/daemon/units.rs
- confidence: cited
  path: crates/conductor/src/doctor.rs
- confidence: cited
  path: crates/conductor/src/init.rs
- confidence: cited
  path: crates/conductor/src/secret.rs
- confidence: cited
  path: crates/conductor/src/trust.rs
- confidence: inferred
  path: crates/conductor/tests/daemon.rs
- confidence: inferred
  path: crates/conductor/tests/doctor.rs
- confidence: inferred
  path: crates/conductor/tests/init.rs
- confidence: inferred
  path: crates/conductor/tests/secrets_source.rs
- confidence: inferred
  path: crates/conductor/tests/trust.rs
- confidence: inferred
  path: docs/config.md
- confidence: inferred
  path: spec/domains/config.yaml
- confidence: inferred
  path: spec/domains/secret.yaml
- confidence: inferred
  path: website/docs/guides/run-in-a-container.md
- confidence: inferred
  path: website/docs/security/operators-and-credentials.md
- confidence: inferred
  path: website/sidebars.ts
revision: 30
transitions:
- {from: "draft", to: "proposed", at: "2026-11-11T11:12:34Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-11T11:12:34Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-11T22:33:09Z", actor: "human:timo", revision: 30, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Why

A consumer control plane asked for a config file that needs no interactive step, so one instance can run in a container and be pointed at a group of repositories (2026-11-10). Today the first start goes through `conductor init`, `conductor trust` and logins of the forge CLI and the harness, each written for a workstation.

## Acceptance

- A documented config file plus environment variables or mounted files for the forge token and the model token start one instance in a container with no prompt: `conductor daemon serve` in the foreground creates the records and trusts the checkouts it is configured for.
- `conductor doctor` in that container ends `ready`, and a test runs the start in a container without a terminal.
- The page names each credential, where it is read from and that none is written to the image or the config file.

## Scope

Derived 2026-11-11 by `story-scoper` at 67b0fb096. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` daemon start-up and secrets source (`src/daemon/mod.rs`, `src/secret.rs`) — cited
- **Files:** `crates/conductor/src/daemon/mod.rs:119-137` (start: opens the store, reads the secrets source, seeds nothing), `crates/conductor/src/init.rs:228`, `crates/conductor/src/trust.rs:365`, `crates/conductor/src/secret.rs:233-245`, `crates/conductor/src/doctor.rs:250` and `:1254` — cited
- **Symbols:** `daemon_serve`, `init::check`/`write`, `trust::paths`/`Edit`, `secret::open`/`at_start`, `Source::{Keychain,Remote}`, `doctor::login`, `Doctor::failures` — cited
- **Also likely:** `crates/conductor/src/config.rs:528-529` (the source words), `spec/domains/secret.yaml:28` and `spec/domains/config.yaml:641` (a new source is modelled first), generated `crates/conductor-model`, `crates/conductor/src/daemon/reconcile.rs:1174-1187` (model token into the box) — inferred
- **Tests:** `crates/conductor/tests/daemon.rs`, `doctor.rs`, `secrets_source.rs`, `init.rs`, `trust.rs`; a new container test file, an image definition for it and `.github/workflows/check.yml` if CI runs it (the tree has no container test and no Dockerfile) — inferred
- **Documents:** a new page under `website/docs/guides/` with its `website/sidebars.ts` entry; `website/docs/security/operators-and-credentials.md:146-157`, `docs/config.md`, `conductor.example.yaml`; generated `website/docs/reference/**` if a config key is added — inferred
- **What stops a start with no prompt today — forge login:** conductor reads no forge token itself. Doctor runs `<gh|glab> auth status --hostname <host>` (`doctor.rs:250-254`) and a failure makes it `not ready` (`doctor.rs:1672-1677`). The documented fix is `gh auth login` or `glab auth login`, which "opens a browser or asks for a token" (`SETUP.md:31-33`). `forge_token_env` only names variables for a session request (`session.rs:1451`, `spawn.rs:436`) — cited
- **— model token:** the source is `keychain` (the OS Secret Service, `secret.rs:135`) or `remote`, which never answers (`secret.rs:242`). A value enters only through `conductor secret put` or `conductor join`; both refuse a non-human actor (`secret.rs:818`, `token.rs:1194`) and read standard input, prompting on a terminal (`secret.rs:754-770`). Without the value the box is refused (`reconcile.rs:1174-1187`) — cited
- **— harness:** with no `model_token` a session runs on the harness's own interactive login (`token.rs:29-30`). A background session in `bypassPermissions` needs the acceptance in user or managed settings (`confine/mod.rs:82-90`) — inferred
- **— records and trust:** `init::init` and `trust::trust` are reached only from the CLI (`cli.rs:547`, `:568`) and `setup --apply` (`setup.rs:593`, `:601`), never from the daemon. A spawn refuses `UntrustedCheckout` (`spawn.rs:160`, `:872`). `trust::paths` refuses while the records directory or checkouts root is absent (`trust.rs:307-330`) — cited
- **— doctor:** a checkouts root outside an activated `worktree` workspace is `not ready` until `conductor setup --apply` (`doctor.rs:1254-1266`). The `daemon unit` line asks `systemctl --user` but changes no verdict (`doctor.rs:1623`) — cited
- **— not a prompt, but stops a container:** a runtime starts through `systemd-run --user` (`daemon/units.rs:121`, `systemd.rs:249`) or a substrate box (`reconcile.rs:1158-1160`); a plain container has neither — cited
- **Shared with `story:release-container-image`:** no tracked file; both need the image definition that does not exist yet, and its program list is `doctor::PROGRAMS` (`doctor.rs:67`) — inferred
- **Shared with `story:daemon-probes-and-metrics`:** `crates/conductor/src/daemon/mod.rs`, `crates/conductor/tests/daemon.rs`, `spec/` with generated `crates/conductor-model` and `website/docs/reference/**` (that story has no scope yet) — inferred
- **Shared with `story:box-reaches-forge-and-daemon`:** `crates/conductor/src/secret.rs`, `crates/conductor/src/daemon/reconcile.rs` — inferred
- **Shared with `story:doctor-checks-guard-forge-writes`:** `crates/conductor/src/doctor.rs`, `crates/conductor/tests/doctor.rs`, `crates/conductor/src/config.rs` — cited
- **Shared with `story:handover-restart-by-controller-id`:** `crates/conductor/src/systemd.rs`, only if this story adds a runtime start without systemd; otherwise none — inferred
- **Shared with `story:need-repository-from-tag`:** none — cited
- **Confidence:** medium — every blocker is read at its line, but the story chooses neither the credential source (a new source, or files the forge CLI and harness read themselves) nor how a runtime starts without systemd, and those choices decide the Also likely files
- **Would collide with:** any unit touching `daemon_serve`, `secret::open` or the `Source` enum, doctor's failure list, or `spec/domains/secret.yaml` — cited
- **Safety fact:** seeding at every start overwrites nothing: `init::check` refuses before any write when a skeleton file is there (`init.rs:155-169`, `:243`), and trust writes nothing when its file changed under it (`trust.rs:248`) — step 2, unproven; a new `Source` variant also reaches store records, `GET /v1/secrets` and `daemon/install.rs:177` — inferred

## Scope as merged (wave 51)

The files the unit's merge into `wave/51` (`fb16ccb43`) changed, from `git diff --name-only`
of the merge against its first parent. Where this list and the Scope section above differ,
this list is what the unit touched.

- `.github/workflows/check.yml`
- `AGENTS.md`
- `Taskfile.yml`
- `crates/conductor-docs/src/config.rs`
- `crates/conductor-model/Cargo.toml`
- `crates/conductor-model/PLAN.md`
- `crates/conductor-model/plan.json`
- `crates/conductor-model/src/actor.rs`
- `crates/conductor-model/src/agent.rs`
- `crates/conductor-model/src/behaviour.rs`
- `crates/conductor-model/src/bin/conductor-instance-server.rs`
- `crates/conductor-model/src/bin/conductord-server.rs`
- `crates/conductor-model/src/config.rs`
- `crates/conductor-model/src/decision.rs`
- `crates/conductor-model/src/direction.rs`
- `crates/conductor-model/src/dispatch.rs`
- `crates/conductor-model/src/host.rs`
- `crates/conductor-model/src/improvement.rs`
- `crates/conductor-model/src/instance.rs`
- `crates/conductor-model/src/lib.rs`
- `crates/conductor-model/src/obligation.rs`
- `crates/conductor-model/src/observation.rs`
- `crates/conductor-model/src/operator.rs`
- `crates/conductor-model/src/placement.rs`
- `crates/conductor-model/src/ports.rs`
- `crates/conductor-model/src/ports/conductor_broker.rs`
- `crates/conductor-model/src/ports/conductor_cli.rs`
- `crates/conductor-model/src/ports/conductor_instance.rs`
- `crates/conductor-model/src/ports/conductord.rs`
- `crates/conductor-model/src/primitives.rs`
- `crates/conductor-model/src/secret.rs`
- `crates/conductor-model/src/server.rs`
- `crates/conductor-model/src/server/conductor-instance.docs.md`
- `crates/conductor-model/src/server/conductor-instance.openapi.json`
- `crates/conductor-model/src/server/conductor_instance.rs`
- `crates/conductor-model/src/server/conductord.docs.md`
- `crates/conductor-model/src/server/conductord.openapi.json`
- `crates/conductor-model/src/server/conductord.rs`
- `crates/conductor-model/src/server/entry.rs`
- `crates/conductor-model/src/server/http.rs`
- `crates/conductor-model/src/server/json.rs`
- `crates/conductor-model/src/server/memory.rs`
- `crates/conductor-model/src/server/static_assets.rs`
- `crates/conductor-model/src/server/wire.rs`
- `crates/conductor-model/src/session.rs`
- `crates/conductor-model/src/store.rs`
- `crates/conductor-model/src/system.rs`
- `crates/conductor-model/src/token.rs`
- `crates/conductor-model/src/tool.rs`
- `crates/conductor/cli.yaml`
- `crates/conductor/http.yaml`
- `crates/conductor/src/cli.rs`
- `crates/conductor/src/config.rs`
- `crates/conductor/src/daemon/api.rs`
- `crates/conductor/src/daemon/install.rs`
- `crates/conductor/src/daemon/metrics.rs`
- `crates/conductor/src/daemon/mod.rs`
- `crates/conductor/src/daemon/unattended.rs`
- `crates/conductor/src/daemon/units.rs`
- `crates/conductor/src/dashboard/answers.rs`
- `crates/conductor/src/init.rs`
- `crates/conductor/src/instance.rs`
- `crates/conductor/src/secret.rs`
- `crates/conductor/src/setup.rs`
- `crates/conductor/src/store/secret.rs`
- `crates/conductor/src/token.rs`
- `crates/conductor/src/trust.rs`
- `crates/conductor/tests/adv_w51_container_start.rs`
- `crates/conductor/tests/container_start.rs`
- `crates/conductor/tests/token_pool.rs`
- `spec/domains/config.yaml`
- `spec/domains/secret.yaml`
- `spec/domains/token.yaml`
- `spec/ess-inputs.yaml`
- `spec/scenarios/secret-put-secret-read-only.yaml`
- `website/docs/concepts/config.md`
- `website/docs/concepts/the-host.md`
- `website/docs/guides/run-in-a-container.md`
- `website/docs/reference/cli.md`
- `website/docs/reference/config.md`
