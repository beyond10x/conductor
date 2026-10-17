//! A report's first line as controllers and conductor write it: a dispatch id of `-` for a report
//! that answers no dispatch, the `ready` report of a restart, and a verb followed by a colon; and
//! the run daemon's hand-over.

use conductor_cli::message::{named_repository, read};
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

/// `story:mailbox-send-and-deliver` (docs/design/conductor-run.md § 3): the run daemon's
/// `[HANDOVER <repo>] context <n>k` and `[HANDOVER <repo>] context shutdown` parse as a hand-over,
/// and name their repository; a controller's own `blocked handover <path>` stays a report.
#[test]
fn a_handover_parses_as_its_own_form() {
    for line in [
        "[HANDOVER cortex] context 210k",
        "[HANDOVER cortex] context 1250k",
        "[HANDOVER cortex] context shutdown",
    ] {
        let reading = read(line);
        assert_eq!(reading.kind, Some(MessageKind::Handover), "{line}");
        assert!(reading.parsed, "{line}: {reading:?}");
        assert_eq!(named_repository(line).as_deref(), Some("cortex"), "{line}");
    }
    let report = read("[REPORT cortex DSP-1] blocked handover .git/handoff/2026-10-17.md");
    assert_eq!(report.kind, Some(MessageKind::Report));
    assert!(report.parsed, "{report:?}");
}

/// A hand-over line with anything but `context <n>k` or `context shutdown` after its brackets, or
/// anything but one repository inside them, is a hand-over that does not parse.
#[test]
fn a_handover_with_other_words_does_not_parse() {
    for line in [
        "[HANDOVER cortex] context",
        "[HANDOVER cortex] context 210",
        "[HANDOVER cortex] context k",
        "[HANDOVER cortex] context 21Ok",
        "[HANDOVER cortex] context 210k now",
        "[HANDOVER cortex] usage 210k",
        "[HANDOVER] context 210k",
        "[HANDOVER cortex DSP-1] context 210k",
        "[HANDOVER cortex]context 210k",
    ] {
        let reading = read(line);
        assert_eq!(reading.kind, Some(MessageKind::Handover), "{line}");
        assert!(!reading.parsed, "{line} does not parse");
    }
}
