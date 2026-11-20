# Wave 62 (2026-11-20: a quoted decision id is no citation, a rejected message logged once)

The seventh wave of the goal that fixes the friction the agent box filed. Both units and the
integration branch fork from `main` after wave 61 landed. The wave before is
[wave 61](wave-61.md).

Width: two units, every ready one (`controllers.max_subagents` is 2). The two touch disjoint
files: the records check, and the run loop's mailbox and its log line. Rust builds run one at a
time (one lock, one build per box). U3 started when U1 merged: the guard denied the bot
route's `run rerun` of this repository, reading the slug `…/conductor` as a `conductor run`.
It went to active without a critic panel, as a one-function defect.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | records-check-quoted-message | #232 | merged into the wave branch |
| U2 | mailbox-rejection-once-dated | #229 | merged into the wave branch |
| U3 | guard-repo-slug-not-program | none | merged into the wave branch |

## Outcome

U1: `records check` reads no decision citation out of a field that quotes another's text:
`line`, `first_line` and `evidence`. A `DEC-` id a controller's message quotes no longer refuses
the daily records commit. The fields conductor writes itself (`reason`, `brief`, `grant`, `id`)
are still checked.

U2: of what #229 asked, 1.17.0 already logged a rejection once (the handled names survive, #219),
the sender of a known repository was already answered once, and every `runtime.log` line was
already dated. What was left: a refused message stayed `Received` in the store; it now moves to
`Rejected` with the reason the log line gives. A `[NEED]` is left to `route-need`.

U3: the guard reads a word naming a repository (the value of `-R`, `--repo` or `--repository`, or
a remote's URL) as no `conductor` program, so the bot route's `run rerun` and `workflow run` of
this repository pass; `conductor run` by name or by path, also through `flock`, `watch` or
`bash -c`, is still denied. A relative path `<dir>/conductor` stays a program word: it is one.

In the box: the touched suites, the specification, a fresh generation, `cargo fmt --check`,
workspace clippy and the docs check; the suites that launch a session are proven by CI.
