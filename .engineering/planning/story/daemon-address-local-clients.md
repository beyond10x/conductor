---
format: aep.planning-md/3
id: story:daemon-address-local-clients
kind: story
status: draft
title: The local clients follow daemon.address, and the specification names every readiness condition
tags:
- from-finding
revision: 1
---
## Why

Left open by unit U7 of wave 51 (`story:container-start-without-prompt`), which added the key
`daemon.address` for the daemon's listener (`review-result:w51-u7-adversary-1`, finding 2, and
the implementor's report of the correction).

- With `daemon.address` on a loopback address other than `127.0.0.1`, the daemon answers on
  that address, but the local clients `conductor status`, `conductor instance upgrade` and
  `conductor daemon start-role` still connect to `127.0.0.1` and do not reach it. The guide
  page says so.
- With `daemon.address` off loopback, every `/v1/` path asks for a bearer, and the local
  clients that use the daemon's own token are refused there too. The page says so.
- The comment of `Readiness` in `spec/domains/store.yaml` names the store condition only; the
  conditions wave 51 added (first publication, held records, seeding) are on the host page and
  not in that comment.

## Acceptance

- The local clients read `daemon.address` and reach a daemon bound to another loopback address;
  a test holds each of the three.
- The page says which client reaches a daemon bound off loopback and with which credential, or
  the clients gain one; decided and held by a test.
- The `Readiness` comment in the specification names every condition `/readyz` answers 503 on.
