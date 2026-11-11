---
format: aep.planning-md/3
id: story:setup-prints-box-row
kind: story
status: draft
title: conductor setup and the release onboarding step agree with doctor's box row
tags:
- from-finding
revision: 1
---
## Why

Found by the release onboarding runs of v1.2.0 (`docs/analysis/onboarding-run.md`, "v1.2.0"),
read in two scratch homes after `conductor setup --apply` at `placement: systemd` with no
substrate daemon started:

- `conductor setup` ended `not ready` on the forge login alone, while `conductor doctor` in the
  same home also named `box <instance> (placement.writable-root-missing)`. `SETUP.md` step 11
  says setup prints doctor's rows; it prints them without the box row and its verdict does not
  name the box.
- `AGENTS.md`, Releases, step 3 asks that both onboarding runs end `ready` with questions only
  for logins and the store migration. Since doctor dry-runs the box, a run that starts no
  substrate daemon cannot end doctor on the logins alone.

## Acceptance

- `conductor setup` prints the box row doctor prints for a `placement: systemd` instance and
  names the box in its verdict, or `SETUP.md` step 11 says which of doctor's rows setup leaves
  out; a test holds whichever is chosen.
- `AGENTS.md`, Releases, step 3 says what a run that starts no daemon ends on.
