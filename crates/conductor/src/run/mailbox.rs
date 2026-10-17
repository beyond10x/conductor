//! Conductor's mailbox (docs/design/conductor-run.md § 3, `story:run-daemon`): each tick reads
//! the files under `<state>/mailbox/conductor/` it has not handled, in name order, each a message
//! a controller left with `conductor message send` (`story:mailbox-send-and-deliver`), its first
//! line one of the design § 6 forms. `<state>/run/mailbox.handled` names each file handled, so
//! none is acted on twice; a file that could not be acted on is named on standard error and
//! handled all the same, never retried in a loop.
//!
//! | first line | what the tick does |
//! |---|---|
//! | `[REPORT <repo> <dispatch-id>] started\|blocked\|unblocked\|done\|failed <rest>` | moves the dispatch as `dispatch report-<verb>` does, `<rest>` its reason or evidence (`story:report-to-state`); `-` for the dispatch is the repository's one open dispatch; `blocked handover <path>` is a report like any |
//! | `[HANDOVER <repo>] <rest>` | recorded as a report: the repository's open dispatch is reported blocked, `handover: <rest>` |
//! | `[NEED <repo> <request-id>] <other-repo> <what>` | the whole message delivered to the running controller of `<other-repo>` (`relayed`), else kept for its next start |
//! | `[DECISION-REQUEST <repo> <request-id>] <question>` | raised (`decision raise-decision-request`), then handed to `decide::handle_request` when this build has it ([`super::HandleRequest`]); else escalated as class O, one of the options its body lists (`A) <label>: <cost>`), its `Recommendation:` line's letter recommended and the default, the deadline 48 hours out |
//! | `[RESOURCE <repo>] <kind> <amount>` | answered "use conductor build run" to the repository's running controller (`relayed`), else kept for its next start |
//!
//! Any other first line is handled with nothing done. A message the store recorded with this file
//! as its `body_path`, still `Received`, is then marked handled (`message handle-message`), but for
//! a `[NEED]`, which only `route-need` moves.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{DispatchStorage, MessageStorage};
use conductor_model::dispatch::{DispatchLogEvent, DispatchState, MessageKind, MessageState};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::handlers::deliver_or_keep;
use super::{Tick, one_line, write_file};
use crate::cli::{
    EscalateRequestArgs, HandleMessageArgs, RaiseDecisionRequestArgs, ReportBlockedArgs,
    ReportDoneArgs, ReportFailedArgs, ReportStartedArgs, ReportUnblockedArgs,
};

/// The files handled, one name per line, in `<state>/run/`.
const HANDLED: &str = "mailbox.handled";

/// How long a class-O escalation waits for the operator before its default stands.
const DEADLINE: time::Duration = time::Duration::hours(48);

/// Reads the mailbox, as the module documentation says; counts each file acted on.
pub fn read(tick: &mut Tick<'_, '_>) {
    if let Err(error) = read_all(tick) {
        tick.note("mailbox", &error);
    }
}

fn read_all(tick: &mut Tick<'_, '_>) -> Result<()> {
    let dir = crate::state::dir(tick.state)?
        .join("mailbox")
        .join("conductor");
    let Ok(entries) = fs::read_dir(&dir) else {
        return Ok(());
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".md"))
        .collect();
    names.sort();
    let handled_file = tick.dir.join(HANDLED);
    let handled: BTreeSet<String> = fs::read_to_string(&handled_file)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect();
    let mut now_handled = Vec::new();
    for name in &names {
        if handled.contains(name) {
            now_handled.push(name.clone());
            continue;
        }
        let file = dir.join(name);
        match act(tick, &file) {
            Ok(true) => tick.lines_handled += 1,
            Ok(false) => {}
            Err(error) => tick.note(&format!("mailbox {name}"), &error),
        }
        now_handled.push(name.clone());
        let text: String = now_handled.iter().map(|name| format!("{name}\n")).collect();
        write_file(&handled_file, &text)?;
    }
    Ok(())
}

/// Acts on the message `file`; whether it did.
fn act(tick: &mut Tick<'_, '_>, file: &Path) -> Result<bool> {
    let text = fs::read_to_string(file).with_context(|| format!("read {}", file.display()))?;
    let first = text.lines().next().unwrap_or_default().trim();
    let Some((inside, rest)) = first
        .strip_prefix('[')
        .and_then(|opened| opened.split_once(']'))
    else {
        return Ok(false);
    };
    let fields: Vec<&str> = inside.split_whitespace().collect();
    let rest = rest.trim();
    let acted = match fields.as_slice() {
        ["REPORT", repo, dispatch] => report(tick, repo, dispatch, rest)?,
        ["HANDOVER", repo, ..] => {
            let dispatch = open_dispatch(tick, repo)?;
            report(tick, repo, &dispatch, &format!("blocked handover: {rest}"))?
        }
        ["NEED", _repo, _request] => need(tick, &text, first, rest)?,
        ["DECISION-REQUEST", repo, request] => decision_request(tick, repo, request, rest, &text)?,
        ["RESOURCE", repo] => resource(tick, repo, first)?,
        _ => false,
    };
    mark_handled(tick, file)?;
    Ok(acted)
}

