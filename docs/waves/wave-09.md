# Wave 09

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Epic: `operating-loop-hardening`.
Approved by the operator's word on 2026-10-12 ("dispatch another 3 /aep:wave") within the standing
approval for waves 03–11. The approval covers one commit per unit on `wave/09`, the closing store
commit and the merge into `main` once its pull request's `task check` job is green.

One managed tree, `conductor-wave-09`, branch `wave/09`. Units run in sequence (they share the
specification, the generated model and `cli.rs`); each is written without a local build and gated
by CI's `task check`; the guard unit gets a security review.

## Selection

The last three stories of the epic, reviewed by the critic panel twice
(`review-result:ops-hardening-*-r1`, `-r2`), in their `depends_on` order.

## Units

| unit | story | stage | commit |
|---|---|---|---|
| U1 | operator-decision-guarded (spec first) | running | — |
| U2 | single-conductor-lease (spec first) | queued | — |
| U3 | lease-guard | queued | — |
