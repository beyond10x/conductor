---
format: aep.planning-md/3
id: story:daemon-remote
kind: story
status: implemented
title: conductord runs on a remote box; its TLS listener with Identity bearers is its only API there
summary: the daemon as a system unit on a shared machine with the config repository cloned; conductor login and --daemon <url>; the dashboard, the TUI and the CLI against it from a workstation; story approval and progress on the dashboard
relations:
- decomposes: epic:architecture-next
- depends_on: story:spec-v1
- depends_on: story:federated-views
- depends_on: story:instance-api
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T00:00:50Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-08T00:00:51Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-08T19:46:02Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Operator, 2026-11-08: "conductor D, the daemon itself, is running also on the remote host, say some EC2 instance. Then we can connect via API. So this must of course be secured somehow. So we can look at the UI bit. We could there, you know, approve stories, look at the progress and so on." Design §4, §5.3, §13 row 10.

## Acceptance

- `conductor daemon --install` on a box with no local operator installs the daemon as a system unit with the loopback listener off and the TLS listener on; the config repository is cloned on the box and every `PutConfig` commits there; `daemon.secrets` names the Secrets service.
- `conductor login <daemon url>` obtains an Identity bearer and keeps it in the client's keychain through the `secrets` crates; `--daemon <url>` or `CONDUCTOR_DAEMON` names the daemon a command talks to, default the local socket; a command without a valid bearer is refused by name.
- The dashboard, `conductor tui` and the CLI work from a workstation against the remote daemon: the federated views, attach as a stream, the Decisions page's answers under the bearer's grant; a story approval is a request the controller raised and an operator answered, after which the controller moves the story in its repository's planning store (no planning fact is copied into conductor's records).
- Every write names the human actor the bearer resolves to; a bearer with no grant for a write is refused with the grant's name.
