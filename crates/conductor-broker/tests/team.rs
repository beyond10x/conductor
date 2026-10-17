//! The team verbs (`story:tool-build-commit-report`, acceptances 4 and 5; design § 2, § 6.3).
//! `send` records a message to a session of the caller's team, named by its session name or its
//! role; a recipient outside the team is refused `OutsideTeam`, one of another instance
//! `ForeignInstance`, and both refusals are recorded. `report` records the report to the session
//! one level above the caller, with the broker's clock, and asks the store to move the caller's
//! open dispatch when the caller is a lead; a report to any other level is refused
//! `NotOneLevelUp`. The broker cannot drive the harness's channel, so each answers, on one line,
//! the recorded message and whom to deliver it to.
//!
//! The store here is a fake that keeps what it was asked to record.

mod support;

use std::sync::{Arc, Mutex, PoisonError};

use conductor_broker::{
    CallOutcome, Level, Messages, Placement, Report, ReportStatus, Reported, Services, TeamMessage,
};
use conductor_model::tool::Verb;
use serde_json::json;
use support::{Case, INSTANCE, ask, grant, lines};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

#[derive(Default)]
struct FakeStore {
    sent: Mutex<Vec<TeamMessage>>,
    reports: Mutex<Vec<Report>>,
}

impl FakeStore {
    fn sent(&self) -> Vec<TeamMessage> {
        self.sent
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn reports(&self) -> Vec<Report> {
        self.reports
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Messages for FakeStore {
    fn send(&self, message: &TeamMessage) -> Result<String, String> {
        let mut sent = self.sent.lock().unwrap_or_else(PoisonError::into_inner);
        sent.push(message.clone());
        Ok(format!("message-{}", sent.len()))
    }

    fn report(&self, report: &Report) -> Result<Reported, String> {
        let mut reports = self.reports.lock().unwrap_or_else(PoisonError::into_inner);
        reports.push(report.clone());
        Ok(Reported {
            message: format!("report-{}", reports.len()),
            dispatch: report.lead.then(|| "DSP-20261017-001".to_owned()),
            note: None,
        })
    }
}

const DIRECTOR: &str = "example-org-conductor";
const LEAD: &str = "example-org-alpha";
const WORKER: &str = "example-org-alpha-implementer";
const OTHER_TEAM: &str = "example-org-beta-implementer";
const FOREIGN: &str = "other-org-alpha-implementer";

fn place(role: &str, level: Level, team: Option<&str>, above: Option<&str>) -> Placement {
    Placement {
        role: Some(role.to_owned()),
        level: Some(level),
        team: team.map(str::to_owned),
        above: above.map(str::to_owned),
    }
}

/// The instance's director, the lead of team `alpha` and its worker, a worker of team `beta`, and
/// a worker of team `alpha` registered under another instance; each granted `send` and `report`.
/// Answers the case, the store and the socket of each, in that order.
fn team() -> (Case, Arc<FakeStore>, [std::path::PathBuf; 4]) {
    let mut case = Case::new();
    let dir = case.workdir("w", &[]);
    let tools = grant(&[Verb::Send, Verb::Report]);
    let director = case.register_placed(
        DIRECTOR,
        INSTANCE,
        &dir,
        tools.clone(),
        place("conductor", Level::Director, None, None),
    );
    let lead = case.register_placed(
        LEAD,
        INSTANCE,
        &dir,
        tools.clone(),
        place("controller", Level::Lead, Some("alpha"), Some(DIRECTOR)),
    );
    let worker = case.register_placed(
        WORKER,
        INSTANCE,
        &dir,
        tools.clone(),
        place("implementer", Level::Worker, Some("alpha"), Some(LEAD)),
    );
    let other = case.register_placed(
        OTHER_TEAM,
        INSTANCE,
        &dir,
        tools.clone(),
        place(
            "implementer",
            Level::Worker,
            Some("beta"),
            Some("example-org-beta"),
        ),
    );
    case.register_placed(
        FOREIGN,
        "other-org",
        &dir,
        tools,
        place("implementer", Level::Worker, Some("alpha"), Some(LEAD)),
    );
    let store = Arc::new(FakeStore::default());
    let messages: Arc<dyn Messages> = store.clone();
    case.serve_with(Services {
        messages,
        ..Services::none()
    });
    (case, store, [director, lead, worker, other])
}

fn within(at: &str, before: OffsetDateTime, after: OffsetDateTime) -> bool {
    OffsetDateTime::parse(at, &Rfc3339)
        .is_ok_and(|at| at >= before.replace_nanosecond(0).unwrap_or(before) && at <= after)
}

#[test]
fn send_by_role_records_a_message_to_that_session_of_the_team() {
    let (case, store, [_, _, worker, _]) = team();
    let before = OffsetDateTime::now_utc();

    let answer = ask(
        &worker,
        "send",
        json!({"to": "controller", "text": "the parser is green"}),
    );

    let after = OffsetDateTime::now_utc();
    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert_eq!(lines(&answer).len(), 1, "{}", answer.text);
    assert!(answer.text.contains("message-1"), "{}", answer.text);
    assert!(answer.text.contains(LEAD), "{}", answer.text);
    let sent = store.sent();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].sender, WORKER);
    assert_eq!(sent[0].recipient, LEAD);
    assert_eq!(sent[0].team, "alpha");
    assert_eq!(sent[0].text, "the parser is green");
    assert!(within(&sent[0].at, before, after), "{}", sent[0].at);
    let calls = case.recorder.calls();
    assert_eq!(calls[0].verb, Verb::Send);
    assert_eq!(calls[0].outcome, CallOutcome::Done);
    assert!(calls[0].paths.is_empty(), "no message text is recorded");
}

#[test]
fn send_by_session_name_reaches_a_worker_of_the_lead() {
    let (_case, store, [_, lead, _, _]) = team();

    let answer = ask(
        &lead,
        "send",
        json!({"to": WORKER, "text": "rebase on main"}),
    );

    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert_eq!(store.sent()[0].recipient, WORKER);
}

#[test]
fn send_outside_the_team_is_refused_and_recorded() {
    let (case, store, [_, _, worker, other]) = team();

    let to_other_team = ask(&worker, "send", json!({"to": OTHER_TEAM, "text": "hello"}));
    let to_director = ask(&worker, "send", json!({"to": DIRECTOR, "text": "hello"}));
    let to_nobody = ask(&worker, "send", json!({"to": "reviewer", "text": "hello"}));
    let to_itself = ask(&worker, "send", json!({"to": WORKER, "text": "hello"}));
    let from_other = ask(&other, "send", json!({"to": "controller", "text": "hello"}));

    for answer in [
        &to_other_team,
        &to_director,
        &to_nobody,
        &to_itself,
        &from_other,
    ] {
        assert_eq!(answer.exit, 1, "{}", answer.text);
        assert!(answer.text.starts_with("OutsideTeam: "), "{}", answer.text);
        assert_eq!(lines(answer).len(), 1, "{}", answer.text);
    }
    assert!(store.sent().is_empty(), "nothing was sent");
    let calls = case.recorder.calls();
    assert_eq!(calls.len(), 5);
    assert!(
        calls
            .iter()
            .all(|call| call.outcome == CallOutcome::OutsideTeam && call.rule.is_some()),
        "{calls:?}"
    );
}

#[test]
fn send_to_a_session_of_another_instance_is_refused_foreign() {
    let (case, store, [_, lead, _, _]) = team();

    let answer = ask(&lead, "send", json!({"to": FOREIGN, "text": "hello"}));

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert!(
        answer.text.starts_with("ForeignInstance: "),
        "{}",
        answer.text
    );
    assert!(store.sent().is_empty());
    assert_eq!(
        case.recorder.calls()[0].outcome,
        CallOutcome::ForeignInstance
    );
}

#[test]
fn a_workers_report_goes_to_its_lead_and_moves_no_dispatch() {
    let (case, store, [_, _, worker, _]) = team();
    let before = OffsetDateTime::now_utc();

    let answer = ask(
        &worker,
        "report",
        json!({"status": "done", "line": "tests green at abc123"}),
    );

    let after = OffsetDateTime::now_utc();
    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert_eq!(lines(&answer).len(), 1, "{}", answer.text);
    assert!(answer.text.contains("report-1"), "{}", answer.text);
    assert!(answer.text.contains(LEAD), "{}", answer.text);
    let reports = store.reports();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].sender, WORKER);
    assert_eq!(reports[0].recipient, LEAD);
    assert_eq!(reports[0].team.as_deref(), Some("alpha"));
    assert!(!reports[0].lead);
    assert_eq!(reports[0].status, ReportStatus::Done);
    assert_eq!(reports[0].line, "tests green at abc123");
    assert!(within(&reports[0].at, before, after), "{}", reports[0].at);
    assert_eq!(case.recorder.calls()[0].outcome, CallOutcome::Done);
}

