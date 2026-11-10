---
format: aep.planning-md/3
id: review-result:w49-u15-adversary-1
kind: review-result
status: active
title: Wave 49 U15 guard-admits-own-mr-auto-merge, adversary pass 1
relations:
- reviews: story:guard-admits-own-mr-auto-merge
revision: 3
---
unit: U15 story:guard-admits-own-mr-auto-merge, commit 80db32914 on base 3a27c6469 (worktree w49-u15, plus my uncommitted test cases)
verdict: NEEDS-CHANGE
cases: executed 166→177, red 6
origin: introduced 4 / pre-existing 3 / undecided 0
wrote-outside-worktree: 11 paths, all under the assigned scratch (part 6)
needs-coordinator: the acceptance says "by path or by numeric id" and the unit denies the numeric id; three pre-existing holes in the direct route need a story outside this unit (F5 is cross-project)

## 1. Diff stat

```
 crates/conductor/tests/guard_rules.rs | 399 ++++++++++++++++++++++++++++++++++
 1 file changed, 399 insertions(+)
```

Test file only. No implementation file touched.

## 2. Cases added

All in `crates/conductor/tests/guard_rules.rs`, 11 cases named `u15_adv_*` (lines 7379-7760). Each collects every command whose verdict differs and asserts the list is empty.

| case | asserts | now |
|---|---|---|
| `u15_adv_a_header_that_retypes_the_body_is_no_setting_of_merge_on_green` | PUT with a `Content-Type` header is denied | red |
| `u15_adv_a_field_value_the_shell_splits_into_flags_is_denied` | PUT whose field value holds `$(…)`, backquotes or `$VAR` is denied | red (7 of 9 commands) |
| `u15_adv_a_wrapper_carries_no_merge_write_the_guard_did_not_read` | PUT under `sh -c`, `env`, a pipe, a subshell, `xargs`, `parallel` | red (6 of 25: all `xargs`/`parallel` feeding the own call) |
| `u15_adv_a_config_directory_the_line_names_is_denied` | PUT with `GLAB_CONFIG_DIR=` or `XDG_CONFIG_HOME=` is denied | red |
| `u15_adv_origin_probe_the_pipeline_post_under_the_same_tricks` | the same tricks on the pipeline POST | red |
| `u15_adv_origin_probe_glab_mr_merge_under_the_same_tricks` | the same tricks on `glab mr merge -R <own>` | red |
| `u15_adv_the_forms_the_cases_build_on_are_admitted` | the plain call, `-X POST -X PUT` (last wins) | green |
| `u15_adv_a_field_spelling_that_unsets_merge_on_green_is_denied` | 37 field spellings | green |
| `u15_adv_a_method_glab_does_not_read_as_put_is_denied` | 10 method spellings | green |
| `u15_adv_another_project_host_or_endpoint_is_denied` | 53 path, placeholder, host, `-R`, variable and cwd tricks | green |
| `u15_adv_the_pipeline_endpoints_keep_their_verdicts` | 8 admitted and 12 denied pipeline forms, as before the unit | green |

Red output of the cases alone, first execution (`cargo nextest run -p conductor-cli --test guard_rules --no-fail-fast u15_adv`, `11 tests run: 5 passed, 6 failed, 168 skipped`, exit 100). Every line below ends `=> Allow: no cd or git -C into another repository, no git write outside the repository's checkout and managed worktrees, and no GitHub or GitLab write`; `M` stands for `glab api -X PUT projects/example-group%2Fwidget/merge_requests/7/merge -f merge_when_pipeline_succeeds=true`:

