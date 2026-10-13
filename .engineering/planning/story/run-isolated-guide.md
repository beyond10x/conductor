---
format: aep.planning-md/3
id: story:run-isolated-guide
kind: story
status: draft
title: A guide to running conductor isolated today
revision: 1
---
## Why

Two engineers asked how to run conductor away from the rest of their machine and got no answer: a
backend/SRE engineer asked whether it should run in a VM or a container to keep it from other data
and ssh keys on the laptop; an IVR engineer asked how people run it given runners with no
permissions (brain's case list, cases 8c and 9). The sandbox (`epic:session-confinement`) is not
built yet.

## What

A guide, `website/docs/guides/run-isolated.md`, that says what to do today:
- what conductor and its sessions can reach on the host now (the guard's limits, what it does not
  stop), linked to the security pages;
- running it in a VM or a dev container: what to mount (the checkouts root, the records, the state,
  the Claude Code config), which credentials it needs (the forge CLI logins, the Claude login) and
  which it does not (no personal ssh key: a deploy key or HTTPS token per forge);
- a CI runner or account without permissions: which commands still work (doctor, setup, snapshot
  read-only), what fails and with what message;
- what the sandbox will change, linked to its stories.

## Acceptance

- The guide exists, is in the sidebar under guides, passes the documentation check, and every
  command it shows is one the CLI has (`conductor-docs` checks commands).
- It names, for each of the two questions above, the answer in one paragraph.
- README's security section links it.

## Scope

`website/docs/guides/run-isolated.md`, `website/sidebars` (the guides list), `README.md`.
