---
format: aep.planning-md/3
id: epic:cross-repository-waves
kind: epic
status: draft
title: conductor orders takt runs across repositories by their needs and releases
summary: One run per repository, a consumer run started only after the run it depends on is done
relations:
- depends_on: epic:loom-agent-runner
revision: 1
---
# Epic: cross-repository-waves

## Outcome

conductor runs one takt run per repository in two or more repositories and orders them by the
`[NEED]`s between them and by releases: a consumer's run starts only after the run it depends on is
done. takt's waves stay inside one repository; the order across repositories is conductor's.

## Acceptance

Two repositories with one `[NEED]` between them: two runs, the second started only after the
first's dispatch was `done`; the `[NEED]` is routed and recorded; `conductor records check` exits 0.

## Source

`docs/design/conductor-on-agency.md` § 4 step 4, § 6 D1, D4.
