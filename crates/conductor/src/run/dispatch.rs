//! The cycle tick and dispatch v1, and the daily tick (docs/design/conductor-run.md § 3,
//! `story:run-daemon`, adopting `story:cycle-command`).
//!
//! **Cycle.** `snapshot start-snapshot`, `snapshot publish-board` and `records check`, each a run
//! of the conductor binary ([`super::Seams::program`]) with the daemon's global `--config` and
//! `--state-dir`, each to its end however it ends; then the controllers are reconciled with the
//! session list ([`reconcile`], `story:controllers-reconciled-with-live-sessions`), so a record
//! that outlived its session (a runtime restart, an OOM kill) holds no `controllers.max_working`
//! slot: each `Running` controller whose session the list (the `child` provider's sessions with
//! it) names neither live (a `pid`) nor `blocked` is recorded stopped with the reason, and the
//! `cycle` line names it; one whose repository has an open dispatch is then started again, its
//! `controller-restarted` line's reason `gone`, as after a `session gone` line. One whose session
//! a usage limit marked is recorded stopped with the reason `usage limit` and stays marked
//! (`story:limit-resume-at-reset`): it is not started again, dispatch v1 starts none for its
//! repository, which waits, and the watch tick past its reset resumes it from its transcript and
//! records it running again, within `controllers.max_working` ([`handlers::pending`]). A list that
//! does not read stops none, and the `cycle` line says so. Then dispatch v1. First the units of
//! work
//! (`story:control-plane-work-handler`): for each `Sent` dispatch with a `reference`, in the order
//! the store keeps them, whose repository has no running or paused controller, Active or not, the
//! controller is started for it as the work call starts one ([`super::start_work`]: under
//! `controllers.max_working`, the default charter where there is none); a refused start writes a
//! `start-refused` line with the tick's id. A unit whose start failed at the launch three times
//! (`StartFailed` or `LaunchFailed`, [`crate::dispatch::failed_starts`]) is tried no more: the
//! `cycle` line that wrote the third says it is left to the operator. Then for each repository Active
//! in `repository activity` that has a charter `<records>/charters/<repo>.md` and no running or
//! paused controller, in name order, `conductor spawn`, while fewer than `controllers.max_working`
//! controllers run; each controller started gets its repository's open dispatch, or one sent in
//! the same tick for the goal the repository serves, and its charter names it ([`Tick::spawn`]);
//! a repository whose charter is conductor's default one, unedited, gets none but for its open
//! dispatch, and waits. The units of work skip the `merge: human` review cap below: the work call
//! is the control plane telling the instance to work the unit. A repository whose charter has the line `merge: human` and that has at least
//! `controllers.max_open_reviews` pull requests waiting for review in the newest complete snapshot
//! (as the board counts them, [`crate::status::waiting_in`]) waits. One `cycle` line names what
//! each step did: `snapshot ok`, `board ok`, `records check ok` or how each ended, then `spawned
//! <repos>` (or `spawned none`) and each repository that waits, with why.
//!
//! **Daily.** `metric compute --day <the previous UTC day>`, then `status`, and the brief
//! `<records>/REPORT.md`: the day, each reading that misses its target, each reading that could not
//! be computed, and what `status` says asks for attention; deviations only, no reading that meets
//! its target; and its `Tuning` section, the lines of the tuner run after `status`
//! ([`crate::tune::daily`], `story:tuner`). Then the records directory, the brief with it, is
//! committed and pushed through the bot route for that day ([`super::commit_records`]), its
//! `records-committed` or `records-commit-refused` line carrying the tick's id.

use std::path::Path;
use std::time::Duration;

use anyhow::{Context as _, Result};
use conductor_model::behaviour::{ControllerStorage, DispatchStorage, SnapshotStorage};
use conductor_model::direction::Activity;
use conductor_model::dispatch::{ControllerState, DispatchLogEvent, DispatchState};
use conductor_model::primitives::Timestamp;
use conductor_model::session::TickKind;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use serde_json::{Value, json};

use super::{Route, Seams, Summary, Tick, commit_records, handlers, one_line, write_file};

/// How long one step of the cycle or the daily tick may run.
const STEP_BOUND: Duration = Duration::from_secs(3600);

