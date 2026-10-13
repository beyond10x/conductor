---
format: aep.planning-md/3
id: epic:goal-documents
kind: epic
status: draft
title: Goal documents from the instance config steer planning and implementation
revision: 1
---
## Why

Approved by the operator on 2026-10-13 (`docs/design/goal-documents.md`, revision 2): conductor and its controllers plan and
implement without the documents that say where the organisation and each repository are going, and
the organisation's goals should come from one shared repository, never a stale local copy.

## Outcome

The instance config names organisation documents (from a shared direction source, at a branch or a
pinned tag) and each repository's goal documents; setup discovers them; the whole config is
validated; every dispatch, controller start and plan carries a Direction block of author-chosen
excerpts; goals cite documents by anchor id; the board shows what is missing, stale or broken.
