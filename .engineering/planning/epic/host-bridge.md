---
format: aep.planning-md/3
id: epic:host-bridge
kind: epic
status: draft
title: One channel from the operator to every instance on the host
revision: 2
---
# Epic: host-bridge

## Outcome

The operator has one channel to every conductor instance on the host. A bridge session routes each request to the right instance and names the route it took. `@<instance>` or `--to` reaches one director directly. One view shows every instance's health, open decisions, waves, KPIs and spend. Design: docs/design/roles-and-surfaces.md § 4, approved by the operator on 2026-10-16.

## Acceptance

1. A host registry lists every instance across config files (shared with story:instance-names-unique).
2. `conductor ask [--to <instance>] "<text>"` and the dashboard's one input reach the bridge, and answers come back labelled with their instance.
3. The bridge's first line names its route (`→ <instance>/<repository>`). An ambiguous request gets one question with options, never a guess.
4. The dashboard home page and `conductor status` cover every instance. The Decisions page lists every instance's open decisions.
5. The bridge never plans or decides for an instance and never carries one instance's data into another (profile rules plus a guard test).

## Out of Scope

The role renames (epic:role-names) and the tool surface (epic:agent-tools).

## Ambiguities

None. The operator chose a session with `--to` always available.
