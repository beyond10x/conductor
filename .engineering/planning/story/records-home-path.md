---
format: aep.planning-md/3
id: story:records-home-path
kind: story
status: draft
title: Records lines never carry the home directory in full
revision: 1
---
# Story: records-home-path

## Outcome

No line conductor appends to its records (decisions, dispatches, messages, STATUS.md, rollups) holds the home directory in full, so a pushed records repository passes the Gates personal-paths scan.

## Context

Conductor's NEED of 2026-10-15 ~15:43Z: `escalate-request` and the raise commands keep their question text verbatim. An H item's script path, written in full, reached the pushed records and was flagged by `b10x-gates scan-text` (personal-paths). The dashboard already writes every path in `/data.json` with `~`.

## Acceptance

- Every text field a records line carries is written with the home directory replaced by `~`, using the dashboard's existing helper. This covers question, reason, words, options, line and first line.
- The store keeps the original text.
- Test: an escalation whose question names `<home>/x/y.sh` appends a line holding `~/x/y.sh` and no `<home>`.

## Out of Scope

Rewriting existing records history; that is the operator's call.

## Ambiguities

None.
