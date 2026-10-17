//! The handler of each watch line (docs/design/conductor-run.md § 3, `story:run-daemon`), and
//! what a later tick finishes of what an earlier one began ([`pending`]).
//!
//! | watch line | handler | lines |
//! |---|---|---|
//! | `context: <repo> <n>k …` over `thresholds.context_handover` | deliver `[HANDOVER <repo>] context <n>k` once per session; over 1.25 times the threshold, at once or in a later tick, or once `<checkout>/.git/handoff/` holds a file newer than the delivery, restart the controller ([`Tick::restart`]) with that file as the charter's resume point | `handover-sent`, `handover`, `controller-restarted` |
//! | `usage limit: …; <n> sessions: <names>` | mark each session; a later tick whose [`Headroom`](super::Headroom) is back resumes each (`spawn::resume`), or restarts one a hand-over was asked of | `usage-limit`, `resumed`, `controller-restarted` |
//! | `PR checks done: <repo>#…`, `CI red on main: <repo> / …`, `CI green again on main: <repo> / …` | deliver `[REPORT <repo> -] watch: <line>` to the running controller; a red `main` with none spawns one when the repository has a charter, within `controllers.max_working` | `relayed`, `red-ci` |
//! | `session stalled: <name> …`, `session silent: <name> …` | first a delivered check-in; the second time, a restart | `stalled`, `controller-restarted` |
//! | `session gone: <name> …`, `session exited (blocked): <name> …` | a restart, when the repository has an open dispatch and no usage limit marked the session | `controller-restarted` |
//! | `disk low: …` | `gc` | `cleanup` |
//! | `escalated …`, `unanswered …` | a desktop notification (`notify-send`, as the watch's usage limit) | none |
//! | `answered <id> <choice>` | deliver `[DECISION <id>] <choice> <reason> (<decision id>)` to the controller of the repository that raised the request, else keep it for that controller's next start | `decision-sent` |
//!
//! A session is a controller's when the store holds a running controller of that session name.
//! A delivery that cannot be made for a controller that is not running (a `[DECISION]`, a mailbox
//! `[NEED]` or `[RESOURCE]` answer) is kept in `<state>/run/pending.json` and made once that
//! repository has a running controller; one refused by a running controller is kept for the next
//! controller, never tried again for the same one. The other files: `handover.json` (each session
//! a hand-over was asked of, its repository and when, in nanoseconds), `limited.json` (each
//! session a usage limit marked) and `stalled.json` (each session a check-in was sent).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime};

use anyhow::{Context as _, Result, anyhow};
use conductor_model::behaviour::{DecisionRequestStorage, DecisionStorage};
use conductor_model::config::SessionName;
use conductor_model::decision::RequestId;
use conductor_model::dispatch::DispatchLogEvent;
use serde_json::{Value, json};

use super::{Tick, nanos, one_line, read_json, write_json};
use crate::cli::GcArgs;
use crate::watch::Watch;

/// The sessions a hand-over was asked of.
const HANDOVER: &str = "handover.json";

/// The sessions a usage limit marked.
const LIMITED: &str = "limited.json";

/// The sessions a check-in was sent.
const STALLED: &str = "stalled.json";

/// The deliveries kept for a controller's next start.
pub(crate) const PENDING: &str = "pending.json";

/// How long the desktop notifier may run.
const NOTIFY_BOUND: Duration = Duration::from_secs(10);

/// What a tick finishes first of what earlier ticks began: the sessions a usage limit marked,
/// once there is headroom; the hand-overs asked for; the deliveries kept.
pub fn pending(tick: &mut Tick<'_, '_>) {
    if let Err(error) = limited(tick) {
        tick.note("resume after a usage limit", &error);
    }
    if let Err(error) = handovers(tick) {
        tick.note("hand-over", &error);
    }
    if let Err(error) = deliveries(tick) {
        tick.note("kept deliveries", &error);
    }
}

/// Runs the handler of each of `lines`, the usage limits first, so a session a limit stopped is
/// marked before its exit is read; counts each line a handler acted on.
pub fn handle(tick: &mut Tick<'_, '_>, lines: &[String]) {
    let (limits, others): (Vec<&String>, Vec<&String>) = lines
        .iter()
        .partition(|line| line.starts_with("usage limit: "));
    for line in limits.into_iter().chain(others) {
        match act(tick, line) {
            Ok(true) => tick.lines_handled += 1,
            Ok(false) => {}
            Err(error) => tick.note(line, &error),
        }
    }
}

