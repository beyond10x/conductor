# Design: short conductor tools for sessions (agent-tools)

Status: draft, design only; the operator, 2026-10-16: "for now only want to create a design".
Epic: `epic:agent-tools`. Nothing here is implemented until the operator approves this design.

## 1. Problem, measured

48 h of the fleet's transcripts, 2026-10-14 to 16: 302 transcripts and 24,647 tool calls.

| figure | value |
|---|---|
| tool-call arguments, share of output tokens | 78 % |
| tool results against arguments | ~3.5× |
| context re-read per turn (mostly cache) | ~153k tokens |
| file-read output that re-reads a file read earlier in the same transcript | 26 % |
| Bash calls that are compound (`cd … &&`, `;`, pipes) | 96 % |
| guard refusals in 48 h | 212, of which 78 were retried in the same form |

Two costs dominate:
- **Round trips.** Every call re-reads the context. A call whose only job is to wait, poll, record or format is expensive however short it is.
- **Large outputs.** Unbounded reads, full test logs and full config dumps.

The long argument strings, such as the commit route boilerplate and the build-run prefix, are visible but rank lower.

## 2. Goals and non-goals

**Goals**
- **One verb per routine operation:** each prints one summary line on success, and on failure at most 40 lines plus a log path.
- **No work the guard can't verify:** every verb is a plain command with argv, never a shell string.
- **Fewer round trips:** waiting, polling and message recording happen without a session turn.

**Non-goals**
- Shortening code payloads (Write and Edit content).
- Replacing the harness's own tools.

## 3. Verbs

All are subcommands of `conductor`, specified in `spec/` like every command (rule 1). Their output follows one contract:
- **line 1:** `ok|fail <verb> <counts>`;
- **then:** failing lines only, at most 40;
- **last:** `log <path>`.

`--format json` gives the same fields.

| verb | replaces | notes |
|---|---|---|
| `conductor tool show <path> [--lines a:b \| --fn <symbol>] [--max 4k]` | `sed -n`, `cat`, and large Read calls; re-reads | Remembers per session what it showed; a re-read of an unchanged range prints `unchanged since <time>`. |
| `conductor test <crate> [--test t] [--filter f] [--reserve 5G] [--wait]` | `build run … cargo test > log; echo EXIT=$?`, then a grep of the log, then a poll | Runs through `build run` (disk, CPU slot, nice). It prints `N passed, M failed`, the failing tests' names and first assertion lines, and the log path. |
| `conductor commit [-C <tree>] -m <msg> -- <paths>` | the commit route boilerplate | The Gates policy and bot route are built in; `git add` of new paths is implied. Prints `<sha> <n> files`. |
| `conductor wait --task <id> \| --log <f> --until <re> \| --pr <repo>#<n> --deadline <dur>` | `until … sleep` loops and `.output` reads | Resolves once, then prints the outcome. Becomes a watcher (process-supervision C7) when that ships. |
| `conductor gate push\|merge <ref>`, `conductor pr checks <repo>#<n> [--wait]` | `b10x-gates check`, push and API calls, `gh pr checks` polls | Prints the failing checks only. |
| `conductor note "<text>"` | `echo "- $(date …)" >> <handoff>` | The time comes from the clock, never typed. |
| `conductor fleet agents [--name n]` | `claude agents --json \| jq …` | |
| messages recorded on delivery | `conductor message receive-message --first-line …` re-typing | Done by a hook on delivery; no verb. |

**Trimmed defaults**
- `rules show` prints titles, and the full text with `--item`;
- `config show` prints a summary, and the full config with `--all`;
- diffs print `--stat` by default.

## 4. Sequences: `conductor run '<flow YAML>'`

The YAML is passed inline on the command line, never through a file:

    conductor run '{seq: [{test: conductor-cli, filter: build_run}, {commit: {m: "fix: …", paths: [crates/a.rs]}}]}'

- Keys: `seq:` runs in order and stops at the first failure; `par:` runs in parallel and reports all. They nest.
- **Steps are verbs from section 3 only, never a shell string.** Measured: the worktree guard accepts the whole call as one plain command and cannot see inside the quoted argument. A `run: "<shell>"` step would carry `&&` or a git command aimed at another repository straight past it. `conductor run` applies the guard's checks (working directory, git target) to each step itself.
- **What it saves is round trips, not bytes.** A YAML step is about as long as the separate call. The estimate is up to ~1,800 avoidable round trips per day, counting only calls whose output the next step does not need.
- **Quoting:** multi-line commit messages need `\n`, and single quotes need escaping. So `commit` inside `run` takes `m:` on one line, and a long message uses `conductor commit -F -` on its own.

## 5. Estimated effect (fleet, per day)

| verb | direct tokens | round trips |
|---|---|---|
| tool show | ~1.6M | — |
| trimmed defaults | ~0.5M | — |
| test | ~0.38M | ~530 |
| wait | ~0.09M | ~400 |
| messages on delivery | ~0.08M | ~320 |
| commit, gate, pr checks, note, fleet | ~0.25M | ~200 |
| run | — | up to ~1,800 (upper bound) |

These are estimates from byte counts (3.5 bytes per token), not a tokenizer.

## 6. Open questions for the operator

1. Order: start with `tool show`, `test`, `wait` and trimmed defaults (the largest four), then the rest? (recommended)
2. Should `conductor run` exist at all, or only the single verbs? Recommendation: single verbs first; `run` once they are measured.
3. Should profiles make the verbs mandatory, the way rules items 139 and 143 forbid wait loops, or only recommended?

## 7. Measurement

Each verb ships with a before and after figure taken from transcripts by the same analysis: bytes per cluster, round trips per cluster, refusals. Those figures are recorded as evidence on its story.
