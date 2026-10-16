---
format: aep.planning-md/3
id: story:status-all-instances
kind: story
status: draft
title: conductor status shows every instance on the host
relations:
- decomposes: epic:host-bridge
- depends_on: story:host-instances-list
- depends_on: story:views-open-as-reader
revision: 2
---
# Story: status-all-instances

## Outcome

`conductor status` shows every registered instance, one section each, with its health, open decisions, waves, KPIs and spend.

## Context

docs/design/roles-and-surfaces.md § 4 ("`conductor status` and the dashboard's home page show every instance: health, open decisions, waves, KPIs and spend"). Today `conductor status` reads one instance's store (crates/conductor/src/status.rs; `status` in crates/conductor/cli.yaml). The figures come from what each instance already records: health is the newest snapshot's age and the live sessions; open decisions are the pending and escalated requests; waves are the open dispatches by repository; KPIs are the metrics `metric compute` records; spend is `resource usage` over the last 24 h. The instance list comes from story:host-instances-list. Opening each store as a reader, without the writer lock, is story:views-open-as-reader's; this story relies on it for every instance it reads.

## Domain relations

- Host status → Instance, one-to-many through the registry, read only; the view owns nothing and writes nothing into any instance — inferable (docs/design/roles-and-surfaces.md § 4, "never carries one instance's data into another", operator-approved 2026-10-16; no ess/1 document declares the view yet: this story's spec step declares it).
- Decision request → Instance, many-to-one, the instance owns it in its records — inferable (inferred from crates/conductor/src/dashboard.rs:60, `decisions/*.jsonl` under the instance's records root; no ess/1 relation names the instance).

## Acceptance

1. Spec first: `spec/domains/observation.yaml` gains `conductor.observation.HostStatusReport` with one `InstanceStatus` per registered instance, and `Status` gains `--instance <name>` to narrow it to one; `ess specify validate --path spec` exits 0 and the model crate is regenerated.
2. With instances `alpha` and `beta` registered, `conductor status --format json` has one section per instance, each with the five figures and labelled with its instance; `crates/conductor/tests/status.rs` holds it.
3. A figure an instance has no data for reads `none`; an instance whose store cannot be read is listed `unreadable` with the reason while the others still print; the same test holds both.
4. `conductor status --instance alpha` prints only `alpha`, in today's form; the same test holds it.
5. Reading both instances writes nothing into either: the same test shows no file under either instance's records or state changes.

## Scope

`spec/domains/observation.yaml`, `crates/conductor/cli.yaml`, `crates/conductor-model`, `crates/conductor/src/status.rs`, `crates/conductor/tests/status.rs`.

## Out of Scope

Opening stores as readers (story:views-open-as-reader). New KPIs or a new health measure (epic:controller-health, epic:measured-self-improvement): this story shows what each instance already records. The dashboard (story:host-dashboard-home).

## Ambiguities

None.
