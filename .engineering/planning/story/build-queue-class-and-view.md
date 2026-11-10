---
format: aep.planning-md/3
id: story:build-queue-class-and-view
kind: story
status: draft
title: A build request has a class, the queue admits by it, and a command shows who holds and who waits
revision: 1
---
## Why

Reported from a trial instance on 2026-11-10, conductor 1.0.1; not reproduced by the filer of this draft.

- `conductor build run --reserve auto -- <a release step>` stayed queued ("position 7 of the disk queue") for more than 28 minutes.
- At that time the host ran 25 `conductor build run` processes: 3 with a child and 22 waiting, from the trees of three instances. The queue is shared by every instance on the host, while `thresholds.build_slots` is set per instance; the docs do not say which of the two holds.
- `build run --help` has no option to order or prioritise a request.
- A request queued for 1 h 47 min stood behind newer requests that had started; the cause is not known.
- A 1-minute typecheck waited 35 minutes behind two cargo builds of 42 and 56 minutes.
- A caller cannot see who holds the slots.

The instance worked around it with its own rules: checks under 2 minutes and 1 G run outside the queue, at most 3 queued requests per lead, and one release step run outside the queue.

## Acceptance

- A request carries a class (release, gate or check) or a priority, and the queue admits by it; a test holds that a release request queued behind check requests is admitted first.
- A command shows the queue: each running and waiting request with its holder (instance and tree), its command, its class and its age.
- Requests of one class are admitted first in, first out by the time they were queued; a test holds it with an old waiter and newer arrivals.
- The docs state whether `thresholds.build_slots` counts per instance or per host, and the code agrees with the statement.