/// The handler of `line`; whether it acted.
fn act(tick: &mut Tick<'_, '_>, line: &str) -> Result<bool> {
    if let Some(rest) = line.strip_prefix("context: ") {
        return context(tick, line, rest);
    }
    if line.starts_with("usage limit: ") {
        return usage_limit(tick, line);
    }
    if let Some(rest) = line.strip_prefix("PR checks done: ") {
        let repo = rest.split('#').next().unwrap_or_default();
        return relay(tick, line, repo);
    }
    for prefix in ["CI red on main: ", "CI green again on main: "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let repo = rest.split(" / ").next().unwrap_or_default();
            return relay(tick, line, repo);
        }
    }
    for prefix in ["session stalled: ", "session silent: "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return stalled(tick, line, first_word(rest));
        }
    }
    if let Some(rest) = line.strip_prefix("session gone: ") {
        return gone(tick, first_word(rest), "gone");
    }
    if let Some(rest) = line.strip_prefix("session exited (blocked): ") {
        return gone(tick, first_word(rest), "exited");
    }
    if line.starts_with("disk low: ") {
        return disk_low(tick, line);
    }
    if line.starts_with("escalated ") || line.starts_with("unanswered ") {
        notify(tick, line);
        return Ok(true);
    }
    if let Some(rest) = line.strip_prefix("answered ") {
        return answered(tick, first_word(rest));
    }
    Ok(false)
}

fn first_word(text: &str) -> &str {
    text.split_whitespace().next().unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------------------------

/// `context: <repo> <n>k tokens (session <id>)`: the running controller of `<repo>` is asked to
/// hand over once, or restarted at once over 1.25 times the threshold.
fn context(tick: &mut Tick<'_, '_>, line: &str, rest: &str) -> Result<bool> {
    let mut words = rest.split_whitespace();
    let repo = words.next().unwrap_or_default();
    let thousands: u64 = words
        .next()
        .and_then(|word| word.strip_suffix('k'))
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| anyhow!("the line names no context in thousands of tokens"))?;
    let Some((_, session)) = tick.running(repo)? else {
        return Ok(false);
    };
    let threshold = u64::try_from(tick.instance().thresholds.context_handover.0).unwrap_or(0);
    let tokens = thousands.saturating_mul(1000);
    if over(tokens, threshold) {
        hand_over(tick, repo, &session, None, "context over 1.25x")?;
        return Ok(true);
    }
    if tokens <= threshold {
        return Ok(false);
    }
    let mut asked = read_json(&tick.dir, HANDOVER, json!({}));
    if asked.get(&session).is_some() {
        return Ok(false);
    }
    tick.deliver(&session, &format!("[HANDOVER {repo}] context {thousands}k"))?;
    asked[session.as_str()] = json!({
        "repo": repo,
        "delivered": nanos(SystemTime::now()).to_string(),
    });
    write_json(&tick.dir, HANDOVER, &asked)?;
    let mut written = tick.line(DispatchLogEvent::HandoverSent, repo)?;
    written.session = Some(session);
    written.line = Some(line.to_owned());
    tick.write(&written)?;
    Ok(true)
}

/// Whether `tokens` lie over 1.25 times `threshold`.
fn over(tokens: u64, threshold: u64) -> bool {
    u128::from(tokens) * 4 > u128::from(threshold) * 5
}

/// Writes `handover` for `session` of `repo`, with `reason` and the hand-over file `file`, and
/// restarts the controller with that file as its resume point.
fn hand_over(
    tick: &mut Tick<'_, '_>,
    repo: &str,
    session: &str,
    file: Option<&Path>,
    reason: &str,
) -> Result<()> {
    let mut written = tick.line(DispatchLogEvent::Handover, repo)?;
    written.session = Some(session.to_owned());
    written.reason = Some(reason.to_owned());
    written.path = file.map(|file| file.display().to_string());
    tick.write(&written)?;
    tick.restart(repo, session, "handover", file)
}

