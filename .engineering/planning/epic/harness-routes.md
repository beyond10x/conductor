---
format: aep.planning-md/3
id: epic:harness-routes
kind: epic
status: draft
title: Each instance declares its harnesses and model routes, and work is routed by role
revision: 1
---
# Epic: harness-routes

## Outcome

An instance's config names the harnesses its sessions may run in (Claude Code, Codex, a beyond10x harness) and the model routes they may use (vendor APIs, or a self-hosted gateway in front of a local model). Each role or task class is routed to one of them, with a fallback, and the dashboard shows usage and cost per route.

## Why

The operator, 2026-10-15: save money, and drive the local GPU to full use as a load test.

## Scope (to decompose)

- **Config:** a `harnesses` and `routes` section per instance, and per role or task class a harness, a route and fallbacks. Spec first.
- **Spawning:** `conductor spawn` and dispatch start the role's harness with its route, through a harness adapter. Claude Code is the first adapter and today's behaviour; Codex and the beyond10x harness follow.
- **Fallback:** on 429 or an outage, the next route in the role's list is used, and the switch is recorded as an observation.
- **Usage and cost:** usage and cost per route as metrics. The Metrics section shows them, and the savings against the vendor route.
- **Self-change:** conductor may change the routing of its own instance within a bound the operator sets. The change is recorded as a decision, and the bound itself stays the operator's.

## Out of Scope

Deploying or operating the gateway or the local model.
