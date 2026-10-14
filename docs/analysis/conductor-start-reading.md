# What a conductor start reads before its first job is armed

Story `conductor-start-reading`. Question: after the profile names `conductor rules show
--section conductor` and design §§ 4–8 for the session-start reading, how much context does a
restarted conductor hold when it arms its first job?

**Target: under 40k tokens of context before the first `CronCreate`.** Before this story a start
read the whole `rules.md`, design §§ 2–12, the north star, the status and up to three earlier
hand-overs: about 1,500 lines and 91k–113k tokens of context before anything was armed.

## What the start reads now

In the order of `.agents/conductor.md`:

1. the conductor profile, appended to the system prompt;
2. `conductor rules show --section conductor`: the items under each `rules.md` heading named
   `Conductor`, verbatim, and no other section;
3. §§ 4–8 of `docs/design/conductor.md`;
4. `NORTHSTAR.md` and the newest `STATUS.md`;
5. the last 50 lines of `decisions/` and `dispatches/`;
6. the hand-over committed last, which the start prompt of `task conductor:restart` names.

## Method

1. Fast-forward the product checkout and run `task install`, so the profile the role starts from
   is this one.
2. Restart conductor once with `task conductor:restart`, as part of the next wave's batch.
3. In the new session's transcript (`~/.claude/projects/<records directory, escaped>/<session
   id>.jsonl`), find the first assistant message that calls `CronCreate`.
4. Read the context of the assistant message just before that call: its `usage`,
   `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`. This is the context the
   start read before the first job was armed. It is the same figure the watch reads for
   `context: conductor`.
5. Record below: the date, the conductor version (`conductor --version`), the line count of
   `conductor rules show --section conductor`, the figure from step 4, and whether it is under
   40k.

## Measurement

Recorded after the next restart of conductor on the instance, as the method above describes. No
restart has run on this profile yet.
