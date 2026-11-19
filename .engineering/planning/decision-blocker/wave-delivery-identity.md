---
format: aep.planning-md/3
id: decision-blocker:wave-delivery-identity
kind: decision-blocker
status: open
title: Nobody has decided how a boxed controller pushes and merges under the bot identity main requires
summary: The box holds no bot credential, and main requires the bot as commit author and committer, which the waves' commits do not carry
revision: 2
---
## Question

How does a controller that runs in a box deliver its wave: push the branch, open the pull request and
merge it, as the bot identity `main` requires of every commit's author and committer?

## Facts

- The box holds no bot credential: `b10x-gates bot` refuses with the protected `b10x-bot.json` not
  found, and `gh` is not authenticated in the box.
- `main` requires the bot as author and committer; the waves' commits carry the identity of the
  git configuration the box reads.

## Blocks

The push and pull request of wave 56 and of every later wave. Work goes on locally meanwhile.