/// A cycle tick, as the module documentation says.
///
/// # Errors
///
/// The tick cannot be started, its line not written, or the tick not recorded.
pub fn cycle_tick(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    seams: &Seams<'_>,
) -> Result<Summary> {
    let mut tick = Tick::start(config_flag, state, TickKind::Cycle, seams)?;
    let mut parts = vec![
        step(&tick, &["snapshot", "start-snapshot"], "snapshot"),
        step(&tick, &["snapshot", "publish-board"], "board"),
        checked(&tick),
    ];
    match reconcile(&mut tick) {
        Ok(Some(said)) => parts.push(said),
        Ok(None) => {}
        Err(error) => {
            tick.note("reconcile the controllers", &error);
            parts.push(format!(
                "controllers not reconciled: {}",
                one_line(&format!("{error:#}"))
            ));
        }
    }
    match dispatch_v1(&mut tick) {
        Ok(said) => parts.push(said),
        Err(error) => {
            tick.note("dispatch v1", &error);
            parts.push(format!(
                "dispatch v1 failed: {}",
                one_line(&format!("{error:#}"))
            ));
        }
    }
    let mut line = tick.line(DispatchLogEvent::Cycle, "-")?;
    line.line = Some(parts.join("; "));
    tick.write(&line)?;
    tick.finish()
}

/// Runs `conductor <words>` with the daemon's global flags, to its end within [`STEP_BOUND`].
fn run_step(tick: &Tick<'_, '_>, words: &[&str]) -> Result<std::process::Output> {
    let mut command = crate::instance::command(&tick.seams.program);
    if let Some(config) = tick.config {
        command.arg("--config").arg(config);
    }
    if let Some(state) = tick.state {
        command.arg("--state-dir").arg(state);
    }
    command.args(words).env(crate::actor::DAEMON_VARIABLE, "1");
    crate::collect::run(&mut command, STEP_BOUND)
        .with_context(|| format!("run conductor {}", words.join(" ")))
}

/// One step of the cycle: `<name> ok`, or how it ended with the first line it said.
fn step(tick: &Tick<'_, '_>, words: &[&str], name: &str) -> String {
    match run_step(tick, words) {
        Ok(output) if output.status.success() => format!("{name} ok"),
        Ok(output) => format!("{name} {}", crate::confine::exited(&output)),
        Err(error) => format!("{name} did not run: {}", one_line(&format!("{error:#}"))),
    }
}

/// `records check`: `records check ok`, or the number of findings it named.
fn checked(tick: &Tick<'_, '_>) -> String {
    match run_step(tick, &["records", "check"]) {
        Ok(output) if output.status.success() => "records check ok".to_owned(),
        Ok(output) if output.status.code() == Some(1) && !output.stdout.is_empty() => format!(
            "records check: {} findings",
            String::from_utf8_lossy(&output.stdout).lines().count()
        ),
        Ok(output) => format!("records check {}", crate::confine::exited(&output)),
        Err(error) => format!(
            "records check did not run: {}",
            one_line(&format!("{error:#}"))
        ),
    }
}

/// The `Running` controllers reconciled with the session list
/// (`story:controllers-reconciled-with-live-sessions`), as the module documentation says; what it
/// did, as the `cycle` line says it, or `None` when every one's session lives or waits.
///
/// # Errors
///
/// The store does not open, a line is not written, or a start fails other than by a refusal.
fn reconcile(tick: &mut Tick<'_, '_>) -> Result<Option<String>> {
    let mut parts = Vec::new();
    let mut stopped = Vec::new();
    {
        // No start counts the controllers while a record it would count is being stopped.
        let _one = super::STARTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let running: Vec<(String, String, String)> = tick
            .controllers()?
            .into_iter()
            .filter(|held| held.state == ControllerState::Running)
            .map(|held| {
                (
                    held.data.controller_id.0.0,
                    held.data.repository.0,
                    held.data.session_name.0,
                )
            })
            .collect();
        if running.is_empty() {
            return Ok(None);
        }
        let listed = match crate::confine::sessions() {
            Ok(listed) => listed,
            Err(unread) => {
                return Ok(Some(format!(
                    "controllers not reconciled: the session list was not read: {}",
                    one_line(&format!("{unread:#}"))
                )));
            }
        };
        let mut limited = super::read_json(&tick.dir, handlers::LIMITED, json!({}));
        for (controller, repo, session) in running {
            if lives_or_waits(&listed, &session) {
                continue;
            }
            // story:limit-resume-at-reset: a session a usage limit stopped whose process is gone
            // gives its place back and stays marked, to be resumed at its reset.
            let marked = limited.get(&session).is_some_and(Value::is_object);
            let reason = if marked {
                "usage limit".to_owned()
            } else {
                format!(
                    "conductor run: its session {session} is gone: the session list names no live \
                     session of that name"
                )
            };
            match handlers::stop_record(tick, &controller, &reason) {
                Ok(()) => {
                    if let Err(error) = crate::token::release_for_stop(tick.instance(), &session) {
                        tick.note(&format!("release the token of {session}"), &error);
                    }
                    if marked {
                        limited[session.as_str()]["stopped"] = json!(true);
                        if let Err(error) =
                            super::write_json(&tick.dir, handlers::LIMITED, &limited)
                        {
                            tick.note(handlers::LIMITED, &error);
                        }
                        parts.push(format!(
                            "stopped {repo} ({session}): usage limit, its process is gone; it is \
                             resumed at {}",
                            limited[session.as_str()]["resets_at"]
                                .as_str()
                                .unwrap_or("its reset")
                        ));
                        continue;
                    }
                    parts.push(format!("stopped {repo} ({session}): its session is gone"));
                    stopped.push((repo, session));
                }
                Err(error) => {
                    tick.note(&format!("stop the controller {controller}"), &error);
                    parts.push(format!(
                        "{repo} ({session}) not stopped: {}",
                        one_line(&format!("{error:#}"))
                    ));
                }
            }
        }
    }
    // As after `session gone`: a repository with an open dispatch starts again; the others are
    // left to dispatch v1.
    for (repo, session) in stopped {
        handlers::forget(tick, &session);
        if !tick.open_dispatch(&repo)? || tick.running(&repo)?.is_some() {
            continue;
        }
        match tick.start_again(&repo, &format!("its session {session} is gone"))? {
            Ok(true) => {
                let mut line = tick.line(DispatchLogEvent::ControllerRestarted, &repo)?;
                line.session = Some(session);
                line.reason = Some("gone".to_owned());
                tick.write(&line)?;
                parts.push(format!("restarted {repo}"));
            }
            Ok(false) => parts.push(format!(
                "{repo} has no charter to start again from: its open dispatches ended"
            )),
            Err(why) => parts.push(format!("{repo} waits: {why}")),
        }
    }
    Ok((!parts.is_empty()).then(|| parts.join("; ")))
}

