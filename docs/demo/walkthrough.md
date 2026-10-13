# Walkthrough: one day with conductor, for a terminal recording

The script behind [Why conductor](../../website/docs/concepts/why-conductor.md). Part A goes from
`conductor setup` to a published board and the dashboard, and part B replays the day's records,
with no step that needs a person. Part C starts the live sessions; each step there that needs a
person is marked **person**.

Record it as a throwaway user or in a VM ([run conductor isolated](../../website/docs/guides/run-isolated.md)):
`conductor setup --apply` writes `~/.b10x/conductor/` and `~/.claude.json`. The instance is
named `alpha`, after the repository setup runs in; its records are `~/.b10x/conductor/alpha/records`.

## Before recording (person)

- Install `conductor` ([getting started](../../website/docs/getting-started.md#install)), Claude
  Code, `git`, `task`, `jq`, `aep`, `worktree` and `ess`. No forge CLI is needed: the sample has a
  `local` source only.
- Log Claude Code in: `claude auth login`. Only part C uses it.
- Clone the conductor repository to `~/conductor`; part C runs its tasks.

## Part A: from setup to the board (no person)

```console
git config --global user.name "Demo" && git config --global user.email demo@example.com
mkdir -p ~/demo/alpha ~/demo/beta
git -C ~/demo/alpha init -q -b main && git -C ~/demo/beta init -q -b main
echo "# alpha" > ~/demo/alpha/README.md && git -C ~/demo/alpha add README.md && git -C ~/demo/alpha commit -q -m "Add a README"
echo "# beta" > ~/demo/beta/README.md && git -C ~/demo/beta add README.md && git -C ~/demo/beta commit -q -m "Add a README"
echo "fn main() {}" > ~/demo/alpha/main.rs && git -C ~/demo/alpha add main.rs && git -C ~/demo/alpha commit -q -m "Add the program"

cd ~/demo/alpha
conductor setup             # not ready: config (unreadable), instance (none), records (missing), trust (not trusted)
conductor setup --apply     # writes the config, runs conductor init and conductor trust; ends `ready`
conductor config show | head -10

id=$(conductor snapshot start-snapshot) && echo "$id"
conductor repository activity                      # alpha Active, beta Inactive
conductor snapshot publish-board --snapshot-id "$id"
sed -n 1,12p ~/.b10x/conductor/alpha/records/STATUS.md

conductor dashboard serve &                        # open http://127.0.0.1:7313/ in a browser
```

Stop the dashboard with `kill %1` when the recording has shown it.

## Part B: the day's records, replayed (no person)

These are the commands the conductor session runs during the day
([Why conductor](../../website/docs/concepts/why-conductor.md#one-day-with-conductor), steps 2 to
6), without starting a session. `spawn` is left out: it starts a session.

```console
conductor goal propose-goal --goal-id G-1 --title "alpha ships its first release" --exit-evidence "a v0.1.0 release of alpha"
conductor goal confirm-goal --goal-id G-1
conductor dispatch send-dispatch --dispatch-id DSP-20261013-01 --repository alpha --goal-id G-1 \
  --brief "plan and deliver the first release" --sent-at 2026-10-13T09:00:00Z --priority 1

conductor decision raise-decision-request --request-id REQ-1 --repository alpha \
  --question "approve the wave of stories release-notes and version-flag?" \
  --options "A approve; B hold" --recommendation A --raised-at 2026-10-13T10:00:00Z
conductor decision record-conductor-decision --decision-class C --repository alpha \
  --question "approve the wave of stories release-notes and version-flag?" --options "A approve; B hold" \
  --choice A --reason "inside the accepted goal; both can be reverted" --evidence REQ-1 \
  --decided-at 2026-10-13T10:02:00Z
conductor decision answer-request --request-id REQ-1 --decision-id DEC-20261013-01
conductor dispatch report-started --dispatch-id DSP-20261013-01

conductor decision raise-decision-request --request-id REQ-2 --repository alpha \
  --question "delete the old prototype branch, the only copy of its experiments?" \
  --options "A delete; B archive first" --recommendation B --raised-at 2026-10-13T11:00:00Z
conductor decision escalate-request --request-id REQ-2 --escalation-class O --reason "data loss cannot be undone"
conductor decision requests --format jsonl | jq -c '{request_id, state, escalation_class}'
conductor decision record-operator-decision --decision-class O --repository alpha \
  --question "delete the old prototype branch?" --options "A delete; B archive first" --choice B \
  --reason "keep the only copy" --evidence REQ-2 --decided-at 2026-10-13T11:20:00Z \
  --recorded-by conductor --operator-words "archive it first"
conductor decision answer-escalated-request --request-id REQ-2 --decision-id DEC-20261013-02

conductor dispatch report-done --dispatch-id DSP-20261013-01 --evidence "alpha v0.1.0 released"
conductor decision decisions --format jsonl | jq -c '{decision_id, decision_class, decided_by, choice}'
conductor dispatch dispatches --format jsonl | jq -c '{dispatch_id, state, repository, goal_id}'
```

Decision ids are allocated for the UTC date of `--decided-at`, so in a fresh store the two record
commands print `DEC-20261013-01` and `DEC-20261013-02`, the ids the answers pass on.

## Part C: the live sessions

1. Give the roles their profiles and the guard's settings. No person: `task profiles` installs
   the profiles, and the instance written in part A is the last entry of its file, so the roles
   can be appended:

   ```console
   cd ~/conductor && task profiles
   cat >> ~/.b10x/conductor/conductor.yaml <<'EOF'
       roles:
         - role: conductor
           harness: claude
           model: opus
           profile: ~/.b10x/conductor/profiles/conductor.md
           settings: ~/conductor/.claude/conductor-settings.json
         - role: controller
           harness: claude
           model: opus
           profile: ~/.b10x/conductor/profiles/repo-controller.md
           settings: ~/conductor/.claude/controller-settings.json
   EOF
   conductor config validate
   ```

2. **Person:** accept Claude Code's bypass-permissions disclaimer once: run
   `claude --dangerously-skip-permissions` in `~/.b10x/conductor/alpha/records` and accept.
3. `task conductor:start`, then `task conductor:attach` to show the session.
4. **Person:** the first hand-over asks the operator to fill in `rules.md` and `NORTHSTAR.md`.
   Answer in the session, and give it the goal "alpha ships its first release".
5. Conductor records the goal, sends the dispatch and runs `conductor spawn --repository alpha`;
   `task sessions` and the dashboard show the controller. A class O question, if one comes, is
   step 4 of the page again: **person** answers it in the session.
