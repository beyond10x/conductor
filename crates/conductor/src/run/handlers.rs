//! The handler of each watch line (docs/design/conductor-run.md § 3, `story:run-daemon`), and
//! what a later tick finishes of what an earlier one began ([`pending`]).
//!
//! | watch line | handler | lines |
//! |---|---|---|
//! | `context: <repo> <n>k …` at or over `thresholds.context_handover` | deliver `[HANDOVER <repo>] context <n>k` once per controller; over 1.25 times the threshold, at once or in a later tick, once `<checkout>/.git/handoff/` holds a file newer than the delivery and the session is idle, or once `thresholds.silent_session` passed since the delivery, restart the controller ([`Tick::restart_with`]) with the newest hand-over file as the charter's resume point, its line carrying the `context:` line; a later tick restarts only the controller, by its id, and the session the hand-over was asked of ([`handovers`]) | `handover-sent`, `handover`, `controller-restarted` |
//! | `usage limit: …; <n> sessions: <names>` | mark each session; a later tick whose [`Headroom`](super::Headroom) is back resumes each once (`spawn::resume`), or restarts one a hand-over was asked of as its controller's; a session resumed and stopped again is said in one journal line | `usage-limit`, `resumed`, `controller-restarted` |
//! | `session exited (blocked): <name> …` | marked as a usage limit marks it, when its repository has an open dispatch | `usage-limit`, `resumed` |
//! | `PR checks done: <repo>#…`, `CI red on main: <repo> / …`, `CI green again on main: <repo> / …` | deliver `[REPORT <repo> -] watch: <line>` to the running controller; a red `main` with none spawns one when the repository has a charter, within `controllers.max_working` | `relayed`, `red-ci` |
//! | `session stalled: <name> …`, `session silent: <name> …` | first a delivered check-in; the second time, a restart | `stalled`, `controller-restarted` |
//! | `session gone: <name> …` | a restart, when the repository has an open dispatch and no usage limit marked the session; the instance's conductor-dev started again in the checkout it worked in, when that checkout's plan holds an `active` story ([`conductor_dev`]) | `controller-restarted` |
//! | `disk low: …` | `gc`, beside the tick ([`cleaned`]) | `cleanup` |
//! | `escalated …`, `unanswered …` | a desktop notification (`notify-send`, as the watch's usage limit) | none |
//! | `answered <id> <choice>` | deliver `[DECISION <id>] <choice> <reason> (<decision id>)` to the controller of the repository that raised the request, else keep it for that controller's next start | `decision-sent` |
//!
//! A session is a controller's when the store holds a running controller of that session name.
//! A delivery that cannot be made for a controller that is not running (a `[DECISION]`, a mailbox
//! `[NEED]` or `[RESOURCE]` answer) is kept in `<state>/run/pending.json` and made once that
//! repository has a running controller; one refused by a running controller is kept for the next
//! controller, never tried again for the same one. Any delivery to a controller refused
//! `SessionNotReachable` records that controller stopped, with one journal line ([`send`],
//! `story:deliver-to-stopped-controller`), unless the session list names its session live or
//! cannot be read: then the controller stays Running and the journal line says so
//! (`story:deliver-reaches-bg-session`). The other files: `handover.json` (each session a
//! hand-over was asked of, its repository, the id of the controller it was asked of and of the
//! session the `context:` line measured, when, in nanoseconds, and the `context:` line),
//! `limited.json` (each session a usage limit marked), `stalled.json` (each session a check-in
//! was sent) and `resumed.json` (each session the loop resumed, and when).
//!
//! The `gc` a `disk low` line starts runs on a thread of its own, so the loop's ticks go on while
//! it works (`story:first-cycle-not-blocked-by-archiving`); one runs at a time, and a `disk low`
//! line read while one runs starts none. Its `cleanup` line is written, with the writing tick's
//! id, by the first watch tick that finds it ended ([`pending`]), or by the tick that started it
//! where that tick waits for it ([`super::handle_lines`]). A process that stops while it runs
//! loses it; the next `disk low` line starts another.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::{Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime};

use crate::config::BuiltInRole;
use anyhow::{Context as _, Result, anyhow};
use conductor_model::behaviour::{DecisionRequestStorage, DecisionStorage};
use conductor_model::config::SessionName;
use conductor_model::decision::RequestId;
use conductor_model::dispatch::obligations::StopControllerBehavior;
use conductor_model::dispatch::{
    ControllerId, DispatchLogEvent, StopController, StopControllerOutcome,
};
use conductor_model::primitives::Uuid;
use serde_json::{Value, json};

use super::{Cleaner, Tick, nanos, one_line, read_json, write_json};
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

/// The sessions the loop resumed after a usage limit, and when, in nanoseconds.
const RESUMED: &str = "resumed.json";

/// How long the desktop notifier may run.
const NOTIFY_BOUND: Duration = Duration::from_secs(10);

