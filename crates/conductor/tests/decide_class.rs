//! `story:decide-model-adapter` and `story:classify-tightened`: a decision request's class is
//! chosen by rules in code (`decide::classify`), over the request's question and its lettered
//! options (`docs/design/conductor-run.md` § 3, design § 5). Each rule matches an action on an
//! object, not a word: data loss, a history rewrite, a new, renamed or retired repository and an
//! architecture decision are class O; a credential and a permission, account or payment only the
//! operator holds are class H; everything else is class C, rule `none`. `classify` answers the
//! words the rule matched with it.
//!
//! The thirty requests of [`CASES`] are written from the shapes of the requests controllers
//! raise. Rows 1 to 12 are the first fixtures: nine that a rule classifies, every rule at least
//! once, and three routine ones. Rows 21 to 38 are shaped after the requests of four days of one
//! instance: nine class C, eight of which the word rules escalated (a build output and a merged
//! branch deleted, a KPI that counts tokens, a token budget, a scratch path wiped, a dependency
//! upgrade, a wave approval, a release go), and nine an action on the object a rule names. They
//! name no real request, decision or repository. [`MORE`] holds the other words a rule reads.

mod active;
mod common;

use std::fs;
use std::path::PathBuf;

use conductor_cli::decide::{self, RULES, rule_name};
use conductor_model::decision::{ClassificationRule, DecisionClass};
use serde_json::Value;

/// One request: its text, as a controller sends it, and the class, rule and matched words code
/// gives it (no words for rule `none`).
struct Case {
    text: &'static str,
    class: DecisionClass,
    rule: ClassificationRule,
    phrase: &'static str,
}

const CASES: [Case; 30] = [
    Case {
        text: "[DECISION-REQUEST example-repo req-01] Delete the worktree another session left with uncommitted work?
A: delete it now (cost: the other session's edits are gone)
B: archive it first, then remove it (cost: 2G for a week)
Recommendation: B, it can be undone",
        class: DecisionClass::O,
        rule: ClassificationRule::DataLoss,
        phrase: "delete the worktree",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-02] The old event export is the only copy of last year's records. What do we do with it?
A: purge the export (cost: frees 40G)
B: keep it (cost: 40G of disk)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::DataLoss,
        phrase: "purge the export",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-03] A commit with a 300 MB video landed on main. How is it removed?
A: force-push main without the commit (cost: every clone rebases)
B: revert it in a new commit (cost: the video stays in the history)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::HistoryRewrite,
        phrase: "force push",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-04] The release commit's message names the wrong version. Amend the published commit?
A: amend and push again (cost: the tag moves)
B: say so in the release notes (cost: none)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::HistoryRewrite,
        phrase: "amend the published",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-05] Where does the shared schema crate live?
A: create a new repository for it (cost: one more CI setup)
B: a workspace member here (cost: consumers pull this repository)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::RepositoryLifecycle,
        phrase: "create a new repository",
    },
    Case {
        text: "[DECISION-REQUEST example-tool req-06] The tool now only serves the command line. Rename the repository to example-cli?
Y: rename it (cost: every remote URL changes)
N: keep the name (cost: none)
Recommendation: N",
        class: DecisionClass::O,
        rule: ClassificationRule::RepositoryLifecycle,
        phrase: "rename the repository",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-07] Accept ADR 0007, events kept as one file per stream?
Y: accept it (cost: a migration of the store)
N: send changes (cost: the store story waits)
Recommendation: Y",
        class: DecisionClass::O,
        rule: ClassificationRule::Architecture,
        phrase: "accept adr",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-08] Which story of the epic goes first?
A: the parser story (cost: the renderer waits a day)
B: the renderer story (cost: the parser waits a day)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-09] The release job needs a deploy token. Set it as a CI secret?
A: set the secret now (cost: five minutes of the operator)
B: release by hand this time (cost: an hour)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Credential,
        phrase: "set the secret",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-10] Branch protection blocks the bot's merge. Grant it the bypass, a permission only the operator holds?
A: grant the bypass (cost: one setting)
B: merge each pull request by hand (cost: a minute each)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Hands,
        phrase: "grant it the bypass",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-11] Run the flaky integration suite in this wave or the next?
