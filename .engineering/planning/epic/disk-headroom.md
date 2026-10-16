---
format: aep.planning-md/3
id: epic:disk-headroom
kind: epic
status: draft
title: The host keeps 300-400G free on its build disk by default
revision: 1
---
# Epic: disk-headroom

## Outcome

Conductor holds 300–400G free on the build disk by default. Reclaim aims at that target, covers every tree of every instance, and names the real holders when it cannot reach it.

## Why

The operator, 2026-10-15: disk headroom is "a huge blocker". A read-only audit that day found 765G of 848G used. Docker accounted for ~228G and other cold data for about 100G. The managed trees' targets held 49G, all younger than a day.

Conductor's reclaim aimed at `disk_admit` + 5G, which was 20G. It ran only under that floor, kept every tree younger than a day, and assessed only the first repository of the active instance. Its last run freed 4096 bytes.

## Stories (to draft one by one)

- **S1 `disk-target`:** a `thresholds.disk_target` (default 350G) that reclaim aims at. `disk_admit` stays the admission floor. See gc/reclaim.rs:159.
- **S2 `watch-reclaim-under-low`:** the watch reclaims to `disk_target` under `disk_low`, at most once per cycle. See watch.rs:1587-1604.
- **S3 `defaults-300g`:**
  - default thresholds: `disk_low` 300G, `disk_clear` 350G, `disk_admit` 50G and `build_slot` 100G;
  - clamped to a fraction of a small disk;
  - the 30G literal in spec/domains/dispatch.yaml:1343-1344 replaced by config values.
- **S4 `reclaim-tree-targets-when-low`:**
  - under `disk_low`, an idle tree's `target/` goes after 10 min without a writer or a process, not after a day;
  - a finished tree's target goes at any level.
- **S5 `gc-scope-profile`:** gc and the sweep cover every repository and every instance in the config, not only the first checkout. See dashboard.rs:257-300.
- **S6 `reclaim-leftover-whole-disk`:** the shortfall names Docker's reclaimable data and the large directories of home and /var/lib. See gc/reclaim.rs:747-767.
- **S7 `reclaim-disk-path`:** reclaim reads `thresholds.disk_path`, not `/`. See gc/reclaim.rs:264,272.
- **S8 `reclaim-shared-targets-idle`:** a shared cargo target is cleaned when no build grant is live and nothing holds it.

## Out of Scope

Host changes: mounting disks, moving Docker or other data, and one-time deletions outside conductor. Those are the operator's, from the audit's command list kept outside this repository.