/// Each session a hand-over was asked of: restarted once its checkout's `.git/handoff/` holds a
/// file newer than the delivery, or once its context is over 1.25 times the threshold; forgotten
/// once its repository's running controller is another session.
fn handovers(tick: &mut Tick<'_, '_>) -> Result<()> {
    let asked = read_json(&tick.dir, HANDOVER, json!({}));
    let Some(asked) = asked.as_object() else {
        return Ok(());
    };
    let threshold = u64::try_from(tick.instance().thresholds.context_handover.0).unwrap_or(0);
    let mut watch: Option<Watch> = None;
    for (session, entry) in asked {
        let repo = entry["repo"].as_str().unwrap_or_default().to_owned();
        if tick
            .running(&repo)?
            .is_none_or(|(_, running)| running != *session)
        {
            forget(tick, session);
            continue;
        }
        let delivered: u128 = entry["delivered"]
            .as_str()
            .and_then(|at| at.parse().ok())
            .unwrap_or(0);
        if let Some(file) = newest_handoff(&tick.checkout(&repo), delivered) {
            hand_over(tick, &repo, session, Some(&file), "hand-over file")?;
            continue;
        }
        if watch.is_none() {
            watch = Some(Watch::of_instance(tick.state)?);
        }
        let tokens = watch
            .as_ref()
            .map(|watch| watch.context_of(session))
            .transpose()?
            .flatten();
        if tokens.is_some_and(|tokens| over(tokens, threshold)) {
            hand_over(tick, &repo, session, None, "context over 1.25x")?;
        }
    }
    Ok(())
}

/// The newest file of `<checkout>/.git/handoff/` written after `since` nanoseconds.
fn newest_handoff(checkout: &Path, since: u128) -> Option<PathBuf> {
    fs::read_dir(checkout.join(".git/handoff"))
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            let written = nanos(meta.modified().ok()?);
            (meta.is_file() && written > since).then(|| (written, entry.path()))
        })
        .max()
        .map(|(_, path)| path)
}

