---
format: aep.planning-md/3
id: story:rules-into-code
kind: story
status: draft
title: Rules sessions forget are enforced by the binary; role rule files stay within one line and 20 rules
relations:
- decomposes: epic:loop-without-handwork
revision: 2
---
# Story: rules-into-code

## Outcome

Rules that sessions kept forgetting are enforced by the conductor binary instead. Each is then deleted from the instance's rules. The role rule files stay at one line per rule and at most 20 rules per role, held by a check.

## Context

The operator, 2026-10-17: a rule must be one line, a role has at most 20, and dropping a rule only because it is over the cap is not acceptable. The lean rewrite (per-role files) moves these items into code instead of a rules file. Workers never read rules.md, so only code reaches them.

## Acceptance

1. `[DECISION …]` lines without a decision id are refused by the message parser with the expected format (old item 73). Test.
2. `message send` writes its record line only after the send succeeded, and writes none on failure (74). Test.
3. `conductor rules show --role <role>` reads the role's file. `rules show --role <role> --changed-since <rev>` answers the changed lines, and a running session is pointed to it after a rules commit (80). Tests.
4. `records check` flags a raw number in a charter where a threshold key covers it (81). Test.
5. The guard denies `timeout … conductor snapshot` and names the reason (106). Test.
6. `build run` runs its command under `nice -n 19` and `ionice -c3` itself, unless the command already starts with them (142). Test.
7. `rules check` (and `records check`) fails when a role file has more than 20 rules or a rule spans more than one line, naming the file and line. Test.
8. The profiles say: read your role file with `rules show --role`; no profile asks a session to remember any behavior that 1 to 6 enforce. The profiles test pins this.

## Out of Scope

Rewriting the rules' content; that is the per-role files.

## Ambiguities

None.
