---
format: aep.planning-md/3
id: review-result:w51-u6-adversary-2
kind: review-result
status: active
title: Wave 51 U6 release-container-image, adversary pass 2
relations:
- reviews: story:release-container-image
revision: 1
---
```
unit: story:release-container-image, branch impl/release-container-image at 68f364915 plus uncommitted test cases in <trees>/w51-u6
verdict: CONFIRMED
cases: executed 24→28, red 2
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 17 paths under <scratch>/w51/u6/adv2 (part 6)
needs-coordinator: none
```

The correction holds where it matters: the push step, the newest-release rule, the image's owners and modes. I found no blocker. Two red cases show the notes script still loses data on two kinds of hand-written body. Nobody is shown to write either. Two green cases cover command failures that the static `$(` rule does not see.

**1. Diff stat of the worktree**
```
 crates/conductor/tests/release_image.rs | 137 ++++++++++++++++++++++++++++++++
 1 file changed, 137 insertions(+)
```
Only a test file changed, with 0 deletions. I edited no implementation file. `rustfmt` changed only my own lines, and the gates scan of the file says `1 file(s) carry no private rule`.

**2. Cases added** (`crates/conductor/tests/release_image.rs`, from line 1563)

Each case runs a step script against stand-in programs.

| case | line | asserts | now |
|---|---|---|---|
| `adversary2_the_notes_step_leaves_the_body_when_it_cannot_be_read` | 1611 | when `gh release view` fails, the step fails and the body is unchanged | green |
| `adversary2_the_pull_step_fails_when_the_pushed_images_cannot_be_listed` | 1646 | when `docker image ls` fails, the step fails before any `pull` | green |
| `adversary2_a_rerun_after_an_unclosed_fence_leaves_one_section` | 1657 | after a write and a re-run, the body holds one section | **red** |
| `adversary2_a_comment_that_quotes_the_heading_keeps_the_text_after_it` | 1687 | an HTML comment quoting the heading, and the text after it, stay whole | **red** |

I also ran two temporary mutant cases, removed before step 3. Both were red, so the two green cases catch the mutants.
- **Notes step:** I changed `> body.md` to `> body.md || true`. The body was overwritten with the section alone: `body now "## Container image\n\n…"`, gh calls `release view …`, then `release edit … --notes-file notes.md`.
- **Pull step:** I replaced the `docker image ls | sort -u | xargs …` pipe with `xargs docker image rm --force < <(docker image ls --quiet "$IMAGE")`. The first pass's `holds_the_public_pull` still passed on this mutant. My case was red with `pull step went on to pull (true): ["image ls …", "image rm --force", "pull …@sha256:ccc…", "run …"]`.

Red output, verbatim, from the first run of my cases alone (`run1-adversary2-cases.log`; `6 tests run: 2 passed, 4 failed, 24 skipped`):
```
panicked at crates/conductor/tests/release_image.rs:1674:5:
assertion `left == right` failed: sections after the write and after the re-run; ...
  left: (1, 2)
 right: (1, 1)
```
```
panicked at crates/conductor/tests/release_image.rs:1692:5:
  left: "- a story\n\n<!--\n\n## Container image\n\n`ghcr.io/example-org/conductor:1.4.0` at digest `sha256:aaaa…`\n\n```console\n…\n```\n"
 right: "- a story\n\n<!--\n## Container image\n-->\n\nvisible\n\n## Container image\n\n…"
```

**3. Suite run, after my cases existed**

`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test release_image --no-fail-fast`
```
FAIL conductor-cli::release_image adversary2_a_comment_that_quotes_the_heading_keeps_the_text_after_it
FAIL conductor-cli::release_image adversary2_a_rerun_after_an_unclosed_fence_leaves_one_section
Summary [   0.313s] 28 tests run: 26 passed, 2 failed, 0 skipped
exit 100
```
The 24 earlier cases all pass. I did not run clippy on the added cases.

**4. Findings** (they cover 68f364915 plus my test file)

- **F1: the `$(` rule cannot see three fail-open forms** (`release_image.rs:1413`, the rule in `every_script_of_the_image_jobs_stops_at_a_failed_command`)
  - What was measured: the rule looks only for `$(`. It passes `cmd || true`, backticks and `< <(cmd)`. Before this pass, no behaviour case made the notes read (`release.yml:401`) or the pull step's image list (`:365`) fail.
  - So both mutants above passed all 24 earlier cases. The notes mutant writes the section over the whole hand-written Release body.
  - What reaches it: a later edit of either script. Today's scripts are correct.
  - My two green cases now hold both places.
  - CONFIRMED, introduced.
