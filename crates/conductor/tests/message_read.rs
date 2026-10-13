//! A report's first line as controllers and conductor write it: a dispatch id of `-` for a report
//! that answers no dispatch, the `ready` report of a restart, and a verb followed by a colon.

use conductor_cli::message::read;
use conductor_model::dispatch::MessageKind;

#[test]
fn a_report_with_no_dispatch_and_the_ready_verb_parses() {
    let reading = read("[REPORT conductor -] ready docs/handoff/x.md");
    assert_eq!(reading.kind, Some(MessageKind::Report));
    assert!(reading.parsed, "{reading:?}");
}

#[test]
fn a_report_verb_followed_by_a_colon_parses() {
    let reading = read("[REPORT alpha DSP-1] progress: unit A merged");
    assert!(reading.parsed, "{reading:?}");
}

#[test]
fn a_report_verb_followed_by_a_semicolon_or_comma_parses() {
    assert!(read("[REPORT gates DSP-1] started; wave 3 in tree t").parsed);
    assert!(read("[REPORT gates DSP-1] done, released v1.0.0").parsed);
}

#[test]
fn a_bare_report_verb_parses() {
    assert!(read("[REPORT gates DSP-1] started").parsed);
    assert!(read("[REPORT conductor -] ready").parsed);
}

#[test]
fn a_report_with_another_word_for_its_dispatch_id_or_verb_still_does_not_parse() {
    assert!(!read("[REPORT alpha x] started").parsed);
    assert!(!read("[REPORT alpha DSP-1] maybe: something").parsed);
}
