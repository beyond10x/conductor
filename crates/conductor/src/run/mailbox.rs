//! Conductor's mailbox (docs/design/conductor-run.md § 3, `story:run-daemon`): each tick reads
//! the files under `<state>/mailbox/conductor/` it has not handled, in name order, each a message
//! a controller left with `conductor message send` (`story:mailbox-send-and-deliver`), its first
//! line one of the design § 6 forms. `<state>/run/mailbox.handled` names each file handled, so
//! none is acted on twice; a file that could not be acted on is named on standard error and
//! handled all the same, never retried in a loop. Its sender is told: the controller of the
//! repository its first line names gets one message, `[REPORT <repo> -] refused: <reason>`, with
//! the message refused and the form its kind expects (`relayed`), else it is kept for its next
//! start (`story:report-on-sent-dispatch`).
//!
//! | first line | what the tick does |
//! |---|---|
//! | `[REPORT <repo> <dispatch-id>] started\|blocked\|unblocked\|done\|failed <rest>` | moves the dispatch as `dispatch report-<verb>` does, `<rest>` its reason or evidence (`story:report-to-state`); `-` for the dispatch is the repository's one open dispatch, and with none open the report moves nothing and is not refused; `blocked handover <path>` is a report like any. A `blocked`, `done` or `failed` on a dispatch still `Sent` is reported `started` first, so the log holds both lines; a `started` on a `Blocked` dispatch is recorded `unblocked`, as a controller restarted after a hand-over reports |
//! | `[HANDOVER <repo>] <rest>` | recorded as a report: the repository's open dispatch is reported blocked, `handover: <rest>` |
//! | `[NEED <repo> <request-id>] <other-repo> <what>` | the whole message delivered to the running controller of `<other-repo>` (`relayed`), else kept for its next start |
//! | `[DECISION-REQUEST <repo> <request-id>] <question>` | handed whole to `decide::handle_request` ([`super::handle_request`]), which raises it with the rule that classified it, answers and records class C and escalates class O and H; a decided request's `[DECISION]` is delivered to the controller that raised it (`decision-sent`), else kept for its next start. With no decider seam ([`super::HandleRequest`]), raised (`decision raise-decision-request`) and escalated as class O, one of the options its body lists (`A) <label>: <cost>`), its `Recommendation:` line's letter recommended and the default, the deadline 48 hours out |
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
use conductor_model::dispatch::{
    DispatchId, DispatchLogEvent, DispatchState, MessageKind, MessageState,
};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::handlers::deliver_or_keep;
use super::{Tick, one_line, write_file};
use crate::cli::{
    EscalateRequestArgs, HandleMessageArgs, RaiseDecisionRequestArgs, ReportBlockedArgs,
    ReportDoneArgs, ReportFailedArgs, ReportStartedArgs, ReportUnblockedArgs,
};
use crate::dispatch::Open;

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
            Err(error) => {
                tick.note(&format!("mailbox {name}"), &error);
                if let Err(error) = answer_refused(tick, &file, &error) {
                    tick.note(&format!("mailbox {name}: answer the sender"), &error);
                }
            }
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

/// The form of each first line a controller sends, by its kind, which a refusal names.
const FORMS: [(&str, &str); 5] = [
    (
        "REPORT",
        "[REPORT <repo> <dispatch-id|->] <started|progress|blocked|unblocked|done|failed> <one line>",
    ),
    ("HANDOVER", "[HANDOVER <repo>] <path of the hand-over>"),
    ("NEED", "[NEED <repo> <request-id>] <other-repo> <what>"),
    (
        "DECISION-REQUEST",
        "[DECISION-REQUEST <repo> <request-id>] <question>, then lettered options `A) <label>: \
         <cost>` and a `Recommendation:` line",
    ),
    (
        "RESOURCE",
        "[RESOURCE <repo>] <build-slot|disk|usage> <amount>",
    ),
];

/// Answers the controller of the repository the message `file` names that the loop refused it,
/// with `error` and the form its kind expects, as one message (`relayed`), or keeps the answer
/// for its next start. A first line that names no repository is answered by no one.
fn answer_refused(tick: &mut Tick<'_, '_>, file: &Path, error: &anyhow::Error) -> Result<()> {
    let text = fs::read_to_string(file).with_context(|| format!("read {}", file.display()))?;
    let first = text.lines().next().unwrap_or_default().trim();
    let Some(inside) = first
        .strip_prefix('[')
        .and_then(|opened| opened.split_once(']'))
        .map(|(inside, _)| inside)
    else {
        return Ok(());
    };
    let fields: Vec<&str> = inside.split_whitespace().collect();
    let (Some(kind), Some(repo)) = (fields.first(), fields.get(1)) else {
        return Ok(());
    };
    let Some((_, form)) = FORMS.iter().find(|(named, _)| named == kind) else {
        return Ok(());
    };
    let reason = one_line(&format!("{error:#}"));
    let form = form.replace("<repo>", repo);
    let mut answer = format!(
        "[REPORT {repo} -] refused: {reason}\n\
         Your message: {first}\n\
         The form: {form}\n"
    );
    // A report names a dispatch: the answer names the one it could have named
    // (`story:spawn-without-dispatch-reports`).
    if matches!(*kind, "REPORT" | "HANDOVER") {
        let open = match crate::dispatch::open_of(tick.state, repo)? {
            Open::One(id) => format!("The open dispatch of {repo}: {id}"),
            Open::None => {
                format!("{repo} has no open dispatch: report with `-`, which then moves nothing")
            }
            Open::Several(ids) => format!(
                "{repo} has {} open dispatches, name one: {}",
                ids.len(),
                ids.join(", ")
            ),
        };
        answer.push_str(&open);
        answer.push('\n');
    }
    deliver_or_keep(
        tick,
        json!({"repo": repo, "text": answer, "event": "relayed", "line": first}),
    )
}

