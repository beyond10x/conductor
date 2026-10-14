---
format: aep.planning-md/3
id: epic:dashboard-decisions
kind: epic
status: draft
title: The operator answers decisions on a Decisions page, in batches, and hears of each at once
summary: Top-level Decisions page, batch answers recorded as the operator's, notification on escalation
relations:
- decomposes: initiative:self-improvement
revision: 1
---
## Why

The operator, 2026-10-14 ~20:5xZ, in conductor's session: "would be awesome to see the decissions in web-ui - infact it should be its own main top level navigation section - we need some UI which would allow us to easily make decissions by clicking options and then sending them in batches... its not acceoptable to have important blocking decissions sitting unnoticed for 2 hours". Conductor's history review of the same day: nine class-O items waited a median 4.9 h, at most 11 h. The dashboard on its port did not answer at 20:5xZ; its process had run 8 h 27 min with a 2.5 G memory peak.

## Outcome

Every open class-O and class-H item is on a top-level Decisions page with its options, costs, recommendation, default and deadline; the operator answers several at once by clicking an option for each and sending the batch; each answer is recorded as his and reaches conductor within one watch interval; the page also reads the decision history; and an escalation reaches him the moment it is raised.

## Boundary

An answer from the page is an operator decision, so the page is the first write path of the dashboard. The stories keep it narrow: one endpoint, which only records answers to open escalated requests; a token the page carries per load; the recording session written as `dashboard`; and a guard rule that refuses a conductor or controller tool call that reaches the endpoint. A local process outside the guard can still post: the stories state that residual risk on the security pages.