/// What a tick finishes first of what earlier ticks began: the `gc` a `disk low` line started,
/// once it ended; the sessions a usage limit marked, once there is headroom; the hand-overs asked
/// for; the deliveries kept.
pub fn pending(tick: &mut Tick<'_, '_>) {
    if let Err(error) = cleaned(tick, false) {
        tick.note("cleanup", &error);
    }
    if let Err(error) = limited(tick) {
        tick.note("resume after a usage limit", &error);
    }
    if let Err(error) = withdrawn(tick) {
        tick.note("resume after a token was withdrawn", &error);
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
        return gone(tick, first_word(rest));
    }
    if let Some(rest) = line.strip_prefix("session exited (blocked): ") {
        return exited(tick, line, first_word(rest));
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

/// `context: <repo> <n>k tokens (session <id>)`: the running controller of `<repo>` at or above
/// the threshold is asked to hand over once, or restarted at once over 1.25 times the
/// threshold. A line whose session id is not the one the controller's envelope
/// (`<state>/sessions/<session>.envelope.json`) records is another session named like the
/// repository, and is left alone (`story:context-watch-by-envelope`). What is kept of the
/// request names the controller, by its id, and the session the line measured
/// (`story:handover-restart-by-controller-id`): a request kept for another controller under
/// the same session name is no request to this one, which is asked.
fn context(tick: &mut Tick<'_, '_>, line: &str, rest: &str) -> Result<bool> {
    let mut words = rest.split_whitespace();
    let repo = words.next().unwrap_or_default();
    let thousands: u64 = words
        .next()
        .and_then(|word| word.strip_suffix('k'))
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| anyhow!("the line names no context in thousands of tokens"))?;
    let Some((controller, session)) = tick.running(repo)? else {
        return Ok(false);
    };
    if !measured_is_enveloped(tick, &session, line) {
        return Ok(false);
    }
    let threshold = u64::try_from(tick.instance().thresholds.context_handover.0).unwrap_or(0);
    let tokens = thousands.saturating_mul(1000);
    if over(tokens, threshold) {
        hand_over(tick, repo, &session, None, "context over 1.25x", Some(line))?;
        return Ok(true);
    }
    if tokens < threshold {
        return Ok(false);
    }
    let mut asked = read_json(&tick.dir, HANDOVER, json!({}));
    if asked
        .get(&session)
        .is_some_and(|entry| entry["controller"].as_str() == Some(controller.as_str()))
    {
        return Ok(false);
    }
    send(
        tick,
        &controller,
        &session,
        &format!("[HANDOVER {repo}] context {thousands}k"),
    )?;
    asked[session.as_str()] = json!({
        "repo": repo,
        "controller": controller,
        "session_id": measured(line),
        "delivered": nanos(SystemTime::now()).to_string(),
        "line": line,
    });
    write_json(&tick.dir, HANDOVER, &asked)?;
    let mut written = tick.line(DispatchLogEvent::HandoverSent, repo)?;
    written.session = Some(session);
    written.line = Some(line.to_owned());
    tick.write(&written)?;
    Ok(true)
}

/// Whether the session a `context:` line measured, `(session <id>)` at its end, is the one the
/// envelope of the session name `session` records under `<state>/sessions/`. A line naming no id,
/// or a session without an envelope or without an id in it, is taken as the controller's, as
/// before envelopes recorded ids.
fn measured_is_enveloped(tick: &Tick<'_, '_>, session: &str, line: &str) -> bool {
    let Some(measured) = measured(line) else {
        return true;
    };
    let Some(state) = tick.dir.parent() else {
        return true;
    };
    let envelope = crate::confine::record_path(state, &SessionName(session.to_owned()));
    let Some(recorded) = fs::read_to_string(envelope)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|value| value["session"]["session_id"].as_str().map(str::to_owned))
        .filter(|id| !id.is_empty())
    else {
        return true;
    };
    recorded.starts_with(measured)
}

/// The session a `context:` line measured: the `<id>` of `(session <id>)` at its end, the first
/// eight characters of the session's id; none when the line names none.
fn measured(line: &str) -> Option<&str> {
    line.rsplit_once("(session ")
        .and_then(|(_, id)| id.strip_suffix(')'))
        .filter(|id| !id.is_empty())
}

/// Whether `tokens` lie over 1.25 times `threshold`.
fn over(tokens: u64, threshold: u64) -> bool {
    u128::from(tokens) * 4 > u128::from(threshold) * 5
}

/// Writes `handover` for `session` of `repo`, with `reason` and the hand-over file `file`, and
/// restarts the controller with that file as its resume point; the `controller-restarted` line
/// carries `cause`, the `context:` line that names the context it was at.
fn hand_over(
    tick: &mut Tick<'_, '_>,
    repo: &str,
    session: &str,
    file: Option<&Path>,
    reason: &str,
    cause: Option<&str>,
) -> Result<()> {
    let mut written = tick.line(DispatchLogEvent::Handover, repo)?;
    written.session = Some(session.to_owned());
    written.reason = Some(reason.to_owned());
    written.path = file.map(|file| file.display().to_string());
    tick.write(&written)?;
    tick.restart_with(repo, session, "handover", file, cause)
}

