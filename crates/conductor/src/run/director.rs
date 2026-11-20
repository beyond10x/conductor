//! The keeper of a boxed instance's director (`story:box-runtime-starts-director`, issue #220).
//!
//! Each watch tick of a runtime that runs in its instance's substrate box
//! ([`crate::collect::checkouts::in_box`]) runs [`keep`] after the pending work
//! ([`super::handlers::pending`]). It acts only when the instance names a director role
//! ([`config::director`]) and its file writes its placement (a `conductor.config/2` file; a `/1`
//! file and placement `none` are unboxed, [`boxed`]). Unboxed, nothing here runs: the director is
//! started by hand.
//!
//! - **Running.** A director runs when the box's registry of child sessions
//!   ([`crate::confine::child`]) holds a live entry under one of the instance's director names
//!   ([`config::director_names`]), whoever started it: a runtime restarted by the daemon finds the
//!   director it started before and starts none.
//! - **Starting.** When none runs and no wait is pending, the keeper runs `conductor session start
//!   --role director` as the loop starts conductor-dev ([`super::handlers::start_role`]): in the
//!   box, in the records directory, under `<session_prefix>-director`, with the profile the binary
//!   carries. It holds [`super::STARTS`] and reads the registry again under it; `session start`
//!   itself refuses `SessionRunning` under its start lock, so a start by hand and the keeper's
//!   cannot both start one. Each start writes a `director-started` line with `session` and
//!   `reason`: `none ran`, `gone`, or `failed: <line>`, the previous start's failure.
//! - **Back-off.** `attempts` (n) counts the starts since the last session that lived at least
//!   [`LIVED`]; before start n+1 the keeper waits [`crate::limit::backoff`]`(n - 1)` after the last
//!   start: at once for the first start and the first restart after an end, then 5, 10, 20, 40 and
//!   at most 60 minutes.
//! - **Usage limits.** While `limited.json` marks the director's session the keeper copies the
//!   mark's reset into its record (state `Limited`) and waits for it on its own copy, since
//!   [`super::handlers`] forgets a mark of no repository once its reset passes; then it starts a
//!   fresh session. A limited director whose process stays alive is a live entry: left alone.
//! - **Stopped.** `session stop --role director` holds the keeper ([`hold`], [`HELD`]): it writes
//!   state `Stopped` and starts nothing until a start resumes it ([`release`]).
//! - **Failures.** `failures` counts the starts in a row no later tick saw running; at
//!   [`FAILURES_MOST`] the keeper raises the alert [`crate::alert::DIRECTOR_NOT_STARTED`] once while
//!   they last, naming the last start's failure line, and keeps trying at the back-off.
//!
//! The record is `<state>/run/director.json` ([`KEEP`], a `conductor.session.DirectorKeep`), which
//! only the keeper writes; doctor and the dashboard read it ([`read`], [`shown`], [`doctor_line`])
//! and never create it.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use conductor_model::config::{Instance, SessionName};
use conductor_model::dispatch::{DispatchLogEvent, DispatchLogLine};
use conductor_model::primitives::Timestamp;
use conductor_model::session::{DirectorKeep, DirectorState};
use serde_json::{Map, Value, json};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use super::Tick;
use crate::config;
use crate::confine::child;

/// The keeper's record, in `<state>/run/`.
pub const KEEP: &str = "director.json";

/// The hold `session stop --role director` writes in `<state>/run/` and a start removes.
pub const HELD: &str = "director.held";

/// How long a director must have lived for its end to start the back-off over.
pub const LIVED: Duration = crate::limit::BACKOFF_MOST;

/// The failed starts in a row at which the keeper raises its alert.
pub const FAILURES_MOST: i64 = 3;

/// The reason an alert gives for starts whose sessions ended before a tick saw them run.
const ENDED_UNSEEN: &str = "each started session ended before a tick saw it run";

