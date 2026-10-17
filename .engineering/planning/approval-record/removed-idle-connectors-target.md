---
format: aep.planning-md/3
id: approval-record:removed-idle-connectors-target
kind: approval-record
status: accepted
title: conductor-dev removed an idle connectors tree's target/ in the disk emergency
revision: 2
transitions:
- {from: "draft", to: "accepted", at: "2026-10-17T19:20:17Z", actor: "human:timo", revision: 2}
---
## What

On 2026-10-17 at about 18:45Z, / was at 18G and the trees conn-w20261017a (29G) and ekr-w17a-u1 (38G) had been doubled by the lean-debuginfo rollout. conductor-dev removed `target/` of conn-w20261017a. Its working tree was left alone. The tree had no process, no running grant and no lease, and its last write was at 18:38Z. Its next build recompiles, as item 151 requires anyway. / went to 48G. connectors' controller was told through conductor.
