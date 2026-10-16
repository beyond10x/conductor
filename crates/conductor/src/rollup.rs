//! The daily rollups (`story:dashboard-history`): what the dashboard's charts and KPIs read
//! beyond the snapshots retention keeps.
//!
//! When a snapshot completes ([`crate::snapshot`], by the driver or by `complete-snapshot`),
//! before retention drops anything, [`update`] works out, from the data the store holds, every
//! item each series counts on each day, and adds them to the rollups
//! (`conductor.observation.DailySeries`, kept in `state/series/`, [`crate::store::series`]): one
//! per series, day (UTC), dimension and group. A rollup keeps the ids of the items it counts, so
//! a later snapshot adds what it sees to what is kept, whatever the snapshot's window has
//! dropped since; a rollup is written only when that adds an item, through `RecordDailySeries` the
//! first time and `ReviseDailySeries` after.
//!
//! | series | by | an item | its day |
//! |---|---|---|---|
//! | `prs_opened` | repository | an open PR of the snapshot, `alpha#12` | its `opened_at` |
//! | `prs_merged` | repository | a merged PR of the snapshot | its `merged_at` |
//! | `reverts` | repository | a merged PR titled `Revert…` | its `merged_at` |
//! | `prs_closed_unmerged` | repository | a PR open in the complete snapshot before, of a repository this one observed, neither open nor merged in this one, when every forge collector of this one completed | this snapshot's start |
//! | `ci_passed`, `ci_failed` | repository, workflow | a `main` run of the snapshot concluding success or failure, `alpha/check@<run_at>` | its `run_at` |
//! | `releases` | repository | a release of the snapshot, `alpha@v1.2.0` | its publication |
//! | `releases_feature`, `releases_patch` | repository | a release whose tag ends in a version `x.y.0`, or `x.y.z` with `z` above 0 | its publication |
//! | `merged_to_released_hours` | total | a merged PR and its age in hours when the first release after it was published, as `repository shipped` pairs them | the release's publication |
//! | `dispatch_lead_hours` | total | a `Done` dispatch and the hours from its `sent_at` to its `done` line in the records' dispatch log | the `done` line |
//! | `operator_touches` | total | a decision the operator took, and a decision request escalated as class O or H | its `decided_at`, the request's `raised_at` |
//! | `escalations` | total | a decision request escalated as class O or H | its `raised_at` |
//! | `escalations_unnecessary` | total | such a request withdrawn, or answered by a decision conductor took | its `raised_at` |
//! | `guard_denials` | rule | a guard decision that denied, keyed by the tool whose rule denied it | its `decided_at` |
//! | `disk_granted`, `disk_refused` | repository | a disk reservation of the host ledger granted (granted, running or released) or refused | its request |
//! | `disk_waited` | repository | a granted disk reservation whose run started [`WAITED`] or more after its request: it waited for live reservations, or for a reclaim | its request |
//! | `disk_peak_vs_reserved` | repository | a released disk reservation with the bytes it reserved and the peak its run recorded (`host record-peak`), `G-3=<peak>/<bytes>` | its request |
//!
//! A series without a dimension is kept under `by` and `key` `total`. A duration's item carries
//! its measure after `=` (`D-7=2.50`), a run's its peak and reserved bytes (`G-3=512/1024`). A
//! reservation without a repository is keyed `(none)`. A run counts once released: only then is
//! its peak final, and a rollup keeps the first form of an item it sees.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result, anyhow};
use conductor_model::behaviour::{
    CollectorRunObservationStorage, DailySeriesStorage, DecisionRequestStorage, DecisionStorage,
    DispatchStorage, Generated, GuardDecisionStorage, HostGrantStorage,
    MergedPullRequestObservationStorage, PullRequestObservationStorage, ReleaseObservationStorage,
    RepositoryObservationStorage, SnapshotStorage, WorkflowRunObservationStorage,
};
use conductor_model::decision::{Decider, DecisionClass, DecisionRequestState};
use conductor_model::dispatch::{DispatchState, Verdict};
use conductor_model::host::{GrantKind, HostGrantState};
use conductor_model::observation::obligations::{
    RecordDailySeriesBehavior, ReviseDailySeriesBehavior,
};
use conductor_model::observation::{
    CollectorRunOutcome, DailySeriesId, DailySeriesSnapshot, RecordDailySeries,
    RecordDailySeriesOutcome, ReviseDailySeries, ReviseDailySeriesOutcome, RunConclusion,
    SnapshotId, SnapshotState,
};
use conductor_model::primitives::Decimal;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, UtcOffset};