/// `[REPORT <repo> <dispatch>] <verb> <rest>`: the dispatch moved as `dispatch report-<verb>`
/// does; `progress` and `ready` move nothing.
fn report(tick: &mut Tick<'_, '_>, repo: &str, dispatch: &str, rest: &str) -> Result<bool> {
    let dispatch = if dispatch == "-" {
        open_dispatch(tick, repo)?
    } else {
        dispatch.to_owned()
    };
    let (verb, said) = rest.split_once(' ').unwrap_or((rest, ""));
    let verb = verb.trim_end_matches([':', ';', ',']);
    let said = Some(said.trim().to_owned()).filter(|said| !said.is_empty());
    let state = tick.state;
    let id = Some(dispatch.clone());
    let event = match verb {
        "started" => {
            crate::dispatch::report_started(
                state,
                ReportStartedArgs {
                    dispatch_id: id,
                    input_json: None,
                },
            )?;
            DispatchLogEvent::Started
        }
        "blocked" => {
            crate::dispatch::report_blocked(
                state,
                ReportBlockedArgs {
                    dispatch_id: id,
                    reason: Some(said.unwrap_or_else(|| "blocked".to_owned())),
                    input_json: None,
                },
            )?;
            DispatchLogEvent::Blocked
        }
        "unblocked" => {
            crate::dispatch::report_unblocked(
                state,
                ReportUnblockedArgs {
                    dispatch_id: id,
                    input_json: None,
                },
            )?;
            DispatchLogEvent::Unblocked
        }
        "done" => {
            crate::dispatch::report_done(
                state,
                ReportDoneArgs {
                    dispatch_id: id,
                    evidence: Some(said.unwrap_or_else(|| "done".to_owned())),
                    input_json: None,
                },
            )?;
            DispatchLogEvent::Done
        }
        "failed" => {
            crate::dispatch::report_failed(
                state,
                ReportFailedArgs {
                    dispatch_id: id,
                    reason: Some(said.unwrap_or_else(|| "failed".to_owned())),
                    input_json: None,
                },
            )?;
            DispatchLogEvent::Failed
        }
        "progress" | "ready" => return Ok(true),
        other => bail!("{other:?} is no report verb"),
    };
    tick.wrote(event);
    Ok(true)
}

/// The one open dispatch of `repo` a report names with `-`: the one sent, started or blocked, or
/// among several the one started or blocked.
fn open_dispatch(tick: &Tick<'_, '_>, repo: &str) -> Result<String> {
    let store = crate::state::open(tick.state)?;
    let open: Vec<(String, DispatchState)> = DispatchStorage::list(&store)
        .into_iter()
        .filter(|held| held.data.repository.0 == repo)
        .filter(|held| {
            matches!(
                held.state,
                DispatchState::Sent | DispatchState::Started | DispatchState::Blocked
            )
        })
        .map(|held| (held.data.dispatch_id.0, held.state))
        .collect();
    let working: Vec<&String> = open
        .iter()
        .filter(|(_, state)| *state != DispatchState::Sent)
        .map(|(id, _)| id)
        .collect();
    match (open.as_slice(), working.as_slice()) {
        ([(one, _)], _) => Ok(one.clone()),
        (_, [one]) => Ok((*one).clone()),
        ([], _) => bail!("no open dispatch of {repo}"),
        _ => bail!("{} open dispatches of {repo}, none moved", open.len()),
    }
}

/// `[NEED <repo> <request>] <other-repo> <what>`: the whole message to `<other-repo>`'s running
/// controller, else kept for its next start.
fn need(tick: &mut Tick<'_, '_>, text: &str, first: &str, rest: &str) -> Result<bool> {
    let other = rest
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("the need names no repository"))?;
    deliver_or_keep(
        tick,
        json!({"repo": other, "text": text, "event": "relayed", "line": first}),
    )?;
    Ok(true)
}

/// `[RESOURCE <repo>] …`: answered "use conductor build run".
fn resource(tick: &mut Tick<'_, '_>, repo: &str, first: &str) -> Result<bool> {
    let text = format!(
        "[REPORT {repo} -] resource: use conductor build run; it reserves the disk in the host \
         ledger and queues for a build slot (answer to: {first})"
    );
    deliver_or_keep(
        tick,
        json!({"repo": repo, "text": text, "event": "relayed", "line": first}),
    )?;
    Ok(true)
}

