//! The health view's data (`story:dashboard-health`): `GET /api/health`, read-only.
//!
//! - `doctor`: the rows and the verdict of `conductor doctor`, computed by doctor's own checks
//!   ([`doctor::check_in`]) on a thread of their own: at start, then whenever the last result is
//!   older than [`HealthSources::doctor_every`] by [`HealthSources::clock`], which a request and a
//!   timer of the server ([`TICK`]) both look at. A request never waits for doctor: it answers
//!   the last result with `taken_at`, the clock when it was taken.
//! - `snapshot`: the newest snapshot of the store under [`HealthSources::state`], in any state,
//!   with its `failure_reason`; `collector_runs` and `member_failures`: that snapshot's rows of the
//!   views `snapshot collector-runs` and `snapshot member-failures`, each failure with its class
//!   ([`failure_class`]) and the fix the table [`super::FAILURE_FIXES`] gives that class.
//! - `disk`: the free bytes of [`HealthSources::disk`], the instance's floors
//!   `thresholds.disk_low` and `thresholds.disk_admit`, and its `state`: `ok` above `disk_low`,
//!   `low` at or below it, `admit-only` at or below `disk_admit`; and the last disk-waste
//!   measurement ([`super::waste`]) with every one of its rows.
//!
//! A part that cannot be read is named in `errors`; the others still show. The store is opened
//! only when it is there, and nothing is written. The snapshot and its rows are read through the
//! store handle the server keeps, once per write to the store (`story:dashboard-fast`,
//! [`super::cache`]); the rest is read for each request.