/// Whether `instance` is one whose runtime keeps a director: its file writes its placement (not a
/// `conductor.config/1` file, whose placement is its confinement's provider) and that placement
/// opens a substrate box.
#[must_use]
pub fn boxed(instance: &Instance) -> bool {
    !config::legacy(instance).shown && crate::placement::boxed(instance)
}

/// The keeper's record under the state directory `state`, if it wrote one; never created here.
#[must_use]
pub fn read(state: &Path) -> Option<DirectorKeep> {
    let text = fs::read_to_string(state.join(super::RUN_DIR).join(KEEP)).ok()?;
    from_json(&serde_json::from_str(&text).ok()?)
}

/// Holds the keeper of the state directory `state`: the director was stopped by hand.
///
/// # Errors
///
/// The hold cannot be written.
pub fn hold(state: &Path) -> Result<()> {
    let dir = state.join(super::RUN_DIR);
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let file = dir.join(HELD);
    fs::write(&file, "stopped by session stop\n")
        .with_context(|| format!("write {}", file.display()))
}

/// Releases the hold of the state directory `state`, if any: the director was resumed.
///
/// # Errors
///
/// The hold is there and cannot be removed.
pub fn release(state: &Path) -> Result<()> {
    let file = state.join(super::RUN_DIR).join(HELD);
    match fs::remove_file(&file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", file.display())),
    }
}

/// The state's name as the specification spells it.
const fn state_name(state: DirectorState) -> &'static str {
    match state {
        DirectorState::Running => "Running",
        DirectorState::Waiting => "Waiting",
        DirectorState::Limited => "Limited",
        DirectorState::Stopped => "Stopped",
    }
}

/// The state `name` spells, if any.
fn state_named(name: &str) -> Option<DirectorState> {
    [
        DirectorState::Running,
        DirectorState::Waiting,
        DirectorState::Limited,
        DirectorState::Stopped,
    ]
    .into_iter()
    .find(|state| state_name(*state) == name)
}

/// `keep` as `director.json` holds it: every field, an absent one left out.
fn to_json(keep: &DirectorKeep) -> Value {
    let mut map = Map::new();
    map.insert("session".to_owned(), json!(keep.session.0));
    map.insert("state".to_owned(), json!(state_name(keep.state)));
    map.insert("attempts".to_owned(), json!(keep.attempts));
    map.insert("failures".to_owned(), json!(keep.failures));
    for (key, value) in [
        ("started_at", keep.started_at.as_ref().map(|at| &at.0)),
        ("next_start_at", keep.next_start_at.as_ref().map(|at| &at.0)),
        ("reason", keep.reason.as_ref()),
    ] {
        if let Some(value) = value {
            map.insert(key.to_owned(), json!(value));
        }
    }
    Value::Object(map)
}

/// The record `value` holds, if it reads as one.
fn from_json(value: &Value) -> Option<DirectorKeep> {
    let text = |key: &str| value[key].as_str().map(str::to_owned);
    Some(DirectorKeep {
        session: SessionName(text("session")?),
        state: state_named(value["state"].as_str()?)?,
        attempts: value["attempts"].as_i64().unwrap_or(0),
        failures: value["failures"].as_i64().unwrap_or(0),
        started_at: text("started_at").map(Timestamp),
        next_start_at: text("next_start_at").map(Timestamp),
        reason: text("reason"),
    })
}

/// The instant `at` holds, if it reads as RFC 3339.
fn instant(at: Option<&Timestamp>) -> Option<OffsetDateTime> {
    at.and_then(|at| OffsetDateTime::parse(&at.0, &Rfc3339).ok())
}

/// `at` as a record's timestamp.
fn stamp(at: OffsetDateTime) -> Timestamp {
    Timestamp(at.format(&Rfc3339).unwrap_or_default())
}