/// One lettered option of a request's body: `A) <label>: <cost>`, `A. …`, `A: …` or `- A) …`.
fn option(line: &str) -> Option<Value> {
    let line = line.trim().trim_start_matches(['-', '*']).trim_start();
    let mut chars = line.chars();
    let letter = chars.next().filter(char::is_ascii_uppercase)?;
    let rest = chars.as_str().strip_prefix([')', '.', ':'])?.trim();
    if rest.is_empty() {
        return None;
    }
    let (label, cost) = rest
        .split_once(": ")
        .or_else(|| rest.split_once(" - "))
        .unwrap_or((rest, "not stated"));
    Some(json!({"letter": letter.to_string(), "label": label.trim(), "cost": cost.trim()}))
}

/// `[DECISION-REQUEST <repo> <request>] <question>`: raised, then decided or escalated.
fn decision_request(
    tick: &mut Tick<'_, '_>,
    repo: &str,
    request: &str,
    question: &str,
    text: &str,
) -> Result<bool> {
    let options: Vec<Value> = text.lines().skip(1).filter_map(option).collect();
    let recommendation = text
        .lines()
        .skip(1)
        .map(str::trim)
        .find(|line| line.to_ascii_lowercase().starts_with("recommendation"))
        .map(|line| {
            line.split_once(':')
                .map_or(line, |(_, said)| said)
                .trim()
                .to_owned()
        });
    let letters: Vec<&str> = options
        .iter()
        .filter_map(|option| option["letter"].as_str())
        .collect();
    let recommended = recommendation
        .as_deref()
        .and_then(|said| {
            said.split(|c: char| !c.is_ascii_alphanumeric())
                .find(|word| letters.contains(word))
        })
        .or_else(|| letters.first().copied())
        .map(str::to_owned);
    let now = OffsetDateTime::now_utc();
    crate::decide::raise_decision_request(
        tick.state,
        RaiseDecisionRequestArgs {
            request_id: Some(request.to_owned()),
            repository: Some(repo.to_owned()),
            question: Some(question.to_owned()),
            options: Some(if options.is_empty() {
                "none listed".to_owned()
            } else {
                options
                    .iter()
                    .map(|option| {
                        format!(
                            "{}) {}",
                            option["letter"].as_str().unwrap_or_default(),
                            option["label"].as_str().unwrap_or_default()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            }),
            recommendation: Some(recommendation.unwrap_or_else(|| "none given".to_owned())),
            raised_at: Some(now.format(&Rfc3339).context("format the clock")?),
            blocker: None,
            classification_rule: None,
            input_json: None,
        },
    )?;
    if let Some(handle) = tick.seams.handle_request {
        handle(tick.state, request)?;
        return Ok(true);
    }
    let Some(recommended) = recommended else {
        bail!("request {request} lists no lettered option, so it is raised and not escalated");
    };
    crate::decide::escalate_request(
        tick.state,
        EscalateRequestArgs {
            request_id: Some(request.to_owned()),
            escalation_class: Some("O".to_owned()),
            reason: Some(
                "conductor run: this build has no decider; the request's own options and \
                 recommendation"
                    .to_owned(),
            ),
            dry_run: None,
            no_dry_run: None,
            kind: Some("one-of".to_owned()),
            lettered_options: Some(Value::Array(options).to_string()),
            recommended_letters: Some(recommended.clone()),
            default_letters: Some(recommended),
            deadline: Some(
                (now + DEADLINE)
                    .format(&Rfc3339)
                    .context("format the deadline")?,
            ),
            input_json: None,
        },
    )?;
    Ok(true)
}

/// Marks handled the message the store recorded with `file` as its body, while it is
/// `Received` and no `[NEED]`.
fn mark_handled(tick: &Tick<'_, '_>, file: &Path) -> Result<()> {
    let store = crate::state::open(tick.state)?;
    let path = file.display().to_string();
    let message = MessageStorage::list(&store).into_iter().find(|held| {
        held.data.body_path.as_deref() == Some(path.as_str())
            && held.state == MessageState::Received
            && held.data.kind != MessageKind::Need
    });
    drop(store);
    if let Some(message) = message {
        crate::message::handle_message(
            tick.state,
            HandleMessageArgs {
                message_id: Some(message.data.message_id.0.0),
                input_json: None,
            },
        )
        .with_context(|| format!("mark the message of {} handled", one_line(&path)))?;
    }
    Ok(())
}
