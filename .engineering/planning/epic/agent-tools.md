---
format: aep.planning-md/3
id: epic:agent-tools
kind: epic
status: draft
title: Sessions call short conductor tools instead of long shell commands
revision: 1
---
# Epic: agent-tools

## Outcome

The fleet's sessions spend fewer tokens and fewer round trips on tool calls. The repeated long command lines are replaced by short `conductor` verbs that print one summary line, and on failure only the failing lines and a log path.

## Why

The operator, 2026-10-16: measure the longest tool calls and wrap them. The analysis covered 48 h of 302 transcripts and 24,647 tool calls:
- tool-call arguments were 78 % of output tokens, and results cost about 3.5× the arguments;
- the context re-read is about 153k tokens per turn, so round trips cost more than bytes;
- 26 % of file-read output re-read a file already read;
- there were 212 guard refusals in 48 h, 78 of them retried;
- the biggest clusters are file reads by sed or cat, grep, SendMessage bodies, the commit boilerplate, build-run test plus log grep plus poll, and receive-message re-typing.

An inline sequence helper (`conductor run '{seq: [...]}'`) saves round trips, not argument bytes. The guard cannot see inside its argument, so it may carry only typed verbs run as argv, never a raw shell step, and it must apply the guard's checks per step.

## Stories, ranked by estimated tokens saved per day

1. `conductor tool show <path> [--fn sym|--lines a:b] [--max 4k]`: bounded reads, which say "unchanged since turn N" on a re-read.
2. Trimmed defaults:
   - `rules show` prints an index unless given `--item`;
   - `config show` prints a summary;
   - diffs print `--stat`.
3. `conductor test <crate> [--test t] [--filter f] [--reserve] [--wait]`: prints `N passed, M failed` and at most 40 failing lines, and keeps the log.
4. `conductor commit [-C tree] -m|-F - -- <paths>`: the bot route and policy are built in, and it prints `<sha> <n> files`.
5. `conductor wait --log f --until <re> | --task <id> | --pr repo#n`: prints the summary once. It is the product form of rules item 139, and joins epic:process-supervision C7.
6. Messages are recorded on delivery by a hook, with no `receive-message` re-typing.
7. `conductor tool append|replace <path>`: edits the guard can verify, in place of heredocs and python replace-scripts.
8. `conductor gate push|merge <ref>` and `conductor pr checks repo#n [--wait]`.
9. `conductor fleet agents [--name n]`.
10. `conductor handoff note "<text>"`.
11. `conductor run '<flow YAML>'`: a carrier for the verbs above, with `seq:`, `par:` and stop-on-fail. It has no shell step, and the guard's checks apply per step.

Every verb prints exit code and counts on its first line, then at most 40 failing lines, then the log path. A success is one line.

## Out of Scope

Code payloads (Write and Edit content), which no wrapper removes.
