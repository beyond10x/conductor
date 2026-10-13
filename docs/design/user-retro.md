# Retro for conductor's users

Status: design for approval, beside `docs/design/self-improvement.md`. It sends data out of the
engineer's machine, so no story is delivered before the operator approves it.

## What it is

`conductor retro` (and a `conductor:retro` skill that runs it): when an engineer finishes work with
conductor, successful or not, it analyses their session(s) against the KPIs of
`self-improvement.md` § 2 and for friction, writes a short report, scrubs it, shows the engineer
exactly what would leave, and on confirmation submits it as a Slack message or a GitHub issue.

```console
$ conductor retro --since 2h            # the sessions of the last two hours in this checkout
$ conductor retro --session <id> ...    # named sessions
$ conductor retro --submit github       # after review: submit to the configured target
```

Without `--submit` it only writes the report locally (`<state>/retro/<date>-<id>.md`) and prints it.

## Report format

One markdown document, at most 60 lines, the same sections every time:

1. **Outcome**: what the engineer said they were doing (their first request, paraphrased by the
   engineer at confirmation, never quoted from the transcript) and whether it ended merged, released,
   abandoned or still open.
2. **KPIs for this work**: the user-side subset of `self-improvement.md` § 2, each as a number with
   its definition id: K3 corrections, K7 idle time, K8 waiting, K9 session loss (limit errors,
   restarts), K10 guard denials, K11 tokens in and out, K6 failed CI runs and patch releases; plus
   **security**: guard denials by rule, secrets-scan hits, writes refused outside the checkout.
3. **Friction**: the top five friction signatures seen (`self-improvement.md` § 5 step 2), each as
   a signature id, a count and the time lost, with no message text.
4. **Versions**: conductor, aep, ess, worktree, Claude Code; the operating system family.
5. **Free text**: what the engineer chooses to add at confirmation.

A machine-readable block (YAML front matter) carries the same numbers, with `format:
conductor-retro/1`, so the loop can parse a submitted report without reading prose.

## Redaction

The report is built from counts, ids of signatures, version strings and the engineer's own words.
No transcript text, tool output or file content is copied into it at any stage, so redaction is a
second line of defence, not the first.

**Method**, applied to the whole report, including the engineer's free text, before display:

1. **Secrets and credentials**: the patterns of the organisation's secret scanner (`b10x-gates
   scan-text` with the configured policy when present; otherwise the built-in set: provider tokens
   such as `ghp_`, `github_pat_`, `glpat-`, `xox[bp]-`, `sk-`, AWS keys, JWTs, PEM blocks, and any
   40- or 64-character hex run), plus high-entropy strings of 20+ characters. Each match becomes
   `[secret]`.
2. **Hosts and URLs**: every URL and host name except an allow list (`github.com`, `gitlab.com` and
   the public documentation hosts conductor itself names) becomes `[host]`. IP addresses become
   `[ip]`.
3. **Paths**: the home directory becomes `~`; any absolute path outside `~` keeps only its last
   component.
4. **People and customers**: the names from a deny list (the instance config's `retro.deny` file,
   one term per line, kept out of any repository) and every e-mail address become `[name]`;
   repository names outside the instance's public sources become `[repository]`.
5. **Length**: a free-text section over 1,000 characters is refused, not cut.

**Tests**: a corpus of seeded reports (`crates/conductor/tests/fixtures/retro/`), each line carrying
a known secret, URL, host, IP, path, e-mail and deny-listed name of every kind above. The test asserts
every seed is replaced and nothing else changed. A property test generates random seeds into random
positions. A second test runs the scrubbed corpus through the organisation's secret scanner and
expects no hit. The release gate runs both.

**What the engineer sees**: the exact bytes that will be sent, with every replacement highlighted
and counted (`3 secrets, 2 hosts, 1 name replaced`), and the target. Submission requires typing
`send` (or `--yes` after reading it in the same command). Nothing is sent without that.

## Submission targets

Configured per instance, all optional:

```yaml
retro:
  deny: ~/.b10x/conductor/retro-deny.txt      # names to replace; never in a repository
  targets:
    github: {repository: example-org/conductor-feedback, labels: [retro]}
    slack:  {channel: "#conductor-retro", as: bot}   # through the instance's existing reports route
```

- **GitHub issue**: `gh issue create` (or `glab` for a GitLab source), with the report as body and
  the configured labels. The engineer's own login is used, not the bot.
- **Slack message**: through the same `reports` route the instance config already declares for
  channels (`reports: [{channel: "slack:…"}]`), never a new webhook in a file.
- No target configured: the report stays local, and the command prints where it is.

## Feeding the loop

The loop's daily run (`self-improvement.md` § 5) reads the submitted reports from the configured
GitHub repository (issues labelled `retro`) and from local reports the engineer allows it to collect.
It parses the YAML block, adds the numbers to the KPIs as a separate "user sessions" series, and turns
each friction signature into a finding the same way it does for conductor's own sessions, citing the
issue number. A signature reported by three engineers becomes a finding even if conductor's own
sessions never hit it.

## Never sent

- Transcript text of any kind: prompts, answers, tool calls, tool output.
- File contents, diffs, code, commit messages.
- Secrets, tokens, keys, certificates.
- Host names, internal URLs, IP addresses, absolute paths.
- People's names, e-mail addresses, customer names, repository names outside the public sources.
- Anything the engineer has not seen on screen in the confirmation step.

## Stories (after approval), under `epic:measured-self-improvement`

1. `retro-spec`: the report and its submission, specified first.
2. `retro-report`: `conductor retro` builds the report from the KPI instruments.
3. `retro-redaction`: the redaction with its corpus and property tests.
4. `retro-submit`: confirmation and the two targets.
5. `retro-loop`: submitted reports feed the KPIs and findings.
6. `retro-skill`: the `conductor:retro` skill and the docs page.

## Left out

- Automatic submission without the engineer's confirmation.
- Sending transcripts "for debugging", even redacted.
- Other targets (e-mail, Jira); they can be added as targets later.
- Analysis of sessions that were not conductor's or its controllers'.