```
guard_rules.rs:7418  header
  M -H 'Content-Type: application/x-www-form-urlencoded'
  M --header 'Content-Type: text/plain'
  M -HContent-Type:text/plain
  glab api -iH content-type:text/plain -X PUT projects/:id/merge_requests/7/merge -f merge_when_pipeline_succeeds=true
field value
  M -f sha=$(echo abc -f merge_when_pipeline_succeeds=false)
  M -f sha=`echo abc -f merge_when_pipeline_succeeds=false`
  M -f sha=$(echo abc --hostname gitlab.other.example)
  M -f squash=$(cat flags.txt)
  EXTRA='abc -f merge_when_pipeline_succeeds=false'; M -f sha=$EXTRA
  EXTRA='abc --hostname gitlab.other.example'; M -f sha=${EXTRA}
  M -H $(echo a:b -f merge_when_pipeline_succeeds=false)
guard_rules.rs:7670  wrappers
  echo -f merge_when_pipeline_succeeds=false | xargs M
  echo false | xargs -I true M
  echo example-group%2Fother | xargs -I example-group%2Fwidget M
  echo --hostname gitlab.other.example | xargs M
  xargs -a flags.txt M
  echo -f merge_when_pipeline_succeeds=false | parallel M
guard_rules.rs:7758  config directory
  GLAB_CONFIG_DIR=./cfg M
  export GLAB_CONFIG_DIR=./cfg; M
  XDG_CONFIG_HOME=./cfg M
origin probe, pipeline POST
  glab api -X POST projects/example-group%2Fwidget/pipeline -f ref=$(echo main --hostname gitlab.other.example)
  echo --hostname gitlab.other.example | xargs glab api -X POST projects/example-group%2Fwidget/pipeline
  echo example-group%2Fother | xargs -I example-group%2Fwidget glab api -X POST projects/example-group%2Fwidget/pipeline
  GLAB_CONFIG_DIR=./cfg glab api -X POST projects/example-group%2Fwidget/pipeline
guard_rules.rs:7743  origin probe, glab mr merge
  glab mr merge 7 -R example-group/widget -m $(echo x -R example-group/other)
  echo -R example-group/other | xargs glab mr merge 7 -R example-group/widget
  echo example-group/other | xargs -I example-group/widget glab mr merge 7 -R example-group/widget
  GLAB_CONFIG_DIR=./cfg glab mr merge 7 -R example-group/widget
```

What `glab` does with these was measured with glab 1.115.0 against a local listener (`adv/run/requests.log`):

```
-H 'Content-Type: application/x-www-form-urlencoded'  ->  PUT …/merge  Content-Type: application/x-www-form-urlencoded  body: {"merge_when_pipeline_succeeds":"true"}
-f sha=$(echo abc -f merge_when_pipeline_succeeds=false)  ->  body: {"merge_when_pipeline_succeeds":"false","sha":"abc"}
-f=merge_when_pipeline_succeeds=false (after =true)  ->  body: {"merge_when_pipeline_succeeds":"false"}   (the guard denies this one)
-X=DELETE (after -X PUT)  ->  DELETE …/merge?merge_when_pipeline_succeeds=true   (the guard denies this one)
a config whose host entry names api_host  ->  the request goes to that api_host
```

Not measured: what a GitLab server does with a JSON body sent as `application/x-www-form-urlencoded` or `text/plain`. No forge was called.

## 3. Suite run

