---
format: aep.planning-md/3
id: story:host-dashboard-home
kind: story
status: draft
title: The host dashboard shows every instance and answers every instance's decisions
relations:
- decomposes: epic:host-bridge
- depends_on: story:status-all-instances
revision: 2
---
# Story: host-dashboard-home

## Outcome

A host dashboard's home page shows every registered instance's health, open decisions, waves, KPIs and spend, and its Decisions page lists and answers every instance's open decisions.

## Context

docs/design/roles-and-surfaces.md § 4 ("the dashboard's home page show every instance"; "A decision asked by any director appears in one list, so the click-to-decide page covers all instances"). Each instance keeps its own dashboard on its own port (§ 5b; docs/config.md `dashboard_port`). The host page is served by `conductor dashboard serve --host --port <port>`, a host unit beside the bridge, which refuses a port a registered instance holds. Its figures are those of story:status-all-instances. The Decisions page and its answer endpoint exist per instance (crates/conductor/src/dashboard/decisions.rs, answers.rs); the host page lists them together and records each answer in the store of the instance that asked.

## Domain relations

- Host dashboard → Instance, one-to-many through the registry, read only but for an answer — inferable (docs/design/roles-and-surfaces.md § 4, operator-approved 2026-10-16; no ess/1 document declares it).
- Decision request → Instance, many-to-one, owned by the instance's records — inferable (inferred from crates/conductor/src/dashboard.rs:60, `decisions/*.jsonl` under the records root).
- Answer → Decision request, many-to-one, recorded in the asking instance's store only — inferable (inferred from crates/conductor/src/dashboard.rs:20, `POST /api/answers` per instance; § 4 forbids writing it anywhere else).

## Acceptance

1. `GET /` of `conductor dashboard serve --host` lists `alpha` and `beta` with the five figures and a link to each instance's own dashboard; `crates/conductor/tests/dashboard_host.rs` holds it.
2. `GET /api/decisions` on the host page lists every open class-O and class-H item of both instances, each labelled with its instance; the same test holds it.
3. A batch answering one `alpha` item and one `beta` item records each in its own instance's decisions and nothing in the other; the same test reads both stores.
4. `--host` with a port a registered instance holds is refused, naming the instance; the same test holds it.
5. The Taskfile's `dashboard:host` task starts the unit `conductor-host-dashboard`; `crates/conductor/tests/taskfile.rs` holds the name.

## Scope

`crates/conductor/cli.yaml` (`dashboard serve --host`), `crates/conductor-model`, `crates/conductor/src/dashboard.rs`, `crates/conductor/src/dashboard/overview.rs`, `crates/conductor/src/dashboard/decisions.rs`, `crates/conductor/src/dashboard/answers.rs`, `crates/conductor/tests/dashboard_host.rs`, `crates/conductor/tests/taskfile.rs`, `Taskfile.yml`, `website/docs` (the host page).

## Out of Scope

The ask input (story:dashboard-ask-input). Changes to a single instance's dashboard. Charts (epic:dashboard-charts).

## Ambiguities

None.