/// The director of the row the dashboard shows for `instance`, from the state directory `state`
/// (`/v1/data.json`'s `director`): `null` for an unboxed instance ([`boxed`]); else `state`,
/// `attempts`, `next_start_at` and `reason` of the keeper's record, each `null` (attempts 0)
/// before the runtime tried.
#[must_use]
pub fn shown(instance: &Instance, state: &Path) -> Value {
    if !boxed(instance) {
        return Value::Null;
    }
    match read(state) {
        Some(keep) => json!({
            "state": state_name(keep.state),
            "attempts": keep.attempts,
            "next_start_at": keep.next_start_at.map(|at| at.0),
            "reason": keep.reason,
        }),
        None => json!({"state": null, "attempts": 0, "next_start_at": null, "reason": null}),
    }
}

/// Doctor's line for the boxed instance `instance` whose director is `session`: `director
/// <instance>: <live> runs in its box` when `live` names the live session, else `director
/// <instance>: no director session runs in its box (<session>); ` and what the keeper's record
/// `keep` says at `now`: the runtime has not tried yet (no record), it is stopped until resumed,
/// or the runtime starts it at once or again at its next start, with the last start's failure.
#[must_use]
pub fn doctor_line(
    instance: &Instance,
    session: &str,
    live: Option<&str>,
    keep: Option<&DirectorKeep>,
    now: OffsetDateTime,
) -> String {
    let name = &instance.name.0;
    if let Some(live) = live {
        return format!("director {name}: {live} runs in its box");
    }
    let what = match keep {
        None => "the runtime has not tried yet".to_owned(),
        Some(keep) if keep.state == DirectorState::Stopped => {
            "stopped, the runtime starts it again once it is resumed".to_owned()
        }
        Some(keep) => {
            let when = match instant(keep.next_start_at.as_ref()).filter(|at| *at > now) {
                Some(at) => format!("again at {}", stamp(at).0),
                None => "at once".to_owned(),
            };
            let failed = keep
                .reason
                .as_ref()
                .map(|reason| format!(": last start failed: {reason}"))
                .unwrap_or_default();
            format!("the runtime starts it {when}{failed}")
        }
    };
    format!("director {name}: no director session runs in its box ({session}); {what}")
}

/// What the keeper reads and does through, so a test gives its own clock, registry and start.
pub struct Seam<'a> {
    /// Now.
    pub now: OffsetDateTime,
    /// The live director session of a name, with when it started where the registry says.
    pub live: &'a dyn Fn(&str) -> Option<Option<OffsetDateTime>>,
    /// Starts the director; its failure line when it did not start.
    pub start: &'a mut dyn FnMut() -> std::result::Result<(), String>,
    /// Appends a dispatch log line.
    pub record: &'a mut dyn FnMut(&DispatchLogLine) -> Result<()>,
    /// Sends a raised alert to the operator's notifier.
    pub notify: bool,
}

/// What one pass of the keeper did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kept {
    /// The record it wrote.
    pub keep: DirectorKeep,
    /// The reason of the start it made, if it made one that started.
    pub started: Option<String>,
    /// Whether it raised the alert.
    pub alerted: bool,
}