/// Forgets what the ticks kept of `session`: a hand-over asked, a usage limit, a check-in.
pub fn forget(tick: &Tick<'_, '_>, session: &str) {
    for file in [HANDOVER, LIMITED, STALLED] {
        let mut kept = read_json(&tick.dir, file, json!({}));
        if let Some(map) = kept.as_object_mut()
            && map.remove(session).is_some()
            && let Err(error) = write_json(&tick.dir, file, &kept)
        {
            tick.note(file, &error);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Usage limit
// ---------------------------------------------------------------------------------------------

/// `usage limit: <text>; <n> sessions: <names>`: marks each session.
fn usage_limit(tick: &mut Tick<'_, '_>, line: &str) -> Result<bool> {
    let names: Vec<String> = line
        .rsplit_once("sessions: ")
        .map(|(_, names)| names.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default();
    let mut marked = read_json(&tick.dir, LIMITED, json!({}));
    for name in &names {
        let repo = tick.repo_of(name)?.unwrap_or_else(|| "-".to_owned());
        marked[name.as_str()] = json!({"repo": repo});
        write_json(&tick.dir, LIMITED, &marked)?;
        let mut written = tick.line(DispatchLogEvent::UsageLimit, &repo)?;
        written.session = Some(name.clone());
        written.line = Some(line.to_owned());
        tick.write(&written)?;
    }
    Ok(!names.is_empty())
}

/// The sessions a usage limit marked, once [`super::Headroom`] answers yes: each resumed from its
/// transcript (`spawn::resume`), or restarted when a hand-over was asked of it; each forgotten.
fn limited(tick: &mut Tick<'_, '_>) -> Result<()> {
    let marked = read_json(&tick.dir, LIMITED, json!({}));
    let Some(marked) = marked.as_object().filter(|marked| !marked.is_empty()) else {
        return Ok(());
    };
    if !tick.seams.headroom.headroom()? {
        return Ok(());
    }
    let asked = read_json(&tick.dir, HANDOVER, json!({}));
    for (session, entry) in marked {
        let repo = entry["repo"].as_str().unwrap_or("-").to_owned();
        let done = if repo == "-" {
            Ok(())
        } else if asked.get(session).is_some() {
            tick.restart(&repo, session, "limit over the context threshold", None)
        } else {
            resume(tick, &repo, session)
        };
        if let Err(error) = done {
            tick.note(&format!("resume {session}"), &error);
        }
        forget(tick, session);
    }
    Ok(())
}

/// Resumes `session` of `repo` from its transcript and writes `resumed`; nothing when there is
/// nothing to resume (no measurement, or a live session).
fn resume(tick: &mut Tick<'_, '_>, repo: &str, session: &str) -> Result<()> {
    let resumed = crate::spawn::resume(
        tick.config,
        tick.state,
        repo,
        &SessionName(session.to_owned()),
        tick.seams.providers,
    )?;
    if resumed.is_some() {
        let mut written = tick.line(DispatchLogEvent::Resumed, repo)?;
        written.session = Some(session.to_owned());
        tick.write(&written)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Relay and red CI
// ---------------------------------------------------------------------------------------------

/// A pull request's checks or `main`'s CI: relayed to the running controller of `repo`; a red
/// `main` with none dispatched ([`red_ci`]).
fn relay(tick: &mut Tick<'_, '_>, line: &str, repo: &str) -> Result<bool> {
    if let Some((_, session)) = tick.running(repo)? {
        tick.deliver(&session, &format!("[REPORT {repo} -] watch: {line}"))?;
        let mut written = tick.line(DispatchLogEvent::Relayed, repo)?;
        written.session = Some(session);
        written.line = Some(line.to_owned());
        tick.write(&written)?;
        return Ok(true);
    }
    if line.starts_with("CI red on main: ") {
        return red_ci(tick, line, repo);
    }
    Ok(false)
}

/// A red `main` with no running controller: spawned from the charter (`red-ci`, `spawned`), or
/// noted (`no charter`, or why the spawn did not happen).
fn red_ci(tick: &mut Tick<'_, '_>, line: &str, repo: &str) -> Result<bool> {
    let mut written = tick.line(DispatchLogEvent::RedCi, repo)?;
    written.line = Some(line.to_owned());
    if tick.charter(repo).is_file() {
        match tick.spawn(repo)? {
            Ok(()) => {
                written.reason = Some("spawned".to_owned());
                written.session = Some(super::session_of(repo).0);
            }
            Err(why) => written.reason = Some(why),
        }
    } else {
        written.reason = Some("no charter".to_owned());
    }
    tick.write(&written)?;
    Ok(true)
}

// ---------------------------------------------------------------------------------------------
// Stalled, silent, gone
// ---------------------------------------------------------------------------------------------

/// A stalled or silent controller: a check-in the first time, a restart the second.
fn stalled(tick: &mut Tick<'_, '_>, line: &str, name: &str) -> Result<bool> {
    let Some(repo) = tick.repo_of(name)? else {
        return Ok(false);
    };
    if tick
        .running(&repo)?
        .is_none_or(|(_, session)| session != name)
    {
        return Ok(false);
    }
    let mut asked = read_json(&tick.dir, STALLED, json!({}));
    if asked.get(name).is_some() {
        tick.restart(&repo, name, "stalled again", None)?;
        return Ok(true);
    }
    tick.deliver(
        name,
        &format!("[REPORT {repo} -] check-in: report your state or blocked"),
    )?;
    asked[name] = json!({"repo": repo});
    write_json(&tick.dir, STALLED, &asked)?;
    let mut written = tick.line(DispatchLogEvent::Stalled, &repo)?;
    written.session = Some(name.to_owned());
    written.line = Some(line.to_owned());
    tick.write(&written)?;
    Ok(true)
}

/// A gone or exited controller session: restarted when its repository has an open dispatch and
/// no usage limit marked it.
fn gone(tick: &mut Tick<'_, '_>, name: &str, reason: &str) -> Result<bool> {
    if read_json(&tick.dir, LIMITED, json!({})).get(name).is_some() {
        return Ok(false);
    }
    let Some(repo) = tick.repo_of(name)? else {
        return Ok(false);
    };
    if !tick.open_dispatch(&repo)? {
        return Ok(false);
    }
    tick.restart(&repo, name, reason, None)?;
    Ok(true)
}

// ---------------------------------------------------------------------------------------------
// Disk, notifications, answers
// ---------------------------------------------------------------------------------------------

/// `disk low`: runs `gc` and writes `cleanup` with how it exited.
fn disk_low(tick: &mut Tick<'_, '_>, line: &str) -> Result<bool> {
    let evidence = match (tick.seams.gc)(&GcArgs {
        dry_run: None,
        format: None,
    }) {
        Ok(code) if code == ExitCode::SUCCESS => "gc exited 0".to_owned(),
        Ok(_) => "gc exited 1: a row was refused".to_owned(),
        Err(error) => format!("gc exited 1: {}", one_line(&format!("{error:#}"))),
    };
    let mut written = tick.line(DispatchLogEvent::Cleanup, "-")?;
    written.line = Some(line.to_owned());
    written.evidence = Some(evidence);
    tick.write(&written)?;
    Ok(true)
}

/// A desktop notification of `line`, titled `<instance>: decision`; one that does not run is
/// named on standard error.
fn notify(tick: &Tick<'_, '_>, line: &str) {
    let title = format!("{}: decision", tick.instance().name.0);
    let ran = crate::collect::run(
        Command::new("notify-send").args(["-u", "critical", &title, line]),
        NOTIFY_BOUND,
    );
    if let Err(error) = ran {
        tick.note("notify-send", &error);
    }
}

/// `answered <id> <choice>`: the decision that answered the request, delivered to the controller
/// of the repository that raised it, else kept for its next start.
fn answered(tick: &mut Tick<'_, '_>, id: &str) -> Result<bool> {
    let store = crate::state::open(tick.state)?;
    let request = DecisionRequestStorage::get(&store, &RequestId(id.to_owned()))
        .ok_or_else(|| anyhow!("no decision request {id}"))?;
    let decision_id = request
        .data
        .decision_id
        .ok_or_else(|| anyhow!("the request {id} names no decision"))?;
    let decision = DecisionStorage::get(&store, &decision_id)
        .ok_or_else(|| anyhow!("no decision {}", decision_id.0))?;
    drop(store);
    let repo = request.data.repository.0;
    let text = format!(
        "[DECISION {id}] {} {} ({})",
        one_line(&decision.data.choice),
        one_line(&decision.data.reason),
        decision_id.0
    );
    let item = json!({
        "repo": repo, "text": text, "event": "decision-sent", "id": id, "grant": decision_id.0,
    });
    deliver_or_keep(tick, item)?;
    Ok(true)
}

// ---------------------------------------------------------------------------------------------
// Deliveries kept for a controller's next start
// ---------------------------------------------------------------------------------------------

/// What became of a delivery to a repository's controller.
enum Made {
    /// It was delivered, and its line written.
    Delivered,
    /// The repository has no running controller, or only the one that refused it.
    Waits,
    /// The running controller, by its id, refused it.
    Refused(String),
}

/// Delivers `item` (`repo`, `text`, `event`, and the line's `id`, `grant` or `line`) to the
/// running controller of its repository and writes its line; keeps it in [`PENDING`] when there
/// is none, or when that controller refused it.
///
/// # Errors
///
/// The store does not open, or the line or the kept file cannot be written.
pub(crate) fn deliver_or_keep(tick: &mut Tick<'_, '_>, item: Value) -> Result<()> {
    let item = match made(tick, &item)? {
        Made::Delivered => return Ok(()),
        Made::Waits => item,
        Made::Refused(controller) => refused(item, &controller),
    };
    let mut kept = read_json(&tick.dir, PENDING, json!([]));
    if let Some(list) = kept.as_array_mut() {
        list.push(item);
    }
    write_json(&tick.dir, PENDING, &kept)
}

/// `item` marked refused by the controller `controller`, so it waits for the next one.
fn refused(mut item: Value, controller: &str) -> Value {
    item["refused_by"] = json!(controller);
    item
}

/// Delivers `item` when its repository has a running controller other than the one that refused
/// it.
fn made(tick: &mut Tick<'_, '_>, item: &Value) -> Result<Made> {
    let repo = item["repo"].as_str().unwrap_or_default();
    let Some((controller, session)) = tick.running(repo)? else {
        return Ok(Made::Waits);
    };
    if item["refused_by"].as_str() == Some(controller.as_str()) {
        return Ok(Made::Waits);
    }
    let text = item["text"].as_str().unwrap_or_default();
    if let Err(error) = tick.deliver(&session, text) {
        tick.note(&format!("deliver to {session}"), &error);
        return Ok(Made::Refused(controller));
    }
    let event = item["event"]
        .as_str()
        .and_then(crate::records::dispatch_event_named)
        .unwrap_or(DispatchLogEvent::Relayed);
    let mut written = tick.line(event, repo)?;
    written.session = Some(session);
    written.id = item["id"].as_str().map(str::to_owned);
    written.grant = item["grant"].as_str().map(str::to_owned);
    written.line = item["line"].as_str().map(str::to_owned);
    tick.write(&written)?;
    Ok(Made::Delivered)
}

/// The deliveries kept: each made once its repository has a running controller; one refused is
/// kept for the next controller.
fn deliveries(tick: &mut Tick<'_, '_>) -> Result<()> {
    let kept = read_json(&tick.dir, PENDING, json!([]));
    let Some(items) = kept.as_array().filter(|items| !items.is_empty()) else {
        return Ok(());
    };
    let mut left = Vec::new();
    for item in items {
        match made(tick, item)? {
            Made::Delivered => {}
            Made::Waits => left.push(item.clone()),
            Made::Refused(controller) => left.push(refused(item.clone(), &controller)),
        }
    }
    write_json(&tick.dir, PENDING, &Value::Array(left)).context("keep the deliveries left")
}
