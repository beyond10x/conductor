//! What the charts and the KPI panels read (`story:dashboard-series-api`,
//! `story:dashboard-kpi-panels`): `/api/series` and `/api/kpis`, from the daily rollups
//! ([`crate::rollup`]) of the store under the instance's state directory.
//!
//! `GET /api/series` lists [`SERIES`]: each one's `series`, `title`, `unit`, the dimensions it is
//! grouped `by`, its `kpi` and `source`, and for a series without a source (`source` null) the
//! `story` that adds one. `GET /api/series?series=<name>&by=<dimension>&days=<n>` answers
//! `{series, by, days, unit, rows}`, each row `{day, key, value}`, by day and key: `day` a UTC
//! date, `key` the group, or `total` without `by`; `days` (default 30, 1 to 365) the days up to
//! today. A day with no data is absent. A series without a source answers 404
//! `{error: "no source yet", story}`; an unknown series or dimension, or a `days` out of bounds,
//! 400 `{error, allowed}`.
//!
//! A count's value is the number of items its rollups hold that day; its total, the items of
//! every group of its first dimension. A duration's rows are its percentiles, keys `p50` and
//! `p90`, nearest rank over the day's items, in hours. A ratio's value is its numerator's total
//! over its denominator's on each day the denominator counts anything ([`Kind::Ratio`]). A
//! usage's value is the peaks of the day's runs over the bytes they reserved, in percent
//! ([`Kind::Usage`]).
//!
//! `GET /api/kpis` answers [`KPIS`], in their order: a measured KPI with its `value` over the
//! last [`WINDOW_DAYS`] days up to today (null when its denominator counts nothing there) and the
//! `series` its trend reads; a KPI without a source with `measured: false` and the `story` that
//! adds it, and no value. The definitions are those of the self-improvement design's § 2.

use std::collections::{BTreeMap, BTreeSet};

use conductor_model::observation::DailySeriesData;
use serde_json::{Value, json};
use time::{Date, Duration};

use crate::rollup::{self, TOTAL};
use crate::store::Store;

/// The path of the series.
pub const SERIES_PATH: &str = "/api/series";

/// The path of the KPIs.
pub const KPIS_PATH: &str = "/api/kpis";

/// The days a series answers without `days`, and the most it answers.
pub const DEFAULT_DAYS: i64 = 30;
pub const MAX_DAYS: i64 = 365;

/// The days a KPI's value is taken over.
pub const WINDOW_DAYS: i64 = 7;

/// The story that adds the transcript series.
const TRANSCRIPTS: &str = "story:kpi-transcripts";

/// The story that adds the per-task KPIs.
const PER_TASK: &str = "story:kpi-per-task";

/// How a series' values are read from the rollups.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// The items of the rollups of the series of this name.
    Count,
    /// The percentiles `p50` and `p90` of the measures of the rollups of the series.
    Duration,
    /// The total of the series `numerator` over that of `denominator`, times `scale`.
    Ratio {
        numerator: &'static str,
        denominator: &'static str,
        scale: f64,
    },
    /// The peaks of the runs of the rollups of the series over the bytes they reserved, in
    /// percent ([`rollup::usage`]).
    Usage,
    /// No source yet: the story that adds one.
    NoSource(&'static str),
}

/// One series `/api/series` lists.
#[derive(Debug, Clone, Copy)]
pub struct Series {
    pub name: &'static str,
    pub title: &'static str,
    pub unit: &'static str,
    pub by: &'static [&'static str],
    pub kpi: Option<&'static str>,
    pub source: Option<&'static str>,
    pub kind: Kind,
}

const fn count(
    name: &'static str,
    title: &'static str,
    by: &'static [&'static str],
    kpi: Option<&'static str>,
    source: &'static str,
) -> Series {
    Series {
        name,
        title,
        unit: "count",
        by,
        kpi,
        source: Some(source),
        kind: Kind::Count,
    }
}

const fn none(
    name: &'static str,
    title: &'static str,
    unit: &'static str,
    kpi: &'static str,
) -> Series {
    Series {
        name,
        title,
        unit,
        by: &["repository", "session"],
        kpi: Some(kpi),
        source: None,
        kind: Kind::NoSource(TRANSCRIPTS),
    }
}