One build request ran both, the cases alone first: `conductor build run --reserve 8G -- sh <scratch>/bin/run.sh`, exit 0 (the script records each run's status).

`cargo nextest run -p conductor-cli --test guard_rules --no-fail-fast`:

```
    Starting 177 tests across 1 binary (2 tests skipped)
     Summary [   1.584s] 177 tests run: 171 passed, 6 failed, 2 skipped
        FAIL conductor-cli::guard_rules u15_adv_a_config_directory_the_line_names_is_denied
        FAIL conductor-cli::guard_rules u15_adv_a_header_that_retypes_the_body_is_no_setting_of_merge_on_green
        FAIL conductor-cli::guard_rules u15_adv_origin_probe_glab_mr_merge_under_the_same_tricks
        FAIL conductor-cli::guard_rules u15_adv_a_field_value_the_shell_splits_into_flags_is_denied
        FAIL conductor-cli::guard_rules u15_adv_origin_probe_the_pipeline_post_under_the_same_tricks
        FAIL conductor-cli::guard_rules u15_adv_a_wrapper_carries_no_merge_write_the_guard_did_not_read
exit 100
```

The 6 red are mine; the 166 cases that were there before are green. `executed <before>` is 177 minus my 11, not a separate run.

## 4. Findings

Covers commit 80db32914. Context for severity: `glab mr merge 7 -R example-group/widget` is already admitted under the direct route, so a controller can merge its own request at once without this unit. F1 and F2 therefore break what the unit and `website/docs/security/the-guard.md:143` say the PUT is limited to; they grant no merge the route did not grant. F5 is the one that crosses to another project.

| # | file:line | finding | what reaches it | verdict | origin | severity |
|---|---|---|---|---|---|---|
| F1 | `crates/conductor/src/guard/rules.rs:3392` (and the `H` arm at `:3402`) | A PUT may carry any header. `glab` sends the JSON body under the `Content-Type` the header names, so the request no longer carries `merge_when_pipeline_succeeds` in a form the forge reads as JSON. Server behaviour not measured. Fix: deny `-H`/`--header` on the PUT. | any controller command line | NEEDS-CHANGE | introduced | warning |
| F2 | `crates/conductor/src/guard/rules.rs:3504` | `sets_merge_on_green` reads a field value that holds `$(…)`, a backquote or `$VAR` as one value. The shell splits it into flags; `glab` takes the last `merge_when_pipeline_succeeds`, measured `false`. The same value can add `--hostname <other>`. Fix: deny a field word that holds an expansion. | any controller command line; `$VAR` splits in bash, not in zsh; `$(…)` splits in both | NEEDS-CHANGE | introduced | warning |
| F3 | `crates/conductor/src/guard/rules.rs:3468` | The acceptance says the own project is named "by path or by numeric id". The unit denies the numeric id and its test pins the denial (`tests/guard_rules.rs:6688`). The guard has no source for the own project's number, so this cannot be met without a forge call or a configured id. The reporter's command in the story used the number. | the story's own example call | INFEASIBLE | introduced | warning |
| F4 | `crates/conductor/tests/guard_rules.rs:6669` | Two forms the unit's test pins as admitted do not reach the endpoint: `api/v4/projects/…` is sent as `/api/v4/api/v4/projects/…`, and `--method=put` is sent as the method `put`, lower case (both measured; a server's answer to `put` is not). Harmless, but the admitted list promises spellings that fail. The `api/v4/` prefix was already admitted for the POST. | the denial message and the doc name neither form; nothing found that tells a controller to use them | INFEASIBLE | introduced | note |
| F5 | `crates/conductor/src/guard/rules.rs:1995` | A direct write is admitted with words the guard did not read. `glab mr merge 7 -R example-group/widget -m $(echo x -R example-group/other)` and `echo -R example-group/other \| xargs glab mr merge 7 -R example-group/widget` are allowed; `glab` takes the last `-R`, so another project's merge request is merged. `xargs -I <own path>` replaces the own path in the words. The pipeline POST and the new PUT take `--hostname <other>` the same way. | any controller command line under `forge.route: direct` | NEEDS-CHANGE | pre-existing | blocker for the route, not for this unit |
| F6 | `crates/conductor/src/guard/rules.rs:811` | `GLAB_CONFIG_DIR` and `XDG_CONFIG_HOME` are not in `FORGE_TARGET_VARIABLES`. A config file there may name `api_host` for the own host; measured, the request goes to that host. | a controller can write a file in its checkout; a token valid on another host: nothing found | INFEASIBLE | pre-existing | warning |
| F7 | `crates/conductor/src/guard/rules.rs:1980` | `xargs` and `parallel` before an own `glab api` write are not counted as a command beside it for a project named by path; the fed words reach `glab` after the read ones (`echo -f merge_when_pipeline_succeeds=false \| xargs M`). Same root as F5; listed because the unit's PUT is where the fed word changes the request's meaning. | any controller command line | CONFIRMED | pre-existing | warning |

Origin basis for F5 to F7: not run at the base. The red probes go through `Session::direct_denial`'s `glab mr` path and `FORGE_TARGET_VARIABLES`, and `git diff 3a27c6469...HEAD` has no hunk in either beyond a comment, a message string and the function's new name; `git show 3a27c6469:crates/conductor/src/guard/rules.rs` shows `own_pipeline_endpoint` skipping `-f` values unread.