/// Whether the session list `listed` names the session `session` live (an entry with a `pid`)
/// or waiting for headroom: an entry the list keeps `blocked`, as a usage limit leaves one.
fn lives_or_waits(listed: &[Value], session: &str) -> bool {
    listed
        .iter()
        .filter(|entry| entry["name"].as_str() == Some(session))
        .any(|entry| !entry["pid"].is_null() || entry["status"].as_str() == Some("blocked"))
}

/// The repositories whose controller the cycle recorded stopped at a usage limit, waiting to be
/// resumed at their reset ([`reconcile`]), each with that reset.
fn waiting_at_limit(tick: &Tick<'_, '_>) -> Vec<(String, String)> {
    super::read_json(&tick.dir, handlers::LIMITED, json!({}))
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, entry)| entry["stopped"] == true)
        .filter_map(|(_, entry)| {
            Some((
                entry["repo"].as_str()?.to_owned(),
                entry["resets_at"].as_str().unwrap_or("-").to_owned(),
            ))
        })
        .collect()
}

/// Dispatch v1, as the module documentation says; what it did, as the `cycle` line says it.
fn dispatch_v1(tick: &mut Tick<'_, '_>) -> Result<String> {
    let instance = tick.instance();
    let (active, mut controlled, full, units) = {
        let store = crate::state::open(tick.state)?;
        let units: Vec<(String, String, Timestamp)> = DispatchStorage::list(&store)
            .into_iter()
            .filter(|held| held.state == DispatchState::Sent && held.data.reference.is_some())
            .map(|held| {
                (
                    held.data.repository.0,
                    held.data.dispatch_id.0,
                    held.data.sent_at,
                )
            })
            .collect();
        let active: Vec<String> = crate::repository::activity(&store)
            .into_iter()
            .filter(|row| matches!(row.activity, Activity::Active))
            .map(|row| row.repository.0)
            .collect();
        let controlled: Vec<String> = ControllerStorage::list(&store)
            .into_iter()
            .filter(|held| {
                matches!(
                    held.state,
                    ControllerState::Running | ControllerState::Paused
                )
            })
            .map(|held| held.data.repository.0)
            .collect();
        let snapshots = SnapshotStorage::list(&store);
        let full = crate::status::newest_complete(&snapshots, None).map_or_else(
            Default::default,
            |held| {
                crate::status::review_full(
                    &crate::status::waiting_in(&store, held),
                    instance.controllers.max_open_reviews,
                )
            },
        );
        store
            .check()
            .context("read the repositories and controllers")?;
        (active, controlled, full, units)
    };
    let limit = instance.controllers.max_open_reviews;
    let mut spawned = Vec::new();
    let mut waits = Vec::new();
    // A repository whose controller waits for its usage limit's reset is resumed then, not
    // started afresh (story:limit-resume-at-reset).
    for (repo, at) in waiting_at_limit(tick) {
        if !controlled.contains(&repo) {
            waits.push(format!("{repo} waits: usage limit until {at}"));
            controlled.push(repo);
        }
    }
    // Units of work first, Active or not: each Sent dispatch with a reference whose repository
    // runs no controller (`story:control-plane-work-handler`).
    // A unit whose start failed at the launch `FAILED_STARTS_MOST` times is left to the operator.
    let records = Path::new(&instance.records);
    for (repo, id, sent_at) in &units {
        if controlled.contains(repo) {
            continue;
        }
        controlled.push(repo.clone());
        let failed = crate::dispatch::failed_starts(records, sent_at, repo, id)?;
        if failed >= crate::dispatch::FAILED_STARTS_MOST {
            continue;
        }
        match super::start_work(tick.config, tick.state, tick.seams.providers, repo, id) {
            None => {
                tick.wrote(DispatchLogEvent::Sent);
                spawned.push(repo.clone());
            }
            Some((refusal, detail)) => {
                let mut line = super::start_refused_line(repo, id, refusal, &detail)?;
                line.tick_id = Some(tick.id.clone());
                tick.write(&line)?;
                waits.push(format!(
                    "{repo} waits: {}: {detail}",
                    super::work_refusal_name(refusal)
                ));
                if crate::dispatch::failed_starts(records, sent_at, repo, id)?
                    >= crate::dispatch::FAILED_STARTS_MOST
                {
                    waits.push(format!(
                        "{id} of {repo} is left to the operator: {} starts of it failed, and the \
                         loop tries it no more",
                        crate::dispatch::FAILED_STARTS_MOST
                    ));
                }
            }
        }
    }
    for repo in &active {
        if controlled.contains(repo) || !tick.charter(repo).is_file() {
            continue;
        }
        if merge_human(&tick.charter(repo))
            && let Some(waiting) = full.get(repo)
        {
            waits.push(format!("{repo} waits: {waiting}/{limit} pull requests"));
            continue;
        }
        match tick.spawn(repo)? {
            Ok(()) => spawned.push(repo.clone()),
            Err(why) => waits.push(format!("{repo} waits: {why}")),
        }
    }
    let mut said = if spawned.is_empty() {
        "spawned none".to_owned()
    } else {
        format!("spawned {}", spawned.join(", "))
    };
    for wait in waits {
        said.push_str("; ");
        said.push_str(&wait);
    }
    Ok(said)
}