const fn ratio(
    name: &'static str,
    title: &'static str,
    unit: &'static str,
    kpi: &'static str,
    source: &'static str,
    (numerator, denominator, scale): (&'static str, &'static str, f64),
) -> Series {
    Series {
        name,
        title,
        unit,
        by: &[],
        kpi: Some(kpi),
        source: Some(source),
        kind: Kind::Ratio {
            numerator,
            denominator,
            scale,
        },
    }
}

const REPOSITORY: &[&str] = &["repository"];

/// Every series `/api/series` answers, in the order it lists them.
pub const SERIES: [Series; 23] = [
    count(
        "prs_opened",
        "PRs opened",
        REPOSITORY,
        None,
        "forge snapshot: pull-requests (opened_at)",
    ),
    count(
        "prs_merged",
        "PRs merged",
        REPOSITORY,
        Some("K1"),
        "forge snapshot: merged-pull-requests",
    ),
    count(
        "prs_closed_unmerged",
        "PRs closed unmerged",
        REPOSITORY,
        Some("K6"),
        "forge snapshot: pull-requests, closed without merge",
    ),
    count(
        "reverts",
        "Reverts",
        REPOSITORY,
        Some("K6"),
        "forge snapshot: merged-pull-requests titled revert",
    ),
    count(
        "ci_passed",
        "CI runs passed",
        &["repository", "workflow"],
        None,
        "forge snapshot: workflow-runs, conclusion success (newest run per workflow per snapshot)",
    ),
    count(
        "ci_failed",
        "CI runs failed",
        &["repository", "workflow"],
        Some("K6"),
        "forge snapshot: workflow-runs, conclusion failure (newest run per workflow per snapshot)",
    ),
    count(
        "releases",
        "Releases",
        REPOSITORY,
        Some("K5"),
        "forge snapshot: releases (published_at)",
    ),
    Series {
        name: "merged_to_released_hours",
        title: "Merged to released",
        unit: "hours",
        by: &[],
        kpi: Some("K5"),
        source: Some("repository shipped: merged-pull-requests and releases"),
        kind: Kind::Duration,
    },
    Series {
        name: "dispatch_lead_hours",
        title: "Dispatch sent to done",
        unit: "hours",
        by: &[],
        kpi: Some("K4"),
        source: Some("dispatch store, sent to done"),
        kind: Kind::Duration,
    },
    count(
        "operator_touches",
        "Operator touches",
        &[],
        Some("K1"),
        "decision store: operator decisions, O escalations, H items",
    ),
    count(
        "guard_denials",
        "Guard denials",
        &["rule"],
        Some("K10"),
        "guard decisions",
    ),
    count(
        "disk_granted",
        "Disk reservations granted",
        REPOSITORY,
        None,
        "host ledger",
    ),
    count(
        "disk_refused",
        "Disk reservations refused",
        REPOSITORY,
        None,
        "host ledger",
    ),
    count(
        "disk_waited",
        "Disk reservations that waited",
        REPOSITORY,
        None,
        "host ledger",
    ),
    Series {
        name: rollup::PEAK_VS_RESERVED,
        title: "Disk peak against reserved",
        unit: "percent",
        by: REPOSITORY,
        kpi: None,
        source: Some("host ledger: released runs, recorded peak over bytes reserved"),
        kind: Kind::Usage,
    },
    none("tokens_in", "Input tokens", "tokens", "K11"),
    none("tokens_out", "Output tokens", "tokens", "K11"),
    none("idle_hours", "Idle controller time", "hours", "K7"),
    none("waiting_hours", "Waiting time", "hours", "K8"),
    ratio(
        "operator_touches_per_pr",
        "Operator touches per merged PR",
        "ratio",
        "K1",
        "decision store + forge snapshot",
        ("operator_touches", "prs_merged", 1.0),
    ),
    ratio(
        "unnecessary_escalations_pct",
        "Unnecessary escalations",
        "percent",
        "K2",
        "decision store: O and H items withdrawn or answered by conductor",
        ("escalations_unnecessary", "escalations", 100.0),
    ),
    ratio(
        "ci_failed_per_pr",
        "Failed CI runs per merged PR",
        "ratio",
        "K6",
        "forge snapshot: workflow-runs and merged-pull-requests",
        ("ci_failed", "prs_merged", 1.0),
    ),
    ratio(
        "patch_per_feature_release",
        "Patch releases per feature release",
        "ratio",
        "K12",
        "forge snapshot: releases",
        ("releases_patch", "releases_feature", 1.0),
    ),
];