/// The one open dispatch of `repo` ([`crate::dispatch::open_of`]), or why there is none.
fn open_dispatch(tick: &Tick<'_, '_>, repo: &str) -> Result<String> {
    match crate::dispatch::open_of(tick.state, repo)? {
        Open::One(id) => Ok(id),
        Open::None => bail!("no open dispatch of {repo}"),
        Open::Several(ids) => bail!(
            "{} open dispatches of {repo} ({}), none moved",
            ids.len(),
            ids.join(", ")
        ),
    }
}

/// `[REPORT <repo> <dispatch>] <verb> <rest>`: the dispatch moved as `dispatch report-<verb>`
/// does; `progress` and `ready` move nothing. A `blocked`, `done` or `failed` on a dispatch still
/// `Sent` reports it `started` first, and a `started` on a `Blocked` one reports it `unblocked`
/// (`story:report-on-sent-dispatch`). With `-` for the dispatch
/// and no dispatch of the repository open, the report moves nothing and is not refused: a
/// controller started with no dispatch reports so (`story:spawn-without-dispatch-reports`).
fn report(tick: &mut Tick<'_, '_>, repo: &str, dispatch: &str, rest: &str) -> Result<bool> {
    let dispatch = if dispatch == "-" {
        if crate::dispatch::open_of(tick.state, repo)? == Open::None {
            return Ok(true);
        }
        open_dispatch(tick, repo)?
    } else {
        dispatch.to_owned()
    };
    let (verb, said) = rest.split_once(' ').unwrap_or((rest, ""));
    let verb = verb.trim_end_matches([':', ';', ',']);
    let said = Some(said.trim().to_owned()).filter(|said| !said.is_empty());
    let state = tick.state;
    let id = Some(dispatch.clone());
    let held = state_of(tick, &dispatch)?;
    if matches!(verb, "blocked" | "done" | "failed") && held == Some(DispatchState::Sent) {
        crate::dispatch::report_started(
            state,
            ReportStartedArgs {
                actor: None,
                dispatch_id: id.clone(),
                input_json: None,
            },
        )?;
        tick.wrote(DispatchLogEvent::Started);
    }
    let event = match verb {
        // A controller restarted after a hand-over reports `started` on its blocked dispatch: it
        // works it again.
        "started" if held == Some(DispatchState::Blocked) => {
            crate::dispatch::report_unblocked(
                state,
                ReportUnblockedArgs {
                    actor: None,
                    dispatch_id: id,
                    input_json: None,
                },
            )?;
            DispatchLogEvent::Unblocked
        }
        "started" => {
            crate::dispatch::report_started(
                state,
                ReportStartedArgs {
                    actor: None,
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
                    actor: None,
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
                    actor: None,
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
                    actor: None,
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
                    actor: None,
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

/// The state the store holds the dispatch `id` in; `None` when it holds no such dispatch.
fn state_of(tick: &Tick<'_, '_>, id: &str) -> Result<Option<DispatchState>> {
    let store = crate::state::open(tick.state)?;
    let held = DispatchStorage::get(&store, &DispatchId(id.to_owned()));
    store.check().context("read the dispatch")?;
    Ok(held.map(|held| held.state))
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

/// One lettered option of a request's body: `A) <label>: <cost>`, `A. …`, `A: …` or `- A) …`;
/// a cost the text names (`. Cost: <cost>`, `— cost <cost>`) is read as `decide` reads it.
fn option(line: &str) -> Option<Value> {
    let line = line.trim().trim_start_matches(['-', '*']).trim_start();
    let mut chars = line.chars();
    let letter = chars.next().filter(char::is_ascii_uppercase)?;
    let rest = chars.as_str().strip_prefix([')', '.', ':'])?.trim();
    if rest.is_empty() {
        return None;
    }
    let stated = crate::decide::stated_cost(rest);
    let (label, cost) = stated
        .as_ref()
        .map(|(label, cost)| (label.as_str(), cost.as_str()))
        .or_else(|| rest.split_once(": "))
        .or_else(|| rest.split_once(" - "))
        .unwrap_or((rest, ""));
    let cost = if cost.trim().is_empty() {
        "not stated"
    } else {
        cost
    };
    Some(json!({"letter": letter.to_string(), "label": label.trim(), "cost": cost.trim()}))
}

/// `[DECISION-REQUEST <repo> <request>] <question>`: handed whole to the decider seam, and its
/// `[DECISION]` delivered when it was decided; with no seam, raised and escalated as class O.
fn decision_request(
    tick: &mut Tick<'_, '_>,
    repo: &str,
    request: &str,
    question: &str,
    text: &str,
) -> Result<bool> {
    if let Some(handle) = tick.seams.handle_request {
        if handle(tick.state, text)? {
            super::handlers::answered(tick, request)?;
        }
        return Ok(true);
    }
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
            actor: None,
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
    let Some(recommended) = recommended else {
        bail!("request {request} lists no lettered option, so it is raised and not escalated");
    };
    crate::decide::escalate_request(
        tick.state,
        EscalateRequestArgs {
            actor: None,
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
                actor: None,
                message_id: Some(message.data.message_id.0.0),
                input_json: None,
            },
        )
        .with_context(|| format!("mark the message of {} handled", one_line(&path)))?;
    }
    Ok(())
}
