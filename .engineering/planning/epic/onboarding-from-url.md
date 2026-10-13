---
format: aep.planning-md/3
id: epic:onboarding-from-url
kind: epic
status: draft
title: A session given conductor's URL sets conductor up in its repository
revision: 2
---
## Why

The operator, 2026-10-13: "I run claude in my repo, then I paste the conductor URL and tell: I want
to use conductor in this repo ... then I would expect it makes sure all pre-requisites are covered
(most recent planning store, gh or glab cli, etc...)". Today an agent given the URL reads a long
README and improvises; a team trying v0.4.0 (2026-10-13) hit a missing `gh`, an old `aep`, an
unmigrated planning store and a source build, and every posted `conductor doctor` exited 1.

## Outcome

A Claude Code session in any repository, given only the conductor repository's URL and "use
conductor in this repo", ends with conductor installed, the repository's forge CLI logged in, its
planning store current, an instance configured for it, and `conductor doctor` ending `ready`,
asking the person only for what needs their hands (a login, a migration that rewrites files); and
it stays that way: two recorded runs prove it and are repeated before a release that changes it.

## Stories

`story:setup-guide-for-agents`, `story:doctor-identity`, `story:setup-command`,
`story:onboarding-run`.