/// The series named `name`.
#[must_use]
pub fn named(name: &str) -> Option<&'static Series> {
    SERIES.iter().find(|series| series.name == name)
}

/// The one dimension a series' total is taken over: its first, or `total` for one without.
fn primary(series: &str) -> &'static str {
    match named(series) {
        Some(known) => known.by.first().copied().unwrap_or(TOTAL),
        // The rollups only a ratio reads.
        None if series.starts_with("releases_") => "repository",
        None => TOTAL,
    }
}

/// `x` rounded to `places`, as a whole JSON number where it is one.
fn number(x: f64, places: i32) -> Value {
    let scale = 10_f64.powi(places);
    let rounded = (x * scale).round() / scale;
    if rounded.fract() == 0.0 && rounded.abs() < 1e15 {
        #[allow(clippy::cast_possible_truncation)]
        let whole = rounded as i64;
        json!(whole)
    } else {
        json!(rounded)
    }
}

/// The nearest-rank `p`th percentile of `values`, which are sorted; none of none.
fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let rank = ((p / 100.0) * values.len() as f64).ceil() as usize;
    values.get(rank.clamp(1, values.len()) - 1).copied()
}

/// The rollups a request reads, as read from the store.
struct Rollups(Vec<DailySeriesData>);

impl Rollups {
    fn of(store: Option<&Store>) -> Self {
        Self(store.map_or_else(Vec::new, |store| {
            store
                .daily_series()
                .into_iter()
                .map(|held| held.data)
                .collect()
        }))
    }

    /// The items of `series` grouped by `by`, by day and key, from `first` on.
    fn grouped(
        &self,
        series: &str,
        by: &str,
        first: Date,
    ) -> BTreeMap<(String, String), BTreeSet<String>> {
        let first = first.to_string();
        let mut out: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for data in &self.0 {
            if data.series == series && data.by == by && data.day >= first {
                out.entry((data.day.clone(), data.key.clone()))
                    .or_default()
                    .extend(data.items.iter().cloned());
            }
        }
        out
    }