#[test]
fn a_leads_report_goes_to_the_director_and_moves_its_open_dispatch() {
    let (_case, store, [_, lead, _, _]) = team();

    let answer = ask(
        &lead,
        "report",
        json!({"status": "blocked", "line": "waiting on beta", "to": DIRECTOR}),
    );

    assert_eq!(answer.exit, 0, "{}", answer.text);
    assert_eq!(lines(&answer).len(), 1, "{}", answer.text);
    assert!(answer.text.contains("DSP-20261017-001"), "{}", answer.text);
    assert!(answer.text.contains(DIRECTOR), "{}", answer.text);
    let reports = store.reports();
    assert_eq!(reports[0].recipient, DIRECTOR);
    assert!(reports[0].lead, "a lead's report moves its dispatch");
    assert_eq!(reports[0].status, ReportStatus::Blocked);
}

#[test]
fn a_report_to_any_other_level_is_refused_and_recorded() {
    let (case, store, [director, lead, worker, _]) = team();

    let past_the_lead = ask(
        &worker,
        "report",
        json!({"status": "done", "line": "green", "to": DIRECTOR}),
    );
    let down = ask(
        &lead,
        "report",
        json!({"status": "done", "line": "green", "to": WORKER}),
    );
    let nothing_above = ask(
        &director,
        "report",
        json!({"status": "progress", "line": "half way"}),
    );

    for answer in [&past_the_lead, &down, &nothing_above] {
        assert_eq!(answer.exit, 1, "{}", answer.text);
        assert!(
            answer.text.starts_with("NotOneLevelUp: "),
            "{}",
            answer.text
        );
        assert_eq!(lines(answer).len(), 1, "{}", answer.text);
    }
    assert!(store.reports().is_empty(), "nothing was reported");
    let calls = case.recorder.calls();
    assert!(
        calls
            .iter()
            .all(|call| call.outcome == CallOutcome::NotOneLevelUp),
        "{calls:?}"
    );
}

#[test]
fn a_report_of_no_known_status_runs_nothing() {
    let (_case, store, [_, _, worker, _]) = team();

    let answer = ask(
        &worker,
        "report",
        json!({"status": "finished", "line": "green"}),
    );

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert!(answer.text.contains("finished"), "{}", answer.text);
    assert!(store.reports().is_empty());
}
