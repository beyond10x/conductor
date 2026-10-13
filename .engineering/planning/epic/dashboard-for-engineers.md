---
format: aep.planning-md/3
id: epic:dashboard-for-engineers
kind: epic
status: draft
title: The dashboard is a Vue app engineers use for their repository
revision: 2
---
## Why

The dashboard (`crates/conductor/src/dashboard.rs`) is one HTML string of six full-width tables,
reloaded every 20 s, built for the operator: no hierarchy, raw ids and timestamps, an 85-row disk
table filling the page, a "last event" that shows a days-old dispatch state on busy sessions, and
none of what the snapshot already holds (repositories, merge and pull requests, pipelines,
releases, blockers, specifications, the status board). The operator, 2026-10-13: "make the web ui /
dashboard more appealing … and make it actually useful for our engineers"; "it can just be a vue
SPA, embedded into the rust binary". Engineers who do not run the instance need to see it too.
Design reference: `docs/design/dashboard-mockup.html`.

## Outcome

An engineer opens the dashboard and sees, for the instance and for their repository, what needs
them, what each controller works on and waits on, its merge requests and pipelines, what shipped
today, the plan and the health of their setup — in a Vue single-page app embedded in the binary,
served read-only on 127.0.0.1, with operator-only items out of an engineer's view, and can be
shared read-only as a static export.

## Stories

`story:dashboard-spa` (the app, today's panels), `story:dashboard-overview` (the overview and the
snapshot endpoints it renders), `story:dashboard-repo-view` (one repository's page, its endpoints
and what it waits on), `story:dashboard-health` (doctor, snapshot failures, collector times, disk),
`story:dashboard-engineer-view`, `story:dashboard-export` (later, after the engineer view).
`story:dashboard-snapshot-api` is retired: its endpoints moved into the stories that render them.
