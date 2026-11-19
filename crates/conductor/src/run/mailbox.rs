//! Conductor's mailbox (docs/design/conductor-run.md § 3, `story:run-daemon`): each tick reads
//! the files under `<state>/mailbox/conductor/` it has not handled, in name order, each a message
//! a controller left with `conductor message send` (`story:mailbox-send-and-deliver`), its first
//! line one of the design § 6 forms. `<state>/run/mailbox.handled` names each file handled, so
//! none is acted on twice; a file that could not be acted on is named on standard error and
//! handled all the same, never retried in a loop. Its sender is told: the controller of the
//! repository its first line names gets one message, `[REPORT <repo> -] refused: <reason>`, with
//! the message refused and the form its kind expects (`relayed`), else it is kept for its next
//! start (`story:report-on-sent-dispatch`). A first line whose own tag names no repository the
//! instance knows is answered to no one, since no controller would take the answer: the line on
//! standard error, in the loop's journal, carries the reason.
//!
//! | first line | what the tick does |
//! |---|---|
//! | `[REPORT <repo> <dispatch-id>] started\|blocked\|unblocked\|done\|failed <rest>` | moves the dispatch as `dispatch report-<verb>` does, `<rest>` its reason or evidence (`story:report-to-state`); `-` for the dispatch is the repository's one open dispatch, and with none open the report moves nothing and is not refused; `blocked handover <path>` is a report like any. A `blocked`, `done` or `failed` on a dispatch still `Sent` is reported `started` first, so the log holds both lines; a `started` on a `Blocked` dispatch is recorded `unblocked`, as a controller restarted after a hand-over reports; a `started` on a `Started` dispatch or a `blocked` on a `Blocked` one, as a controller resumed after a runtime restart repeats, moves nothing and is not refused |
//! | `[REPORT <repo> <dispatch-id>] done <evidence>`, held | a `done` is held as a claim, not taken (issue #202), when the repository's controller session was resumed from its transcript less than [`crate::spawn::RESUME_HOLD`] before ([`crate::spawn::resumed_at`]), or on a `Blocked` dispatch, which no `unblocked` followed: the dispatch does not move, a `done-held` line names the evidence and why, and the controller is answered with its dispatch (id, goal, brief) and what takes a `done` then. The operator, or the loop's decider as conductor, confirms it with `dispatch report-done` ([`hold`]) |
//! | `[REPORT <repo> <dispatch-id>] withdrawn <why>` | the controller withdraws its own `done`: a `Done` dispatch whose `done` this mailbox took after the last cycle started ([`TAKEN`]) is reopened by the loop (`dispatch reopen-dispatch`, `reopened`); an older one is refused, the answer naming `dispatch reopen-dispatch`, the operator's; on an open dispatch it moves nothing ([`withdrawn`]) |
//! | `[REPORT <repo> <dispatch-id\|->] review <url> [<words>]` | the controller's pull request ([`review`]): a `Sent` dispatch is reported `started`, another moves nothing, as `progress`; the URL, the first word after the verb, is then recorded on the dispatch as `dispatch record-link` records it. A first word that is no link a dispatch takes, or none, is a `progress` report with nothing linked, and a dispatch that holds the most links takes the report without it: neither is refused, and the journal names it in one line. No other verb records a link |
//! | `[HANDOVER <repo>] <rest>` | recorded as a report: the repository's open dispatch is reported blocked, `handover: <rest>` |
//! | `[NEED <repo> <request-id>] <other-repo> <what>` | the whole message delivered to the running controller of `<other-repo>` (`relayed`), else kept for its next start; the repository is the field's leading run of name characters (`docs:` names `docs`). A repository other than `<repo>` with no charter, no running controller and no open dispatch gets a unit of work instead ([`unit`], `story:need-starts-unit-of-work`): one dispatch, `need-<repo>-<request-id>` its reference and the whole message its brief, which the message is routed to (`message route-need`) and the cycle starts; the same need again answers that dispatch, and `<repo>`'s controller is told `[REPORT <repo> -] need <request-id> opened <dispatch-id> for <other-repo>`. A field that names no repository the instance knows is not relayed and nothing is kept for it: the file stays in the mailbox, where the supervisor reads it, the sender is told so in one message, `[REPORT <repo> -] not relayed: <reason>`, and one alert `need-unroutable` is raised ([`crate::alert`]); one that names the mailbox's own recipient, `conductor`, is the supervisor's and is answered by no one (`story:need-repository-from-tag`) |
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
    DispatchId, DispatchLogEvent, DispatchState, MessageKind, MessageState, WorkRequest,
};
use conductor_model::observation::RepositoryName;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::handlers::deliver_or_keep;
use super::{Tick, one_line, write_file};
use crate::cli::{
    EscalateRequestArgs, HandleMessageArgs, RaiseDecisionRequestArgs, ReportBlockedArgs,
    ReportDoneArgs, ReportFailedArgs, ReportStartedArgs, ReportUnblockedArgs, RouteNeedArgs,
};
use crate::dispatch::{Linking, Open, Reopening, Sent};

