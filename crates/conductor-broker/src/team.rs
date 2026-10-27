//! The team verbs (`story:tool-build-commit-report`, design § 2, § 6.3).
//!
//! - `send <role-or-name> <text>` goes to another session of the caller's team, named by its
//!   session name or, when no session has that name, by its role. A recipient registered under
//!   another instance is refused `ForeignInstance`; any other recipient outside the team, and a
//!   caller with no team, `OutsideTeam`.
//! - `report <status> <line> [--to <session>]` goes to the session one level above the caller;
//!   `--to` naming any other, or a caller with nothing above it, is refused `NotOneLevelUp`. A
//!   lead's report asks the store to move its team's open dispatch.
//!
//! The broker records each through its [`Messages`](crate::Messages), stamped with its clock.
//! It cannot drive the harness's own channel (SendMessage), so the answer names the recorded
//! message and whom the caller delivers it to, on one line.

use serde_json::Value;

use crate::grant::Refusal;
use crate::registry::{self, well_formed};
use crate::server::rfc3339;
use crate::services::{Report, TeamMessage};
use crate::verbs::{Context, failure};
use crate::wire::Answer;
use crate::{CallOutcome, Level, STATUSES, status_named, status_word};

/// The note every team answer ends in: the broker records, the caller delivers.
const DELIVER: &str = "the broker cannot reach the harness: deliver it with SendMessage to";

fn outside(rule: String) -> Refusal {
    Refusal {
        outcome: CallOutcome::OutsideTeam,
        rule,
    }
}

fn not_one_level_up(rule: String) -> Refusal {
    Refusal {
        outcome: CallOutcome::NotOneLevelUp,
        rule,
    }
}

pub(crate) fn send(context: &Context<'_>, args: &Value) -> Result<Answer, Refusal> {
    let to = args["to"].as_str().unwrap_or_default().trim();
    let text = args["text"].as_str().unwrap_or_default();
    if to.is_empty() || text.trim().is_empty() {
        return Ok(failure(
            context.dir,
            "send: the call names no recipient or no text: write send <role-or-name> <text>",
            "",
        ));
    }
    let caller = context.session;
    let Some(team) = context.registration.place.team.as_deref() else {
        return Err(outside(format!("team: session {caller} is in no team")));
    };
    let recipient = recipient(context, team, to)?;
    let message = TeamMessage {
        sender: caller.to_owned(),
        recipient: recipient.clone(),
        team: team.to_owned(),
        text: text.to_owned(),
        at: rfc3339(context.now),
    };
    Ok(match context.services.messages.send(&message) {
        Ok(id) => Answer::done(format!(
            "sent {id} to {recipient} of team {team}; {DELIVER} {recipient}\n"
        )),
        Err(why) => failure(
            context.dir,
            &format!("send: the message to {recipient} was not recorded: {why}"),
            "",
        ),
    })
}

/// The session of `team` that `to` names: the session of that name, else the one other session
/// of the team, in this broker's instance, whose role it is.
fn recipient(context: &Context<'_>, team: &str, to: &str) -> Result<String, Refusal> {
    let caller = context.session;
    if well_formed(to)
        && let Some(named) = registry::read(context.dir, to)
    {
        let named = named.map_err(|why| outside(format!("team: {to} cannot be read: {why}")))?;
        if named.instance != context.instance {
            return Err(Refusal {
                outcome: CallOutcome::ForeignInstance,
                rule: format!(
                    "session {to} belongs to instance {}; this broker serves {}",
                    named.instance, context.instance
                ),
            });
        }
        if to == caller {
            return Err(outside(format!("team: {caller} does not send to itself")));
        }
        if named.place.team.as_deref() != Some(team) {
            return Err(outside(format!("team: {to} is not in team {team}")));
        }
        return Ok(to.to_owned());
    }
    let holders: Vec<String> = registry::sessions(context.dir)
        .into_iter()
        .filter(|name| name != caller)
        .filter_map(|name| registry::read(context.dir, &name)?.ok())
        .filter(|other| {
            other.instance == context.instance
                && other.place.team.as_deref() == Some(team)
                && other.place.role.as_deref() == Some(to)
        })
        .map(|other| other.session)
        .collect();
    match holders.as_slice() {
        [] => Err(outside(format!(
            "team: no other session of team {team} is named {to} or has that role"
        ))),
        [one] => Ok(one.clone()),
        many => Err(outside(format!(
            "team: the role {to} names {} sessions of team {team} ({}); name one",
            many.len(),
            many.join(", ")
        ))),
    }
}

pub(crate) fn report(context: &Context<'_>, args: &Value) -> Result<Answer, Refusal> {
    let word = args["status"].as_str().unwrap_or_default();
    let Some(status) = status_named(word) else {
        let known: Vec<&str> = STATUSES.into_iter().map(status_word).collect();
        return Ok(failure(
            context.dir,
            &format!(
                "report: {word:?} is no status: write one of {}",
                known.join(", ")
            ),
            "",
        ));
    };
    let line = args["line"].as_str().unwrap_or_default();
    if line.trim().is_empty() {
        return Ok(failure(
            context.dir,
            "report: the call names no line: write report <status> <line>",
            "",
        ));
    }
    let caller = context.session;
    let place = &context.registration.place;
    let Some(above) = place.above.as_deref() else {
        return Err(not_one_level_up(format!(
            "level: nothing is one level above session {caller}"
        )));
    };
    if let Some(to) = args["to"]
        .as_str()
        .map(str::trim)
        .filter(|to| !to.is_empty())
        && to != above
    {
        return Err(not_one_level_up(format!(
            "level: {to} is not one level above {caller}; {above} is"
        )));
    }
    let report = Report {
        sender: caller.to_owned(),
        recipient: above.to_owned(),
        team: place.team.clone(),
        lead: place.level == Some(Level::Lead),
        status,
        line: line.to_owned(),
        at: rfc3339(context.now),
    };
    Ok(match context.services.messages.report(&report) {
        Ok(reported) => {
            let mut said = format!(
                "reported {} {} to {above}",
                reported.message,
                status_word(status)
            );
            if let Some(dispatch) = &reported.dispatch {
                said.push_str(&format!(", dispatch {dispatch} moved"));
            }
            if let Some(note) = &reported.note {
                said.push_str(&format!(" ({note})"));
            }
            Answer::done(format!("{said}; {DELIVER} {above}\n"))
        }
        Err(why) => failure(
            context.dir,
            &format!("report: the report to {above} was not recorded: {why}"),
            "",
        ),
    })
}