- **F2: an unclosed fence breaks the one-section rule** (`release.yml:414`, the fence tracking)
  - What was measured: a body with a fence that is never closed. The first section is written inside the open fence, so it renders as code. A re-run adds a second section, and the stale digest stays for ever. AGENTS.md steps 5 and 6 promise one section.
  - What reaches it: nothing found. Step 5 writes one line per story.
  - Possible fix: when the input ends inside an open fence, close it before appending the section.
  - INFEASIBLE, introduced by the correction. Pass 1's script did not track fences.
- **F3: an HTML comment quoting the heading loses text** (`release.yml:426`)
  - What was measured: `## Container image` inside `<!-- … -->` is taken as the section. The closing `-->` and the text after it are dropped. The new section then sits inside a comment that never closes, so the Release page does not show it.
  - What reaches it: nothing found.
  - INFEASIBLE, introduced.
- **F4: comments misstate how `--no-same-permissions` sets the mode** (`Dockerfile:12-13`, and the doc comment at `release_image.rs:1433-1435`)
  - Both say a file "takes root's owner and the umask, not the archive's". In fact the mode is the archive's mode with the umask bits removed.
  - Measured with GNU tar under umask 022: a `0700` member came out `700`.
  - Today's archives are 755, so no program is affected. A future 0700 member would pass the build, which runs as root. The workflow's smoke run, which runs every program as user 10001, would catch it.
  - CONFIRMED, introduced.

**5. Attacked and could not break**
- **The newest-release rule:**
  - `image` needs `release`, so this run's Release exists before `releases/latest` is read.
  - A first release cannot get a 404: the `release` job creates the Release when none exists.
  - The comparison is exact equality, so text ordering (1.10 before 1.9) and suffixed or leading-zero tags do not matter.
  - The repository's releases are all `v<x.y.z>`, with no draft and no pre-release. The newest is `v1.2.0`.
  - Every tag is cut on `main`, so a patch of an older line does not happen. A backfill or re-run of an older tag leaves `latest`.
- **The order read, login, push:** checked with the behaviour cases. `local`/`export x=$(…)` and two `$(` on one line are refused by the static rule.
- **The image** (local build of the worktree `Dockerfile`, then removed):
  - `/usr/local/bin` and all seven programs there are `0:0`, mode 755.
  - Nothing under `/usr`, `/etc` or `/opt` is owned by another uid.
  - The only group-writable paths are the base image's man directories, group 0.
  - All ten programs start as 10001, and `conductor doctor` ends `ready`.
- **`ADD` and `COPY`:** `ADD` is forbidden by an existing case, and the one `COPY` is pinned, so a tar-only check is enough.
- **The notes awk, gawk against gawk `--posix` against busybox awk:** identical output on 13 bodies. mawk is not on this host.
- **The notes awk, bodies handled correctly:**
  - CRLF with a fence;
  - an info string with a longer closing fence;
  - a tilde fence that a backtick fence does not close;
  - a fence indented inside a list;
  - a heading with trailing spaces;
  - a thematic break inside the section;
  - two old sections, both replaced.

  Results that only a human editing the job's own heading reaches: `## Container image ##` and a setext `Container image` are taken for other headings, so a second section is added.
- **The removed substring checks:**
  - `releases/latest` is held by `adversary_the_push_step_moves_latest_for_the_newest_release_only`;
  - `id -u` by `adversary_the_smoke_run_refuses_root_and_another_version`;
  - `docker pull` by `adversary_the_pull_step_fails_while_the_package_is_not_public`.
- **The page and the workflow comments:** they match the scripts.

**6. Paths written outside the worktree** (all under `<scratch>/w51/u6/adv2/`)
- Probe programs: `notes.awk`, `probe.sh`, `busybox.sh`, `inspect.sh`
- Probe inputs and outputs: `bodies/`, `out.notes`, `first.notes`, `second.notes`, `tarmode/`
- Logs: `probe-gawk.log`, `probe-posix.log`, `gawk-raw.log`, `busybox-raw.log`, `docker-build.log`, `inspect.log`, `run1-adversary2-cases.log`, `run2-suite.log`

The build context `context/` and the image `conductor:w51-u6-adv2` are already removed.

```findings
- file: crates/conductor/tests/release_image.rs
  line: 1413
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the static rule sees only $( so `|| true` on the notes read and a `< <(…)` image list pass it and all 24 earlier cases, the first overwriting the Release body with the section alone'
- file: .github/workflows/release.yml
  line: 414
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'after a body with an unclosed fence the section is written inside the fence and a re-run adds a second section, the stale one kept for ever'
- file: .github/workflows/release.yml
  line: 426
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'a heading quoted in an HTML comment is taken for the section, so the closing --> and the text after it are dropped and the new section is hidden in an open comment'
- file: Dockerfile
  line: 12
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the comment says unpacked programs take the umask and not the archive''s mode, but --no-same-permissions keeps the archive mode minus the umask, so a 0700 member stays 0700'
```