A: this wave (cost: one more build slot today)
B: the next wave (cost: a day)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-12] The dependency upgrade passed CI. Merge it now or with the next release?
A: merge now (cost: none)
B: wait for the release branch (cost: two days)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    // Rows 21 to 29: routine requests that the word rules escalated, a rule word on an object
    // it does not cover, or no action at all.
    Case {
        text: "[DECISION-REQUEST example-repo req-21] Delete the build output of the idle managed tree to free 12G?
A: delete it now (cost: the next build there starts cold)
B: leave it (cost: 12G of disk)
Recommendation: A, it can be rebuilt",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-22] The wave branch is merged into main. Delete the merged branch?
A: delete it (cost: none)
B: keep it (cost: one more branch in the list)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-23] The cost KPI counts tokens per task. Count the input tokens of cached prompts too?
A: count them (cost: the series restarts)
B: leave them out (cost: the KPI reads low)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-24] Set a token budget of 2M for the nightly run?
A: set it (cost: the run may stop early)
B: no budget (cost: none)
Recommendation: A, it can be raised later",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-25] Wipe the scratch path the last test run left under the cache directory?
A: wipe it (cost: none)
B: keep it (cost: 3G)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-26] Upgrade the pinned web framework dependency from 3.1 to 3.2?
A: upgrade now (cost: one rebuild)
B: wait for the next release (cost: a week)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-27] Approve wave 12: five units, one of which deletes the stale fixtures of the old parser?
A: approve it (cost: none)
B: hold it a day (cost: a day)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-28] Release go for example-repo 0.19.0, the first release with the architecture page?
A: go (cost: none)
B: wait for the next wave (cost: a day)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-29] The pull request that drops the deprecated flag has green checks. Merge it now?
A: merge now (cost: none)
B: wait for a review (cost: a day)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
        phrase: "",
    },
    // Rows 30 to 38: the operator's, each an action on the object its rule names.
    Case {
        text: "[DECISION-REQUEST example-store req-30] Erase the stored evidence of the withdrawn claims from the knowledge store?
A: erase it (cost: the audit trail is gone)
B: mark the claims withdrawn (cost: none)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::DataLoss,
        phrase: "erase the stored evidence",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-31] The wave branch was rewritten with filter-branch to drop a large file. Force-push it?
A: force-push (cost: every clone of the branch resets)
B: open a new branch from main (cost: the pull request is reopened)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::HistoryRewrite,
        phrase: "filter branch",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-32] Move the parser crate into a new repository of its own?
A: a new repository (cost: one more CI setup)
B: keep it in this workspace (cost: none)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::RepositoryLifecycle,
        phrase: "new repository",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-33] Accept ADR 0012, the event envelope as a cross-repository contract?
Y: accept it (cost: two consumers change their readers)
N: send it back (cost: the epic waits)
Recommendation: Y",
        class: DecisionClass::O,
        rule: ClassificationRule::Architecture,
        phrase: "accept adr",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-34] CI needs to publish to the package registry. Set a repository secret for it?
A: set the secret now (cost: five minutes of the operator)
B: publish by hand (cost: an hour each release)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Credential,
        phrase: "set a repository secret",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-35] The review tool needs an API key. Enter one into the session's state?
A: enter the API key now (cost: the operator types it once)
B: run without the review tool (cost: no reviews this week)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Credential,
        phrase: "enter the api key",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-36] Create an organisation ruleset that requires signed commits? Only the organisation owner may create one.
A: create it (cost: ten minutes of the owner)
B: a branch rule per repository (cost: one rule in each repository)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Hands,
        phrase: "organisation owner",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-37] The free tier of the model provider ran out. Pay for a provider account?
A: pay for it (cost: a monthly fee)
B: pause the reviews (cost: no reviews until next month)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Hands,
        phrase: "pay for a provider account",
    },
    Case {
        text: "[DECISION-REQUEST example-store req-38] Retype the claim node kind in the knowledge store's kernel from text to a structured record?
A: retype it (cost: a migration of every store)
B: add a second kind beside it (cost: two kinds for a while)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::Architecture,
        phrase: "retype the claim node kind",
    },
];

/// The other words the rules read, one request each.
const MORE: [Case; 7] = [
    Case {
        text: "[DECISION-REQUEST example-repo req-13] The report endpoint's cross-repository contract carries JSON. Which encoding does version 2 use?
A: keep JSON (cost: none)
B: a binary encoding (cost: every consumer changes)
Recommendation: A",
        class: DecisionClass::O,
        rule: ClassificationRule::Architecture,
        phrase: "cross repository contract",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-14] Erase the cached transcripts of the stopped sessions?
Y: erase them (cost: no replay of those sessions)
N: keep them (cost: 3G)
Recommendation: N",
        class: DecisionClass::O,
        rule: ClassificationRule::DataLoss,
        phrase: "erase the cached transcripts",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-15] Strip the large binaries from the published history with filter-branch?
Y: run it (cost: every open pull request is rebased)
N: leave them (cost: clones stay slow)
Recommendation: N",
        class: DecisionClass::O,
        rule: ClassificationRule::HistoryRewrite,
        phrase: "filter branch",
    },
    Case {
        text: "[DECISION-REQUEST example-old req-16] Nothing has used it for a year. Retire the repository?
Y: retire it (cost: none)
N: keep it (cost: its CI keeps running)
Recommendation: Y",
        class: DecisionClass::O,
        rule: ClassificationRule::RepositoryLifecycle,
        phrase: "retire the repository",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-17] Which web framework does the dashboard use?
A: the one the other dashboards use (cost: none)
B: a lighter one (cost: two days to learn it)
Recommendation: A",
        class: DecisionClass::O,
        rule: ClassificationRule::Architecture,
        phrase: "which web framework",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-18] Publishing needs a login to the package registry. Who logs in?
A: the operator logs in once (cost: five minutes)
B: skip publishing this release (cost: users build from source)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Credential,
        phrase: "logs in",
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-19] The free CI minutes ran out. Buy a paid account?
A: buy it (cost: a monthly fee)
B: run CI on the host (cost: one build slot)
Recommendation: B",
        class: DecisionClass::H,
        rule: ClassificationRule::Hands,
        phrase: "buy a paid account",
    },
];

