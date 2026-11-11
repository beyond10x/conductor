---
format: aep.planning-md/3
id: review-result:w51-u6-adversary-1
kind: review-result
status: active
title: Wave 51 U6 release-container-image, adversary pass 1
relations:
- reviews: story:release-container-image
revision: 1
---
```
unit: story:release-container-image, branch impl/release-container-image at f26d86fdc plus uncommitted test cases in <trees>/w51-u6
verdict: NEEDS-CHANGE
cases: executed 15→20, red 1
origin: introduced 7 / pre-existing 0 / undecided 0
wrote-outside-worktree: 4 paths (part 6)
needs-coordinator: none
```

**1. Diff stat of the worktree**
```
 crates/conductor/tests/release_image.rs | 300 ++++++++++++++++++++++++++++++++
```
Only a test file changed. I edited no implementation file. `rustfmt` ran on this one file only, and the existing lines are unchanged (0 deletions).

**2. Cases added** (`crates/conductor/tests/release_image.rs`, from line 1017). They run the `image` job's step scripts against stand-in `docker`, `gh` and `tar` programs.
- `adversary_the_push_step_fails_when_the_newest_release_cannot_be_read`: **red**. The real defect. Red output, from the first run of my cases alone:
  ```
  panicked at crates/conductor/tests/release_image.rs:1190:5:
  the API answered no newest release, `latest` was not moved (false), and the push step still succeeded: docker calls ["login ghcr.io --username example-bot --password-stdin", "push ghcr.io/example-org/conductor:1.4.0", "logout ghcr.io", "image inspect --format {{range .RepoDigests}}{{println .}}{{end}} ghcr.io/example-org/conductor:1.4.0"]
  ```
- Four behaviour cases, **green** now:
  - `…moves_latest_for_the_newest_release_only`: `latest` is pushed only when the API names the tag; the reference and digest outputs are checked against literal values.
  - `…smoke_run_refuses_root_and_another_version`
  - `…pull_step_fails_while_the_package_is_not_public`
  - `…build_step_says_built_when_the_tag_holds_a_dockerfile`
- Four temporary mutant cases, deleted before step 3. Each runs a behaviour check on the step script with one line broken, and all four were red in the same run:
  - `=` changed to `!=` in the `latest` test: `the newest release moves 'latest'` failed.
  - The root check changed to `= root`: `the smoke run passed an image that runs as root`.
  - `exit 1` removed after the failed pull: `the pull step passed while the image cannot be pulled without a login`.
  - `echo "image=built"` removed: `left: "" right: "image=built\n"`.

  None of the unit's 15 cases would catch these: every substring they check survives each mutation. The run gave `9 tests run: 4 passed, 5 failed`.

**3. Suite run, after my cases existed**
`conductor build run --reserve 8G -- cargo nextest run -p conductor-cli --test release_image --no-fail-fast`
```
     Summary [   0.082s] 20 tests run: 19 passed, 1 failed, 0 skipped
exit 100
```
The one failure is `adversary_the_push_step_fails_when_the_newest_release_cannot_be_read`. I did not run clippy on the added cases.

**4. Findings** (they cover f26d86fdc plus my test file)
- **F1** `.github/workflows/release.yml:331`. `[ "$(gh api …/releases/latest …)" = "$TAG" ]` fails open: under `set -e`, a failing command substitution inside `[` does not stop the script. On an API error the job ends green and `latest` stays on the previous release. Step 6 of AGENTS.md checks only `:<version>`, so nothing notices.
  - Reached by: any error from `gh api` (5xx, rate limit).
  - Fix: assign the answer to a variable first (`newest=$(gh api …)`) so that `set -e` stops the step.
  - NEEDS-CHANGE, introduced.
- **F2 to F5**: tests that cannot fail. They check substrings, so the four mutants above survive them:
  - F2 `release_image.rs:452` (`latest`)
  - F3 `:512` (root check)
  - F4 `:525` (pull exit)
  - F5 `:671` (no case checks `image=built`)

  My behaviour cases are the replacements. CONFIRMED, introduced.
- **F6** `Dockerfile:38,46,54,70`. `tar` running as root keeps each archive's owner, so `/usr/local/bin/aep`, `ess`, `worktree` and `gh` are owned by uid 1001 (seen with `ls -ln` in a local build). `docker run --user 1001 … sh -c 'printf … > /usr/local/bin/aep && aep'` printed `replaced`.
  - Reached by: only a run as uid 1001; the default user 10001 cannot write these files.
  - Fix: `tar --no-same-owner`.
  - INFEASIBLE, introduced.
- **F7** `release.yml:392`. The notes script's section ends only at the next `## ` heading. A probe against a file-backed `gh` showed two losses: a `# ` heading after the section was dropped, and a fenced block holding `## Container image` lost everything after it, leaving the fence open.
  - Reached by: nothing found; step 5 writes the body as one line per story.
  - INFEASIBLE, introduced.

**5. Attacked and could not break**
- **Script injection:** no `${{ }}` appears inside any `run:` script (YAML parsed); the dispatch tag is checked against the `v<x.y.z>` pattern.
- **Permissions:** `image` holds contents:read and packages:write, `image-notes` holds contents:write only, and no pull-request trigger can reach either job.
- **Action pins:** every action is pinned by a 40-character commit sha, the same as the rest of the workflow.
- **Downloads:** all over TLS with `sha256sum --check --ignore-missing`, which fails when no file was checked (coreutils 9). If `curl | bash` for the harness fails, `claude --version` stops the build. No build args and no secrets go into a layer.
- **Runtime:**
  - With no command the image prints `conductor --help`.
  - `--read-only` with `conductor --version` and `conductor doctor` ends `ready`.
  - uids 12345 and 1000660000:0 start `conductor --version`.
  - `doctor` output matches the guide (gh and glab `not needed`, the rest `present`, ends `ready`).
- **Notes script:** a body with no trailing newline, an empty body, CRLF line ends and the section in the middle all came out right; the digest is checked with a regex before it is written.
- **The `latest` rule:** a backfill of an older tag leaves `latest`, and no world-writable files were found.
- **Docs:** the sidebar id, the README and getting-started links, and AGENTS.md steps 5 and 6 (job and output names, the five assets) all match.

**6. Paths written outside the worktree** (all under `<scratch>/w51/u6/adv1/`)
- `docker-build.log`
- `run1-adversary-cases.log`, `run2-suite.log`
- `probe-notes.py` and the directory `notes-probe/`

The image `conductor:w51-u6-adv` and its build context are already removed.

```findings
- file: .github/workflows/release.yml
  line: 331
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'a failing gh api call inside the [ test is ignored under set -e, so the push step ends green and latest silently stays on the previous release'
- file: crates/conductor/tests/release_image.rs
  line: 452
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the latest rule is held by the substring releases/latest only, so an inverted comparison stays green'
- file: crates/conductor/tests/release_image.rs
  line: 512
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the root check is held by the substring id -u only, so comparing the uid to a name stays green'
- file: crates/conductor/tests/release_image.rs
  line: 525
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'the anonymous pull is held by the docker pull substring only, so dropping its exit 1 stays green'
- file: crates/conductor/tests/release_image.rs
  line: 671
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'no case asserts the build step writes image=built, so removing it, which skips the push and the notes, stays green'
- file: Dockerfile
  line: 38
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'tar as root keeps archive owner uid 1001 on aep, ess, worktree and gh, so a run as uid 1001 can replace them; tar --no-same-owner fixes it'
- file: .github/workflows/release.yml
  line: 392
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: 'the notes script ends its section only at a level-two heading, so a level-one heading or a fenced block quoting the heading after it is deleted'
```