use crate::store::Store;

/// The dimension and group of a series that has none.
pub const TOTAL: &str = "total";

/// The group of a disk reservation that names no repository.
pub const NO_REPOSITORY: &str = "(none)";

/// How long after its request a granted reservation's run may start before it counts as waited.
pub const WAITED: time::Duration = time::Duration::minutes(1);

/// The series whose items carry a measure (`id=hours`): their values are read as percentiles.
pub const DURATIONS: [&str; 2] = ["merged_to_released_hours", "dispatch_lead_hours"];

/// The series of the runs' peaks against their reservations: its items carry `peak/bytes`.
pub const PEAK_VS_RESERVED: &str = "disk_peak_vs_reserved";

/// The id of the rollup of `series` grouped by `by` as `key` on `day`.
#[must_use]
pub fn rollup_id(series: &str, by: &str, key: &str, day: &str) -> String {
    format!("{series}|{by}|{key}|{day}")
}

/// What an item is known by in its rollup: the part before its measure, for a duration.
#[must_use]
pub fn item_id<'a>(series: &str, item: &'a str) -> &'a str {
    if DURATIONS.contains(&series) || series == PEAK_VS_RESERVED {
        item.rsplit_once('=').map_or(item, |(id, _)| id)
    } else {
        item
    }
}

/// The measure of a duration's item, in hours.
#[must_use]
pub fn measure(item: &str) -> Option<f64> {
    item.rsplit_once('=')?.1.parse().ok()
}

/// The peak and the reserved bytes of a run's item (`G-3=512/1024`).
#[must_use]
pub fn usage(item: &str) -> Option<(f64, f64)> {
    let (peak, bytes) = item.rsplit_once('=')?.1.split_once('/')?;
    Some((peak.parse().ok()?, bytes.parse().ok()?))
}

/// The UTC date of an RFC 3339 instant.
fn day_of(at: &str) -> Option<Date> {
    Some(
        OffsetDateTime::parse(at, &Rfc3339)
            .ok()?
            .to_offset(UtcOffset::UTC)
            .date(),
    )
}

fn instant(at: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(at, &Rfc3339).ok()
}

fn hours(from: OffsetDateTime, to: OffsetDateTime) -> f64 {
    (to - from).as_seconds_f64() / 3600.0
}

/// The items one completed snapshot sees, `(series, by, key, day)` to their ids.
#[derive(Default)]
struct Seen(BTreeMap<(String, String, String, String), BTreeSet<String>>);

impl Seen {
    /// `item` of `series` on the day of `at`, under each `(by, key)` of `groups`.
    fn add(&mut self, series: &str, groups: &[(&str, &str)], at: &str, item: String) {
        let Some(day) = day_of(at) else {
            return;
        };
        for (by, key) in groups {
            self.0
                .entry((
                    series.to_owned(),
                    (*by).to_owned(),
                    (*key).to_owned(),
                    day.to_string(),
                ))
                .or_default()
                .insert(item.clone());
        }
    }

    fn total(&mut self, series: &str, at: &str, item: String) {
        self.add(series, &[(TOTAL, TOTAL)], at, item);
    }
}

/// Whether a release tag ends in a version, and that version's patch number.
fn patch_of(tag: &str) -> Option<u64> {
    let start = tag
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_digit() || *c == '.')
        .last()
        .map(|(at, _)| at)?;
    let parts: Vec<&str> = tag[start..].split('.').collect();
    let [major, minor, patch] = parts.as_slice() else {
        return None;
    };
    major.parse::<u64>().ok()?;
    minor.parse::<u64>().ok()?;
    patch.parse().ok()
}

/// The `at` of the newest `done` line of each dispatch in the records' dispatch log under
/// `records`; none where the log is not there. A line that does not read is passed over: the log
/// has its own check (`records check`).
fn done_lines(records: &Path) -> HashMap<String, String> {
    let mut done = HashMap::new();
    let Ok(entries) = fs::read_dir(records.join(crate::records::DISPATCHES)) else {
        return done;
    };
    let mut files: Vec<_> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    files.sort();
    for file in files {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        for line in text.lines() {
            let Ok(line) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if line["event"] == "done"
                && let (Some(id), Some(at)) = (line["id"].as_str(), line["at"].as_str())
            {
                done.insert(id.to_owned(), at.to_owned());
            }
        }
    }
    done
}