/// One pass of the keeper over the run directory `dir` of the instance `instance` whose director
/// is `session` under the names `names`, through `seam`: reads its record, the hold and the
/// usage-limit marks there, starts the director when it is due, and writes its record.
///
/// # Errors
///
/// The record cannot be written, or a line not appended.
pub fn pass(
    dir: &Path,
    instance: &str,
    session: &SessionName,
    names: &[String],
    seam: &mut Seam<'_>,
) -> Result<Kept> {
    let now = seam.now;
    let before = super::read_json(dir, KEEP, Value::Null);
    let before = from_json(&before);
    let mut keep = before.clone().unwrap_or(DirectorKeep {
        session: session.clone(),
        state: DirectorState::Waiting,
        attempts: 0,
        failures: 0,
        started_at: None,
        next_start_at: None,
        reason: None,
    });
    keep.session = session.clone();
    let kept = |keep: DirectorKeep, started: Option<String>, alerted: bool| -> Result<Kept> {
        super::write_json(dir, KEEP, &to_json(&keep))?;
        Ok(Kept {
            keep,
            started,
            alerted,
        })
    };
    // A live director, whoever started it, is running: its start is confirmed.
    if let Some(started) = names.iter().find_map(|name| (seam.live)(name)) {
        if keep.state != DirectorState::Running || keep.started_at.is_none() {
            keep.started_at = Some(stamp(started.unwrap_or(now)));
        }
        keep.state = DirectorState::Running;
        keep.failures = 0;
        keep.next_start_at = None;
        keep.reason = None;
        crate::alert::clear(dir, crate::alert::DIRECTOR_NOT_STARTED, |_| false)?;
        return kept(keep, None, false);
    }
    if dir.join(HELD).exists() {
        keep.state = DirectorState::Stopped;
        keep.attempts = 0;
        keep.failures = 0;
        keep.started_at = None;
        keep.next_start_at = None;
        keep.reason = None;
        return kept(keep, None, false);
    }
    let was = before.as_ref().map(|before| before.state);
    // A director that ended after it lived an hour starts the back-off over.
    if was == Some(DirectorState::Running)
        && instant(keep.started_at.as_ref()).is_some_and(|at| now - at >= LIVED)
    {
        keep.attempts = 0;
    }
    let mut alerted = false;
    if keep.failures >= FAILURES_MOST {
        alerted |= alert(dir, instance, &keep, seam)?;
    }
    // A usage limit's reset, copied while the mark is there and waited for on the copy.
    let marked = super::read_json(dir, super::handlers::LIMITED, json!({}));
    let reset = names
        .iter()
        .filter_map(|name| marked.get(name))
        .filter_map(|entry| entry["resets_at"].as_str())
        .filter_map(|at| OffsetDateTime::parse(at, &Rfc3339).ok())
        .max();
    if let Some(reset) = reset.filter(|reset| *reset > now) {
        keep.state = DirectorState::Limited;
        keep.next_start_at = Some(stamp(reset));
    }
    let due = if keep.state == DirectorState::Limited {
        instant(keep.next_start_at.as_ref()).filter(|at| *at > now)
    } else if keep.attempts == 0 {
        None
    } else {
        let wait = crate::limit::backoff(
            u32::try_from(keep.attempts.saturating_sub(1)).unwrap_or(u32::MAX),
        );
        instant(keep.started_at.as_ref())
            .map(|at| at + wait)
            .filter(|at| *at > now)
    };
    if let Some(due) = due {
        if keep.state != DirectorState::Limited {
            keep.state = DirectorState::Waiting;
        }
        keep.next_start_at = Some(stamp(due));
        return kept(keep, None, alerted);
    }
    let reason = match &keep.reason {
        Some(failed) => format!("failed: {failed}"),
        None if before.is_some_and(|before| {
            before.started_at.is_some() && before.state != DirectorState::Stopped
        }) =>
        {
            "gone".to_owned()
        }
        None => "none ran".to_owned(),
    };
    keep.attempts = keep.attempts.saturating_add(1);
    keep.failures = keep.failures.saturating_add(1);
    keep.started_at = Some(stamp(now));
    match (seam.start)() {
        Ok(()) => {
            keep.state = DirectorState::Running;
            keep.next_start_at = None;
            keep.reason = None;
            let mut line = crate::records::dispatch_line(DispatchLogEvent::DirectorStarted, "-")?;
            line.session = Some(session.0.clone());
            line.reason = Some(reason.clone());
            (seam.record)(&line)?;
            kept(keep, Some(reason), alerted)
        }
        Err(failed) => {
            keep.state = DirectorState::Waiting;
            keep.reason = Some(super::one_line(&failed));
            let wait = crate::limit::backoff(
                u32::try_from(keep.attempts.saturating_sub(1)).unwrap_or(u32::MAX),
            );
            keep.next_start_at = Some(stamp(now + wait));
            if keep.failures >= FAILURES_MOST {
                alerted |= alert(dir, instance, &keep, seam)?;
            }
            kept(keep, None, alerted)
        }
    }
}

