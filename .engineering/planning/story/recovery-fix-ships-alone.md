---
format: aep.planning-md/3
id: story:recovery-fix-ships-alone
kind: story
status: draft
title: A merged fix to the loop's recovery path is released alone at its merge
summary: A resume fix took 10 h 34 min from commit to install while 246 resumes failed
revision: 2
---
## Why

Host teacher's day retro, 2026-11-08, relayed by the supervisor: a fix to resume after a usage
limit was committed at 02:01, merged at 08:15 together with two other fixes and a profile change,
tagged at 12:24 and installed at 12:35, 10 h 34 min after its commit. Meanwhile the three loops'
journals logged 246 resume lines exiting 3 (01:54 to 12:34), and 11 usage-limit records waited for
the next reset instead of resuming.

## Acceptance

1. `.agents/conductor-dev.md` (release step): a merged fix to the loop's recovery path (resume
   after a usage limit, session restart, hand-over delivery, mailbox) is released alone as a patch
   at its merge; the wave's release follows. Pinned in `tests/profiles.rs`.
2. `.agents/conductor-supervisor.md` (K5 check): a recovery fix has its own clock of 4 h from merge
   to install. Pinned in `tests/profiles.rs`; the profile stays within its cap.

## Ambiguities

None.
