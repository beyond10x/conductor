//! The cycle tick and dispatch v1, and the daily tick (docs/design/conductor-run.md § 3,
//! `story:run-daemon`, adopting `story:cycle-command`).
//!
//! **Cycle.** `snapshot start-snapshot`, `snapshot publish-board` and `records check`, each a run
//! of the conductor binary ([`super::Seams::program`]) with the daemon's global `--config` and
//! `--state-dir`, each to its end however it ends; then dispatch v1: for each repository Active
//! in `repository activity` that has a charter `<records>/charters/<repo>.md` and no running or
//! paused controller, in name order, `conductor spawn`, while fewer than `controllers.max_working`
//! controllers run. A repository whose charter has the line `merge: human` and that has at least
//! `controllers.max_open_reviews` pull requests waiting for review in the newest complete snapshot
//! (as the board counts them, [`crate::status::waiting_in`]) waits. One `cycle` line names what
//! each step did: `snapshot ok`, `board ok`, `records check ok` or how each ended, then `spawned
//! <repos>` (or `spawned none`) and each repository that waits, with why.
//!
//! **Daily.** `metric compute --day <the previous UTC day>`, then `status`, and the brief
//! `<records>/REPORT.md`: the day, each reading that misses its target, each reading that could not
//! be computed, and what `status` says asks for attention; deviations only, no reading that meets
//! its target.

use std::path::Path;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context as _, Result};
use conductor_model::behaviour::{ControllerStorage, SnapshotStorage};
use conductor_model::direction::Activity;
use conductor_model::dispatch::{ControllerState, DispatchLogEvent};
use conductor_model::session::TickKind;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{Seams, Summary, Tick, one_line, write_file};

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
    let mut command = Command::new(&tick.seams.program);
    if let Some(config) = tick.config {
        command.arg("--config").arg(config);
    }
    if let Some(state) = tick.state {
        command.arg("--state-dir").arg(state);
    }
    command.args(words);
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

/// Dispatch v1, as the module documentation says; what it did, as the `cycle` line says it.
fn dispatch_v1(tick: &mut Tick<'_, '_>) -> Result<String> {
    let instance = tick.instance();
    let (active, controlled, full) = {
        let store = crate::state::open(tick.state)?;
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
        (active, controlled, full)
    };
    let limit = instance.controllers.max_open_reviews;
    let mut spawned = Vec::new();
    let mut waits = Vec::new();
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
/// The tick cannot be started, the brief not written, or the tick not recorded.
pub fn daily_tick(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    day: &str,
    seams: &Seams<'_>,
) -> Result<Summary> {
    let tick = Tick::start(config_flag, state, TickKind::Daily, seams)?;
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
         ## Attention\n\n{}\n",
        section(&metrics),
        section(&attention)
    );
    write_file(
        &Path::new(&tick.instance().records).join("REPORT.md"),
        &brief,
    )?;
    tick.finish()
}