use std::env;
use std::ffi::OsString;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use anyhow::Result;
use conductor_model::behaviour::{
    CollectorRunObservationStorage, Generated, MemberFailureObservationStorage, SnapshotStorage,
};
use conductor_model::config::Instance;
use conductor_model::observation::obligations::{
    CollectorRunsQuery, MemberFailuresQuery, SnapshotsQuery,
};
use conductor_model::observation::{
    CollectorRunObservationSnapshot, CollectorRuns, DashboardMemory, FailureClass,
    MemberFailureObservationSnapshot, MemberFailures, ObservationId, SnapshotId, SnapshotSnapshot,
};
use serde_json::{Map, Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::cache::Held;
use super::{FAILURE_FIXES, Sources, rfc3339, tilde_all};
use crate::store::Store;
use crate::{config, doctor};

/// How often the server's timer looks whether doctor is due again.
pub(super) const TICK: Duration = Duration::from_secs(1);

const GIB: u64 = 1 << 30;

/// What the health view reads besides the disk-waste measurement: the store, doctor's
/// environment, cache interval and clock, and the free disk with the instance's floors. A test
/// points each at a fixture.
#[derive(Debug, Clone)]
pub struct HealthSources {
    /// The state directory whose store holds the snapshots: the instance's `state`.
    pub state: PathBuf,
    /// The home directory doctor reads its config under, as `HOME` gives it to `conductor doctor`.
    pub home: PathBuf,
    /// The config file doctor reads, as `CONDUCTOR_CONFIG` would name it; none reads the one
    /// `CONDUCTOR_CONFIG` names, else the default file under [`HealthSources::home`].
    pub config: Option<PathBuf>,
    /// The `PATH` doctor looks for each program on; none is this process's.
    pub path: Option<OsString>,
    /// How long one doctor result is served before the next is taken: 10 minutes.
    pub doctor_every: Duration,
    /// The clock doctor's results are timed by.
    pub clock: fn() -> OffsetDateTime,
    /// A path on the file system whose free space is held to the floors: the instance's
    /// `thresholds.disk_path`.
    pub disk: PathBuf,
    /// Reads the free bytes of [`HealthSources::disk`]: what an unprivileged writer may still use.
    pub free_bytes: fn(&Path) -> Result<u64>,
    /// `thresholds.disk_low`, in GiB.
    pub disk_low: u64,
    /// `thresholds.disk_admit`, in GiB.
    pub disk_admit: u64,
    /// The host directory whose `falls/` the disk part lists, every instance's
    /// (`story:host-fall-per-instance`); none lists no fall.
    pub host: Option<PathBuf>,
}

impl HealthSources {
    /// The health sources without a config file: those of the built-in instance
    /// ([`config::built_in`]) for the home directory `home` and the working directory `cwd`.
    #[must_use]
    pub fn new(home: PathBuf, cwd: &Path) -> Self {
        let built_in = config::built_in(&home, cwd);
        Self::of(home, &built_in)
    }

    /// The health sources of `instance` under the home directory `home`: its `state`, its
    /// `thresholds.disk_path`, `thresholds.disk_low` and `thresholds.disk_admit`; doctor with the
    /// config `CONDUCTOR_CONFIG` names and this process's `PATH`, every 10 minutes by the system
    /// clock.
    #[must_use]
    pub fn of(home: PathBuf, instance: &Instance) -> Self {
        let gib = |count: i64| u64::try_from(count).unwrap_or(0);
        Self {
            state: PathBuf::from(&instance.state),
            home,
            config: None,
            path: None,
            doctor_every: Duration::from_secs(600),
            clock: OffsetDateTime::now_utc,
            disk: PathBuf::from(&instance.thresholds.disk_path),
            free_bytes: |path| crate::disk::space(path).map(|space| space.available),
            disk_low: gib(instance.thresholds.disk_low.0),
            disk_admit: gib(instance.thresholds.disk_admit.0),
            host: None,
        }
    }
}

/// The last doctor result, and whether the next is being taken; shared by every request.
#[derive(Debug, Clone, Default)]
pub(super) struct Doctor(Arc<Mutex<Taking>>);

#[derive(Debug, Default)]
struct Taking {
    /// When the last result was taken, and its fields.
    last: Option<(OffsetDateTime, Value)>,
    running: bool,
}

impl Doctor {
    fn lock(&self) -> MutexGuard<'_, Taking> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes doctor's rows over `health` on a thread of its own, unless they are being taken or
    /// the last were taken less than [`HealthSources::doctor_every`] ago. Returns at once.
    pub(super) fn refresh(&self, health: &HealthSources) {
        let now = (health.clock)();
        let mut taking = self.lock();
        let fresh = taking
            .last
            .as_ref()
            .is_some_and(|(at, _)| younger(now - *at, health.doctor_every));
        if taking.running || fresh {
            return;
        }
        taking.running = true;
        drop(taking);
        let cache = self.clone();
        let health = health.clone();
        std::thread::spawn(move || {
            let taken = catch_unwind(AssertUnwindSafe(|| take(&health)))
                .unwrap_or_else(|_| failed("doctor panicked"));
            let mut taking = cache.lock();
            taking.running = false;
            taking.last = Some((now, taken));
        });
    }

    /// The `doctor` object: `taken_at` (null before the first result), `taking`, and the last
    /// result's `rows`, `verdict`, `ready` and `error`.
    fn view(&self) -> Value {
        let taking = self.lock();
        let mut view = Map::new();
        view.insert(
            "taken_at".to_owned(),
            taking
                .last
                .as_ref()
                .map_or(Value::Null, |(at, _)| Value::from(rfc3339(*at))),
        );
        view.insert("taking".to_owned(), Value::from(taking.running));
        let result = taking.last.as_ref().map_or_else(
            || json!({"rows": [], "verdict": null, "ready": null, "error": null}),
            |(_, result)| result.clone(),
        );
        if let Value::Object(result) = result {
            view.extend(result);
        }
        Value::Object(view)
    }
}

/// Whether `age` is less than `every`; an age below zero, from a clock set back, is.
fn younger(age: time::Duration, every: Duration) -> bool {
    age.whole_milliseconds() < i128::try_from(every.as_millis()).unwrap_or(i128::MAX)
}

/// Doctor's rows over `health`: what `conductor doctor` prints but its verdict
/// ([`doctor::Doctor::lines`]), and the verdict.
fn take(health: &HealthSources) -> Value {
    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => return failed(&format!("doctor: read the working directory: {error}")),
    };
    let variable = |name: &str| env::var_os(name).filter(|value| !value.is_empty());
    let environment = config::Environment {
        home: Some(health.home.clone()),
        cwd: cwd.clone(),
        config: health
            .config
            .clone()
            .map(PathBuf::into_os_string)
            .or_else(|| variable(config::CONFIG_VARIABLE)),
        instance: variable(config::INSTANCE_VARIABLE),
    };
    let search = health.path.clone().or_else(|| env::var_os("PATH"));
    match doctor::check_in(Some(&environment), None, &cwd, search.as_deref()) {
        Ok(found) => {
            let verdict = doctor::verdict_of(&found.failures());
            json!({
                "rows": found.lines(),
                "ready": verdict == doctor::READY,
                "verdict": verdict,
                "error": null,
            })
        }
        Err(error) => failed(&format!("{error:#}")),
    }
}

/// A doctor result that has no rows, because of `error`.
fn failed(error: &str) -> Value {
    json!({"rows": [], "verdict": null, "ready": false, "error": error})
}

