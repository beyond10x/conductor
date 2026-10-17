---
format: aep.planning-md/3
id: story:host-dashboard-instances
kind: story
status: draft
title: One host view shows every instance's sessions, with a switcher and watch or attach
relations:
- decomposes: epic:host-bridge
- depends_on: story:session-attach-watch
revision: 2
---
# Story: host-dashboard-instances

## Outcome

One host-level view shows the sessions of every conductor instance on the host, with a switcher to open one instance's full dashboard. From any session row the operator can watch it live or attach to it.

## Context

The operator, 2026-10-17: he works with several instances on one host and cannot observe the sessions conductor starts. Each instance today has its own dashboard on its own port (7313 and 7333 on the operator's host), and nothing shows them together. The host level, the maintainer and the bridge, sits above the instances by design (docs/design/roles-and-surfaces.md), so a host view reading every instance is that level's surface. It breaks no instance's isolation, because instances still never read each other.

## Acceptance

1. Spec first: model the host view's read model, `HostSessions` (instance, session name, role, provider, state, attachable), in `spec/domains/host.yaml`, validated with ess.
2. `conductor host dashboard` serves one page listing every configured instance with its sessions, state and the age of each session's last activity. It reads each instance's store as a reader only. Test with two fixture instances.
3. An instance switcher opens that instance's own dashboard view, in-page or by link to its port. Test.
4. Each session row offers watch and attach where story:session-attach-command and story:session-attach-watch provide them, and the page shows the exact command beside each. Test.
5. `conductor tui` gets the same switcher: an instance list and per-instance screens. This depends on story:tui-overview.
6. The view is host-local. It binds 127.0.0.1 like the instance dashboards, and its design note says why reading every instance is allowed at the host level.

## Out of Scope

Typing into a session from the browser. Remote access.

## Ambiguities

None.