/// Every item the store shows for the snapshot `snapshot`, which completed.
fn seen(store: &Store, snapshot: &SnapshotId, records: &Path) -> Result<Seen> {
    let mut seen = Seen::default();
    let snapshots = SnapshotStorage::list(store);
    let this = snapshots
        .iter()
        .find(|held| held.data.snapshot_id == *snapshot)
        .ok_or_else(|| anyhow!("snapshot {} is not in the store", snapshot.0.0))?;
    let started = this.data.started_at.0.clone();
    let start = instant(&started);
    // The complete snapshot before this one: the newest that started before it.
    let before = snapshots
        .iter()
        .filter(|held| held.state == SnapshotState::Complete && held.data.snapshot_id != *snapshot)
        .filter_map(|held| Some((instant(&held.data.started_at.0)?, held)))
        .filter(|(at, _)| start.is_some_and(|start| *at <= start))
        .max_by_key(|(at, _)| *at)
        .map(|(_, held)| held.data.snapshot_id.clone());

    let of = |id: &SnapshotId| id == snapshot;
    let open: Vec<_> = PullRequestObservationStorage::list(store)
        .into_iter()
        .map(|held| held.data)
        .collect();
    let merged: Vec<_> = MergedPullRequestObservationStorage::list(store)
        .into_iter()
        .map(|held| held.data)
        .filter(|data| of(&data.snapshot_id))
        .collect();
    let pr = |repository: &str, number: i64| format!("{repository}#{number}");

    for data in open.iter().filter(|data| of(&data.snapshot_id)) {
        let repository = data.repository.0.as_str();
        seen.add(
            "prs_opened",
            &[("repository", repository)],
            &data.opened_at.0,
            pr(repository, data.number),
        );
    }
    for data in &merged {
        let repository = data.repository.0.as_str();
        let item = pr(repository, data.number);
        seen.add(
            "prs_merged",
            &[("repository", repository)],
            &data.merged_at.0,
            item.clone(),
        );
        if data.title.trim_start().to_lowercase().starts_with("revert") {
            seen.add(
                "reverts",
                &[("repository", repository)],
                &data.merged_at.0,
                item,
            );
        }
    }
    // A closure is read from a PR gone missing, so only where each forge that lists PRs answered in
    // full: a forge stopped over budget leaves every PR missing, and none of them closed.
    let forges_answered = CollectorRunObservationStorage::list(store)
        .into_iter()
        .filter(|held| of(&held.data.snapshot_id))
        .filter(|held| matches!(held.data.collector.as_str(), "github" | "gitlab"))
        .all(|held| held.data.outcome == CollectorRunOutcome::Completed);
    if let Some(before) = &before
        && forges_answered
    {
        let observed: BTreeSet<String> = RepositoryObservationStorage::list(store)
            .into_iter()
            .filter(|held| of(&held.data.snapshot_id))
            .map(|held| held.data.repository.0)
            .collect();
        let still: BTreeSet<String> = open
            .iter()
            .filter(|data| of(&data.snapshot_id))
            .map(|data| pr(&data.repository.0, data.number))
            .chain(
                merged
                    .iter()
                    .map(|data| pr(&data.repository.0, data.number)),
            )
            .collect();
        for data in open.iter().filter(|data| data.snapshot_id == *before) {
            let repository = data.repository.0.as_str();
            let item = pr(repository, data.number);
            if observed.contains(repository) && !still.contains(&item) {
                seen.add(
                    "prs_closed_unmerged",
                    &[("repository", repository)],
                    &started,
                    item,
                );
            }
        }
    }
    for data in WorkflowRunObservationStorage::list(store)
        .into_iter()
        .map(|held| held.data)
        .filter(|data| of(&data.snapshot_id))
    {
        let series = match data.conclusion {
            RunConclusion::Success => "ci_passed",
            RunConclusion::Failure => "ci_failed",
            _ => continue,
        };
        let repository = data.repository.0.as_str();
        seen.add(
            series,
            &[("repository", repository), ("workflow", &data.workflow)],
            &data.run_at.0,
            format!("{repository}/{}@{}", data.workflow, data.run_at.0),
        );
    }
    let releases: Vec<_> = ReleaseObservationStorage::list(store)
        .into_iter()
        .map(|held| held.data)
        .filter(|data| of(&data.snapshot_id))
        .collect();
    for data in &releases {
        let repository = data.repository.0.as_str();
        let item = format!("{repository}@{}", data.tag);
        let groups = [("repository", repository)];
        seen.add("releases", &groups, &data.published_at.0, item.clone());
        match patch_of(&data.tag) {
            Some(0) => seen.add("releases_feature", &groups, &data.published_at.0, item),
            Some(_) => seen.add("releases_patch", &groups, &data.published_at.0, item),
            None => {}
        }
    }
    // Merged → released, paired by time as `repository shipped` pairs them: a PR belongs to the
    // first release published at or after its merge.
    let mut by_repository: BTreeMap<&str, Vec<(OffsetDateTime, &str)>> = BTreeMap::new();
    for data in &releases {
        if let Some(at) = instant(&data.published_at.0) {
            by_repository
                .entry(data.repository.0.as_str())
                .or_default()
                .push((at, data.published_at.0.as_str()));
        }
    }
    for list in by_repository.values_mut() {
        list.sort();
    }
    for data in &merged {
        let Some(at) = instant(&data.merged_at.0) else {
            continue;
        };
        let Some((published, published_text)) = by_repository
            .get(data.repository.0.as_str())
            .and_then(|list| list.iter().find(|(release, _)| *release >= at))
        else {
            continue;
        };
        seen.total(
            "merged_to_released_hours",
            published_text,
            format!(
                "{}={:.2}",
                pr(&data.repository.0, data.number),
                hours(at, *published)
            ),
        );
    }

    // The records conductor keeps beside the snapshots.
    let done = done_lines(records);
    for dispatch in DispatchStorage::list(store) {
        if dispatch.state != DispatchState::Done {
            continue;
        }
        let id = &dispatch.data.dispatch_id.0;
        let (Some(sent), Some(done_text)) =
            (instant(&dispatch.data.sent_at.0), done.get(id.as_str()))
        else {
            continue;
        };
        let Some(finished) = instant(done_text) else {
            continue;
        };
        seen.total(
            "dispatch_lead_hours",
            done_text,
            format!("{id}={:.2}", hours(sent, finished)),
        );
    }
    let decisions = DecisionStorage::list(store);
    for decision in &decisions {
        if decision.data.decided_by == Decider::Operator {
            seen.total(
                "operator_touches",
                &decision.data.decided_at.0,
                decision.data.decision_id.0.clone(),
            );
        }
    }
    for request in DecisionRequestStorage::list(store) {
        let data = &request.data;
        if !matches!(
            data.escalation_class,
            Some(DecisionClass::O | DecisionClass::H)
        ) {
            continue;
        }
        let id = data.request_id.0.clone();
        seen.total("operator_touches", &data.raised_at.0, id.clone());
        seen.total("escalations", &data.raised_at.0, id.clone());
        let by_conductor = data.decision_id.as_ref().is_some_and(|decision| {
            decisions.iter().any(|held| {
                held.data.decision_id == *decision && held.data.decided_by == Decider::Conductor
            })
        });
        if request.state == DecisionRequestState::Withdrawn
            || (request.state == DecisionRequestState::Answered && by_conductor)
        {
            seen.total("escalations_unnecessary", &data.raised_at.0, id);
        }
    }
    for decision in GuardDecisionStorage::list(store) {
        let data = &decision.data;
        if data.verdict == Verdict::Deny {
            seen.add(
                "guard_denials",
                &[("rule", &data.tool)],
                &data.decided_at.0,
                data.guard_decision_id.0.0.clone(),
            );
        }
    }
    for grant in grants(store) {
        let data = &grant.data;
        if data.kind != GrantKind::Disk {
            continue;
        }
        let repository = data
            .repository
            .as_ref()
            .map_or(NO_REPOSITORY, |name| name.0.as_str());
        let groups = [("repository", repository)];
        let id = data.grant_id.0.clone();
        match grant.state {
            HostGrantState::Refused => {
                seen.add("disk_refused", &groups, &data.requested_at.0, id);
            }
            HostGrantState::Granted | HostGrantState::Running | HostGrantState::Released => {
                let waited = data
                    .started_at
                    .as_ref()
                    .and_then(|started| instant(&started.0))
                    .zip(instant(&data.requested_at.0))
                    .is_some_and(|(started, requested)| started - requested >= WAITED);
                if waited {
                    seen.add("disk_waited", &groups, &data.requested_at.0, id.clone());
                }
                if grant.state == HostGrantState::Released
                    && let (Some(bytes), Some(peak)) = (data.bytes, data.peak_bytes)
                    && bytes > 0
                {
                    seen.add(
                        PEAK_VS_RESERVED,
                        &groups,
                        &data.requested_at.0,
                        format!("{id}={peak}/{bytes}"),
                    );
                }
                seen.add("disk_granted", &groups, &data.requested_at.0, id);
            }
            // A withdrawn request was neither answered nor run: its `build run` was stopped while
            // it waited.
            HostGrantState::Requested | HostGrantState::Withdrawn => {}
        }
    }
    Ok(seen)
}