/// Each case of `cases` whose class or rule is not its row's, one line each.
fn misclassified(cases: &[Case]) -> Vec<String> {
    cases
        .iter()
        .filter_map(|case| {
            let got = decide::classify(case.text);
            ((got.class, got.rule, got.phrase.as_str()) != (case.class, case.rule, case.phrase))
                .then(|| {
                    format!(
                        "{}: got class {:?} rule {} {:?}, want class {:?} rule {} {:?}",
                        case.text.lines().next().unwrap_or_default(),
                        got.class,
                        rule_name(got.rule),
                        got.phrase,
                        case.class,
                        rule_name(case.rule),
                        case.phrase
                    )
                })
        })
        .collect()
}

#[test]
fn each_of_the_thirty_requests_has_the_class_rule_and_phrase_of_its_row() {
    active::isolate();
    let wrong = misclassified(&CASES);
    assert!(
        wrong.is_empty(),
        "misclassified {} of {}:\n{}",
        wrong.len(),
        CASES.len(),
        wrong.join("\n")
    );
}

#[test]
fn the_thirty_cover_every_rule_and_twelve_class_c_requests() {
    active::isolate();
    for rule in RULES {
        let rows: Vec<&Case> = CASES.iter().filter(|case| case.rule == rule).collect();
        match rule {
            ClassificationRule::None => assert_eq!(rows.len(), 12, "twelve rows are class C"),
            _ => assert!(!rows.is_empty(), "no row has rule {}", rule_name(rule)),
        }
        let class = match rule {
            ClassificationRule::None => DecisionClass::C,
            ClassificationRule::Credential | ClassificationRule::Hands => DecisionClass::H,
            _ => DecisionClass::O,
        };
        for row in rows {
            assert_eq!(
                row.class,
                class,
                "rule {} is class {class:?}",
                rule_name(rule)
            );
            assert_eq!(
                row.phrase.is_empty(),
                rule == ClassificationRule::None,
                "a rule names the words it matched, and `none` names none: {}",
                row.text.lines().next().unwrap_or_default()
            );
        }
    }
}

#[test]
fn every_word_a_rule_reads_classifies() {
    active::isolate();
    let wrong = misclassified(&MORE);
    assert!(wrong.is_empty(), "misclassified:\n{}", wrong.join("\n"));
}

#[test]
fn a_rule_word_outside_the_question_and_options_does_not_classify() {
    active::isolate();
    let text = "[DECISION-REQUEST example-repo req-20] Run the nightly suite at two or at four?
A: at two (cost: none)
B: at four (cost: results an hour later)
Recommendation: A; the old runner used to delete its cache at three, which no longer matters";
    let got = decide::classify(text);
    assert_eq!(
        (got.class, got.rule, got.phrase.as_str()),
        (DecisionClass::C, ClassificationRule::None, "")
    );
}

/// `conductor decision handle` prints the rule and the words it matched before the § 6 line
/// (`rule none` for class C), and an escalation's reason names those words.
#[test]
fn decision_handle_prints_the_rule_and_its_phrase_and_the_reason_names_it() {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("decide_class")
        .join("handle");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(dir.join("home")).expect("create the case's home");
    let run = |args: &[&str]| {
        common::conductor(dir.join("home"))
            .current_dir(&dir)
            .arg("--state-dir")
            .arg(dir.join("state"))
            .args(args)
            .output()
            .expect("the conductor binary runs")
    };
    let handle = |row: usize| {
        let file = dir.join(format!("request-{row}.md"));
        fs::write(&file, CASES[row].text).expect("write the request");
        let output = run(&[
            "decision",
            "handle",
            "--request",
            &file.display().to_string(),
            "--decider",
            "fake",
        ]);
        assert!(
            output.status.success(),
            "decision handle: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("UTF-8")
    };

    let escalated = handle(0);
    assert_eq!(
        escalated.lines().collect::<Vec<_>>(),
        [
            "rule data-loss: \"delete the worktree\"",
            "[ESCALATION req-01] O Delete the worktree another session left with uncommitted work?",
        ]
    );
    let decided = handle(7);
    let lines: Vec<&str> = decided.lines().collect();
    assert_eq!(lines.len(), 2, "{decided}");
    assert_eq!(lines[0], "rule none");
    assert!(lines[1].starts_with("[DECISION req-08] A "), "{decided}");

    let requests = run(&["decision", "requests", "--format", "json"]);
    let rows: Vec<Value> = serde_json::from_slice(&requests.stdout).expect("the requests as JSON");
    let reason = rows
        .iter()
        .find(|row| row["request_id"] == "req-01")
        .and_then(|row| row["escalation_reason"].as_str())
        .unwrap_or_default()
        .to_owned();
    assert!(
        reason.contains("data-loss") && reason.contains("\"delete the worktree\""),
        "the reason names the rule and the words it matched: {reason}"
    );
    fs::remove_dir_all(&dir).expect("remove the case's directory");
}