    /// The items of `series` over every group of its primary dimension, by day, from `first` on.
    fn totals(&self, series: &str, first: Date) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for ((day, _), items) in self.grouped(series, primary(series), first) {
            out.entry(day).or_default().extend(
                items
                    .iter()
                    .map(|item| rollup::item_id(series, item).to_owned()),
            );
        }
        out
    }

    /// The measures of the duration `series` on each day from `first` on, sorted.
    fn measures(&self, series: &str, first: Date) -> BTreeMap<String, Vec<f64>> {
        let mut out: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        for ((day, _), items) in self.grouped(series, TOTAL, first) {
            let values = out.entry(day).or_default();
            values.extend(items.iter().filter_map(|item| rollup::measure(item)));
            values.sort_by(f64::total_cmp);
        }
        out
    }

    /// The peaks of `series` over their reservations in percent, by day and key: grouped by
    /// `by`, or under `total` over every group of its primary dimension; from `first` on.
    fn usage(&self, series: &str, by: Option<&str>, first: Date) -> Vec<(String, String, Value)> {
        let mut runs: BTreeMap<(String, String), BTreeMap<String, String>> = BTreeMap::new();
        let grouping = by.unwrap_or_else(|| primary(series));
        for ((day, key), items) in self.grouped(series, grouping, first) {
            let key = if by.is_some() { key } else { TOTAL.to_owned() };
            let held = runs.entry((day, key)).or_default();
            for item in items {
                held.entry(rollup::item_id(series, &item).to_owned())
                    .or_insert(item);
            }
        }
        runs.into_iter()
            .filter_map(|((day, key), items)| {
                let (peak, bytes) = items
                    .values()
                    .filter_map(|item| rollup::usage(item))
                    .fold((0.0, 0.0), |(peak, bytes), (one, two)| {
                        (peak + one, bytes + two)
                    });
                (bytes > 0.0).then(|| (day, key, number(peak / bytes * 100.0, 1)))
            })
            .collect()
    }

    /// `(day, key, value)` of `series` grouped by `by` (its total without), from `first` on.
    fn rows(&self, series: &Series, by: Option<&str>, first: Date) -> Vec<(String, String, Value)> {
        match series.kind {
            Kind::NoSource(_) => Vec::new(),
            Kind::Usage => self.usage(series.name, by, first),
            Kind::Count => match by {
                Some(by) => self
                    .grouped(series.name, by, first)
                    .into_iter()
                    .map(|((day, key), items)| (day, key, json!(items.len())))
                    .collect(),
                None => self
                    .totals(series.name, first)
                    .into_iter()
                    .map(|(day, items)| (day, TOTAL.to_owned(), json!(items.len())))
                    .collect(),
            },
            Kind::Duration => self
                .measures(series.name, first)
                .into_iter()
                .flat_map(|(day, values)| {
                    [("p50", 50.0), ("p90", 90.0)]
                        .into_iter()
                        .filter_map(|(key, p)| {
                            Some((
                                day.clone(),
                                key.to_owned(),
                                number(percentile(&values, p)?, 2),
                            ))
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
            Kind::Ratio {
                numerator,
                denominator,
                scale,
            } => {
                let above = self.totals(numerator, first);
                self.totals(denominator, first)
                    .into_iter()
                    .filter(|(_, below)| !below.is_empty())
                    .map(|(day, below)| {
                        let above = above.get(&day).map_or(0, BTreeSet::len);
                        #[allow(clippy::cast_precision_loss)]
                        let value = above as f64 / below.len() as f64 * scale;
                        (day, TOTAL.to_owned(), number(value, 3))
                    })
                    .collect()
            }
        }
    }
}

/// The first day of the `days` days up to `today`.
fn first_day(today: Date, days: i64) -> Date {
    today
        .checked_sub(Duration::days(days - 1))
        .unwrap_or(Date::MIN)
}

/// The list of every series, as `GET /api/series` answers it.
#[must_use]
pub fn list() -> Value {
    Value::Array(
        SERIES
            .iter()
            .map(|series| {
                let mut entry = json!({
                    "series": series.name,
                    "title": series.title,
                    "unit": series.unit,
                    "by": series.by,
                    "kpi": series.kpi,
                    "source": series.source,
                });
                if let Kind::NoSource(story) = series.kind {
                    entry["story"] = json!(story);
                }
                entry
            })
            .collect(),
    )
}

/// The status and JSON body `/api/series` answers the request target `target` with, over `store`
/// (none without one), its days ending `today`.
#[must_use]
pub fn answer(target: &str, store: Option<&Store>, today: Date) -> (u16, Value) {
    let query = |key: &str| super::repo::query(target, key);
    let Some(name) = query("series") else {
        return (200, list());
    };
    let Some(series) = named(&name) else {
        return (
            400,
            json!({
                "error": format!("unknown series {name:?}"),
                "allowed": SERIES.iter().map(|series| series.name).collect::<Vec<_>>(),
            }),
        );
    };
    if let Kind::NoSource(story) = series.kind {
        return (404, json!({"error": "no source yet", "story": story}));
    }
    let by = query("by").filter(|by| !by.is_empty());
    if let Some(by) = &by
        && !series.by.contains(&by.as_str())
    {
        return (
            400,
            json!({
                "error": format!("series {} is not grouped by {by:?}", series.name),
                "allowed": series.by,
            }),
        );
    }
    let days = match query("days") {
        None => DEFAULT_DAYS,
        Some(text) => match text.parse::<i64>() {
            Ok(days) if (1..=MAX_DAYS).contains(&days) => days,
            _ => {
                return (
                    400,
                    json!({
                        "error": format!("days={text} is not a whole number from 1 to {MAX_DAYS}"),
                    }),
                );
            }
        },
    };
    let first = first_day(today, days);
    let last = today.to_string();
    let rows: Vec<Value> = Rollups::of(store)
        .rows(series, by.as_deref(), first)
        .into_iter()
        .filter(|(day, _, _)| *day <= last)
        .map(|(day, key, value)| json!({"day": day, "key": key, "value": value}))
        .collect();
    (
        200,
        json!({
            "series": series.name,
            "by": by,
            "days": days,
            "unit": series.unit,
            "rows": rows,
        }),
    )
}

/// How a KPI's value is worked out over its window.
#[derive(Debug, Clone, Copy)]
enum Measure {
    /// The total of one series over another's, times a scale.
    Ratio(&'static str, &'static str, f64),
    /// The 90th percentile of a duration.
    P90(&'static str),
    /// The total of a series per day of the window.
    PerDay(&'static str),
    /// Not measured yet: the story that adds it.
    Story(&'static str),
}

/// One KPI `/api/kpis` answers.
#[derive(Debug, Clone, Copy)]
pub struct Kpi {
    pub id: &'static str,
    pub title: &'static str,
    pub definition: &'static str,
    pub unit: &'static str,
    /// The target; none until the first measurement sets one.
    pub target: Option<f64>,
    pub source: &'static str,
    /// The `/api/series` series its trend reads, and the key of it where it has several.
    pub series: Option<(&'static str, Option<&'static str>)>,
    measure: Measure,
}

/// Every KPI, K1–K12 then T1–T4.
pub const KPIS: [Kpi; 16] = [
    Kpi {
        id: "K1",
        title: "Operator touches per merged PR",
        definition: "operator-decided decisions + class-O escalations + class-H items, divided by PRs merged",
        unit: "ratio",
        target: Some(0.10),
        source: "decision store + forge snapshot",
        series: Some(("operator_touches_per_pr", None)),
        measure: Measure::Ratio("operator_touches", "prs_merged", 1.0),
    },
    Kpi {
        id: "K2",
        title: "Unnecessary escalations",
        definition: "share of O and H items withdrawn, or resolved without the operator",
        unit: "percent",
        target: Some(10.0),
        source: "decision store",
        series: Some(("unnecessary_escalations_pct", None)),
        measure: Measure::Ratio("escalations_unnecessary", "escalations", 100.0),
    },
    Kpi {
        id: "K3",
        title: "Operator corrections per day",
        definition: "human turns that correct conductor or conductor-dev, per day",
        unit: "count",
        target: Some(2.0),
        source: "session transcripts",
        series: None,
        measure: Measure::Story(TRANSCRIPTS),
    },
    Kpi {
        id: "K4",
        title: "Intent → merged, p90",
        definition: "dispatch sent → done, 90th percentile",
        unit: "hours",
        target: Some(3.0),
        source: "dispatch store + dispatch log",
        series: Some(("dispatch_lead_hours", Some("p90"))),
        measure: Measure::P90("dispatch_lead_hours"),
    },
    Kpi {
        id: "K5",
        title: "Merged → released, p90",
        definition: "PR merged → the release after it, 90th percentile",
        unit: "hours",
        target: Some(4.0),
        source: "forge snapshot: merged-pull-requests and releases",
        series: Some(("merged_to_released_hours", Some("p90"))),
        measure: Measure::P90("merged_to_released_hours"),
    },
    Kpi {
        id: "K6",
        title: "Failed CI runs per merged PR",
        definition: "failed CI runs divided by PRs merged",
        unit: "ratio",
        target: Some(0.10),
        source: "forge snapshot: workflow-runs and merged-pull-requests",
        series: Some(("ci_failed_per_pr", None)),
        measure: Measure::Ratio("ci_failed", "prs_merged", 1.0),
    },
    Kpi {
        id: "K7",
        title: "Idle controller hours",
        definition: "gaps over 15 min in a controller session holding an open dispatch",
        unit: "hours",
        target: Some(34.0),
        source: "session transcripts + dispatch log",
        series: None,
        measure: Measure::Story(TRANSCRIPTS),
    },
    Kpi {
        id: "K8",
        title: "Waiting hours",
        definition: "time sessions spend stopped on CI, disk, a slot or the operator",
        unit: "hours",
        target: Some(67.0),
        source: "session transcripts + host ledger",
        series: None,
        measure: Measure::Story(TRANSCRIPTS),
    },
    Kpi {
        id: "K9",
        title: "Sessions lost to limits",
        definition: "sessions lost to usage limits that never resumed",
        unit: "count",
        target: Some(0.0),
        source: "session transcripts + hand-overs",
        series: None,
        measure: Measure::Story(TRANSCRIPTS),
    },
    Kpi {
        id: "K10",
        title: "Guard denials per day",
        definition: "guard denials per day",
        unit: "count",
        target: Some(8.0),
        source: "guard decisions",
        series: Some(("guard_denials", None)),
        measure: Measure::PerDay("guard_denials"),
    },
    Kpi {
        id: "K11",
        title: "Tokens per merged PR",
        definition: "assistant output tokens per merged PR",
        unit: "tokens",
        target: None,
        source: "session transcripts + forge",
        series: None,
        measure: Measure::Story(TRANSCRIPTS),
    },
    Kpi {
        id: "K12",
        title: "Patch releases per feature release",
        definition: "patch releases divided by feature releases",
        unit: "ratio",
        target: Some(0.2),
        source: "forge snapshot: releases",
        series: Some(("patch_per_feature_release", None)),
        measure: Measure::Ratio("releases_patch", "releases_feature", 1.0),
    },
    Kpi {
        id: "T1",
        title: "Tokens per task (in, out, cache)",
        definition: "input, output and cache tokens per task",
        unit: "tokens",
        target: None,
        source: "session transcripts",
        series: None,
        measure: Measure::Story(PER_TASK),
    },
    Kpi {
        id: "T2",
        title: "Time per task",
        definition: "hours from a task's start to its end",
        unit: "hours",
        target: None,
        source: "session transcripts",
        series: None,
        measure: Measure::Story(PER_TASK),
    },
    Kpi {
        id: "T3",
        title: "Cost per task (list-price equivalent)",
        definition: "tokens per task at list price",
        unit: "USD",
        target: None,
        source: "session transcripts",
        series: None,
        measure: Measure::Story(PER_TASK),
    },
    Kpi {
        id: "T4",
        title: "LLM calls per task",
        definition: "model calls per task",
        unit: "count",
        target: None,
        source: "session transcripts",
        series: None,
        measure: Measure::Story(PER_TASK),
    },
];

/// What `/api/kpis` answers over `store` (none without one), its window ending `today`.
#[must_use]
pub fn kpis(store: Option<&Store>, today: Date) -> Value {
    let rollups = Rollups::of(store);
    let first = first_day(today, WINDOW_DAYS);
    let last = today.to_string();
    let in_window = |series: &str| -> usize {
        rollups
            .totals(series, first)
            .into_iter()
            .filter(|(day, _)| *day <= last)
            .flat_map(|(_, items)| items)
            .collect::<BTreeSet<_>>()
            .len()
    };
    Value::Array(
        KPIS.iter()
            .map(|kpi| {
                let mut row = json!({
                    "id": kpi.id,
                    "title": kpi.title,
                    "definition": kpi.definition,
                    "unit": kpi.unit,
                    "target": kpi.target.map(|target| number(target, 3)),
                    "direction": "lower",
                });
                #[allow(clippy::cast_precision_loss)]
                let value = match kpi.measure {
                    Measure::Story(story) => {
                        row["measured"] = json!(false);
                        row["story"] = json!(story);
                        return row;
                    }
                    Measure::Ratio(above, below, scale) => {
                        let below = in_window(below);
                        (below > 0).then(|| in_window(above) as f64 / below as f64 * scale)
                    }
                    Measure::P90(series) => {
                        let mut values: Vec<f64> = rollups
                            .measures(series, first)
                            .into_iter()
                            .filter(|(day, _)| *day <= last)
                            .flat_map(|(_, values)| values)
                            .collect();
                        values.sort_by(f64::total_cmp);
                        percentile(&values, 90.0)
                    }
                    Measure::PerDay(series) => Some(in_window(series) as f64 / WINDOW_DAYS as f64),
                };
                row["value"] = value.map_or(Value::Null, |value| number(value, 3));
                row["window_days"] = json!(WINDOW_DAYS);
                if let Some((series, key)) = kpi.series {
                    row["series"] = json!(series);
                    if let Some(key) = key {
                        row["series_key"] = json!(key);
                    }
                }
                row["source"] = json!(kpi.source);
                row["measured"] = json!(true);
                row
            })
            .collect(),
    )
}