## 5. Attacked and not broken

- Field spellings: `-f=`, `-F=`, `--raw-field=`, cluster `-if`, quoted and escaped keys, `$'…'`, `True`/`1`/`yes`/`null`, empty value, key case, url-encoded key, `--form`, `--input -`, `--`, `@file`, `squash[x]`, `auto_merge`: all denied.
- Method: `-X PUT -X DELETE`, `-X=DELETE`, `--method=PATCH` after PUT, `-XPUTT`, no method: denied; `-X POST -X PUT` is admitted and `glab` does send PUT.
- Another project by path: `..`, `%2F..%2F`, double encoding, trailing `%2F`, a longer path, a subgroup, a prefix group, an unencoded slash: denied.
- Endpoints near merge: `/merge/`, `merge_ref`, `merge?`, `merge#x`, `approve`, `rebase`, `merge/../approve`, a non-ASCII digit iid: denied.
- Placeholders other than `:id` and `:fullpath`, `:ID`, `:id%2F..`, a full URL, a host-prefixed path: denied.
- Numeric id spelled `%34%32`, `0042`, `+42`: denied.
- `--hostname` twice, `--hostname=` to another host, `-R`/`--repo` on `glab api`, `glab -R … api`: denied.
- `GL_HOST`, `GITLAB_URI`, `GITLAB_REPO`, `export GITLAB_HOST`, `env GITLAB_URI=…`: denied.
- `:id` from another clone as the cwd, after `cd`, after `git -C`: denied.
- Another project under `env`, `sh -c`, `bash -c`, `&&`, `;`, a pipe, a subshell, `eval`, `time`, `nohup`, `command`, `if`, a function, `&`, `$(echo glab)`, `xargs -I{}`: denied.
- The pipeline endpoints: 8 admitted and 12 denied forms keep their verdicts, including `--input` on a POST and a PUT to a pipeline endpoint.
- The unit's tests: no case that cannot fail found; the mutants I tried by reading (drop the `@` check, the allowlist, the duplicate-key check, the query check, the unread-flag check) each have a case that names them.
- The gates route: the unit's own case holds; not attacked further.

## 6. Paths written outside the worktree

All under `~/.cache/scratch/<session>/w49/u15/`:

- `adv/cases.rs` (the cases as appended)
- `adv/bin/srv.py`, `adv/bin/probe.sh`, `adv/bin/run.sh`
- `adv/glabcfg/config.yml` (a fake token for `localhost`)
- `adv/run/requests.log`, `adv/run/out.log`
- `adv/alone.log`, `adv/suite.log`, `adv/build-run.log`
- `adv-report-1.md` (this report)

The build went to the shared target directory through `conductor build run`.

## 7. Findings block

```findings
- file: crates/conductor/src/guard/rules.rs
  line: 3392
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: the PUT that sets merge-on-green admits any header, and a Content-Type header makes glab send the JSON body under a type the forge does not read the field from
- file: crates/conductor/src/guard/rules.rs
  line: 3504
  category: boundary
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a field value holding a command substitution or a variable is read as one value, and the flags the shell splits out of it override merge_when_pipeline_succeeds or the host
- file: crates/conductor/src/guard/rules.rs
  line: 3468
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: the acceptance admits the own project by numeric id and the unit denies it, because the guard has no source for that number
- file: crates/conductor/tests/guard_rules.rs
  line: 6669
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the unit's test pins as admitted the api/v4/ prefix and a lower-case method, which glab sends as /api/v4/api/v4/ and as the method put
- file: crates/conductor/src/guard/rules.rs
  line: 1995
  category: boundary
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: pre-existing
  message: a direct write is admitted with an expansion or an xargs feed that adds a later -R or --hostname, so glab mr merge reaches another project's merge request
- file: crates/conductor/src/guard/rules.rs
  line: 811
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: GLAB_CONFIG_DIR and XDG_CONFIG_HOME are not forge target variables, and a config they name redirects the own host's requests through api_host
- file: crates/conductor/src/guard/rules.rs
  line: 1980
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: xargs or parallel before an own glab api write named by path feeds words glab reads after the ones the guard read
```