/// The grants the disk rollups count: those of `store`, kept there by a binary before the host
/// ledger moved to the host directory, and the instance's own of the host ledger
/// ([`crate::config::host_dir`], `story:instance-isolation`), which is read only when it is
/// there. Another instance's grants are not counted.
fn grants(store: &Store) -> Vec<conductor_model::host::HostGrantSnapshot> {
    let mut grants = HostGrantStorage::list(store);
    let instance = &crate::config::active().instance.name;
    let host = crate::config::host_dir()
        .ok()
        .and_then(|host| crate::store::open_existing(&host).ok());
    if let Some(host) = host {
        for grant in HostGrantStorage::list(&host) {
            if grant.data.instance == *instance
                && !grants
                    .iter()
                    .any(|held| held.data.grant_id == grant.data.grant_id)
            {
                grants.push(grant);
            }
        }
    }
    grants
}

/// The rollups' port over a store a caller holds.
struct Rollups<'a>(&'a mut Store);

impl DailySeriesStorage for Rollups<'_> {
    fn get(&self, identity: &DailySeriesId) -> Option<DailySeriesSnapshot> {
        DailySeriesStorage::get(self.0, identity)
    }

    fn put(&mut self, snapshot: DailySeriesSnapshot) {
        DailySeriesStorage::put(self.0, snapshot);
    }

    fn delete(&mut self, identity: &DailySeriesId) {
        DailySeriesStorage::delete(self.0, identity);
    }
}

