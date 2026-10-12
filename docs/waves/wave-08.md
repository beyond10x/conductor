# Wave 08

Coordinator: conductor-dev, `aep:implementing` 0.22.4, wave mode. Epic: `operating-loop-hardening`.
Approved by the operator's word on 2026-10-12 within the standing approval for waves 03–11, as
waves 06 and 07. The approval covers one commit per unit on `wave/08`, the merge of `main` into it,
the closing store commit and the merge into `main` (pull request 26) once `task check` is green.

One managed tree, `conductor-wave-08`, branch `wave/08`, started beside wave 07 because its files
were disjoint; `main` (wave 07) was merged in before the gate. No local build; CI's `task check`
was the gate.

## Selection

The two stories with no dependency on an open story: `guard-expanded-conductor-words` (found by
wave 06's review) and `watch-two-conductors`. The conductor lease (`single-conductor-lease`,
`lease-guard`) and `operator-decision-guarded` need a specification change each and stay for the
next wave.

## Units

| unit | story | commit |
|---|---|---|
| U1 | guard-expanded-conductor-words | `3d24ac7`; review fix `e45c22c`; limits page `9b9514c` |
| U2 | watch-two-conductors (with the design § 8 row and the conductor profile's wake list) | `c81276e` |

## Review

An `aep:security-reviewer` pass on U1 found one false denial the change introduced (F1: a word
naming `conductor` as a grep pattern or a path was denied as a run), fixed so that only a
`conductor` word that runs as the program meets the expansion rule. Two older gaps are kept as
ignored tests and on the limits page: a program word the shell builds without naming `conductor`,
and a script a shell reads from stdin or a here-document. Nothing denied before the change is
allowed after it.

## Outcome

`task check` green in CI at `06567b2` (https://github.com/beyond10x/conductor/actions/runs/38288311266).
Stories moved to `implemented` on recorded evidence.
