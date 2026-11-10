---
format: aep.planning-md/3
id: story:repository-page-live-controller
kind: story
status: draft
title: The repository page shows a controller that is live whatever the store recorded
scope:
- confidence: cited
  path: crates/conductor/src/dashboard/repo.rs
revision: 2
---
## Why

Found by the adversary pass on story:dashboard-live-controllers-and-report-text in wave 49, by reading the code; no case written. The repository page keeps `Running` controller records only (crates/conductor/src/dashboard/repo.rs:303). A controller recorded stopped whose session is live is counted on the Sessions page and is not that repository's controller session for the `usage-limit` and `silent` blockers.

## Acceptance

- The repository page takes a repository's controller from the same roster as the Sessions page; a case with a controller recorded stopped and live shows it on both pages and in the repository's blockers.
