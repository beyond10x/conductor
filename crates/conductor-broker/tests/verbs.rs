//! The one list of verbs, [`conductor_broker::VERBS`], holds every verb of the specification
//! once, and every word reads back as its verb (`story:tool-build-commit-report`). The config
//! file, the store and `tool mcp` read verbs through this list, so a verb missing here is one a
//! grant cannot name and a record cannot be read back with. The same holds for the outcomes, the
//! report statuses and the levels.

use std::collections::BTreeSet;

use conductor_broker::{
    CallOutcome, LEVELS, Level, OUTCOMES, ReportStatus, STATUSES, VERBS, Verb, level_named,
    level_word, outcome_word, status_named, status_word, verb_named, verb_word,
};

/// Each verb's place in the specification's order. Exhaustive: a verb the specification adds
/// fails to compile here until it has one.
fn place(verb: Verb) -> usize {
    match verb {
        Verb::Show => 0,
        Verb::Search => 1,
        Verb::Diff => 2,
        Verb::Test => 3,
        Verb::Check => 4,
        Verb::Fmt => 5,
        Verb::Commit => 6,
        Verb::Send => 7,
        Verb::Report => 8,
    }
}

fn outcome_place(outcome: CallOutcome) -> usize {
    match outcome {
        CallOutcome::Done => 0,
        CallOutcome::Failed => 1,
        CallOutcome::NotGranted => 2,
        CallOutcome::GrantExpired => 3,
        CallOutcome::ForeignInstance => 4,
        CallOutcome::UnknownSession => 5,
        CallOutcome::OutsideTeam => 6,
        CallOutcome::NotOneLevelUp => 7,
    }
}

fn status_place(status: ReportStatus) -> usize {
    match status {
        ReportStatus::Started => 0,
        ReportStatus::Progress => 1,
        ReportStatus::Blocked => 2,
        ReportStatus::Unblocked => 3,
        ReportStatus::Done => 4,
        ReportStatus::Failed => 5,
    }
}

fn level_place(level: Level) -> usize {
    match level {
        Level::Director => 0,
        Level::Lead => 1,
        Level::Worker => 2,
    }
}

/// Whether `places` are exactly `0..places.len()`, each once.
fn each_once(places: impl IntoIterator<Item = usize>) -> bool {
    let places: Vec<usize> = places.into_iter().collect();
    let distinct: BTreeSet<usize> = places.iter().copied().collect();
    distinct.len() == places.len() && distinct.into_iter().eq(0..places.len())
}

#[test]
fn the_verb_list_holds_every_verb_once_and_each_word_reads_back() {
    assert!(each_once(VERBS.map(place)), "{VERBS:?}");
    assert_eq!(VERBS.len(), place(Verb::Report) + 1);
    for verb in VERBS {
        assert_eq!(verb_named(verb_word(verb)), Some(verb));
    }
}

#[test]
fn the_outcome_status_and_level_lists_hold_each_once() {
    assert!(each_once(OUTCOMES.map(outcome_place)), "{OUTCOMES:?}");
    assert_eq!(
        OUTCOMES.len(),
        outcome_place(CallOutcome::NotOneLevelUp) + 1
    );
    let words: BTreeSet<&str> = OUTCOMES.into_iter().map(outcome_word).collect();
    assert_eq!(words.len(), OUTCOMES.len());
    assert!(each_once(STATUSES.map(status_place)), "{STATUSES:?}");
    assert_eq!(STATUSES.len(), status_place(ReportStatus::Failed) + 1);
    for status in STATUSES {
        assert_eq!(status_named(status_word(status)), Some(status));
    }
    assert!(each_once(LEVELS.map(level_place)), "{LEVELS:?}");
    assert_eq!(LEVELS.len(), level_place(Level::Worker) + 1);
    for level in LEVELS {
        assert_eq!(level_named(level_word(level)), Some(level));
    }
}
