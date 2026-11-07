---
format: aep.planning-md/3
id: story:dashboard-instance-switch
kind: story
status: implemented
title: One dashboard switches between the instances of the host
summary: the dashboard lists the host registry instances and shows the selected one
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T09:34:34Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T09:34:34Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-07T11:11:23Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

The operator, 2026-11-07: "one dashboard can switch between instances".

Today each instance serves its own dashboard on its own port (`dashboard_port` in the host
registry `~/.b10x/conductor/host/instances.json`; three instances on one host use three ports),
so seeing another instance means another URL.

## Proposal

One dashboard lists the instances of the host registry and switches between them: an instance
selector in the page header, each page reading the selected instance's state and records. The
per-instance ports may stay as entry points or redirect to it.

## Acceptance

- On a host with three registered instances, one dashboard URL lists all three, and selecting one
  shows that instance's pages.
- The Decisions page's answers still go to the instance the page shows.
