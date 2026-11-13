---
format: aep.planning-md/3
id: story:check-workflow-names-its-runner-image
kind: story
status: draft
title: The check workflow names its runner image
tags:
- from-finding
revision: 1
---
## Why

The check workflow runs on whatever image `ubuntu-latest` stands for that day, so the image can
change under a pull request with no commit. Read at `v1.7.0` (`951330f4f4`).

- `.github/workflows/check.yml:29` (job `task check`) and `.github/workflows/check.yml:103` (job
  `container start`) say `runs-on: ubuntu-latest`.
- `.github/workflows/release.yml:46`, `:175`, `:252` and `:382` say `runs-on: ubuntu-latest`;
  its build matrix names its images (`release.yml:93`, `:100-106`).
- On 2026-11-13 the `task check` job of run 43333948176 (push, created 00:03 UTC) ran on image
  `ubuntu-24.04`, Ubuntu 24.04.5 LTS; that of run 43364879095 (pull request, created 04:20 UTC)
  ran on image `ubuntu-26.04`. Both read from the jobs' logs.
- On the 26.04 image the collector's case `a_program_running_from_a_target_keeps_it`
  (`crates/conductor/tests/gc.rs:1789`) failed; the log of run 43364879095 prints `FAIL` for it.
  The message of commit `951330f4f4` says why: `/usr/bin/sleep` there is a link to a file that
  holds many programs and reads which one it is from its own name, so the case's copy under
  another name ended at once. That commit changed the test, not the code; that the case failed
  twice is the commit message's, not read in a second log.

## Acceptance

- Every job of the check and release workflows names the image it runs on; no job says
  `ubuntu-latest`.
- A change of image is a commit to the workflow.

## Notes

- `.github/workflows/pages.yml:19` and `.github/workflows/guard-latency.yml:35` say
  `runs-on: ubuntu-latest` too; they are outside this story's acceptance.