/// The class of a member failure whose first line is `line`: [`FailureClass::OldStore`] when it
/// holds `[unsupported_protocol_version]`; [`FailureClass::TimedOut`] for a command stopped at its
/// bound; [`FailureClass::NeedsLogin`] for a prompt for a credential or a credential refused;
/// [`FailureClass::UnreachableHost`] for a host that does not resolve or answer;
/// [`FailureClass::Other`] for anything else.
#[must_use]
pub fn failure_class(line: &str) -> FailureClass {
    const LOGIN: [&str; 7] = [
        "could not read username",
        "could not read password",
        "permission denied (publickey",
        "terminal prompts disabled",
        "authentication failed",
        "host key verification failed",
        "invalid username or password",
    ];
    const HOST: [&str; 5] = [
        "could not resolve hostname",
        "could not resolve host",
        "network is unreachable",
        "no route to host",
        "connection refused",
    ];
    let lower = line.to_lowercase();
    let holds = |needles: &[&str]| needles.iter().any(|needle| lower.contains(needle));
    if line.contains("[unsupported_protocol_version]") {
        FailureClass::OldStore
    } else if lower.contains("gave no answer within") {
        FailureClass::TimedOut
    } else if holds(&LOGIN) {
        FailureClass::NeedsLogin
    } else if holds(&HOST) {
        FailureClass::UnreachableHost
    } else {
        FailureClass::Other
    }
}

/// The name and the fix [`FAILURE_FIXES`] gives `class`.
fn fix(class: FailureClass) -> (&'static str, &'static str) {
    FAILURE_FIXES
        .iter()
        .find(|(known, _, _)| *known == class)
        .map_or(("other", ""), |(_, name, fix)| (*name, *fix))
}

/// `/api/health` from `sources`, with the last disk-waste measurement `waste`, the doctor cache
/// `doctor` and the newest snapshot with its rows `newest` ([`newest`], which the server keeps
/// until the store is written), the home directory written `~`.
pub(super) fn answer(
    sources: &Sources,
    doctor: &Doctor,
    waste: Value,
    newest: Newest,
    memory: &DashboardMemory,
) -> Value {
    let health = &sources.health;
    let (snapshot, collector_runs, member_failures, mut errors) = newest;
    let data = json!({
        "generated_at": rfc3339((health.clock)()),
        "doctor": doctor.view(),
        "snapshot": snapshot,
        "collector_runs": collector_runs,
        "member_failures": member_failures,
        "disk": disk(health, waste, &mut errors),
        "memory": memory_view(memory),
        "errors": errors,
    });
    let home = sources.home.display().to_string();
    tilde_all(data, home.trim_end_matches('/'))
}

/// The `memory` part of the answer: `conductor.observation.DashboardMemory`'s fields.
fn memory_view(memory: &DashboardMemory) -> Value {
    json!({
        "rss_bytes": memory.rss_bytes,
        "swap_bytes": memory.swap_bytes,
        "store_handles": memory.store_handles,
        "store_replays": memory.store_replays,
        "store_replays_peak": memory.store_replays_peak,
        "store_bytes": memory.store_bytes,
        "memo_entries": memory.memo_entries,
        "memo_entries_peak_endpoint": memory.memo_entries_peak_endpoint,
        "memo_bytes": memory.memo_bytes,
    })
}

/// This process's resident set in bytes, `VmRSS` of `/proc/self/status`; none where the system
/// does not report it there.
pub(super) fn rss_bytes() -> Option<u64> {
    status_bytes("VmRSS:")
}

/// What of this process the system swapped out, in bytes, `VmSwap` of `/proc/self/status`
/// (`story:dashboard-memory-live`); none where the system does not report it there.
pub(super) fn swap_bytes() -> Option<u64> {
    status_bytes("VmSwap:")
}

/// The size `/proc/self/status` gives on its line `field` (the name with its colon), in `kB`,
/// in bytes.
fn status_bytes(field: &str) -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find_map(|line| line.strip_prefix(field))?;
    let kib: u64 = line.trim().strip_suffix("kB")?.trim().parse().ok()?;
    kib.checked_mul(1024)
}

/// The newest snapshot, its collector runs, its member failures, and what could not be read.
pub(super) type Newest = (Value, Vec<Value>, Vec<Value>, Vec<String>);

/// The store the server keeps, read through the generated queries without being owned by them:
/// the queries only `get` and `list`, so a write through it writes nothing.
struct Reading<'a>(&'a Store);

/// The three storages the health view's queries read, each delegating its reads to the store.
macro_rules! read_only {
    ($storage:ident, $identity:ty, $snapshot:ty) => {
        impl $storage for Reading<'_> {
            fn get(&self, identity: &$identity) -> Option<$snapshot> {
                $storage::get(self.0, identity)
            }

            fn put(&mut self, _snapshot: $snapshot) {}

            fn delete(&mut self, _identity: &$identity) {}

            fn list(&self) -> Vec<$snapshot> {
                $storage::list(self.0)
            }
        }
    };
}

read_only!(SnapshotStorage, SnapshotId, SnapshotSnapshot);
read_only!(
    CollectorRunObservationStorage,
    ObservationId,
    CollectorRunObservationSnapshot
);
read_only!(
    MemberFailureObservationStorage,
    ObservationId,
    MemberFailureObservationSnapshot
);

