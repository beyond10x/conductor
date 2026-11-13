---
format: aep.planning-md/3
id: story:instance-removal-list-state
kind: story
status: draft
title: 'The instance list after a removal: gone, or in a declared state'
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Observed once on a Kubernetes cluster at 1.7.0, reported by a consumer: a client sent
`DELETE /v1/instances/<name>`, got 200, and sent `GET /v1/instances` directly after it. The list
answered 200 and named the instance once. The body was not kept, so the state it showed for the
instance is not known, and no later read of the list was made.

Not read in the code yet: whether the `DELETE` answers before the config write is visible to the
list, or the list shows an instance until its pod is gone.

A caller that removes an instance and then reads the list cannot tell a removal in progress from a
removal that did not happen.

## What

One of the two, decided in the story and written in the guide:

- the `DELETE` answers only after the instance has left the list; or
- the list entry of an instance being removed carries a state that says so, until its pod and
  Secret are gone.

## Acceptance

- A case sends the `DELETE` and reads the list in the same connection order as the report. It
  reads either no entry of that name, or an entry whose state is the declared one for a removal.
- `spec/` declares the state if the second form is taken.
- The guide's page on removing an instance says which of the two holds.
- The case fails before the change, or the story records that the report cannot be reproduced and
  what the list showed instead.
