# Wave 59 (2026-11-19: the cycle's places and reasons, rules show, boxed spawn, limits and upgrade skew)

The fourth wave of the goal that fixes the friction the agent box filed. Forked from `main` at
release 1.16.0. The wave before is [wave 58](wave-58.md).

Width: four units, coded in parallel; their Rust builds run one at a time (one lock, one build per
box, the operator's memory rule). Story `resumed-controller-done-held` (#202) shares the dispatch
and report code with U1 and waits for wave 60; the loop's routine duties (#207) are decomposed into
stories in this wave and delivered from wave 60.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | dispatch-waiting-reason, idle-controller-yields-place, dispatch-v1-once-per-goal | #194 #203 #204 | started |
| U2 | rules show reads unnumbered items, disk floor defaults, host role names, daily tick causes | #221 #213 #214 #212 | started |
| U3 | boxed spawn and director | #216 #220 #218 #215 | started |
| U4 | limited entries without a reset, mailbox replay, upgrade against an older daemon | #219 #223 | started |

## Outcome

To be written when the wave merges.
