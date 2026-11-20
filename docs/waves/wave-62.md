# Wave 62 (2026-11-20: a quoted decision id is no citation, a rejected message logged once)

The seventh wave of the goal that fixes the friction the agent box filed. Both units and the
integration branch fork from `main` after wave 61 landed. The wave before is
[wave 61](wave-61.md).

Width: two units, every ready one (`controllers.max_subagents` is 2). The two touch disjoint
files: the records check, and the run loop's mailbox and its log line. Rust builds run one at a
time (one lock, one build per box).

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | records-check-quoted-message | #232 | building |
| U2 | mailbox-rejection-once-dated | #229 | building |

## Outcome

Written when the wave merges.
