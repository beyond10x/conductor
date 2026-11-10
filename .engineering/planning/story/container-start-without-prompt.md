---
format: aep.planning-md/3
id: story:container-start-without-prompt
kind: story
status: draft
title: One instance starts in a container from a config file and mounted credentials, with no prompt
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

A consumer control plane asked for a config file that needs no interactive step, so one instance can run in a container and be pointed at a group of repositories (2026-11-10). Today the first start goes through `conductor init`, `conductor trust` and logins of the forge CLI and the harness, each written for a workstation.

## Acceptance

- A documented config file plus environment variables or mounted files for the forge token and the model token start one instance in a container with no prompt: `conductor daemon serve` in the foreground creates the records and trusts the checkouts it is configured for.
- `conductor doctor` in that container ends `ready`, and a test runs the start in a container without a terminal.
- The page names each credential, where it is read from and that none is written to the image or the config file.
