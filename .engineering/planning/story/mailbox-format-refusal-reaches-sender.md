---
format: aep.planning-md/3
id: story:mailbox-format-refusal-reaches-sender
kind: story
status: draft
title: A decision request refused by format reaches its sender
summary: the loop tells the sending controller why its request was refused, instead of only logging it
revision: 1
---
## Why

Observed 2026-11-07 05:13:17Z in the journal of `conductor run`: a controller's decision request
in conductor's mailbox was refused by format ("request <id> lists no lettered option: one line
each, `A: <label> (cost: <cost>)`") because it wrote its options on one line (`Options: A ...;
B ...; C ...`). The refusal was only logged; the controller was not told and kept waiting for an
answer.

## Proposal

Either or both:
- the loop sends the refusal and its reason back to the sending session, through the route it
  delivers answers on;
- the parser also accepts the options written on one line, `A ...; B ...; C ...`.

## Acceptance

- A request refused by format reaches its sender as a message naming the reason, within one
  cycle of the loop.
- (If the parser is widened) a request whose options are on one line is accepted as three options.
