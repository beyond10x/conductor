//! `story:decide-model-adapter`: a decision request's class is chosen by rules in code
//! (`decide::classify`), over the request's question and its lettered options
//! (`docs/design/conductor-run.md` § 3, design § 5). Data loss, a history rewrite, a new, renamed
//! or retired repository and an architecture decision are class O; a credential and a permission
//! only the operator holds are class H; everything else is class C, rule `none`.
//!
//! The twelve requests of [`CASES`] are written from the shapes of the requests controllers
//! raise: nine that a rule classifies, every rule at least once, and three routine ones. They
//! name no real request, decision or repository. [`MORE`] holds the other words a rule reads.

mod active;

use conductor_cli::decide::{self, RULES, rule_name};
use conductor_model::decision::{ClassificationRule, DecisionClass};

/// One request: its text, as a controller sends it, and the class and rule code gives it.
struct Case {
    text: &'static str,
    class: DecisionClass,
    rule: ClassificationRule,
}

const CASES: [Case; 12] = [
    Case {
        text: "[DECISION-REQUEST example-repo req-01] Delete the worktree another session left with uncommitted work?
A: delete it now (cost: the other session's edits are gone)
B: archive it first, then remove it (cost: 2G for a week)
Recommendation: B, it can be undone",
        class: DecisionClass::O,
        rule: ClassificationRule::DataLoss,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-02] The old event export is the only copy of last year's records. What do we do with it?
A: purge the export (cost: frees 40G)
B: keep it (cost: 40G of disk)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::DataLoss,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-03] A commit with a 300 MB video landed on main. How is it removed?
A: force-push main without the commit (cost: every clone rebases)
B: revert it in a new commit (cost: the video stays in the history)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::HistoryRewrite,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-04] The release commit's message names the wrong version. Amend the published commit?
A: amend and push again (cost: the tag moves)
B: say so in the release notes (cost: none)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::HistoryRewrite,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-05] Where does the shared schema crate live?
A: create a new repository for it (cost: one more CI setup)
B: a workspace member here (cost: consumers pull this repository)
Recommendation: B",
        class: DecisionClass::O,
        rule: ClassificationRule::RepositoryLifecycle,
    },
    Case {
        text: "[DECISION-REQUEST example-tool req-06] The tool now only serves the command line. Rename the repository to example-cli?
Y: rename it (cost: every remote URL changes)
N: keep the name (cost: none)
Recommendation: N",
        class: DecisionClass::O,
        rule: ClassificationRule::RepositoryLifecycle,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-07] Accept ADR 0007, events kept as one file per stream?
Y: accept it (cost: a migration of the store)
N: send changes (cost: the store story waits)
Recommendation: Y",
        class: DecisionClass::O,
        rule: ClassificationRule::Architecture,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-08] Which story of the epic goes first?
A: the parser story (cost: the renderer waits a day)
B: the renderer story (cost: the parser waits a day)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-09] The release job needs a deploy token. Set it as a CI secret?
A: set the secret now (cost: five minutes of the operator)
B: release by hand this time (cost: an hour)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Credential,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-10] Branch protection blocks the bot's merge. Grant it the bypass, a permission only the operator holds?
A: grant the bypass (cost: one setting)
B: merge each pull request by hand (cost: a minute each)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Hands,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-11] Run the flaky integration suite in this wave or the next?
A: this wave (cost: one more build slot today)
B: the next wave (cost: a day)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-12] The dependency upgrade passed CI. Merge it now or with the next release?
A: merge now (cost: none)
B: wait for the release branch (cost: two days)
Recommendation: A",
        class: DecisionClass::C,
        rule: ClassificationRule::None,
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
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-14] Erase the cached transcripts of the stopped sessions?
Y: erase them (cost: no replay of those sessions)
N: keep them (cost: 3G)
Recommendation: N",
        class: DecisionClass::O,
        rule: ClassificationRule::DataLoss,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-15] Strip the large binaries from the published history with filter-branch?
Y: run it (cost: every open pull request is rebased)
N: leave them (cost: clones stay slow)
Recommendation: N",
        class: DecisionClass::O,
        rule: ClassificationRule::HistoryRewrite,
    },
    Case {
        text: "[DECISION-REQUEST example-old req-16] Nothing has used it for a year. Retire the repository?
Y: retire it (cost: none)
N: keep it (cost: its CI keeps running)
Recommendation: Y",
        class: DecisionClass::O,
        rule: ClassificationRule::RepositoryLifecycle,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-17] Which web framework does the dashboard use?
A: the one the other dashboards use (cost: none)
B: a lighter one (cost: two days to learn it)
Recommendation: A",
        class: DecisionClass::O,
        rule: ClassificationRule::Architecture,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-18] Publishing needs a login to the package registry. Who logs in?
A: the operator logs in once (cost: five minutes)
B: skip publishing this release (cost: users build from source)
Recommendation: A",
        class: DecisionClass::H,
        rule: ClassificationRule::Credential,
    },
    Case {
        text: "[DECISION-REQUEST example-repo req-19] The free CI minutes ran out. Buy a paid account?
A: buy it (cost: a monthly fee)
B: run CI on the host (cost: one build slot)
Recommendation: B",
        class: DecisionClass::H,
        rule: ClassificationRule::Credential,
    },
];

/// Each case of `cases` whose class or rule is not its row's, one line each.
fn misclassified(cases: &[Case]) -> Vec<String> {
    cases
        .iter()
        .filter_map(|case| {
            let (class, rule) = decide::classify(case.text);
            ((class, rule) != (case.class, case.rule)).then(|| {
                format!(
                    "{}: got class {class:?} rule {}, want class {:?} rule {}",
                    case.text.lines().next().unwrap_or_default(),
                    rule_name(rule),
                    case.class,
                    rule_name(case.rule)
                )
            })
        })
        .collect()
}

#[test]
fn each_of_the_twelve_requests_has_the_class_and_rule_of_its_row() {
    active::isolate();
    let wrong = misclassified(&CASES);
    assert!(wrong.is_empty(), "misclassified:\n{}", wrong.join("\n"));
}

#[test]
fn the_twelve_cover_every_rule_and_three_class_c_requests() {
    active::isolate();
    for rule in RULES {
        let rows: Vec<&Case> = CASES.iter().filter(|case| case.rule == rule).collect();
        match rule {
            ClassificationRule::None => assert_eq!(rows.len(), 3, "three rows are class C"),
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
    assert_eq!(
        decide::classify(text),
        (DecisionClass::C, ClassificationRule::None)
    );
}
