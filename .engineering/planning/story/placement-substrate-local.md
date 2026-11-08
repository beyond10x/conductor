---
format: aep.planning-md/3
id: story:placement-substrate-local
kind: story
status: implemented
title: a placement is a substrate box on this host; the runtime and every agent run inside it with sealed secrets
summary: the substrate-daemon on the control host, the adapters systemd, linux and oci, the profile of design §6, named refusals, no provider of conductor's own
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:daemon-reconcile
- depends_on: story:agent-kinds
- depends_on: story:secrets-source
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T23:41:27Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T23:41:27Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-08T20:42:42Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-07: "the actual instance should run in a confined way ... look at substrate from B10X ... Docker, or Firecracker ... Substrate should support all of that"; 2026-11-08: "rely on substrate for local and remote confinement (bwrap, docker, k8s)". Design §6; ADR 0109.

## Acceptance

- `placement: {daemon: local, adapter: systemd | linux | oci}` opens one substrate box per instance through `substrate_host::confinement::open` (or the SDK against `substrate-daemon`'s socket) with the profile of design §6: argv and cwd of `conductor instance serve`, the read-only roots, one workspace holding today's six writable places, cpu, memory, pids and storage limits, default-deny egress with the forge and model hosts, and every credential as a sealed slot; `OpenBox` and `CloseBox` are recorded.
- Every agent is a substrate PTY or exec session inside the box, started by the runtime through the same SDK; the `claude-code` kind runs tmux inside it and `conductor agent attach|deliver` work unchanged.
- A refusal (`exec.sandbox-unavailable`, a box short of the request, an adapter the host policy refuses) is recorded as `PlacementRefused` with substrate's name; the instance does not start; nothing falls back.
- `confine/systemd.rs`, `confine/none.rs`, `confinement.provider` and `confinement.mode` are deleted; `conductor doctor` checks `substrate-daemon`'s version against the pin in `b10x.toml`; the daemon keeps `substrate-daemon` running (`epic:process-supervision` C5).
- The scratch instance on this host runs its loop, controller and one worker inside `linux` boxes for one cycle; the terminal log and the envelope are written inside the box.