/// Whether the charter `file` has the line `merge: human`.
fn merge_human(file: &Path) -> bool {
    std::fs::read_to_string(file).is_ok_and(|text| {
        text.lines().any(|line| {
            line.trim()
                .strip_prefix("merge:")
                .is_some_and(|who| who.trim() == "human")
        })
    })
}

/// A daily tick for the UTC day `day`, as the module documentation says.
///
/// # Errors
///
/// The tick cannot be started, the brief not written, the commit's line not written, or the tick
/// not recorded.
pub fn daily_tick(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    day: &str,
    seams: &Seams<'_>,
) -> Result<Summary> {
    let mut tick = Tick::start(config_flag, state, TickKind::Daily, seams)?;
    let mut metrics = Vec::new();
    match run_step(&tick, &["metric", "compute", "--day", day]) {
        Ok(output) => {
            metrics.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|line| line.contains("misses its target") || line.contains("no value"))
                    .map(|line| format!("- {line}")),
            );
            if !output.status.success() {
                metrics.push(format!(
                    "- metric compute {}",
                    crate::confine::exited(&output)
                ));
            }
        }
        Err(error) => metrics.push(format!(
            "- metric compute did not run: {}",
            one_line(&format!("{error:#}"))
        )),
    }
    let attention: Vec<String> = match run_step(&tick, &["status"]) {
        Ok(output) => String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| format!("- {line}"))
            .collect(),
        Err(error) => vec![format!(
            "- status did not run: {}",
            one_line(&format!("{error:#}"))
        )],
    };
    let tuning = crate::tune::daily(tick.config, tick.state, day);
    let section = |lines: &[String]| {
        if lines.is_empty() {
            "None.".to_owned()
        } else {
            lines.join("\n")
        }
    };
    let written = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format the clock")?;
    let brief = format!(
        "# Report {day}\n\n\
         Written by `conductor run` at {written} for the UTC day {day}; deviations only.\n\n\
         ## Metrics\n\n{}\n\n\
         ## Attention\n\n{}\n\n\
         ## Tuning\n\n{}\n",
        section(&metrics),
        section(&attention),
        section(&tuning)
    );
    let records = Path::new(&tick.instance().records);
    write_file(&records.join("REPORT.md"), &brief)?;
    let route = Route::new(
        tick.instance().forge.route,
        &tick.seams.program,
        tick.config,
        tick.state,
    )
    .with_forge_access(tick.instance().forge_access);
    if let Some(mut line) = commit_records(records, &route, day).line()? {
        line.tick_id = Some(tick.id.clone());
        tick.write(&line)?;
    }
    tick.finish()
}
