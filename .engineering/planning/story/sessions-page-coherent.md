---
format: aep.planning-md/3
id: story:sessions-page-coherent
kind: story
status: draft
title: The Sessions page shows the instance's sessions and what each works on, consistently
relations:
- decomposes: epic:dashboard-for-engineers
revision: 2
---
# Story: sessions-page-coherent

## Outcome

The dashboard's Sessions page shows exactly the instance's sessions, conductor included, with what each one works on. Every count on it agrees with the page it summarizes.

## Context

The operator, 2026-10-17, on `#/sessions` under 0.18.1: "I find it highly confusing". conductor-dev's reading of the rendered page:
1. The title is "What every session works on", but the dispatch, goal, brief and last-event columns read "—" for every row. Since the instance got `session_prefix`, sessions are named `<prefix>-<repo>`, and the page likely still matches dispatches by the bare repository name.
2. The rows come from "Sessions under ~/beyond10x", a working-directory filter:
   - the conductor session (cwd: the records directory) is missing;
   - `<prefix>-assistant` is missing;
   - a session the operator started by hand, not one of the instance's, is listed.
   - The header says "controllers running: 7" while the table has 8 rows.
3. "Awaiting the operator (3)" lists escalations from 2026-10-10/11 (`ESC-…`). The Decisions page lists three different open class-H requests. Both use the same words and disagree.
4. "Usage limit and context" lists every "usage limit seen today" line, including those whose reset time has passed. It also lists context warnings for sessions that are not the instance's.
5. "Recent decisions" mixes request ids with decision ids, and shows rows of "— — —", one request repeated three times.
6. "Disk waste" (the gc table) and "Red CI on main" sit on this page. Red CI shows one repository, while `conductor status` lists four red mains.

## Acceptance

1. Session rows are the instance's sessions: conductor, the maintainer if it belongs to this instance, and every controller, from the session records, not from a cwd filter. A session that is not the instance's is not listed. Test with a fixture holding a foreign session.
2. Each controller row shows its open dispatch, goal, brief and last event, matched through the session record's repository, with or without `session_prefix`. Test with a prefixed fixture.
3. "Awaiting the operator" and the Decisions page read one source (open class-O and class-H requests), so they show the same count and the same ids. The legacy `ESC-` escalations appear only if they are still open in that source. Test.
4. A usage line shows only while its limit is in force, with its reset time. Context warnings name only the instance's sessions. Tests.
5. "Recent decisions" lists decisions only (decision id, class, choice, reason), newest first, with no empty rows. Test.
6. Disk waste moves to Health and red CI to Overview, and both show what `conductor status` shows. Test.
7. A Vitest snapshot of the page on a prefixed two-controller fixture is pinned.

## Out of Scope

Attach and watch from the page (epic:session-attach).

## Ambiguities

None.
