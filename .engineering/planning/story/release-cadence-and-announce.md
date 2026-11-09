---
format: aep.planning-md/3
id: story:release-cadence-and-announce
kind: story
status: active
title: Releases are cut on a cadence and announced through a configured route
summary: the operator asked for a default rule to cut and announce releases, with names only in config
scope:
- confidence: inferred
  path: crates/conductor-model/src/config.rs
- confidence: inferred
  path: crates/conductor/cli.yaml
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: inferred
  path: crates/conductor/src/cli.rs
- confidence: inferred
  path: crates/conductor/src/collect/gitlab.rs
- confidence: inferred
  path: crates/conductor/src/config.rs
- confidence: inferred
  path: crates/conductor/src/records.rs
- confidence: inferred
  path: crates/conductor/src/shipped.rs
- confidence: inferred
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: docs/config.md
- confidence: inferred
  path: spec/domains/config.yaml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T16:52:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T16:52:27Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07, relayed by the host supervisor: "make sure the supervisor is basically kind of knowing how to announce releases and updates ... whether it's a new version of Conductor itself or a new version of anything ... there should be a default rule for when releases are being created and cut and therefore also announced." And: "These names ... shouldn't go anywhere into conductor because this is company sensitive ... if at all, then it's part of the configuration itself." Today announcing is an instance rule carried out by hand through another session.

## Acceptance

- Each instance can set an `announce` key in the config: channel kind, target and route. No channel or workspace name appears in code, tests, the specification or the docs; the docs use `example-org` placeholders (AGENTS.md rule 4). `config validate` names a bad field by its YAML path.
- `conductor release announce <repository> <tag>` renders the body from the forge: repository, version, the titles of the pull or merge requests merged since the previous tag, and the install line. It sends the body through the configured route and writes `<records>/docs/announcements/<repository>-<tag>.md`. A second run for the same tag sends nothing and says it was already announced. A test with a fake forge and a fake route reads the body and the record.
- Each watch tick names a repository whose `main` has had a merge with no release for longer than the K5 target, and a tag with no announcement record. Each case is one journal line; a test covers both.
- With no `announce` key set, `release announce` refuses with one line naming the key.
