---
format: aep.planning-md/3
id: story:box-reaches-forge-and-daemon
kind: story
status: active
title: A boxed runtime reaches its forges and accepts the daemon's proxied writes
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: SETUP.md
- confidence: inferred
  path: crates/conductor/Cargo.toml
- confidence: cited
  path: crates/conductor/src/daemon/reconcile.rs
- confidence: inferred
  path: crates/conductor/src/instance.rs
- confidence: cited
  path: crates/conductor/src/placement.rs
- confidence: inferred
  path: crates/conductor/src/secret.rs
- confidence: inferred
  path: crates/conductor/tests/placement.rs
- confidence: inferred
  path: website/docs/concepts/the-host.md
- confidence: inferred
  path: website/docs/getting-started.md
- confidence: inferred
  path: website/docs/guides/run-isolated.md
revision: 22
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T00:18:07Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-10T00:18:07Z", actor: "human:timo", revision: 7}
---
## Why

Reported by the operator session of a trial instance on 2026-11-10 from conductor 1.0.1 at `placement: systemd`
(the reporter's defect list items 17 and 18; inferred from the
source, not observed, because the runtime did not get past its start, see
story:box-runtime-starts).

- `BoxRequest.egress` and `cwd` are never sent (placement.rs:527, 651-662, 1297-1367); the
  session asks for the one aperture `conductor` (:1341), one host and port.
  guides/run-isolated.md:21 says the box reaches its forges.
- `CONDUCTOR_API_TOKEN` is stripped from the box (reconcile.rs:880-887), so the runtime mints
  its own and daemon-proxied writes would be refused.

## Acceptance

- The box request names egress to each forge host the instance's repositories use and its cwd;
  the runtime in the box accepts a write the daemon proxies with its token; tests against a fake
  substrate hold both.

## Scope

Derived 2026-11-10 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/conductor` placement and box request (`placement.rs`, `daemon/reconcile.rs`) — cited
- **Files:** `crates/conductor/src/placement.rs:1297`, `crates/conductor/src/placement.rs:651`, `crates/conductor/src/placement.rs:527`, `crates/conductor/src/daemon/reconcile.rs:880` — cited
- **Symbols:** `BoxRequest.egress`, `BoxRequest.cwd`, `EGRESS_APERTURE`, `Sdk::runtime`, `Reconciler::box_request`, `TOKEN_VARIABLE` (`CONDUCTOR_API_TOKEN`) — cited
- **Also likely:** `crates/conductor/tests/placement.rs` (lines 500-509 hold egress, 598 and 1362 assert the token is absent from the box and must flip), `crates/conductor/src/instance.rs` (line 193 reads the token from the environment; changes if the token arrives by a sealed slot instead), `crates/conductor/src/secret.rs` (only if the token becomes a second slot beside `MODEL_TOKEN_SLOT`), `crates/conductor/Cargo.toml` and `Cargo.lock` (only if the substrate pin moves to get a per-session cwd or egress) — inferred
- **Documents:** `website/docs/getting-started.md` (line 190 declares the aperture with the model host only, no forge), `SETUP.md` (line 148, same command), `website/docs/guides/run-isolated.md` (line 21, the claim the story cites), `website/docs/concepts/the-host.md` (line 64, the token sentence) — inferred
- **Confidence:** medium — the two defect sites are cited and verified in the tree, but the pinned substrate SDK takes neither egress hosts nor a cwd per session, so where the egress half lands (code, operator documents, or a pin bump) is a design choice the story does not make
- **Would collide with:** any unit touching `crates/conductor/src/placement.rs` (`request`, `Sdk::runtime`), `Reconciler::box_request` and `service` in `crates/conductor/src/daemon/reconcile.rs`, `crates/conductor/tests/placement.rs`, or the substrate-daemon command in `website/docs/getting-started.md` section 5 and `SETUP.md` — cited
- **Safety fact:** the box's environment is documented "never a credential" (`crates/conductor/src/placement.rs:518`) and two tests hold it (`crates/conductor/tests/placement.rs:598`, `:1362`); handing the token in is safe only if it reaches the runtime without widening who can read it, and the relay's environment (`:1362`) stays without it — step 2, unproven — inferred