/// The newest snapshot of the store `held` (the one under [`HealthSources::state`]), by
/// `started_at`, in any state, with its collector runs and member failures; null and none when the
/// store or its snapshots cannot be read, with why.
pub(super) fn newest(held: Held<'_>) -> Newest {
    let mut errors = Vec::new();
    let (snapshot, runs, failures) = newest_in(held, &mut errors);
    (snapshot, runs, failures, errors)
}

fn newest_in(held: Held<'_>, errors: &mut Vec<String>) -> (Value, Vec<Value>, Vec<Value>) {
    let none = || (Value::Null, Vec::new(), Vec::new());
    let store = match held {
        Ok(Some(store)) => store,
        Ok(None) => return none(),
        Err(error) => {
            errors.push(format!("snapshots: {error:#}"));
            return none();
        }
    };
    let generated = Generated::new(Reading(store));
    let snapshots = match generated.snapshots() {
        Ok(rows) => rows,
        Err(unmet) => {
            errors.push(format!("snapshot snapshots: {unmet}"));
            return none();
        }
    };
    let started = |at: &str| OffsetDateTime::parse(at, &Rfc3339).ok();
    let Some(newest) = snapshots
        .into_iter()
        .max_by_key(|row| started(&row.started_at.0))
    else {
        return none();
    };
    let id = newest.snapshot_id.clone();
    let mut runs: Vec<CollectorRuns> = match generated.collector_runs() {
        Ok(rows) => rows
            .into_iter()
            .filter(|row| row.snapshot_id == id)
            .collect(),
        Err(unmet) => {
            errors.push(format!("snapshot collector-runs: {unmet}"));
            Vec::new()
        }
    };
    runs.sort_by_key(|run| started(&run.started_at.0));
    let failures: Vec<MemberFailures> = match generated.member_failures() {
        Ok(rows) => rows
            .into_iter()
            .filter(|row| row.snapshot_id == id)
            .collect(),
        Err(unmet) => {
            errors.push(format!("snapshot member-failures: {unmet}"));
            Vec::new()
        }
    };
    let snapshot = json!({
        "snapshot_id": newest.snapshot_id.0.0,
        "state": crate::snapshot::snapshot_state_name(newest.state),
        "started_at": newest.started_at.0,
        "disk_free_bytes": newest.disk_free_bytes,
        "failure_reason": newest.failure_reason,
    });
    (
        snapshot,
        runs.into_iter().map(collector_run).collect(),
        failures.into_iter().map(member_failure).collect(),
    )
}

/// One row of `snapshot collector-runs`: the collector, its start, its duration in milliseconds,
/// how it ended (`Completed`, `Failed`, `OverBudget`) and the bytes read and written while it ran
/// (`null` where they were not recorded).
fn collector_run(run: CollectorRuns) -> Value {
    json!({
        "collector": run.collector,
        "started_at": run.started_at.0,
        "duration_ms": run.duration_ms,
        "outcome": crate::snapshot::collector_run_outcome_name(run.outcome),
        "read_bytes": run.read_bytes,
        "written_bytes": run.written_bytes,
    })
}

/// One row of `snapshot member-failures`, with its class and that class's fix.
fn member_failure(failure: MemberFailures) -> Value {
    let (class, fix) = fix(failure_class(&failure.error_line));
    json!({
        "member": failure.member.0,
        "collector": failure.collector,
        "error_line": failure.error_line,
        "class": class,
        "fix": fix,
    })
}

/// The `disk` object: the free bytes of `health.disk` against the floors, its `state`, and the
/// disk-waste measurement `waste` with all its rows. The free bytes and the state are null when
/// the free space cannot be read, with why in `errors`.
fn disk(health: &HealthSources, waste: Value, errors: &mut Vec<String>) -> Value {
    let free = match (health.free_bytes)(&health.disk) {
        Ok(free) => Some(free),
        Err(error) => {
            errors.push(format!("{error:#}"));
            None
        }
    };
    let low = health.disk_low.saturating_mul(GIB);
    let admit = health.disk_admit.saturating_mul(GIB);
    let state = free.map(|free| {
        if free <= admit {
            "admit-only"
        } else if free <= low {
            "low"
        } else {
            "ok"
        }
    });
    let falls: Vec<Value> = health
        .host
        .as_deref()
        .map(crate::host::falls)
        .unwrap_or_default()
        .into_iter()
        .map(|fall| json!({"prefix": fall.key, "instance": fall.instance, "item": fall.item}))
        .collect();
    let mut disk = json!({
        "path": health.disk.display().to_string(),
        "free_bytes": free,
        "disk_low_bytes": low,
        "disk_admit_bytes": admit,
        "state": state,
        "falls": falls,
    });
    if let (Value::Object(disk), Value::Object(waste)) = (&mut disk, waste) {
        disk.extend(waste);
    }
    disk
}
