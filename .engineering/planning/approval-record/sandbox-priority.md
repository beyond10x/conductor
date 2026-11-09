---
format: aep.planning-md/3
id: approval-record:sandbox-priority
kind: approval-record
status: archived
title: The operator puts the sandbox first
relations:
- decides: story:session-envelope-seam
- decides: epic:session-confinement
revision: 3
transitions:
- {from: "draft", to: "archived", at: "2026-11-09T10:28:15Z", actor: "human:timo", revision: 3}
---
The operator to conductor-dev, 2026-10-13, after the guard's security review named calls only a
sandbox bounds: "we need to prioritise the sandbox".

Read as: implement `epic:session-confinement` next (seam, systemd limits, bwrap writes, forge
credentials), including the `.agents/conductor.md` change `story:session-envelope-seam` merges only
under an operator approval-record. Recorded by conductor-dev as that approval; the operator may
withdraw it before the wave that carries the change merges.

## Archived 2026-11-09 (draft sweep)

Decided epic:session-confinement, superseded by the substrate placement and the secret slots (docs/design/architecture-next.md section 11).
