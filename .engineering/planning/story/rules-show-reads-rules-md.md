---
format: aep.planning-md/3
id: story:rules-show-reads-rules-md
kind: story
status: draft
title: conductor rules show reads a role section of rules.md
summary: rules show --role controller is refused for want of rules/controller.md
revision: 1
---
## Why

Reported 2026-11-07 by a controller: `conductor rules show --role controller` is refused because
the records hold no `rules/controller.md`. The instance's rules are one file, `rules.md`, with a
section per role. Not yet verified whether the command reads a `rules/` directory only, or whether
the instance's records lack a file a migration should have written.

## Acceptance

- On an instance whose rules are one `rules.md` with role sections, `conductor rules show --role
  controller` prints that role's section, or the refusal names the file to create and the command
  that creates it.