/// Each session a hand-over was asked of (`story:loop-restarts-controller-at-handover`):
/// restarted once its checkout's `.git/handoff/` holds a file newer than the delivery and the
/// session list shows it idle ([`Watch::idle`]); once its context is over 1.25 times the
/// threshold; or once `thresholds.silent_session` passed since the delivery, from the newest
/// hand-over file it has, older or newer. Forgotten once its repository's running controller is
/// another session.
///
/// The request was made to one controller and to the session it ran then
/// (`story:handover-restart-by-controller-id`), so it is dropped, with one journal line and
/// nothing stopped, once that is no longer who holds the session name ([`superseded`]): the
/// repository runs a controller of another id, the session asked is not among the live
/// sessions of the name while another is, or it started after the delivery. An entry that
/// names no controller, as the binary before that story wrote it, stands for the session its
/// line names while the session list names that session, and is dropped the same way
/// otherwise.
fn handovers(tick: &mut Tick<'_, '_>) -> Result<()> {
    let asked = read_json(&tick.dir, HANDOVER, json!({}));
    let Some(asked) = asked.as_object() else {
        return Ok(());
    };
    let thresholds = &tick.instance().thresholds;
    let threshold = u64::try_from(thresholds.context_handover.0).unwrap_or(0);
    let silent = Duration::from_secs(
        crate::config::seconds(&thresholds.silent_session)
            .unwrap_or(crate::config::SILENT_SESSION_MINUTES * 60),
    )
    .as_nanos();
    let mut watch: Option<Watch> = None;
    for (session, entry) in asked {
        let repo = entry["repo"].as_str().unwrap_or_default().to_owned();
        let Some((controller, _)) = tick
            .running(&repo)?
            .filter(|(_, running)| running == session)
        else {
            forget(tick, session);
            continue;
        };
        let delivered = instant(&entry["delivered"]).unwrap_or(0);
        if let Some(why) = superseded(entry, session, Some(&controller))? {
            dropped(tick, session, &why);
            unkeep(tick, HANDOVER, session);
            continue;
        }
        let cause = entry["line"].as_str();
        if watch.is_none() {
            watch = Some(Watch::of_instance(tick.state)?);
        }
        let Some(watch) = watch.as_ref() else {
            continue;
        };
        let checkout = tick.checkout(&repo);
        let written = newest_handoff(&checkout, delivered);
        if let Some(file) = &written
            && watch.idle(session)?
        {
            hand_over(tick, &repo, session, Some(file), "hand-over file", cause)?;
            continue;
        }
        if watch
            .context_of(session)?
            .is_some_and(|tokens| over(tokens, threshold))
        {
            hand_over(tick, &repo, session, None, "context over 1.25x", cause)?;
            continue;
        }
        if nanos(SystemTime::now()).saturating_sub(delivered) > silent {
            let (file, reason) = match written {
                Some(file) => (Some(file), "hand-over file, not idle within silent_session"),
                None => (
                    newest_handoff(&checkout, 0),
                    "no hand-over within silent_session",
                ),
            };
            hand_over(tick, &repo, session, file.as_deref(), reason, cause)?;
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

/// Why the hand-over `entry`, kept under the session name `session`, is no request to who holds
/// that name now, with `controller` the repository's running controller, if it runs one; none
/// when it is one (`story:handover-restart-by-controller-id`).
///
/// The session asked is the one whose id starts with the entry's `session_id`, or, for an
/// entry the binary before that story wrote, with the id its `line` names. The sessions of the
/// name are read from the session list ([`crate::confine::listed`]), which is bound to no
/// directory, so the name may be held by more than one. No request:
///
/// - an entry that names another controller than `controller`: its controller was replaced
///   under the same session name;
/// - an entry that names no controller and whose session the list does not name, live or
///   stopped: nothing ties it to the running controller;
/// - an entry whose session is not among the live ones of the name while another is: the
///   session was ended and another started by hand, the controller's record staying;
/// - an entry whose session is live and started after the delivery, the same way. A start
///   up to the instant the loop itself resumed the session (`resumed`, [`resume`]) is the
///   loop's own and not one by hand.
///
/// A live session of the name beside the one asked supersedes nothing, whichever the list
/// prints first; neither does a name no live session holds, nor an entry or a list that names
/// no id or no start.
///
/// # Errors
///
/// The session list cannot be read: nothing is told then, and so nothing stopped.
fn superseded(entry: &Value, session: &str, controller: Option<&str>) -> Result<Option<String>> {
    let of = entry["controller"].as_str();
    if let (Some(asked), Some(controller)) = (of, controller)
        && asked != controller
    {
        return Ok(Some(format!(
            "it was asked of the controller {asked}, and the repository's controller is \
             {controller} now"
        )));
    }
    let asked = entry["session_id"]
        .as_str()
        .or_else(|| entry["line"].as_str().and_then(measured))
        .filter(|id| !id.is_empty());
    let id = |listed: &Value| {
        listed["sessionId"]
            .as_str()
            .or_else(|| listed["id"].as_str())
            .unwrap_or_default()
            .to_owned()
    };
    let is_asked = |listed: &Value| asked.is_some_and(|asked| id(listed).starts_with(asked));
    let listed = crate::confine::listed(session)?;
    if of.is_none() && !listed.iter().any(&is_asked) {
        let untied = asked.map_or_else(
            || "its line names no session".to_owned(),
            |asked| format!("the session list names no session {asked} under that name"),
        );
        return Ok(Some(format!(
            "the request kept names no controller, as an earlier version wrote it, and {untied}"
        )));
    }
    let since = instant(&entry["delivered"])
        .unwrap_or(0)
        .max(instant(&entry["resumed"]).unwrap_or(0));
    // `startedAt` is in milliseconds since the epoch, the delivery in nanoseconds.
    let younger = |listed: &Value| {
        listed["startedAt"]
            .as_u64()
            .is_some_and(|started| u128::from(started) * 1_000_000 > since)
    };
    let live: Vec<&Value> = listed
        .iter()
        .filter(|listed| !listed["pid"].is_null())
        .collect();
    let started_after = |listed: &Value| {
        format!(
            "the session {} that holds the name now started after it was asked",
            id(listed)
        )
    };
    if let Some(own) = live.iter().copied().find(|&listed| is_asked(listed)) {
        return Ok(younger(own).then(|| started_after(own)));
    }
    if let Some(asked) = asked
        && let Some(other) = live.iter().copied().find(|&listed| !id(listed).is_empty())
    {
        return Ok(Some(format!(
            "it was asked of the session {asked}, and the session {} holds the name now",
            id(other)
        )));
    }
    Ok(live
        .first()
        .copied()
        .filter(|_| live.iter().copied().all(&younger))
        .map(started_after))
}

/// An instant a kept entry holds, nanoseconds since the epoch written as a string.
fn instant(kept: &Value) -> Option<u128> {
    kept.as_str().and_then(|at| at.parse().ok())
}

/// Says in the journal that the hand-over asked under `session` is dropped, and `why`.
fn dropped(tick: &Tick<'_, '_>, session: &str, why: &str) {
    tick.note(
        &format!("hand-over of {session}"),
        &anyhow!("{why}; the request is dropped and nothing is stopped"),
    );
}

/// Whether the hand-over `entry`, kept under `session`, is a request to `repo`'s controller and
/// to the session that holds the name: nothing supersedes it ([`superseded`]), with the
/// controller the repository runs, if it runs one. One that is none is said in the journal.
///
/// # Errors
///
/// The store or the session list cannot be read: nothing is told then.
fn stands(tick: &Tick<'_, '_>, entry: &Value, session: &str, repo: &str) -> Result<bool> {
    let running = tick.running(repo)?;
    let controller = running.as_ref().map(|(controller, _)| controller.as_str());
    Ok(match superseded(entry, session, controller)? {
        Some(why) => {
            dropped(tick, session, &why);
            false
        }
        None => true,
    })
}

/// Forgets what the ticks kept of `session`: a hand-over asked, a usage limit, a check-in, a
/// resume.
pub fn forget(tick: &Tick<'_, '_>, session: &str) {
    for file in [HANDOVER, LIMITED, STALLED, RESUMED] {
        unkeep(tick, file, session);
    }
}

/// Takes `session` out of the kept file `file`.
fn unkeep(tick: &Tick<'_, '_>, file: &str, session: &str) {
    let mut kept = read_json(&tick.dir, file, json!({}));
    if let Some(map) = kept.as_object_mut()
        && map.remove(session).is_some()
        && let Err(error) = write_json(&tick.dir, file, &kept)
    {
        tick.note(file, &error);
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
    for name in &names {
        let repo = tick.repo_of(name)?.unwrap_or_else(|| "-".to_owned());
        mark(tick, name, &repo, line, None)?;
        on_limit(tick, name, &repo);
    }
    Ok(!names.is_empty())
}

/// `story:token-pool`: the token `name` ran on is Limited; when another token of the pool is
/// Available the session is resumed on it now, rather than once the account has headroom. A
/// hand-over asked of it stays kept (`story:handover-restart-by-controller-id`): the session
/// asked runs on under its id.
fn on_limit(tick: &mut Tick<'_, '_>, name: &str, repo: &str) {
    let instance = &crate::config::active().instance;
    match crate::token::on_limit(instance, name) {
        Ok(true) if repo != "-" => match resume(tick, repo, name) {
            Ok(true) => {
                for file in [LIMITED, STALLED, RESUMED] {
                    unkeep(tick, file, name);
                }
                let mut resumed = read_json(&tick.dir, RESUMED, json!({}));
                resumed[name] = json!(nanos(SystemTime::now()).to_string());
                if let Err(error) = write_json(&tick.dir, RESUMED, &resumed) {
                    tick.note(RESUMED, &error);
                }
            }
            Ok(false) => {}
            Err(error) => tick.note(&format!("resume {name} on another token"), &error),
        },
        Ok(_) => {}
        Err(error) => tick.note(&format!("mark the token of {name} limited"), &error),
    }
}

/// `story:token-pool`: each session on a token its owner withdrew (`conductor leave`), named in
/// `<state>/run/tokens-withdrawn.json`: ended, then resumed on another token.
fn withdrawn(tick: &mut Tick<'_, '_>) -> Result<()> {
    for session in crate::token::take_withdrawn(&tick.dir) {
        let Some(repo) = tick.repo_of(&session)? else {
            continue;
        };
        let name = SessionName(session.clone());
        if let Err(error) = crate::spawn::end(tick.state, &repo, &name, tick.seams.providers) {
            tick.note(&format!("end {session}"), &error);
            continue;
        }
        if let Err(error) = resume(tick, &repo, &session) {
            tick.note(&format!("resume {session} on another token"), &error);
        }
    }
    Ok(())
}

/// `session exited (blocked): <name> <id>`, a session the list keeps blocked with no `pid`, as
/// it keeps one a usage limit stopped (`story:auto-resume`): marked as a usage limit marks it,
/// so the first tick with headroom resumes it, when its repository has an open dispatch and no
/// usage limit marked it already.
fn exited(tick: &mut Tick<'_, '_>, line: &str, name: &str) -> Result<bool> {
    if read_json(&tick.dir, LIMITED, json!({})).get(name).is_some() {
        return Ok(false);
    }
    let Some(repo) = tick.repo_of(name)? else {
        return Ok(false);
    };
    if !tick.open_dispatch(&repo)? {
        return Ok(false);
    }
    mark(tick, name, &repo, line, Some("blocked with no pid"))?;
    Ok(true)
}

/// Marks the session `name` of `repo` stopped by `line` and writes `usage-limit` with `cause`.
/// A session the loop resumed within [`crate::watch::LIMIT_AGAIN`] is stopped a second time:
/// its line says so, and so does one journal line.
fn mark(
    tick: &mut Tick<'_, '_>,
    name: &str,
    repo: &str,
    line: &str,
    cause: Option<&str>,
) -> Result<()> {
    let mut marked = read_json(&tick.dir, LIMITED, json!({}));
    marked[name] = json!({"repo": repo});
    write_json(&tick.dir, LIMITED, &marked)?;
    let again = read_json(&tick.dir, RESUMED, json!({}))[name]
        .as_str()
        .and_then(|at| at.parse::<u128>().ok())
        .is_some_and(|at| {
            nanos(SystemTime::now()).saturating_sub(at) < crate::watch::LIMIT_AGAIN.as_nanos()
        });
    let mut written = tick.line(DispatchLogEvent::UsageLimit, repo)?;
    written.session = Some(name.to_owned());
    written.line = Some(line.to_owned());
    written.reason = if again {
        tick.note(
            name,
            &anyhow!(
                "a second usage limit after the loop resumed it: {}; it is resumed again once \
                 there is headroom",
                one_line(line)
            ),
        );
        Some("second usage limit after a resume".to_owned())
    } else {
        cause.map(str::to_owned)
    };
    tick.write(&written)
}

/// The sessions a usage limit marked, once [`super::Headroom`] answers yes: each resumed from its
/// transcript (`spawn::resume`) once per stop, or restarted when a hand-over was asked of it as
/// its repository's controller ([`stands`]: the checks a hand-over's own restart makes, so a
/// session started by hand after the delivery is not stopped here either); each forgotten, and
/// one resumed kept in [`RESUMED`]. A session whose request cannot be told, the store or the
/// session list unread, stays marked for the next tick.
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
        let standing = match asked.get(session).filter(|_| repo != "-") {
            Some(entry) => match stands(tick, entry, session, &repo) {
                Ok(standing) => standing,
                Err(error) => {
                    tick.note(&format!("hand-over of {session}"), &error);
                    continue;
                }
            },
            None => false,
        };
        let done = if repo == "-" {
            Ok(false)
        } else if standing {
            tick.restart(&repo, session, "limit over the context threshold", None)
                .map(|()| false)
        } else {
            resume(tick, &repo, session)
        };
        forget(tick, session);
        match done {
            Ok(true) => {
                let mut resumed = read_json(&tick.dir, RESUMED, json!({}));
                resumed[session.as_str()] = json!(nanos(SystemTime::now()).to_string());
                if let Err(error) = write_json(&tick.dir, RESUMED, &resumed) {
                    tick.note(RESUMED, &error);
                }
            }
            Ok(false) => {}
            Err(error) => tick.note(&format!("resume {session}"), &error),
        }
    }
    Ok(())
}

/// Resumes `session` of `repo` from its transcript, with the controller role's launch and so on
/// its model, and writes `resumed`; answers whether it did. Nothing when there is nothing to
/// resume (no measurement, or a live session).
///
/// A hand-over asked of the session and still kept is dated `resumed`, the instant the resume
/// returned (`story:handover-restart-by-controller-id`): the session runs on under its id in a
/// process started after the delivery, by the loop and not by hand, so [`superseded`] takes a
/// start up to that instant as the loop's own.
fn resume(tick: &mut Tick<'_, '_>, repo: &str, session: &str) -> Result<bool> {
    let resumed = crate::spawn::resume(
        tick.config,
        tick.state,
        repo,
        &SessionName(session.to_owned()),
        tick.seams.providers,
    )?;
    if resumed.is_some() {
        let mut asked = read_json(&tick.dir, HANDOVER, json!({}));
        if let Some(entry) = asked.get_mut(session).and_then(Value::as_object_mut) {
            entry.insert(
                "resumed".to_owned(),
                json!(nanos(SystemTime::now()).to_string()),
            );
            if let Err(error) = write_json(&tick.dir, HANDOVER, &asked) {
                tick.note(HANDOVER, &error);
            }
        }
        let mut written = tick.line(DispatchLogEvent::Resumed, repo)?;
        written.session = Some(session.to_owned());
        tick.write(&written)?;
    }
    Ok(resumed.is_some())
}

// ---------------------------------------------------------------------------------------------
// Relay and red CI
// ---------------------------------------------------------------------------------------------

/// A pull request's checks or `main`'s CI: relayed to the running controller of `repo`; a red
/// `main` with none dispatched ([`red_ci`]).
fn relay(tick: &mut Tick<'_, '_>, line: &str, repo: &str) -> Result<bool> {
    if let Some((controller, session)) = tick.running(repo)? {
        send(
            tick,
            &controller,
            &session,
            &format!("[REPORT {repo} -] watch: {line}"),
        )?;
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
    let Some((controller, _)) = tick.running(&repo)?.filter(|(_, session)| session == name) else {
        return Ok(false);
    };
    let mut asked = read_json(&tick.dir, STALLED, json!({}));
    if asked.get(name).is_some() {
        tick.restart(&repo, name, "stalled again", None)?;
        return Ok(true);
    }
    send(
        tick,
        &controller,
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

/// A gone session: the instance's conductor-dev started again while its plan holds an active
/// story ([`conductor_dev`]); a controller restarted when its repository has an open dispatch
/// and no usage limit marked it.
fn gone(tick: &mut Tick<'_, '_>, name: &str) -> Result<bool> {
    if let Some(acted) = conductor_dev(tick, name)? {
        return Ok(acted);
    }
    if read_json(&tick.dir, LIMITED, json!({})).get(name).is_some() {
        return Ok(false);
    }
    let Some(repo) = tick.repo_of(name)? else {
        return Ok(false);
    };
    if !tick.open_dispatch(&repo)? {
        return Ok(false);
    }
    tick.restart(&repo, name, "gone", None)?;
    Ok(true)
}

// ---------------------------------------------------------------------------------------------
// conductor-dev
// ---------------------------------------------------------------------------------------------

/// How long `conductor session start --role maintainer` may take.
const START_BOUND: Duration = Duration::from_secs(120);

/// The gone session `name` when it is the instance's conductor-dev, the maintainer role's
/// (`story:loop-restarts-gone-conductor-dev`): `None` when it is not. Started again, as `conductor
/// session start --role maintainer` starts it, in the directory the watch read it gone from
/// ([`Watch::gone_directory`]), the product checkout, when that checkout's plan holds an `active`
/// story; its first prompt sends it to its newest hand-over. One `controller-restarted` line
/// names the session, the checkout as `repo` and the count of active stories. With none, nothing.
fn conductor_dev(tick: &mut Tick<'_, '_>, name: &str) -> Result<Option<bool>> {
    if !maintainer_names(tick).iter().any(|own| own == name) {
        return Ok(None);
    }
    let Some(cwd) = Watch::of_instance(tick.state)?.gone_directory(name)? else {
        return Err(anyhow!(
            "the watch kept no directory {name} worked in; it is not started again"
        ));
    };
    let active = active_stories(&cwd)?;
    if active == 0 {
        return Ok(Some(false));
    }
    start_maintainer(tick, &cwd)?;
    let root = Path::new(&tick.instance().checkouts.root);
    let repo = cwd
        .strip_prefix(root)
        .ok()
        .and_then(|below| below.components().next())
        .or_else(|| cwd.components().next_back())
        .map_or_else(
            || "-".to_owned(),
            |part| part.as_os_str().to_string_lossy().into_owned(),
        );
    let mut written = tick.line(DispatchLogEvent::ControllerRestarted, &repo)?;
    written.session = Some(name.to_owned());
    written.reason = Some(format!("conductor-dev gone; {active} active stories"));
    tick.write(&written)?;
    Ok(Some(true))
}

/// The session names the instance's maintainer may carry: its role's
/// ([`crate::config::host_maintainer`]), and its old-name alias.
fn maintainer_names(tick: &Tick<'_, '_>) -> Vec<String> {
    let instance = tick.instance();
    crate::config::host_maintainer(instance)
        .map(|role| {
            let name = role
                .session_name
                .unwrap_or_else(|| crate::config::session_name(instance, &role.role));
            crate::config::session_names_of(BuiltInRole::Maintainer, &name.0)
        })
        .unwrap_or_default()
}

/// How many stories the planning store of the checkout `checkout` holds `active`: `aep plan
/// artifact list --kind story --status active --format json` there; none without a store.
///
/// # Errors
///
/// `aep` does not run, fails, or prints no JSON list.
fn active_stories(checkout: &Path) -> Result<usize> {
    if !checkout.join(".engineering").join("planning").is_dir() {
        return Ok(0);
    }
    let shown = "aep plan artifact list --kind story --status active --format json";
    let output = crate::collect::run(
        Command::new("aep")
            .args([
                "plan", "artifact", "list", "--kind", "story", "--status", "active", "--format",
                "json",
            ])
            .current_dir(checkout)
            .stdin(Stdio::null()),
        crate::collect::BOUND,
    )
    .with_context(|| format!("run `{shown}` in {}", checkout.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "`{shown}` in {} {}",
            checkout.display(),
            crate::confine::exited(&output)
        ));
    }
    let stories: Vec<Value> = serde_json::from_slice(&output.stdout)
        .with_context(|| format!("`{shown}` printed no JSON list"))?;
    Ok(stories.len())
}

/// Runs `conductor session start --role maintainer` (this binary, the tick's `--config` and
/// `--state-dir`) in `cwd`.
fn start_maintainer(tick: &Tick<'_, '_>, cwd: &Path) -> Result<()> {
    let mut command = Command::new(&tick.seams.program);
    if let Some(config) = tick.config {
        command
            .arg("--config")
            .arg(std::path::absolute(config).unwrap_or_else(|_| config.to_owned()));
    }
    if let Some(state) = tick.state {
        command
            .arg("--state-dir")
            .arg(std::path::absolute(state).unwrap_or_else(|_| state.to_owned()));
    }
    command
        .args(["session", "start", "--role", "maintainer"])
        .env(crate::actor::DAEMON_VARIABLE, "1")
        .current_dir(cwd)
        .stdin(Stdio::null());
    let shown = "conductor session start --role maintainer";
    let output = crate::collect::run(&mut command, START_BOUND)
        .with_context(|| format!("run `{shown}` in {}", cwd.display()))?;
    if !output.status.success() {
        return Err(anyhow!(
            "`{shown}` in {} {}",
            cwd.display(),
            crate::confine::exited(&output)
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Disk, notifications, answers
// ---------------------------------------------------------------------------------------------

/// The `gc` a `disk low` line started, while it runs and until a tick writes its `cleanup` line.
struct Cleaning {
    /// The `disk low` line.
    line: String,
    /// The thread that runs it, which answers the line's evidence.
    thread: JoinHandle<String>,
}

/// The one [`Cleaning`] of this process.
static CLEANING: Mutex<Option<Cleaning>> = Mutex::new(None);

/// `disk low`: starts `gc` on a thread of its own, unless one was started and its `cleanup` line
/// is not written yet; [`cleaned`] writes that line.
fn disk_low(tick: &Tick<'_, '_>, line: &str) -> Result<bool> {
    let mut cleaning = CLEANING.lock().unwrap_or_else(PoisonError::into_inner);
    if cleaning.is_some() {
        return Ok(false);
    }
    let gc = tick.seams.gc;
    let thread = thread::Builder::new()
        .name("gc".to_owned())
        .spawn(move || evidence(gc))
        .context("start gc on a thread of its own")?;
    *cleaning = Some(Cleaning {
        line: line.to_owned(),
        thread,
    });
    Ok(true)
}

/// Runs `gc` and answers how it exited, the `cleanup` line's evidence.
fn evidence(gc: Cleaner) -> String {
    match gc(&GcArgs {
        dry_run: None,
        format: None,
    }) {
        Ok(code) if code == ExitCode::SUCCESS => "gc exited 0".to_owned(),
        Ok(_) => "gc exited 1: a row was refused".to_owned(),
        Err(error) => format!("gc exited 1: {}", one_line(&format!("{error:#}"))),
    }
}

/// Writes `cleanup` with the `disk low` line and how `gc` exited, for the `gc` a `disk low` line
/// started, once it ended; with `wait`, waits for it to end first. Nothing when none was started.
///
/// # Errors
///
/// The line cannot be written.
pub(crate) fn cleaned(tick: &mut Tick<'_, '_>, wait: bool) -> Result<()> {
    let mut cleaning = CLEANING.lock().unwrap_or_else(PoisonError::into_inner);
    if !cleaning
        .as_ref()
        .is_some_and(|running| wait || running.thread.is_finished())
    {
        return Ok(());
    }
    let Some(ended) = cleaning.take() else {
        return Ok(());
    };
    let evidence = ended
        .thread
        .join()
        .unwrap_or_else(|_| "gc exited 1: it panicked".to_owned());
    drop(cleaning);
    let mut written = tick.line(DispatchLogEvent::Cleanup, "-")?;
    written.line = Some(ended.line);
    written.evidence = Some(evidence);
    tick.write(&written)
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
pub(crate) fn answered(tick: &mut Tick<'_, '_>, id: &str) -> Result<bool> {
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

/// Delivers `text` to `session`, the running controller `controller`
/// (`story:deliver-to-stopped-controller`). A delivery refused `SessionNotReachable` records that
/// controller stopped, so no later tick delivers to it again and a delivery kept for its
/// repository waits for the next controller start; the error answered then names both, so the
/// journal says it in one line.
///
/// A refusal is no proof that the session ended (`story:deliver-reaches-bg-session`): a session
/// the session list (`claude agents --json`, [`crate::confine::live_session`]) names live keeps
/// its controller Running, as does a list that cannot be read, which is how `spawn` reads a
/// failed launch; the one line says which.
///
/// # Errors
///
/// The deliverer refused the text.
fn send(tick: &Tick<'_, '_>, controller: &str, session: &str, text: &str) -> Result<()> {
    offer(tick, controller, session, text).map_err(|refusal| refusal.error)
}

/// Why a delivery to a running controller was refused, and what that leaves of the controller.
struct Refusal {
    /// The one line for the journal.
    error: anyhow::Error,
    /// The refusal is `SessionNotReachable`, which `session deliver` answers only while nothing
    /// was typed, and the controller stays Running, because its session is live or the session
    /// list was not read: a kept delivery is offered to this controller again, [`OFFERS`] times
    /// at most, since no other one starts while it runs. Every other refusal is false here: the
    /// text may be in front of the session (`DeliveryNotSeenRead`, a delivery past its bound),
    /// or the controller was recorded stopped.
    again: bool,
}

/// How often a kept delivery is offered to one controller that stays Running before it is
/// marked refused by it.
const OFFERS: u64 = 5;

/// [`send`], answering with a refusal whether the same controller is to be asked again.
fn offer(tick: &Tick<'_, '_>, controller: &str, session: &str, text: &str) -> Result<(), Refusal> {
    let Err(error) = tick.deliver(session, text) else {
        return Ok(());
    };
    let said = one_line(&format!("{error:#}"));
    if !said.contains("SessionNotReachable") {
        // The text may have been typed before the deliverer failed: not offered twice.
        return Err(Refusal {
            error,
            again: false,
        });
    }
    match crate::confine::live_session(session) {
        Ok(None) => {}
        Ok(Some(id)) => {
            return Err(Refusal {
                error: anyhow!(
                    "{said}; the session list names its session live ({id}), so its controller \
                     {controller} stays Running"
                ),
                again: true,
            });
        }
        Err(unread) => {
            return Err(Refusal {
                error: anyhow!(
                    "{said}; the session list was not read ({}), so its controller {controller} \
                     stays Running",
                    one_line(&format!("{unread:#}"))
                ),
                again: true,
            });
        }
    }
    let error = match record_stopped(tick, controller, &said) {
        Ok(()) => anyhow!(
            "{said}; its controller {controller} is recorded stopped, and \
             what waits for it is delivered at the repository's next controller start"
        ),
        Err(stop) => anyhow!(
            "{said}; its controller {controller} could not be recorded \
             stopped: {}",
            one_line(&format!("{stop:#}"))
        ),
    };
    Err(Refusal {
        error,
        again: false,
    })
}

/// Records the running controller `controller` stopped, as the delivery that found its session
/// unreachable said in `said`.
fn record_stopped(tick: &Tick<'_, '_>, controller: &str, said: &str) -> Result<()> {
    let mut generated = crate::controller::record(tick.state)?;
    let outcome = generated
        .stop_controller(StopController {
            actor: crate::actor::chosen(),
            controller_id: ControllerId(Uuid(controller.to_owned())),
            reason: format!("conductor run: a delivery was refused: {said}"),
        })
        .map_err(|unmet| anyhow!("{unmet}"))?;
    generated.ports.store.check().context("keep the stop")?;
    match outcome {
        StopControllerOutcome::Stopped { .. } => Ok(()),
        StopControllerOutcome::NoSuchController { .. } => {
            Err(anyhow!("the store holds no controller {controller}"))
        }
        StopControllerOutcome::WrongState { .. } => {
            Err(anyhow!("the controller {controller} is stopped already"))
        }
    }
}

/// What became of a delivery to a repository's controller.
enum Made {
    /// It was delivered, and its line written.
    Delivered,
    /// The repository has no running controller, or only the one that refused it.
    Waits,
    /// The running controller, by its id, refused it and is asked again ([`Refusal::again`]);
    /// this was its offer of that number.
    Again(String, u64),
    /// The running controller, by its id, refused it, and is not asked again; the refusal.
    Refused(String, String),
}

impl Made {
    /// `item` as it is kept after this; nothing once it was delivered.
    fn kept(self, mut item: Value) -> Option<Value> {
        match self {
            Self::Delivered => return None,
            Self::Waits => {}
            Self::Again(controller, offers) => {
                item["offers"] = json!({"controller": controller, "count": offers});
            }
            Self::Refused(controller, refusal) => {
                item["refused_by"] = json!(controller);
                item["refusal"] = json!(refusal);
                if let Some(fields) = item.as_object_mut() {
                    fields.remove("offers");
                }
            }
        }
        Some(item)
    }
}

/// Delivers `item` (`repo`, `text`, `event`, and the line's `id`, `grant` or `line`) to the
/// running controller of its repository and writes its line; keeps it in [`PENDING`] when there
/// is none, or when that controller refused it: for the next controller, or, when the refusal
/// left this one Running with nothing typed, for this one's next tick ([`OFFERS`] times at
/// most). A kept item names who refused it for good (`refused_by`) and that refusal
/// (`refusal`), or how often its running controller was offered it (`offers`).
///
/// # Errors
///
/// The store does not open, or the line or the kept file cannot be written.
pub(crate) fn deliver_or_keep(tick: &mut Tick<'_, '_>, item: Value) -> Result<()> {
    let Some(item) = made(tick, &item)?.kept(item) else {
        return Ok(());
    };
    let mut kept = read_json(&tick.dir, PENDING, json!([]));
    if let Some(list) = kept.as_array_mut() {
        list.push(item);
    }
    write_json(&tick.dir, PENDING, &kept)
}

/// Delivers `item` when its repository has a running controller other than the one that refused
/// it. A controller whose refusal leaves it Running is not marked as that one
/// (`story:deliver-reaches-bg-session`): it would hold the item for good, since no next
/// controller starts while it runs. It is offered the item again, until its refusal of offer
/// [`OFFERS`], which marks it with one journal line of its own.
fn made(tick: &mut Tick<'_, '_>, item: &Value) -> Result<Made> {
    let repo = item["repo"].as_str().unwrap_or_default();
    let Some((controller, session)) = tick.running(repo)? else {
        return Ok(Made::Waits);
    };
    if item["refused_by"].as_str() == Some(controller.as_str()) {
        return Ok(Made::Waits);
    }
    let text = item["text"].as_str().unwrap_or_default();
    if let Err(refusal) = offer(tick, &controller, &session, text) {
        tick.note(&format!("deliver to {session}"), &refusal.error);
        let said = one_line(&format!("{:#}", refusal.error));
        if !refusal.again {
            return Ok(Made::Refused(controller, said));
        }
        let offers = item["offers"]
            .as_object()
            .filter(|offers| offers["controller"].as_str() == Some(controller.as_str()))
            .and_then(|offers| offers["count"].as_u64())
            .unwrap_or(0)
            + 1;
        if offers < OFFERS {
            return Ok(Made::Again(controller, offers));
        }
        tick.note(
            &format!("deliver to {session}"),
            &anyhow!(
                "refused {offers} times, so it is not offered again to its controller \
                 {controller}, and waits for the repository's next controller start; the last \
                 refusal: {said}"
            ),
        );
        return Ok(Made::Refused(controller, said));
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
/// kept for the next controller, or for the same one while it stays Running and has not
/// refused it [`OFFERS`] times.
fn deliveries(tick: &mut Tick<'_, '_>) -> Result<()> {
    let kept = read_json(&tick.dir, PENDING, json!([]));
    let Some(items) = kept.as_array().filter(|items| !items.is_empty()) else {
        return Ok(());
    };
    let mut left = Vec::new();
    for item in items {
        left.extend(made(tick, item)?.kept(item.clone()));
    }
    write_json(&tick.dir, PENDING, &Value::Array(left)).context("keep the deliveries left")
}
