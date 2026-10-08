---
format: aep.planning-md/3
id: epic:multi-instance-host
kind: epic
status: draft
title: Several conductor instances share one host safely
relations:
- designs: specification:multi-instance-host
- informed_by: specification:multi-instance-host
revision: 3
---
## Outcome

Several conductor instances run on one machine without touching each other's sessions or messages,
share its disk and controller slots through one ledger, and the operator sees what each instance
costs the host. Design and findings: `specification:multi-instance-host`.

## Acceptance

With two instances in one config file on one host, each checked by a test:

- no `task` of one instance starts, stops, attaches to or restarts a session of the other
  (story `instance-scoped-tasks`);
- the guard denies a SendMessage from a session of one instance to a session of the other
  (story `cross-instance-message-guard`);
- a host grant the second instance asks for beyond the host's floor or slot count is refused with
  the numbers (story `host-grants`);
- usage per instance and the limits that bound it are epic `session-confinement`.

Confinement (limits, writes, usage, quotas, substrate) moved to epic `session-confinement`
(architecture-decision-record:external-confinement).