/// Raises the alert of a director the keeper could not start, once while its failures last.
fn alert(dir: &Path, instance: &str, keep: &DirectorKeep, seam: &mut Seam<'_>) -> Result<bool> {
    let reason = format!(
        "{} starts of the director {} in a row did not run: {}",
        keep.failures,
        keep.session.0,
        keep.reason.as_deref().unwrap_or(ENDED_UNSEEN)
    );
    let line = crate::alert::line(crate::alert::DIRECTOR_NOT_STARTED, instance, None, &reason)?;
    let record = &mut *seam.record;
    let raised = crate::alert::mark(dir, &line, |line| record(line))?;
    if raised
        && seam.notify
        && let Err(error) = crate::alert::notify(instance, "alert", &reason)
    {
        eprintln!("conductor alert: notify-send: {error:#}");
    }
    Ok(raised)
}

/// The keeper's pass in the watch tick `tick` ([`pass`]), as the module documentation says: only
/// in the box of an instance that names a director role.
pub fn keep(tick: &mut Tick<'_, '_>) {
    let instance = tick.instance();
    if !boxed(instance) || !crate::collect::checkouts::in_box(instance) {
        return;
    }
    let Some(role) = config::director(instance) else {
        return;
    };
    let session = config::session_of_role(instance, role);
    let names = config::director_names(instance, &session.0);
    let Some(registry) = child::registry_here() else {
        tick.note(
            "director",
            &anyhow::anyhow!("HOME is not an absolute path, so the box's registry is unread"),
        );
        return;
    };
    let live = |name: &str| {
        child::live_named(&registry, name).map(|entry| {
            OffsetDateTime::from_unix_timestamp_nanos(i128::from(entry.started_at) * 1_000_000).ok()
        })
    };
    let records = std::path::PathBuf::from(&instance.records);
    let dir = tick.dir.clone();
    let id = tick.id.clone();
    let mut lines: Vec<DispatchLogLine> = Vec::new();
    let passed = {
        let _one = super::STARTS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let shared: &Tick<'_, '_> = tick;
        let mut start = || {
            super::handlers::start_role(shared, "director", &records)
                .map_err(|error| format!("{error:#}"))
        };
        let mut record = |line: &DispatchLogLine| {
            let mut line = line.clone();
            line.tick_id = Some(id.clone());
            lines.push(line);
            Ok(())
        };
        pass(
            &dir,
            &instance.name.0,
            &session,
            &names,
            &mut Seam {
                now: OffsetDateTime::now_utc(),
                live: &live,
                start: &mut start,
                record: &mut record,
                notify: true,
            },
        )
    };
    for line in &lines {
        if let Err(error) = tick.write(line) {
            tick.note("director", &error);
        }
    }
    if let Err(error) = passed {
        tick.note("director", &error);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;

    use super::*;

    /// A run directory of its own for `name`, empty.
    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("conductor-director-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a directory");
        dir
    }

    fn at(minutes: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_800_000_000).expect("an instant")
            + Duration::minutes(minutes)
    }

    const SESSION: &str = "ex-director";

    /// A keeper over `dir` with an injected clock, registry and start.
    struct World {
        dir: PathBuf,
        /// The run directory the keeper reads and writes: `dir`, unless a case moves it.
        run: PathBuf,
        names: Vec<String>,
        /// The live sessions, by name, with when each started.
        live: RefCell<Vec<(String, OffsetDateTime)>>,
        /// What the next start answers.
        fails: RefCell<Option<String>>,
        /// The starts made.
        starts: Cell<usize>,
        /// The lines recorded.
        lines: RefCell<Vec<DispatchLogLine>>,
    }

    impl World {
        fn new(name: &str) -> Self {
            let dir = scratch(name);
            Self {
                run: dir.clone(),
                dir,
                names: vec![SESSION.to_owned()],
                live: RefCell::new(Vec::new()),
                fails: RefCell::new(None),
                starts: Cell::new(0),
                lines: RefCell::new(Vec::new()),
            }
        }

        /// One pass at `now`; a start that starts makes the director live from `now`.
        fn pass(&self, now: OffsetDateTime) -> Kept {
            let live = |name: &str| {
                self.live
                    .borrow()
                    .iter()
                    .find(|(each, _)| each == name)
                    .map(|(_, started)| Some(*started))
            };
            let mut start = || {
                self.starts.set(self.starts.get() + 1);
                match self.fails.borrow().clone() {
                    Some(line) => Err(line),
                    None => {
                        self.live.borrow_mut().push((SESSION.to_owned(), now));
                        Ok(())
                    }
                }
            };
            let mut record = |line: &DispatchLogLine| {
                self.lines.borrow_mut().push(line.clone());
                Ok(())
            };
            pass(
                &self.run,
                "example-org",
                &SessionName(SESSION.to_owned()),
                &self.names,
                &mut Seam {
                    now,
                    live: &live,
                    start: &mut start,
                    record: &mut record,
                    notify: false,
                },
            )
            .expect("the keeper's pass")
        }

        /// The director's session ends.
        fn end(&self) {
            self.live.borrow_mut().clear();
        }

        fn started_lines(&self) -> Vec<DispatchLogLine> {
            self.lines
                .borrow()
                .iter()
                .filter(|line| line.event == DispatchLogEvent::DirectorStarted)
                .cloned()
                .collect()
        }
    }

    impl Drop for World {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    /// The first start and the first restart after an end at once; each further early end waits
    /// 5, 10, 20, 40, then 60 minutes after the last start.
    #[test]
    fn the_keeper_backs_off_after_starts_that_end_early() {
        let world = World::new("backoff");
        let mut now = at(0);
        let mut waits = Vec::new();
        let mut last: Option<OffsetDateTime> = None;
        for _ in 0..8 {
            let before = world.starts.get();
            let mut waited = 0;
            // A tick each minute until the keeper starts the director.
            loop {
                let kept = world.pass(now);
                if world.starts.get() > before {
                    break;
                }
                assert_eq!(kept.keep.state, DirectorState::Waiting, "{kept:?}");
                waited = (instant(kept.keep.next_start_at.as_ref()).expect("a next start")
                    - instant(kept.keep.started_at.as_ref()).expect("a last start"))
                .whole_minutes();
                now += Duration::minutes(1);
            }
            if let Some(last) = last
                && waited > 0
            {
                assert_eq!((now - last).whole_minutes(), waited, "started when due");
            }
            waits.push(waited);
            last = Some(now);
            // It ends a minute later, seen by the next tick.
            world.end();
            now += Duration::minutes(1);
        }
        assert_eq!(waits, [0, 0, 5, 10, 20, 40, 60, 60]);
        let reasons: Vec<String> = world
            .started_lines()
            .iter()
            .filter_map(|line| line.reason.clone())
            .collect();
        assert_eq!(reasons[0], "none ran");
        assert!(
            reasons[1..].iter().all(|reason| reason == "gone"),
            "{reasons:?}"
        );
        assert_eq!(world.started_lines()[0].session.as_deref(), Some(SESSION));
    }

    /// A director that lived an hour or more is started again at once, its back-off over.
    #[test]
    fn a_director_that_lived_an_hour_is_started_again_at_once() {
        let world = World::new("lived");
        for minute in [0, 1, 2] {
            world.pass(at(minute));
            world.end();
        }
        // Two starts, and a wait of 5 minutes; the third start lives 2 hours.
        assert_eq!(world.starts.get(), 2);
        world.pass(at(20));
        assert_eq!(world.starts.get(), 3);
        world.pass(at(30));
        world.end();
        let kept = world.pass(at(20 + 120));
        assert_eq!(world.starts.get(), 4, "{kept:?}");
        assert_eq!(kept.keep.attempts, 1);
        assert_eq!(kept.started.as_deref(), Some("gone"));
    }

    /// The limit's mark is gone from `limited.json` before its reset: the keeper waits on its own
    /// copy, then starts once.
    #[test]
    fn a_limited_director_waits_for_its_mark() {
        let world = World::new("limited");
        world.pass(at(0));
        world.pass(at(5));
        world.end();
        super::super::write_json(
            &world.dir,
            super::super::handlers::LIMITED,
            &json!({SESSION: {"repo": "-", "resets_at": stamp(at(60)).0}}),
        )
        .expect("mark the limit");
        let kept = world.pass(at(10));
        assert_eq!(kept.keep.state, DirectorState::Limited);
        assert_eq!(kept.keep.next_start_at, Some(stamp(at(60))));
        assert_eq!(world.starts.get(), 1);
        super::super::write_json(&world.dir, super::super::handlers::LIMITED, &json!({}))
            .expect("forget the mark");
        let kept = world.pass(at(30));
        assert_eq!(kept.keep.state, DirectorState::Limited);
        assert_eq!(world.starts.get(), 1, "waits on its copy");
        let kept = world.pass(at(61));
        assert_eq!(world.starts.get(), 2, "{kept:?}");
        assert_eq!(kept.keep.state, DirectorState::Running);
        world.pass(at(62));
        assert_eq!(world.starts.get(), 2, "once");
    }

    /// A usage-limited director whose process stays alive is a live entry of the `child`
    /// provider's registry: past the reset the keeper neither stops it nor starts a second.
    #[test]
    fn a_limited_director_that_stays_alive_is_not_started_twice() {
        let world = World::new("limited-alive");
        let registry = world.dir.join("registry");
        fs::create_dir_all(&registry).expect("a registry");
        let id = format!("d{}-limited", std::process::id());
        let mut process = crate::instance::command("sh")
            .args(["-c", "sleep 60; :", &id])
            .spawn()
            .expect("a stand-in session");
        let pid = process.id();
        fs::write(
            registry.join(format!("{id}.json")),
            json!({"name": SESSION, "session_id": id, "cwd": "/", "pid": pid,
                   "started_at": 1_800_000_000_000_u64, "exit_code": null})
            .to_string(),
        )
        .expect("write the entry");
        super::super::write_json(
            &world.dir,
            super::super::handlers::LIMITED,
            &json!({SESSION: {"repo": "-", "resets_at": stamp(at(30)).0}}),
        )
        .expect("mark the limit");
        let live = |name: &str| child::live_named(&registry, name).map(|_| None);
        let starts = Cell::new(0);
        let mut start = || {
            starts.set(starts.get() + 1);
            Ok(())
        };
        let mut record = |_: &DispatchLogLine| Ok(());
        for minute in [0, 31, 90] {
            let kept = pass(
                &world.dir,
                "example-org",
                &SessionName(SESSION.to_owned()),
                &world.names,
                &mut Seam {
                    now: at(minute),
                    live: &live,
                    start: &mut start,
                    record: &mut record,
                    notify: false,
                },
            )
            .expect("the keeper's pass");
            assert_eq!(kept.keep.state, DirectorState::Running, "{kept:?}");
        }
        assert_eq!(starts.get(), 0, "no second director");
        assert!(child::alive(pid, &id), "not stopped");
        let _ = process.kill();
        let _ = process.wait();
    }

    /// After `session stop` the record is `Stopped` and nothing starts; once a start resumes it
    /// (the hold released), the next tick starts it.
    #[test]
    fn a_stopped_director_is_not_restarted_until_resumed() {
        let mut world = World::new("stopped");
        let state = world.dir.join("state");
        let run = state.join(super::super::RUN_DIR);
        fs::create_dir_all(&run).expect("the run directory");
        world.run = run;
        world.pass(at(0));
        world.end();
        hold(&state).expect("hold");
        for minute in [1, 30, 300] {
            let kept = world.pass(at(minute));
            assert_eq!(kept.keep.state, DirectorState::Stopped);
        }
        assert_eq!(world.starts.get(), 1);
        release(&state).expect("release");
        let kept = world.pass(at(301));
        assert_eq!(world.starts.get(), 2, "{kept:?}");
        assert_eq!(kept.started.as_deref(), Some("none ran"));
    }

    /// A fresh keeper (a runtime restarted) that finds a live director it did not start counts
    /// it as running and starts none.
    #[test]
    fn a_live_adopted_director_is_not_started_again() {
        let world = World::new("adopted");
        world.live.borrow_mut().push((SESSION.to_owned(), at(-90)));
        let kept = world.pass(at(0));
        assert_eq!(kept.keep.state, DirectorState::Running);
        assert_eq!(kept.keep.started_at, Some(stamp(at(-90))));
        assert_eq!(world.starts.get(), 0);
        assert!(world.started_lines().is_empty());
    }

    /// With a checkout named `conductor`, the live `<session_prefix>-conductor` is that
    /// repository's controller, not the director: the keeper starts `<session_prefix>-director`.
    #[test]
    fn the_conductor_controller_is_not_counted_as_the_director() {
        let mut world = World::new("alias");
        let root = world.dir.join("checkouts");
        fs::create_dir_all(root.join("conductor")).expect("a conductor checkout");
        let text = format!(
            "version: conductor.config/2\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   placement: systemd\n\
             \x20   session_prefix: ex\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n",
            trees = world.dir.join("trees"),
            records = world.dir.join("records"),
            state = world.dir.join("state"),
        );
        let parsed = config::parse(&text, &world.dir).expect("the config parses");
        let instance = &parsed.instances[0];
        let role = config::director(instance).expect("a director role");
        let session = config::session_of_role(instance, role);
        assert_eq!(session.0, SESSION);
        let names = config::director_names(instance, &session.0);
        assert!(
            !names.iter().any(|name| name == "ex-conductor"),
            "{names:?}"
        );
        world.names = names;
        world
            .live
            .borrow_mut()
            .push(("ex-conductor".to_owned(), at(-5)));
        let kept = world.pass(at(0));
        assert_eq!(world.starts.get(), 1, "{kept:?}");
        assert_eq!(world.started_lines()[0].session.as_deref(), Some(SESSION));
    }

    /// Three failed starts in a row raise `director-not-started` once, naming the start's failure
    /// line with the path it could not read; the keeper keeps trying at the back-off.
    #[test]
    fn three_failed_starts_raise_director_not_started_once() {
        let world = World::new("failed");
        let line = "session start: read the role's profile /opt/roles/director.md: Permission \
                    denied";
        *world.fails.borrow_mut() = Some(line.to_owned());
        let mut now = at(0);
        let mut alerts = 0;
        for _ in 0..200 {
            let kept = world.pass(now);
            alerts += usize::from(kept.alerted);
            now += Duration::minutes(1);
        }
        assert!(world.starts.get() > 3, "it keeps trying");
        assert_eq!(alerts, 1);
        let raised: Vec<DispatchLogLine> = world
            .lines
            .borrow()
            .iter()
            .filter(|line| line.event == DispatchLogEvent::Alert)
            .cloned()
            .collect();
        assert_eq!(raised.len(), 1, "{raised:?}");
        assert_eq!(
            raised[0].condition.as_deref(),
            Some(crate::alert::DIRECTOR_NOT_STARTED)
        );
        assert_eq!(raised[0].repo, "example-org");
        let reason = raised[0].reason.clone().unwrap_or_default();
        assert!(reason.contains("/opt/roles/director.md"), "{reason}");
        assert!(world.started_lines().is_empty());
        // A start that runs clears it, so a later run of failures alerts again.
        *world.fails.borrow_mut() = None;
        world.pass(now + Duration::hours(2));
        world.pass(now + Duration::hours(2) + Duration::minutes(1));
        let seen = super::super::read_json(&world.run, crate::alert::SEEN, json!({}));
        assert_eq!(seen, json!({}), "cleared");
    }
}