/// The files handled, one name per line, in `<state>/run/`.
const HANDLED: &str = "mailbox.handled";

/// The dispatches whose `done` this mailbox took, each with the instant it took it (a JSON object
/// of RFC 3339 instants by dispatch id), in `<state>/run/`: a controller withdraws such a `done`
/// until the next cycle starts ([`withdrawn`]). An entry no later than the last cycle's start is
/// dropped when the file is written again.
const TAKEN: &str = "done.taken";

/// The recipient whose mailbox the tick reads, `<state>/mailbox/conductor/`: what a controller
/// names with `conductor message send --to`.
const RECIPIENT: &str = "conductor";

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
        .join(RECIPIENT);
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
    // Issue #219: every name handled before is kept whole each time the file is written, not
    // only those that sort before the file handled now, so none is acted on again after a
    // restart or an upgrade.
    let mut now_handled: BTreeSet<String> = names
        .iter()
        .filter(|name| handled.contains(*name))
        .cloned()
        .collect();
    for name in &names {
        if handled.contains(name) {
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
        now_handled.insert(name.clone());
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
    // The dispatch a `[REPORT]` was handled for, which its message then holds
    // (story:control-plane-work-handler): the work call reads a dispatch's last report from it.
    let mut reported = None;
    let acted = match fields.as_slice() {
        ["REPORT", repo, dispatch] => {
            let (acted, moved) = report(tick, repo, dispatch, rest)?;
            reported = moved;
            acted
        }
        ["HANDOVER", repo, ..] => {
            let dispatch = open_dispatch(tick, repo)?;
            report(tick, repo, &dispatch, &format!("blocked handover: {rest}"))?.0
        }
        ["NEED", repo, request] => need(tick, file, &text, first, rest, repo, request)?,
        ["DECISION-REQUEST", repo, request] => decision_request(tick, repo, request, rest, &text)?,
        ["RESOURCE", repo] => resource(tick, repo, first)?,
        _ => false,
    };
    mark_handled(tick, file, reported)?;
    Ok(acted)
}

/// The form of each first line a controller sends, by its kind, which a refusal names.
const FORMS: [(&str, &str); 5] = [
    (
        "REPORT",
        "[REPORT <repo> <dispatch-id|->] <started|progress|blocked|unblocked|done|failed> <one line>, \
         or [REPORT <repo> <dispatch-id|->] review <url> [<words>], or [REPORT <repo> \
         <dispatch-id>] withdrawn <why>",
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
/// for its next start. A `[NEED]` the loop did not relay ([`NotRelayed`]) is answered `not
/// relayed`, not `refused`: it stays in the mailbox. A first line that names no repository, or
/// one the instance does not know ([`known`]), is answered by no one: no controller would ever
/// take the answer.
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
    if !known(tick, repo)? {
        return Ok(());
    }
    let reason = one_line(&format!("{error:#}"));
    let form = form.replace("<repo>", repo);
    let said = if error.is::<NotRelayed>() {
        "not relayed"
    } else {
        "refused"
    };
    let mut answer = format!(
        "[REPORT {repo} -] {said}: {reason}\n\
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
/// does; `progress` and `ready` move nothing, and `review` links its URL ([`review`]). A
/// `blocked`, `done` or `failed` on a dispatch still `Sent` reports it `started` first, and a
/// `started` on a `Blocked` one reports it `unblocked` (`story:report-on-sent-dispatch`). With `-` for the dispatch
/// and no dispatch of the repository open, the report moves nothing and is not refused: a
/// controller started with no dispatch reports so (`story:spawn-without-dispatch-reports`).
/// Answers whether it acted, and the dispatch the report was for: its own id, or the open one `-`
/// named; none when `-` named none, or the id names no dispatch the store holds.
///
/// An id that names a dispatch of another repository than `repo` is no report of that dispatch
/// (story:control-plane-work-handler): the report is handled as one of `repo` naming no
/// dispatch, so it moves nothing, writes no log line, and the message holds no dispatch.
fn report(
    tick: &mut Tick<'_, '_>,
    repo: &str,
    dispatch: &str,
    rest: &str,
) -> Result<(bool, Option<String>)> {
    let dispatch = if dispatch == "-" {
        if crate::dispatch::open_of(tick.state, repo)? == Open::None {
            return Ok((true, None));
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
    let held = held_of(tick, &dispatch)?;
    if held.as_ref().is_some_and(|(_, of)| of != repo) {
        return Ok((true, None));
    }
    let reported = held.is_some().then(|| dispatch.clone());
    let held = held.map(|(state, _)| state);
    if verb == "review" {
        review(tick, &dispatch, held, said.as_deref())?;
        return Ok((true, reported));
    }
    if verb == "withdrawn" {
        withdrawn(tick, &dispatch, held, said.as_deref())?;
        return Ok((true, reported));
    }
    if verb == "done"
        && let Some(why) = held_why(tick, repo, held)
    {
        hold(tick, repo, &dispatch, &why, said.as_deref())?;
        return Ok((true, reported));
    }
    // A controller resumed after a runtime restart repeats the report it made before
    // (story:started-report-after-restart): the dispatch is where the report would move it, so
    // the report is progress, not a refusal.
    if repeated(verb, held) {
        return Ok((true, reported));
    }
    if matches!(verb, "blocked" | "done" | "failed") && held == Some(DispatchState::Sent) {
        let _ = crate::dispatch::report_started(
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
            let _ = crate::dispatch::report_unblocked(
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
            let _ = crate::dispatch::report_started(
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
            let _ = crate::dispatch::report_blocked(
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
            let _ = crate::dispatch::report_unblocked(
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
            let _ = crate::dispatch::report_done(
                state,
                ReportDoneArgs {
                    actor: None,
                    dispatch_id: id,
                    evidence: Some(said.unwrap_or_else(|| "done".to_owned())),
                    input_json: None,
                },
            )?;
            take(tick, &dispatch);
            DispatchLogEvent::Done
        }
        "failed" => {
            let _ = crate::dispatch::report_failed(
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
        "progress" | "ready" => return Ok((true, reported)),
        other => bail!("{other:?} is no report verb"),
    };
    tick.wrote(event);
    Ok((true, reported))
}

/// Why a controller's `done` on a dispatch of `repo` held in `held` is held as a claim rather
/// than taken (issue #202), if it is: the repository's controller session was resumed from its
/// transcript less than [`crate::spawn::RESUME_HOLD`] before (`resumed <at>`), or the dispatch is
/// `Blocked` with no `unblocked` since (`blocked`). A dispatch that is not open, or that the store
/// does not hold, is never held: the report meets its refusal as before.
fn held_why(tick: &Tick<'_, '_>, repo: &str, held: Option<DispatchState>) -> Option<String> {
    if !matches!(
        held,
        Some(DispatchState::Sent | DispatchState::Started | DispatchState::Blocked)
    ) {
        return None;
    }
    let session = crate::config::session_name(tick.instance(), repo);
    let now = OffsetDateTime::now_utc();
    if let Some(at) = crate::spawn::resumed_at(tick.state, &session)
        && now - at < crate::spawn::RESUME_HOLD
    {
        return Some(format!(
            "resumed {}",
            at.format(&Rfc3339).unwrap_or_default()
        ));
    }
    (held == Some(DispatchState::Blocked)).then(|| "blocked".to_owned())
}

/// Holds a controller's `done` on the open dispatch `id` of `repo` for `why` ([`held_why`]):
/// nothing moves, a `done-held` line keeps the evidence `said` and the reason, and the controller
/// is answered (`relayed`, or kept for its next start) with its dispatch, its goal and brief, and
/// what takes a `done` then.
///
/// # Errors
///
/// The store does not open, or the line or the answer cannot be written.
fn hold(
    tick: &mut Tick<'_, '_>,
    repo: &str,
    id: &str,
    why: &str,
    said: Option<&str>,
) -> Result<()> {
    let evidence = said.unwrap_or("done").to_owned();
    let mut line = tick.line(DispatchLogEvent::DoneHeld, repo)?;
    line.id = Some(id.to_owned());
    line.evidence = Some(evidence.clone());
    line.reason = Some(why.to_owned());
    tick.write(&line)?;
    let store = crate::state::open(tick.state)?;
    let held = DispatchStorage::get(&store, &DispatchId(id.to_owned()));
    store.check().context("read the dispatch")?;
    let (goal, brief) = held.map_or_else(
        || ("-".to_owned(), String::new()),
        |held| {
            (
                held.data
                    .goal_id
                    .map_or_else(|| "-".to_owned(), |goal| goal.0),
                held.data.brief,
            )
        },
    );
    let then = match why.strip_prefix("resumed ") {
        Some(at) => format!(
            "your session was resumed from its transcript at {at}, less than {} minutes ago, and \
             may be reporting an older dispatch's work. Check your evidence against the brief \
             below, and report `done` again once that time has passed",
            crate::spawn::RESUME_HOLD.whole_minutes()
        ),
        None => "the dispatch is blocked and you reported no `unblocked`. Report `unblocked` \
                 first, then `done` with the evidence of the brief below"
            .to_owned(),
    };
    let first = format!("[REPORT {repo} {id}] done {evidence}");
    let (fence, block) = crate::spawn::fenced(&brief);
    let answer = format!(
        "[REPORT {repo} {id}] done held: {why}\n\
         Your done on {id} is held as a claim, not taken: {then}; or the operator confirms it \
         with `conductor dispatch report-done --dispatch-id {id} --evidence <evidence>`.\n\
         Your message: {}\n\
         Your dispatch: {id}, goal {goal}. Its brief, verbatim, between the two lines of {fence} \
         backticks:\n\n{block}",
        one_line(&first)
    );
    deliver_or_keep(
        tick,
        json!({"repo": repo, "text": answer, "event": "relayed", "line": first}),
    )
}

/// Keeps that this mailbox took a controller's `done` on the dispatch `id` now ([`TAKEN`]), and
/// drops the entries no later than the last cycle's start. A file that cannot be written is named
/// on standard error: the `done` stands, and only its withdrawal is lost.
fn take(tick: &Tick<'_, '_>, id: &str) {
    let file = tick.dir.join(TAKEN);
    let since = cycle_started(&tick.dir);
    let mut taken: serde_json::Map<String, Value> = fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    taken.retain(|_, at| instant(at).is_some_and(|at| since.is_none_or(|since| at > since)));
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_default();
    taken.insert(id.to_owned(), Value::from(now));
    if let Err(error) = write_file(&file, &format!("{}\n", Value::Object(taken))) {
        tick.note(TAKEN, &error);
    }
}

/// The RFC 3339 instant `value` holds.
fn instant(value: &Value) -> Option<OffsetDateTime> {
    value
        .as_str()
        .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok())
}

/// The start of the last cycle, the one running or the last one that ended, whichever is later
/// (`cycle.running`, `cycle.last` in `dir`); `None` before any.
fn cycle_started(dir: &Path) -> Option<OffsetDateTime> {
    [super::CYCLE_RUNNING, super::CYCLE_LAST]
        .iter()
        .filter_map(|name| fs::read_to_string(dir.join(name)).ok())
        .filter_map(|text| OffsetDateTime::parse(text.trim(), &Rfc3339).ok())
        .max()
}

/// `withdrawn <why>` on the dispatch `id`, held in `held`: the controller withdraws its own
/// `done` (issue #202). A `Done` dispatch whose `done` this mailbox took after the last cycle
/// started ([`TAKEN`]) is reopened as the loop (`dispatch reopen-dispatch`, `reopened`, the
/// reason `withdrawn by the controller: <why>`); an open one moves nothing, as its `done` was
/// held or never sent.
///
/// # Errors
///
/// No dispatch has the id; the dispatch ended otherwise, or its `done` was taken before the last
/// cycle started, so only the operator reopens it, which the answer names; or the reopen failed.
fn withdrawn(
    tick: &mut Tick<'_, '_>,
    id: &str,
    held: Option<DispatchState>,
    said: Option<&str>,
) -> Result<()> {
    match held {
        None => bail!("DispatchNotFound: no dispatch has the id {id}"),
        Some(DispatchState::Sent | DispatchState::Started | DispatchState::Blocked) => {
            return Ok(());
        }
        Some(DispatchState::Done) => {}
        Some(_) => bail!(
            "DispatchStateConflict: dispatch {id} did not move: only a done dispatch is reopened"
        ),
    }
    let taken: Value = fs::read_to_string(tick.dir.join(TAKEN))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let since = cycle_started(&tick.dir);
    let open = instant(&taken[id]).is_some_and(|at| since.is_none_or(|since| at > since));
    if !open {
        bail!(
            "the done of {id} was not taken from you since the last cycle started, so you cannot \
             withdraw it; the operator reopens it with `conductor dispatch reopen-dispatch \
             --dispatch-id {id} --reason <why>`"
        );
    }
    let reason = format!(
        "withdrawn by the controller: {}",
        said.map_or_else(|| "no reason given".to_owned(), one_line)
    );
    match crate::dispatch::reopen(
        tick.state,
        DispatchId(id.to_owned()),
        reason,
        crate::actor::chosen(),
        Some(&tick.id),
    )? {
        Reopening::Reopened => {
            tick.wrote(DispatchLogEvent::Reopened);
            Ok(())
        }
        Reopening::Refused { error, detail } => {
            bail!("dispatch reopen-dispatch: {error}: {detail}")
        }
    }
}

/// Whether the report `verb` on a dispatch held in `held` repeats the move that put it there: a
/// `started` on a `Started` dispatch or a `blocked` on a `Blocked` one.
pub(crate) fn repeated(verb: &str, held: Option<DispatchState>) -> bool {
    matches!(
        (verb, held),
        ("started", Some(DispatchState::Started)) | ("blocked", Some(DispatchState::Blocked))
    )
}

/// `review <url> [<words>]` on the dispatch `id`, which the store holds in `held` (`None`: no
/// dispatch has the id), of the report's own repository. `said` is the line after the verb.
///
/// The first word of `said` that is a link a dispatch takes ([`crate::dispatch::valid_link`]) is
/// the pull request's URL: a `Sent` dispatch is reported `started` first, as a `blocked`, `done`
/// or `failed` on it is (`story:report-on-sent-dispatch`), a dispatch in any other state does not
/// move, and the URL is recorded through [`crate::dispatch::link`], the code `dispatch
/// record-link` runs: `linked` where the dispatch did not hold it, nothing where it did. A
/// dispatch that holds the most links (`DispatchLinksFull`) takes the report without the link.
/// A first word that is no such link, or none, makes the report a `progress` one: nothing moves
/// and nothing is linked. Neither is a refusal; each is one line of the journal, the word written
/// as Rust escapes it, so no control character of it reaches the journal.
///
/// # Errors
///
/// The store does not open or keep the move or the link, or no dispatch has the id
/// (`DispatchNotFound`), as for the verbs that move a dispatch.
fn review(
    tick: &mut Tick<'_, '_>,
    id: &str,
    held: Option<DispatchState>,
    said: Option<&str>,
) -> Result<()> {
    let word = said
        .and_then(|said| said.split_whitespace().next())
        .unwrap_or_default();
    if !crate::dispatch::valid_link(word) {
        let why = if word.is_empty() {
            anyhow!("names no URL; handled as progress, nothing linked")
        } else {
            anyhow!(
                "{word:?} is no link a dispatch takes (InvalidLink); handled as progress, nothing \
                 linked"
            )
        };
        tick.note(&format!("mailbox: review on {id}"), &why);
        return Ok(());
    }
    if held == Some(DispatchState::Sent) {
        let _ = crate::dispatch::report_started(
            tick.state,
            ReportStartedArgs {
                actor: None,
                dispatch_id: Some(id.to_owned()),
                input_json: None,
            },
        )?;
        tick.wrote(DispatchLogEvent::Started);
    }
    match crate::dispatch::link(
        tick.state,
        DispatchId(id.to_owned()),
        word.to_owned(),
        crate::actor::chosen(),
    )? {
        Linking::Linked => tick.wrote(DispatchLogEvent::Linked),
        Linking::Held => {}
        Linking::Refused {
            error: "DispatchLinksFull",
            detail,
        } => tick.note(
            &format!("mailbox: review on {id}"),
            &anyhow!("{word:?} is not linked: DispatchLinksFull: {detail}"),
        ),
        Linking::Refused { error, detail } => bail!("dispatch record-link: {error}: {detail}"),
    }
    Ok(())
}

/// The state the store holds the dispatch `id` in, and its repository; `None` when it holds no
/// such dispatch.
fn held_of(tick: &Tick<'_, '_>, id: &str) -> Result<Option<(DispatchState, String)>> {
    let store = crate::state::open(tick.state)?;
    let held = DispatchStorage::get(&store, &DispatchId(id.to_owned()));
    store.check().context("read the dispatch")?;
    Ok(held.map(|held| (held.state, held.data.repository.0)))
}

/// `[NEED <repo> <request>] <other-repo> <what>`: the whole message to `<other-repo>`'s running
/// controller, else kept for its next start. The repository is read from the `<other-repo>`
/// field alone ([`other_repository`]), never from the words after it. A field that names no
/// repository the instance knows is not relayed and nothing is kept for it: the file stays in
/// the mailbox, where the supervisor reads it. The sender is told so ([`NotRelayed`]), but for a
/// field that names the mailbox's own recipient ([`RECIPIENT`]), a need for the supervisor that
/// is answered by no one here (`story:need-repository-from-tag`), and raises one alert
/// `need-unroutable` ([`unroutable`]).
///
/// A need of `repo` for another repository that can only start for a unit of work becomes one
/// ([`unit`], `story:need-starts-unit-of-work`): its message, the file `file`, is routed to that
/// dispatch, and `repo`'s controller is told so, delivered or kept for its next start.
fn need(
    tick: &mut Tick<'_, '_>,
    file: &Path,
    text: &str,
    first: &str,
    rest: &str,
    repo: &str,
    request: &str,
) -> Result<bool> {
    let field = rest
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("the need names no repository"))?;
    let other = match other_repository(tick, field) {
        Ok(Some(other)) => other,
        Ok(None) => return Ok(false),
        Err(error) => {
            if error.is::<NotRelayed>() {
                unroutable(tick, repo, request, &error);
            }
            return Err(error);
        }
    };
    // A need that names its own repository is no work for another: it waits for its own
    // controller, as before.
    if other != repo
        && let Some(id) = unit(tick, &other, repo, request, text)?
    {
        route(tick, file, &id)?;
        deliver_or_keep(
            tick,
            json!({
                "repo": repo,
                "text": format!("[REPORT {repo} -] need {request} opened {id} for {other}\n"),
                "event": "relayed",
                "line": first,
            }),
        )?;
        return Ok(true);
    }
    deliver_or_keep(
        tick,
        json!({"repo": other, "text": text, "event": "relayed", "line": first}),
    )?;
    Ok(true)
}

/// The condition of a `[NEED]` whose `<other-repo>` names no repository of the instance.
const NEED_UNROUTABLE: &str = crate::alert::NEED_UNROUTABLE;

/// The reference of the unit of work the need `request` of `repo` opens: `need-<repo>-<request>`,
/// every character a reference does not take (`A-Z a-z 0-9 . _ -`) written `-`, cut to 64.
fn need_reference(repo: &str, request: &str) -> String {
    format!("need-{repo}-{request}")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect()
}

/// The dispatch the need `request` of `repo`, whose whole message is `text`, is a unit of work
/// for in `other`, if it is one: the dispatch that already holds its reference, or one sent now
/// through [`crate::dispatch::send_work`] while `other` has no charter, no running controller and
/// no open dispatch. `None` otherwise, and when the send is refused (named on standard error):
/// the need is then relayed as before.
///
/// # Errors
///
/// The store does not open, or the send fails.
fn unit(
    tick: &mut Tick<'_, '_>,
    other: &str,
    repo: &str,
    request: &str,
    text: &str,
) -> Result<Option<String>> {
    let reference = need_reference(repo, request);
    let held = crate::dispatch::held_by(tick.state, &reference)?;
    if held.is_none()
        && (tick.charter(other).is_file()
            || tick.running(other)?.is_some()
            || crate::dispatch::open_of(tick.state, other)? != Open::None)
    {
        return Ok(None);
    }
    let work = WorkRequest {
        repository: RepositoryName(other.to_owned()),
        brief: text.to_owned(),
        reference,
        start_ref: None,
        issue: None,
    };
    match crate::dispatch::send_work(tick.state, &work, crate::actor::chosen())? {
        Sent::Created(id) => {
            tick.wrote(DispatchLogEvent::Sent);
            Ok(Some(id))
        }
        Sent::Held(id) => Ok(Some(id)),
        Sent::Refused { error, detail, .. } => {
            tick.note(
                &format!("mailbox: need {request} of {repo}"),
                &anyhow!("no unit of work for {other}: {error}: {detail}; relayed as before"),
            );
            Ok(None)
        }
    }
}

/// Routes the `[NEED]` message the store recorded with `file` as its body, while it is
/// `Received`, to the dispatch `id` (`message route-need`): it reads `Handled` with that dispatch.
/// A file the store recorded no message for routes nothing.
///
/// # Errors
///
/// The store does not open, or the route is refused.
fn route(tick: &Tick<'_, '_>, file: &Path, id: &str) -> Result<()> {
    let dir = crate::state::dir(tick.state)?;
    let store = crate::state::open(tick.state)?;
    let path = file.display().to_string();
    let message = MessageStorage::list(&store).into_iter().find(|held| {
        held.data
            .body_path
            .as_deref()
            .is_some_and(|body| body == path || dir.join(body) == file)
            && held.state == MessageState::Received
            && held.data.kind == MessageKind::Need
    });
    drop(store);
    if let Some(message) = message {
        let _ = crate::message::route_need(
            tick.state,
            RouteNeedArgs {
                actor: None,
                message_id: Some(message.data.message_id.0.0),
                dispatch_id: Some(id.to_owned()),
                input_json: None,
            },
        )
        .with_context(|| format!("route the need of {} to {id}", one_line(&path)))?;
    }
    Ok(())
}

/// Raises one alert `need-unroutable` for the need `request` of `repo`, its subject `<repo>
/// <request>`, saying `error`; a failure to raise it is named on standard error.
fn unroutable(tick: &mut Tick<'_, '_>, repo: &str, request: &str, error: &anyhow::Error) {
    let dir = tick.dir.clone();
    let instance = &tick.instance().name.0;
    let subject = format!("{repo} {request}");
    let raised = crate::alert::line(NEED_UNROUTABLE, repo, Some(&subject), &format!("{error:#}"))
        .and_then(|mut line| {
            line.tick_id = Some(tick.id.clone());
            crate::alert::raise(&dir, instance, &line, |line| tick.write(line))
        });
    if let Err(error) = raised {
        tick.note("alert", &error);
    }
}

/// Why a `[NEED]` was not relayed: the word its `<other-repo>` field names is no repository of
/// the instance. No refusal: the message stays in the mailbox.
#[derive(Debug)]
struct NotRelayed(String);

impl std::fmt::Display for NotRelayed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the need names {:?}, no repository of this instance (none has a running controller \
             or a checkout under checkouts.root); it stays in {RECIPIENT}'s mailbox, where the \
             supervisor reads it",
            self.0
        )
    }
}

impl std::error::Error for NotRelayed {}

/// The repository the `<other-repo>` field `field` of a `[NEED]` names: its leading run of name
/// characters ([`crate::spawn::name_character`], and `/` between a group and its repository), so
/// `docs:` and `specs,` name `docs` and `specs`. A name character that ends the run (`docs.`) is
/// part of the name only when the instance knows a repository of that name; otherwise the name
/// is the run without the punctuation at its end. `None` for a field that names no repository
/// the instance knows and names the mailbox's own recipient ([`RECIPIENT`]).
///
/// # Errors
///
/// The instance knows no repository of that name ([`known`]), and the name is not the mailbox's
/// recipient: [`NotRelayed`], naming the word read.
fn other_repository(tick: &Tick<'_, '_>, field: &str) -> Result<Option<String>> {
    let end = field
        .find(|c: char| !(crate::spawn::name_character(c) || c == '/'))
        .unwrap_or(field.len());
    let run = &field[..end];
    let name = run.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
    if known(tick, run)? {
        return Ok(Some(run.to_owned()));
    }
    if name != run && known(tick, name)? {
        return Ok(Some(name.to_owned()));
    }
    if name == RECIPIENT {
        return Ok(None);
    }
    let read = if name.is_empty() { field } else { name };
    Err(NotRelayed(read.to_owned()).into())
}

/// Whether the instance knows the repository `name`: a controller of it runs, or one can start,
/// in its checkout under the checkouts root ([`crate::spawn::checkout`]). An item kept for any
/// other name would wait in `pending.json` for a controller that never starts.
fn known(tick: &Tick<'_, '_>, name: &str) -> Result<bool> {
    Ok(!name.is_empty()
        && (tick.running(name)?.is_some() || crate::spawn::checkout(tick.instance(), name).is_ok()))
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
    let _ = crate::decide::raise_decision_request(
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
    let _ = crate::decide::escalate_request(
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
/// `Received` and no `[NEED]`, with `dispatch`, the dispatch a `[REPORT]` was handled for. A
/// body path is kept relative to the state directory (`conductor message send`) or whole; either
/// names the file.
fn mark_handled(tick: &Tick<'_, '_>, file: &Path, dispatch: Option<String>) -> Result<()> {
    let dir = crate::state::dir(tick.state)?;
    let store = crate::state::open(tick.state)?;
    let path = file.display().to_string();
    let message = MessageStorage::list(&store).into_iter().find(|held| {
        held.data
            .body_path
            .as_deref()
            .is_some_and(|body| body == path || dir.join(body) == file)
            && held.state == MessageState::Received
            && held.data.kind != MessageKind::Need
    });
    drop(store);
    if let Some(message) = message {
        let _ = crate::message::handle_message(
            tick.state,
            HandleMessageArgs {
                actor: None,
                message_id: Some(message.data.message_id.0.0),
                dispatch_id: dispatch,
                input_json: None,
            },
        )
        .with_context(|| format!("mark the message of {} handled", one_line(&path)))?;
    }
    Ok(())
}
