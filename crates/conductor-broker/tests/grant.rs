//! Every call is checked against the caller's grant and recorded (`story:tool-broker-read`,
//! acceptance 5): a verb absent from the grant, a path outside it and an expired grant are each
//! refused on one line naming the grant rule, with exit status 1, and recorded as a `ToolCall`
//! with the rule.

mod support;

use conductor_broker::SessionGrant;
use conductor_model::tool::{CallOutcome, Verb};
use serde_json::json;
use support::{Case, INSTANCE, ask, grant, lines};

#[test]
fn a_verb_absent_from_the_grant_is_refused_naming_the_tools_rule() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("src/lib.rs", "fn main() {}\n")]);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Show]));
    case.serve();

    let answer = ask(&socket, "search", json!({"pattern": "main"}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert_eq!(lines(&answer), vec!["NotGranted: tools: search"]);
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].verb, Verb::Search);
    assert_eq!(calls[0].outcome, CallOutcome::NotGranted);
    assert_eq!(calls[0].rule.as_deref(), Some("tools: search"));
}

#[test]
fn a_path_outside_the_grants_paths_is_refused_naming_the_paths_rule() {
    let mut case = Case::new();
    let dir = case.workdir(
        "a",
        &[("src/lib.rs", "fn main() {}\n"), ("secret/key.txt", "k\n")],
    );
    let socket = case.register(
        "example-org-a",
        INSTANCE,
        &dir,
        SessionGrant {
            paths: vec!["src".to_owned()],
            ..grant(&[Verb::Show])
        },
    );
    case.serve();

    let inside = ask(&socket, "show", json!({"path": "src/lib.rs"}));
    let outside = ask(&socket, "show", json!({"path": "secret/key.txt"}));
    let escaping = ask(&socket, "show", json!({"path": "src/../secret/key.txt"}));

    assert_eq!(inside.exit, 0, "{}", inside.text);
    for answer in [&outside, &escaping] {
        assert_eq!(answer.exit, 1, "{}", answer.text);
        assert_eq!(lines(answer), vec!["NotGranted: paths: src"]);
    }
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[1].outcome, CallOutcome::NotGranted);
    assert_eq!(calls[1].rule.as_deref(), Some("paths: src"));
    assert_eq!(calls[1].paths, vec!["secret/key.txt".to_owned()]);
    assert_eq!(calls[2].outcome, CallOutcome::NotGranted);
}

#[test]
fn a_path_outside_the_working_directory_is_refused_even_with_no_paths_rule() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("a.txt", "a\n")]);
    case.workdir("b", &[("b.txt", "b\n")]);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Show]));
    case.serve();

    let answer = ask(&socket, "show", json!({"path": "../b/b.txt"}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert_eq!(lines(&answer).len(), 1, "{}", answer.text);
    assert!(
        answer.text.starts_with("NotGranted: paths: "),
        "{}",
        answer.text
    );
    assert_eq!(case.recorder.calls()[0].outcome, CallOutcome::NotGranted);
}

#[test]
fn an_expired_grant_is_refused_grant_expired() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("a.txt", "a\n")]);
    let socket = case.register(
        "example-org-a",
        INSTANCE,
        &dir,
        SessionGrant {
            expires_at: Some("2020-01-01T00:00:00Z".to_owned()),
            ..grant(&[Verb::Show])
        },
    );
    case.serve();

    let answer = ask(&socket, "show", json!({"path": "a.txt"}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert_eq!(
        lines(&answer),
        vec!["GrantExpired: ttl: expired at 2020-01-01T00:00:00Z"]
    );
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].outcome, CallOutcome::GrantExpired);
    assert_eq!(
        calls[0].rule.as_deref(),
        Some("ttl: expired at 2020-01-01T00:00:00Z")
    );
}

#[test]
fn every_answered_call_is_recorded_with_its_duration_and_bytes() {
    let mut case = Case::new();
    let dir = case.workdir("a", &[("a.txt", "0123456789\n")]);
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Show]));
    case.serve();

    let answer = ask(&socket, "show", json!({"path": "a.txt"}));

    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].outcome, CallOutcome::Done);
    assert_eq!(calls[0].rule, None);
    assert!(calls[0].duration_ms >= 0);
    assert_eq!(calls[0].output_bytes, answer.text.len() as i64);
    assert!(calls[0].called_at.ends_with('Z'), "{}", calls[0].called_at);
}