/// Adds what the store shows for the completed snapshot `snapshot`, with the dispatch log under
/// `records`, to the rollups of `store`, as the module says; answers how many rollups it recorded
/// or revised.
///
/// # Errors
///
/// The snapshot is not in the store, a record the rollups read or write cannot be read or kept
/// ([`Store::check`]), or a command declares no outcome or is refused.
pub fn update(store: &mut Store, snapshot: &SnapshotId, records: &Path) -> Result<usize> {
    let seen = seen(store, snapshot, records)?;
    store.check().context("read what the rollups count")?;
    let kept: HashMap<String, DailySeriesSnapshot> = store
        .daily_series()
        .into_iter()
        .map(|held| (held.data.daily_series_id.0.clone(), held))
        .collect();
    store.check().context("read the rollups")?;
    let mut written = 0;
    let mut generated = Generated::new(Rollups(store));
    for ((series, by, key, day), items) in seen.0 {
        let id = rollup_id(&series, &by, &key, &day);
        let mut union: BTreeMap<String, String> = BTreeMap::new();
        let held = kept.get(&id);
        for item in held.iter().flat_map(|held| held.data.items.iter()) {
            union.insert(item_id(&series, item).to_owned(), item.clone());
        }
        let before = union.len();
        for item in items {
            union
                .entry(item_id(&series, &item).to_owned())
                .or_insert(item);
        }
        if held.is_some() && union.len() == before {
            continue;
        }
        let value = Decimal(union.len().to_string());
        let items: Vec<String> = union.into_values().collect();
        let daily_series_id = DailySeriesId(id.clone());
        let snapshot_id = snapshot.clone();
        if held.is_some() {
            match generated
                .revise_daily_series(ReviseDailySeries {
                    daily_series_id,
                    value,
                    items,
                    snapshot_id,
                })
                .map_err(|unmet| anyhow!("revise-daily-series: {unmet}"))?
            {
                ReviseDailySeriesOutcome::Revised { .. } => {}
                ReviseDailySeriesOutcome::NoSuchRollup { error } => {
                    return Err(anyhow!(
                        "DailySeriesNotFound: no rollup has the id {}",
                        error.daily_series_id.0
                    ));
                }
            }
        } else {
            match generated
                .record_daily_series(RecordDailySeries {
                    daily_series_id,
                    series,
                    day,
                    by,
                    key,
                    value,
                    items,
                    snapshot_id,
                })
                .map_err(|unmet| anyhow!("record-daily-series: {unmet}"))?
            {
                RecordDailySeriesOutcome::Recorded { .. } => {}
                RecordDailySeriesOutcome::AlreadyRecorded { error } => {
                    return Err(anyhow!(
                        "DailySeriesExists: a rollup has the id {} already",
                        error.daily_series_id.0
                    ));
                }
            }
        }
        generated
            .ports
            .0
            .check()
            .with_context(|| format!("keep the rollup {id}"))?;
        written += 1;
    }
    Ok(written)
}
