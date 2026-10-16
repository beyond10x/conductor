---
format: aep.planning-md/3
id: story:host-instances-list
kind: story
status: draft
title: conductor host instances lists every instance registered on the host
relations:
- decomposes: epic:host-bridge
- depends_on: story:instance-names-unique
revision: 2
---
# Story: host-instances-list

## Outcome

`conductor host instances` lists every instance registered on the host, across config files, with where each one lives, so every view across instances reads one list.

## Context

docs/design/roles-and-surfaces.md § 4 (the bridge reads across instances) and § 5b ("The bridge keeps one registry per host of every instance across config files"); § 7 step 2. story:instance-names-unique writes the registry (`<host>/instances.json`, by `config validate` and session start) and owns every collision check on it. This story only reads it: it adds no write and no check.

## Domain relations

- Registry → Instance, one-to-many, the far side may be zero; the registry owns only its entries, never the instance; an entry exists only after its instance's file has run on the host — inferable (decided by story:instance-names-unique acceptance 2; no ess/1 document declares the registry yet: this story's spec step declares the read side and leaves any write-side relation as an `UNMAPPED:` marker for story:instance-names-unique).
- Config file → Instance, one-to-many, the file owns its instances — inferable (spec/domains/config.yaml, `conductor.config.Config.instances`, a field rather than a `relations:` entry).

## Acceptance

1. Spec first: `spec/domains/host.yaml` gains `conductor.host.ListInstances` and the view `conductor.host.RegisteredInstance` (name, config file, session prefix, dashboard port, records, state, `present` or `stale`); `crates/conductor/cli.yaml` gains `host instances` with `--format text|json`; `ess specify validate --path spec` exits 0 and the model crate is regenerated.
2. With two config files registered (instances `alpha` and `beta`), `conductor host instances --format json` lists both with their files and paths; `crates/conductor/tests/host_instances.rs` holds it.
3. An entry whose config file is gone, or no longer names the instance, is listed `stale`, not dropped and not deleted; the same test holds it.
4. With no registry, the command prints no instances and exits 0; the same test holds it.
5. `crates/conductor/tests/cli_tree.rs` lists `host instances`; `task check` exits 0.

## Scope

`spec/domains/host.yaml`, `crates/conductor/cli.yaml`, `crates/conductor-model`, `crates/conductor/src/host.rs`, `crates/conductor/tests/host_instances.rs`, `crates/conductor/tests/cli_tree.rs`, `docs/config.md` (the registry's read side).

## Out of Scope

Writing the registry and refusing collisions (story:instance-names-unique). Removing stale entries.

## Ambiguities

None.
