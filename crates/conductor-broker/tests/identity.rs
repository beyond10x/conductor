//! The broker knows a caller by the socket the call comes in on, never by what the call says
//! (`story:tool-broker-read`, acceptance 3; design § 6.2): a call on session A's socket acts as A
//! whatever session it claims, a call on the socket of a session that stopped is refused
//! `UnknownSession`, and a session of another instance is refused `ForeignInstance`. Each is
//! recorded.

mod support;

use conductor_broker::{socket_path, unregister};
use conductor_model::tool::{CallOutcome, Verb};
use serde_json::json;
use support::{Case, INSTANCE, ask, lines, reader};

#[test]
fn a_call_on_a_sessions_socket_acts_as_that_session_whatever_it_claims() {
    let mut case = Case::new();
    let a = case.workdir("a", &[("note.txt", "written by a\n")]);
    let b = case.workdir("b", &[("note.txt", "written by b\n")]);
    let socket_a = case.register("example-org-a", INSTANCE, &a, reader());
    case.register("example-org-b", INSTANCE, &b, reader());
    assert_eq!(socket_a, socket_path(&case.dir(), "example-org-a"));
    case.serve();

    let answer = ask(
        &socket_a,
        "show",
        json!({"path": "note.txt", "session": "example-org-b"}),
    );

    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert!(answer.text.contains("written by a"), "{}", answer.text);
    assert!(!answer.text.contains("written by b"), "{}", answer.text);
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].session, "example-org-a");
    assert_eq!(calls[0].instance, INSTANCE);
    assert_eq!(calls[0].verb, Verb::Show);
    assert_eq!(calls[0].outcome, CallOutcome::Done);
    assert_eq!(calls[0].paths, vec!["note.txt".to_owned()]);
}

#[test]
fn the_socket_of_a_session_that_stopped_is_refused_unknown_session() {
    let mut case = Case::new();
    let a = case.workdir("a", &[("note.txt", "text\n")]);
    let socket = case.register("example-org-a", INSTANCE, &a, reader());
    case.serve();
    assert_eq!(ask(&socket, "show", json!({"path": "note.txt"})).exit, 0);

    unregister(&case.dir(), "example-org-a").expect("unregister the session");
    let answer = ask(&socket, "show", json!({"path": "note.txt"}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert_eq!(lines(&answer).len(), 1, "{}", answer.text);
    assert!(
        answer.text.starts_with("UnknownSession: "),
        "{}",
        answer.text
    );
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[1].session, "example-org-a");
    assert_eq!(calls[1].outcome, CallOutcome::UnknownSession);
}

#[test]
fn a_session_of_another_instance_is_refused_foreign_instance() {
    let mut case = Case::new();
    let other = case.workdir("other", &[("note.txt", "text\n")]);
    let socket = case.register("other-org-a", "other-org", &other, reader());
    case.serve();

    let answer = ask(&socket, "show", json!({"path": "note.txt"}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert_eq!(lines(&answer).len(), 1, "{}", answer.text);
    assert!(
        answer.text.starts_with("ForeignInstance: "),
        "{}",
        answer.text
    );
    assert!(answer.text.contains("other-org"), "{}", answer.text);
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].outcome, CallOutcome::ForeignInstance);
}

#[test]
fn a_session_registered_while_the_broker_serves_gets_its_socket() {
    let mut case = Case::new();
    case.serve();
    let a = case.workdir("a", &[("note.txt", "late\n")]);

    let socket = case.register("example-org-late", INSTANCE, &a, reader());

    let answer = ask(&socket, "show", json!({"path": "note.txt"}));
    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert!(answer.text.contains("late"), "{}", answer.text);
}
