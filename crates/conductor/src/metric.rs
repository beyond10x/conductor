//! `conductor metric`: the measured quantities of the self-improvement loop, their daily readings
//! and the targets that make some of them KPIs (`docs/design/self-improvement.md` § 2,
//! `story:metrics-naming`), and `metric compute`, the instrument of the metrics computed from the
//! store, the records and the forge snapshot (`story:kpi-records`).
//!
//! A command reads its input from its flags, opens the store under the state directory
//! ([`crate::state::open`]), runs the generated behaviour (`SetMetricTarget` is an obligation of
//! the generated model, implemented here against its contract by [`Ports`]), and checks that the
//! store kept what the behaviour reported before it answers. A refused command exits 1 with one
//! line on standard error: the command, the error the specification declares, and what it
//! means. `--input-json` is not read yet, and answers [`NotImplemented`](crate::NotImplemented).
//!
//! A view opens only an existing store, creates nothing, and prints exactly the fields its
//! specification declares. Decimals are printed as the text they were given in, so a value reads
//! back exactly. `metric kpis` lists the KPIs with their targets; `metric kpi-readings` lists
//! each KPI's newest instance-wide reading, the target it was judged against and whether it met
//! it.
//!
//! **`metric compute --day <YYYY-MM-DD>`** reads the UTC day `[day, day + 1)` and records one
//! reading per metric and scope through `RecordMetricReading`, with the reading id
//! `<metric>|<scope>|<day>`. It first defines each metric of [`METRICS`] that is absent, without a
//! target. A metric that has a target judges each reading against it when it is recorded: the
//! reading carries that `target` and whether it was `met`; one without leaves both absent. A
//! reading the store already holds is kept as it is, so a second run for a day records nothing
//! new. After recording, each metric and scope it touched keeps one `Recorded` reading, the
//! newest (the latest day, then the latest recorded); compute supersedes every other one through
//! `SupersedeMetricReading`, so computing an older day supersedes only the new older reading. A metric with nothing to measure on the day (no
//! complete snapshot, no pull request merged, no item closed, no dispatch done, no release)
//! records no
//! reading and is named on standard output. The metrics, each in the unit its definition says:
//!
//! - **K1**, operator touches per merged PR: the `decisions/` lines of the day that are a
//!   decision the operator decided (`decided`, `decided_by: Operator`) or an escalation as class
//!   O or H (`escalated`), divided by the pull requests the newest complete snapshot holds merged
//!   on the day. Scope `instance`.
//! - **K2.O**, **K2.H**, unnecessary escalations: of the requests escalated as that class (on any
//!   day) whose first closing line (`withdrawn` or `answered`) is on the day, the percentage
//!   withdrawn: moot, or resolved without the operator. Scope `instance`.
//! - **K4.median**, **K4.p90**, intent to merged: hours from a dispatch's first `sent` line to
//!   its first `done` line, over the dispatches done on the day. Scope `instance` and each
//!   repository the `sent` line names.
//! - **K5.median**, **K5.p90**, merged to released: hours from a pull request's merge to the first
//!   release of its repository published after it, over the pull requests whose release was
//!   published on the day, read from the newest complete snapshot (as `repository shipped`
//!   associates them). Scope `instance` and each repository.
//! - **K5.unreleased**: the pull requests the newest complete snapshot holds merged before the
//!   day's end with no release of their repository published after them before then. Scope
//!   `instance` and each repository with a pull request merged by then.
//! - **K6.ci_failed**, rework: the CI runs that failed on the day over the pull requests merged
//!   on the day, as the store's daily rollups count them (`ci_failed`, `prs_merged`), which keep
//!   what every completed snapshot saw: a snapshot holds only each workflow's newest run. Scope
//!   `instance` and each repository with a pull request merged on the day.
//! - **K6.closed_unmerged**: of the pull requests closed on the day, merged or closed without a
//!   merge (the rollups' `prs_merged` and `prs_closed_unmerged`), the percentage closed without a
//!   merge. Scope `instance` and each repository with a pull request closed on the day.
//! - **K6.quick_patches**: of the releases the newest complete snapshot holds published on the
//!   day, the percentage that are a patch release (a tag ending in `x.y.z`, `z` above 0) published
//!   at most 24 h after the release of their repository before it. Scope `instance` and each
//!   repository with a release on the day.
//! - **K12.fix_releases**, defects shipped: the patch releases published on the day over the
//!   feature releases (`x.y.0`), as the rollups count them (`releases_patch`,
//!   `releases_feature`). Scope `instance` and each repository with a feature release on the
//!   day. K12's second quantity, issues labelled bug, has no instrument: no collector records
//!   issues or their labels.
//!
//! A median of an even count is the mean of the two middle values; a p90 is the nearest rank,
//! the ⌈0.9 n⌉-th smallest. Values are rounded to three decimals.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{Generated, MetricReadingStorage, MetricStorage};
use conductor_model::improvement::obligations::{
    DefineMetricBehavior, KpiReadingsQuery, KpisQuery, MetricReadingsQuery, MetricsQuery,
    RecordMetricReadingBehavior, SetMetricTargetBehavior, SupersedeMetricReadingBehavior,
};
use conductor_model::improvement::{
    AnyMetric, DefineMetric, DefineMetricOutcome, MetricId, MetricNotFound, MetricReadingId,
    MetricReadingSnapshot, MetricReadingState, MetricSnapshot, MetricState, MetricTargetSet,
    RecordMetricReading, RecordMetricReadingOutcome, SetMetricTarget, SetMetricTargetOutcome,
    SupersedeMetricReading, SupersedeMetricReadingOutcome, TargetDirection,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::SnapshotState;
use conductor_model::observation::obligations::{
    MergedPullRequestsQuery, ReleasesQuery, SnapshotsQuery,
};
use conductor_model::primitives::{Decimal, Timestamp};
use serde_json::{Map, Value};
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::cli::{
    DefineMetricArgs, JsonInput, MetricComputeArgs, RecordMetricReadingArgs, SetMetricTargetArgs,
    SupersedeMetricReadingArgs, ViewArgs,
};
use crate::not_implemented;
use crate::store::Store;

// ---------------------------------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------------------------------

/// `conductor metric define-metric`: the command `conductor.improvement.DefineMetric`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep the metric, or
/// the specification refuses it: `MetricExists`.
pub fn define_metric(state: Option<&Path>, args: DefineMetricArgs) -> Result<ExitCode> {
    const COMMAND: &str = "metric define-metric";
    flags_only(args.input_json, "metric define-metric --input-json")?;
    let input = DefineMetric {
        metric_id: MetricId(flag(COMMAND, "--metric-id", args.metric_id)?),
        name: flag(COMMAND, "--name", args.name)?,
        unit: flag(COMMAND, "--unit", args.unit)?,
        instrument: flag(COMMAND, "--instrument", args.instrument)?,
        definition: flag(COMMAND, "--definition", args.definition)?,
        baseline: args.baseline.map(Decimal),
        baseline_period: args.baseline_period,
    };
    let mut generated = Generated::new(Ports::open(state)?);
    let outcome = generated
        .define_metric(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        DefineMetricOutcome::Defined { .. } => Ok(ExitCode::SUCCESS),
        DefineMetricOutcome::AlreadyDefined { error } => refused(
            COMMAND,
            "MetricExists",
            &format!(
                "a metric already has the id {:?}; the held metric is unchanged",
                error.metric_id.0
            ),
        ),
    }
}

/// `conductor metric set-metric-target`: the command `conductor.improvement.SetMetricTarget`,
/// whose behaviour [`Ports`] implements.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep the target, or
/// the specification refuses it: `MetricNotFound`.
pub fn set_metric_target(state: Option<&Path>, args: SetMetricTargetArgs) -> Result<ExitCode> {
    const COMMAND: &str = "metric set-metric-target";
    flags_only(args.input_json, "metric set-metric-target --input-json")?;
    let direction = flag(COMMAND, "--direction", args.direction)?;
    let input = SetMetricTarget {
        metric_id: MetricId(flag(COMMAND, "--metric-id", args.metric_id)?),
        target: Decimal(flag(COMMAND, "--target", args.target)?),
        direction: direction_named(&direction).ok_or_else(|| {
            anyhow!("{COMMAND}: --direction {direction:?} is not AtMost or AtLeast")
        })?,
        target_owner: flag(COMMAND, "--target-owner", args.target_owner)?,
        target_set_at: Timestamp(flag(COMMAND, "--target-set-at", args.target_set_at)?),
    };
    let mut generated = Generated::new(Ports::open(state)?);
    let outcome = generated
        .set_metric_target(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        SetMetricTargetOutcome::Targeted { .. } | SetMetricTargetOutcome::Retargeted { .. } => {
            Ok(ExitCode::SUCCESS)
        }
        SetMetricTargetOutcome::NoSuchMetric { error } => {
            refused(COMMAND, "MetricNotFound", &no_metric(&error.metric_id))
        }
    }
}

/// `conductor metric record-metric-reading`: the command
/// `conductor.improvement.RecordMetricReading`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, `--sources` holds an item that is not text, the
/// store does not open or keep the reading, or the specification refuses it: `MetricNotFound`,
/// `MetricReadingExists`.
pub fn record_metric_reading(
    state: Option<&Path>,
    args: RecordMetricReadingArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "metric record-metric-reading";
    flags_only(args.input_json, "metric record-metric-reading --input-json")?;
    let sources = flag(COMMAND, "--sources", args.sources)?;
    let sources: Vec<String> = serde_json::from_str(&sources)
        .map_err(|_| anyhow!("{COMMAND}: --sources is not a JSON array of strings"))?;
    let input = RecordMetricReading {
        reading_id: MetricReadingId(flag(COMMAND, "--reading-id", args.reading_id)?),
        metric_id: MetricId(flag(COMMAND, "--metric-id", args.metric_id)?),
        day: flag(COMMAND, "--day", args.day)?,
        scope: flag(COMMAND, "--scope", args.scope)?,
        value: Decimal(flag(COMMAND, "--value", args.value)?),
        sources,
        target: args.target.map(Decimal),
        met: args.met,
        recorded_at: Timestamp(flag(COMMAND, "--recorded-at", args.recorded_at)?),
    };
    let mut generated = Generated::new(Ports::open(state)?);
    let outcome = generated
        .record_metric_reading(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        RecordMetricReadingOutcome::Recorded { .. } => Ok(ExitCode::SUCCESS),
        RecordMetricReadingOutcome::NoSuchMetric { error } => {
            refused(COMMAND, "MetricNotFound", &no_metric(&error.metric_id))
        }
        RecordMetricReadingOutcome::AlreadyRecorded { error } => refused(
            COMMAND,
            "MetricReadingExists",
            &already_recorded(&error.reading_id),
        ),
    }
}

/// `conductor metric supersede-metric-reading`: the command
/// `conductor.improvement.SupersedeMetricReading`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given, the store does not open or keep the move, or the
/// specification refuses it: `MetricReadingNotFound`, `MetricReadingStateConflict`.
pub fn supersede_metric_reading(
    state: Option<&Path>,
    args: SupersedeMetricReadingArgs,
) -> Result<ExitCode> {
    const COMMAND: &str = "metric supersede-metric-reading";
    flags_only(
        args.input_json,
        "metric supersede-metric-reading --input-json",
    )?;
    let input = SupersedeMetricReading {
        reading_id: MetricReadingId(flag(COMMAND, "--reading-id", args.reading_id)?),
        by: MetricReadingId(flag(COMMAND, "--by", args.by)?),
    };
    let mut generated = Generated::new(Ports::open(state)?);
    let outcome = generated
        .supersede_metric_reading(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    superseded(COMMAND, outcome).map(|()| ExitCode::SUCCESS)
}

/// `Ok` when the reading was superseded, else the refusal the specification declares.
fn superseded(command: &str, outcome: SupersedeMetricReadingOutcome) -> Result<()> {
    match outcome {
        SupersedeMetricReadingOutcome::Superseded { .. } => Ok(()),
        SupersedeMetricReadingOutcome::NoSuchReading { error } => bail!(
            "{command}: MetricReadingNotFound: no reading has the id {:?}",
            error.reading_id.0
        ),
        SupersedeMetricReadingOutcome::WrongState { error } => bail!(
            "{command}: MetricReadingStateConflict: the reading {:?} is superseded already, and \
             nothing moved",
            error.reading_id.0
        ),
    }
}

// ---------------------------------------------------------------------------------------------
// The views
// ---------------------------------------------------------------------------------------------

/// `conductor metric metrics`: the view `conductor.improvement.Metrics`, every metric, KPI or
/// not.
///
/// # Errors
///
/// There is no store, it does not open, or the rows cannot be written.
pub fn metrics(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 12] = [
        "metric_id",
        "state",
        "name",
        "unit",
        "instrument",
        "definition",
        "baseline",
        "baseline_period",
        "target",
        "direction",
        "target_owner",
        "target_set_at",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let mut rows = generated
        .metrics()
        .map_err(|unmet| anyhow!("metric metrics: {unmet}"))?;
    rows.sort_by(|one, other| one.metric_id.0.cmp(&other.metric_id.0));
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.metric_id.0),
                Value::from(metric_state_name(row.state)),
                Value::from(row.name),
                Value::from(row.unit),
                Value::from(row.instrument),
                Value::from(row.definition),
                decimal(row.baseline.as_ref()),
                Value::from(row.baseline_period),
                decimal(row.target.as_ref()),
                Value::from(row.direction.map(direction_name)),
                Value::from(row.target_owner),
                Value::from(row.target_set_at.map(|at| at.0)),
            ]
        })
        .collect();
    crate::store::show(&generated.ports, "metrics", &view.render(&COLUMNS, &rows))
}

/// `conductor metric kpis`: the view `conductor.improvement.Kpis`, the metrics with a target.
///
/// # Errors
///
/// There is no store, it does not open, or the rows cannot be written.
pub fn kpis(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 10] = [
        "metric_id",
        "state",
        "name",
        "unit",
        "instrument",
        "baseline",
        "target",
        "direction",
        "target_owner",
        "target_set_at",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let mut rows = generated
        .kpis()
        .map_err(|unmet| anyhow!("metric kpis: {unmet}"))?;
    rows.sort_by(|one, other| one.metric_id.0.cmp(&other.metric_id.0));
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.metric_id.0),
                Value::from(metric_state_name(row.state)),
                Value::from(row.name),
                Value::from(row.unit),
                Value::from(row.instrument),
                decimal(row.baseline.as_ref()),
                decimal(row.target.as_ref()),
                Value::from(row.direction.map(direction_name)),
                Value::from(row.target_owner),
                Value::from(row.target_set_at.map(|at| at.0)),
            ]
        })
        .collect();
    crate::store::show(&generated.ports, "KPIs", &view.render(&COLUMNS, &rows))
}

/// `conductor metric kpi-readings`: the view `conductor.improvement.KpiReadings`, each KPI's
/// newest instance-wide reading with the target it was judged against and whether it met it.
///
/// # Errors
///
/// There is no store, it does not open, or the rows cannot be written.
pub fn kpi_readings(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 7] = [
        "metric_id",
        "scope",
        "day",
        "value",
        "target",
        "met",
        "reading_id",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let mut rows = generated
        .kpi_readings()
        .map_err(|unmet| anyhow!("metric kpi-readings: {unmet}"))?;
    rows.sort_by(|one, other| one.metric_id.0.cmp(&other.metric_id.0));
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.metric_id.0),
                Value::from(row.scope),
                Value::from(row.day),
                Value::from(row.value.0),
                decimal(row.target.as_ref()),
                Value::from(row.met),
                Value::from(row.reading_id.0),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "KPI readings",
        &view.render(&COLUMNS, &rows),
    )
}

/// `conductor metric metric-readings`: the view `conductor.improvement.MetricReadings`, every
/// reading, by metric, scope and day.
///
/// # Errors
///
/// There is no store, it does not open, or the rows cannot be written.
pub fn metric_readings(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 10] = [
        "reading_id",
        "state",
        "metric_id",
        "day",
        "scope",
        "value",
        "sources",
        "target",
        "met",
        "recorded_at",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let mut rows = generated
        .metric_readings()
        .map_err(|unmet| anyhow!("metric metric-readings: {unmet}"))?;
    rows.sort_by(|one, other| {
        (&one.metric_id.0, &one.scope, &one.day).cmp(&(
            &other.metric_id.0,
            &other.scope,
            &other.day,
        ))
    });
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.reading_id.0),
                Value::from(reading_state_name(row.state)),
                Value::from(row.metric_id.0),
                Value::from(row.day),
                Value::from(row.scope),
                Value::from(row.value.0),
                Value::from(row.sources),
                decimal(row.target.as_ref()),
                Value::from(row.met),
                Value::from(row.recorded_at.0),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "metric readings",
        &view.render(&COLUMNS, &rows),
    )
}

// ---------------------------------------------------------------------------------------------
// metric compute
// ---------------------------------------------------------------------------------------------

/// The scope of a reading over the whole instance.
const INSTANCE: &str = "instance";

/// The days the design's baselines cover.
const BASELINE_PERIOD: &str = "2026-10-06..13";

/// One metric `metric compute` records, as it defines it when it is absent.
pub struct Definition {
    /// The id, as design § 2 names it; a row that reports two quantities is two metrics.
    pub id: &'static str,
    pub name: &'static str,
    pub unit: &'static str,
    pub instrument: &'static str,
    pub definition: &'static str,
    /// The design's baseline over [`BASELINE_PERIOD`].
    pub baseline: &'static str,
}

const DECISION_LOG: &str = "the records' decisions/ log";
const DISPATCH_LOG: &str = "the records' dispatches/ log";
const SNAPSHOT: &str = "the newest complete snapshot's merged pull requests and releases";

/// The metrics `metric compute` records (design § 2: K1, K2, K4, K5, K6, K12).
pub const METRICS: [Definition; 12] = [
    Definition {
        id: "K1",
        name: "operator touches per merged PR",
        unit: "ratio",
        instrument: "the records' decisions/ log + the newest complete snapshot's merged pull requests",
        definition: "decisions the operator decided, escalations as class O and escalations as class H on the day, divided by the pull requests merged on the day",
        baseline: "0.245",
    },
    Definition {
        id: "K2.O",
        name: "unnecessary O escalations",
        unit: "percent",
        instrument: DECISION_LOG,
        definition: "of the requests escalated as class O whose first withdrawal or answer is on the day, the share withdrawn: moot, or resolved without the operator",
        baseline: "33",
    },
    Definition {
        id: "K2.H",
        name: "unnecessary H items",
        unit: "percent",
        instrument: DECISION_LOG,
        definition: "of the requests escalated as class H whose first withdrawal or answer is on the day, the share withdrawn: moot, or resolved without the operator",
        baseline: "76",
    },
    Definition {
        id: "K4.median",
        name: "intent to merged, median",
        unit: "hours",
        instrument: DISPATCH_LOG,
        definition: "hours from a dispatch's sent line to its done line, median over the dispatches done on the day",
        baseline: "0.75",
    },
    Definition {
        id: "K4.p90",
        name: "intent to merged, p90",
        unit: "hours",
        instrument: DISPATCH_LOG,
        definition: "hours from a dispatch's sent line to its done line, nearest-rank p90 over the dispatches done on the day",
        baseline: "4.17",
    },
    Definition {
        id: "K5.median",
        name: "merged to released, median",
        unit: "hours",
        instrument: SNAPSHOT,
        definition: "hours from a pull request's merge to the first release of its repository published after it, median over the pull requests released on the day",
        baseline: "0.79",
    },
    Definition {
        id: "K5.p90",
        name: "merged to released, p90",
        unit: "hours",
        instrument: SNAPSHOT,
        definition: "hours from a pull request's merge to the first release of its repository published after it, nearest-rank p90 over the pull requests released on the day",
        baseline: "21.97",
    },
    Definition {
        id: "K5.unreleased",
        name: "merged and not released",
        unit: "count",
        instrument: SNAPSHOT,
        definition: "pull requests merged before the day's end with no release of their repository published after them before then",
        baseline: "44",
    },
    Definition {
        id: "K6.ci_failed",
        name: "failed CI runs per merged PR",
        unit: "ratio",
        instrument: "the store's daily rollups: ci_failed and prs_merged",
        definition: "CI runs that failed on the day, divided by the pull requests merged on the day",
        baseline: "0.18",
    },
    Definition {
        id: "K6.closed_unmerged",
        name: "PRs closed unmerged",
        unit: "percent",
        instrument: "the store's daily rollups: prs_closed_unmerged and prs_merged",
        definition: "of the pull requests closed on the day, merged or closed without a merge, the share closed without a merge",
        baseline: "1.5",
    },
    Definition {
        id: "K6.quick_patches",
        name: "patch releases within 24 h of a release",
        unit: "percent",
        instrument: "the newest complete snapshot's releases",
        definition: "of the releases published on the day, the share that are a patch release published at most 24 h after the release of their repository before it",
        baseline: "20",
    },
    Definition {
        id: "K12.fix_releases",
        name: "patch releases per feature release",
        unit: "ratio",
        instrument: "the store's daily rollups: releases_patch and releases_feature",
        definition: "patch releases (x.y.z, z above 0) published on the day, divided by the feature releases (x.y.0) published on the day",
        baseline: "0.45",
    },
];

/// One value a metric takes for one scope on the day, with the evidence ids it was computed
/// from; or, with no value, why there is none.
struct Measured {
    metric: &'static str,
    scope: String,
    value: Result<f64, String>,
    sources: Vec<String>,
}

impl Measured {
    fn value(metric: &'static str, scope: &str, value: f64, sources: Vec<String>) -> Self {
        Self {
            metric,
            scope: scope.to_owned(),
            value: Ok(value),
            sources,
        }
    }

    fn none(metric: &'static str, scope: &str, why: &str) -> Self {
        Self {
            metric,
            scope: scope.to_owned(),
            value: Err(why.to_owned()),
            sources: Vec::new(),
        }
    }
}

/// The day `metric compute` reads: `[start, end)`, UTC.
struct Day {
    text: String,
    start: OffsetDateTime,
    end: OffsetDateTime,
}

impl Day {
    fn parse(text: &str) -> Result<Self> {
        let well_formed = text.len() == 10
            && text.bytes().enumerate().all(|(at, byte)| {
                if at == 4 || at == 7 {
                    byte == b'-'
                } else {
                    byte.is_ascii_digit()
                }
            });
        let start = OffsetDateTime::parse(&format!("{text}T00:00:00Z"), &Rfc3339)
            .ok()
            .filter(|_| well_formed)
            .ok_or_else(|| anyhow!("--day {text:?} is not a date, YYYY-MM-DD"))?;
        Ok(Self {
            text: text.to_owned(),
            start,
            end: start + Duration::days(1),
        })
    }

    fn holds(&self, at: OffsetDateTime) -> bool {
        self.start <= at && at < self.end
    }
}

/// `conductor metric compute --day <YYYY-MM-DD>`, as the module documentation describes. It opens
/// the store first, creating it where none is, and reads the snapshot through a second handle.
///
/// # Errors
///
/// `--day` is missing or no date, the records' logs cannot be read, the store does not open,
/// read or keep what was recorded, or standard output cannot be written.
pub fn compute(state: Option<&Path>, args: &MetricComputeArgs) -> Result<ExitCode> {
    const COMMAND: &str = "metric compute";
    let given = args
        .day
        .as_deref()
        .ok_or_else(|| anyhow!("{COMMAND}: --day is missing"))?;
    let day = Day::parse(given).with_context(|| COMMAND.to_owned())?;
    let records = PathBuf::from(&crate::config::active().instance.records);
    let (decisions, unread_decisions) =
        crate::records::log_objects(&records, crate::records::DECISIONS)
            .with_context(|| COMMAND.to_owned())?;
    let (dispatches, unread_dispatches) =
        crate::records::log_objects(&records, crate::records::DISPATCHES)
            .with_context(|| COMMAND.to_owned())?;
    if unread_decisions + unread_dispatches > 0 {
        let _ = writeln!(
            std::io::stderr().lock(),
            "{COMMAND}: {} line(s) of the records' logs are no JSON object and are not read; `conductor records check` names them",
            unread_decisions + unread_dispatches
        );
    }

    let mut generated = Generated::new(Ports::open(state)?);
    let forge = Forge::read(state).with_context(|| COMMAND.to_owned())?;

    let mut measured = Vec::new();
    measured.push(k1(&day, &decisions, forge.as_ref()));
    measured.extend(k2(&day, &decisions));
    measured.extend(k4(&day, &dispatches));
    measured.extend(k5(&day, forge.as_ref()));
    let rollups = Rollups::read(&generated.ports.store, &day);
    kept(COMMAND, &generated.ports.store)?;
    measured.extend(k6(&day, &rollups, forge.as_ref()));
    measured.extend(k12(&rollups));

    for metric in &METRICS {
        let outcome = generated
            .define_metric(DefineMetric {
                metric_id: MetricId(metric.id.to_owned()),
                name: metric.name.to_owned(),
                unit: metric.unit.to_owned(),
                instrument: metric.instrument.to_owned(),
                definition: metric.definition.to_owned(),
                baseline: Some(Decimal(metric.baseline.to_owned())),
                baseline_period: Some(BASELINE_PERIOD.to_owned()),
            })
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        kept(COMMAND, &generated.ports.store)?;
        match outcome {
            DefineMetricOutcome::Defined { .. } | DefineMetricOutcome::AlreadyDefined { .. } => {}
        }
    }

    let recorded_at = crate::records::now()?;
    let mut out = String::new();
    let mut touched: BTreeSet<(&'static str, String)> = BTreeSet::new();
    for reading in measured {
        let id = format!("{}|{}|{}", reading.metric, reading.scope, day.text);
        let value = match reading.value {
            Ok(value) => value,
            Err(why) => {
                let _ = writeln!(out, "{id}: no value: {why}");
                continue;
            }
        };
        let metric_id = MetricId(reading.metric.to_owned());
        touched.insert((reading.metric, reading.scope.clone()));
        let held = MetricStorage::get(&generated.ports.store, &metric_id);
        let met = held.as_ref().and_then(|metric| meets(metric, value));
        let target = held
            .and_then(|metric| metric.data.target)
            .filter(|_| met.is_some());
        let text = rendered(value);
        let outcome = generated
            .record_metric_reading(RecordMetricReading {
                reading_id: MetricReadingId(id.clone()),
                metric_id,
                day: day.text.clone(),
                scope: reading.scope,
                value: Decimal(text.clone()),
                sources: reading.sources,
                target,
                met,
                recorded_at: recorded_at.clone(),
            })
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        kept(COMMAND, &generated.ports.store)?;
        match outcome {
            RecordMetricReadingOutcome::Recorded { .. } => {
                let verdict = match met {
                    Some(true) => " (meets its target)",
                    Some(false) => " (misses its target)",
                    None => "",
                };
                let _ = writeln!(out, "{id}: recorded {text}{verdict}");
            }
            RecordMetricReadingOutcome::AlreadyRecorded { .. } => {
                let _ = writeln!(out, "{id}: already recorded, kept as it is");
            }
            RecordMetricReadingOutcome::NoSuchMetric { error } => {
                bail!("{COMMAND}: MetricNotFound: {}", no_metric(&error.metric_id));
            }
        }
    }

    // Each metric and scope touched keeps one `Recorded` reading, its newest.
    let held = MetricReadingStorage::list(&generated.ports.store);
    kept(COMMAND, &generated.ports.store)?;
    for (metric, scope) in touched {
        let mut current: Vec<&MetricReadingSnapshot> = held
            .iter()
            .filter(|reading| {
                reading.state == MetricReadingState::Recorded
                    && reading.data.metric_id.0 == metric
                    && reading.data.scope == scope
            })
            .collect();
        current.sort_by(|one, other| {
            (&one.data.day, &one.data.recorded_at).cmp(&(&other.data.day, &other.data.recorded_at))
        });
        let Some((newest, older)) = current.split_last() else {
            continue;
        };
        for reading in older {
            let outcome = generated
                .supersede_metric_reading(SupersedeMetricReading {
                    reading_id: reading.data.reading_id.clone(),
                    by: newest.data.reading_id.clone(),
                })
                .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
            kept(COMMAND, &generated.ports.store)?;
            superseded(COMMAND, outcome)?;
            let _ = writeln!(
                out,
                "{}: superseded by {}",
                reading.data.reading_id.0, newest.data.reading_id.0
            );
        }
    }
    std::io::stdout()
        .lock()
        .write_all(out.as_bytes())
        .with_context(|| format!("{COMMAND}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}

/// Whether `value` meets the target `metric` holds, or none when it holds none.
fn meets(metric: &MetricSnapshot, value: f64) -> Option<bool> {
    let target: f64 = metric.data.target.as_ref()?.0.parse().ok()?;
    Some(match metric.data.direction? {
        TargetDirection::AtMost => value <= target,
        TargetDirection::AtLeast => value >= target,
    })
}

/// `value` rounded to three decimals, without trailing zeros.
fn rendered(value: f64) -> String {
    let text = format!("{value:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// A line's text field `key`.
fn field<'a>(line: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    line.get(key).and_then(Value::as_str)
}

/// A line's `at`, when it is an RFC 3339 instant.
fn at(line: &Map<String, Value>) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(field(line, "at")?, &Rfc3339).ok()
}

fn hours(from: OffsetDateTime, to: OffsetDateTime) -> f64 {
    (to - from).as_seconds_f64() / 3600.0
}

/// The median of `values`: the middle one, or the mean of the two middle ones.
fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        f64::midpoint(sorted[n / 2 - 1], sorted[n / 2])
    }
}

/// The nearest-rank p90 of `values`: the ⌈0.9 n⌉-th smallest.
fn p90(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (9 * sorted.len()).div_ceil(10);
    sorted[rank.max(1) - 1]
}

/// K1 for the instance.
fn k1(day: &Day, decisions: &[Map<String, Value>], forge: Option<&Forge>) -> Measured {
    const ID: &str = "K1";
    let Some(forge) = forge else {
        return Measured::none(ID, INSTANCE, "no complete snapshot in the store");
    };
    let mut sources = Vec::new();
    for line in decisions {
        if !at(line).is_some_and(|at| day.holds(at)) {
            continue;
        }
        let touch = match field(line, "event") {
            Some("decided") => field(line, "decided_by") == Some("Operator"),
            Some("escalated") => matches!(field(line, "escalation_class"), Some("O" | "H")),
            _ => false,
        };
        if touch && let Some(id) = field(line, "id") {
            sources.push(id.to_owned());
        }
    }
    let touches = sources.len();
    let merged: Vec<&Pull> = forge
        .pulls
        .iter()
        .filter(|pull| day.holds(pull.merged_at))
        .collect();
    if merged.is_empty() {
        return Measured::none(ID, INSTANCE, "no pull request merged on the day");
    }
    sources.extend(merged.iter().map(|pull| pull.id()));
    #[allow(clippy::cast_precision_loss)]
    let value = touches as f64 / merged.len() as f64;
    Measured::value(ID, INSTANCE, value, sources)
}

/// K2.O and K2.H for the instance.
fn k2(day: &Day, decisions: &[Map<String, Value>]) -> Vec<Measured> {
    let mut class: BTreeMap<&str, &str> = BTreeMap::new();
    let mut closed: BTreeMap<&str, (OffsetDateTime, bool)> = BTreeMap::new();
    for line in decisions {
        let (Some(id), Some(at)) = (field(line, "id"), at(line)) else {
            continue;
        };
        match field(line, "event") {
            Some("escalated") => {
                if let Some(escalated) = field(line, "escalation_class") {
                    class.entry(id).or_insert(escalated);
                }
            }
            Some(event @ ("withdrawn" | "answered")) => {
                let withdrawn = event == "withdrawn";
                closed
                    .entry(id)
                    .and_modify(|first| {
                        if at < first.0 {
                            *first = (at, withdrawn);
                        }
                    })
                    .or_insert((at, withdrawn));
            }
            _ => {}
        }
    }
    [("K2.O", "O"), ("K2.H", "H")]
        .into_iter()
        .map(|(metric, wanted)| {
            let items: Vec<(&str, bool)> = closed
                .iter()
                .filter(|(id, (at, _))| {
                    class.get(*id).is_some_and(|class| *class == wanted) && day.holds(*at)
                })
                .map(|(id, (_, withdrawn))| (*id, *withdrawn))
                .collect();
            if items.is_empty() {
                return Measured::none(
                    metric,
                    INSTANCE,
                    &format!("no request escalated as class {wanted} was closed on the day"),
                );
            }
            let withdrawn = items.iter().filter(|(_, withdrawn)| *withdrawn).count();
            #[allow(clippy::cast_precision_loss)]
            let value = 100.0 * withdrawn as f64 / items.len() as f64;
            let sources = items.iter().map(|(id, _)| (*id).to_owned()).collect();
            Measured::value(metric, INSTANCE, value, sources)
        })
        .collect()
}

/// K4.median and K4.p90, for the instance and each repository.
fn k4(day: &Day, dispatches: &[Map<String, Value>]) -> Vec<Measured> {
    let mut sent: BTreeMap<&str, (OffsetDateTime, &str)> = BTreeMap::new();
    let mut done: BTreeMap<&str, OffsetDateTime> = BTreeMap::new();
    for line in dispatches {
        let (Some(id), Some(at)) = (field(line, "id"), at(line)) else {
            continue;
        };
        match field(line, "event") {
            Some("sent") if sent.get(id).is_none_or(|(held, _)| at < *held) => {
                sent.insert(id, (at, field(line, "repo").unwrap_or_default()));
            }
            Some("done") if done.get(id).is_none_or(|held| at < *held) => {
                done.insert(id, at);
            }
            _ => {}
        }
    }
    let mut samples: Vec<(&str, &str, f64)> = Vec::new();
    for (id, done_at) in &done {
        if !day.holds(*done_at) {
            continue;
        }
        if let Some((sent_at, repo)) = sent.get(id)
            && sent_at <= done_at
        {
            samples.push((id, repo, hours(*sent_at, *done_at)));
        }
    }
    let scoped = scopes(
        samples
            .iter()
            .map(|(id, repo, hours)| (*repo, (*id).to_owned(), *hours)),
    );
    quantiles(
        ("K4.median", "K4.p90"),
        &scoped,
        "no dispatch was done on the day",
    )
}

/// K5.median, K5.p90 and K5.unreleased, for the instance and each repository.
fn k5(day: &Day, forge: Option<&Forge>) -> Vec<Measured> {
    let Some(forge) = forge else {
        return ["K5.median", "K5.p90", "K5.unreleased"]
            .into_iter()
            .map(|metric| Measured::none(metric, INSTANCE, "no complete snapshot in the store"))
            .collect();
    };
    let mut released = Vec::new();
    let mut unreleased: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    unreleased.insert(INSTANCE, Vec::new());
    for pull in &forge.pulls {
        if pull.merged_at >= day.end {
            continue;
        }
        let release = forge
            .releases
            .get(pull.repository.as_str())
            .and_then(|releases| {
                releases
                    .iter()
                    .copied()
                    .filter(|published_at| *published_at > pull.merged_at)
                    .min()
            });
        match release {
            Some(published_at) if day.holds(published_at) => released.push((
                pull.repository.as_str(),
                pull.id(),
                hours(pull.merged_at, published_at),
            )),
            Some(published_at) if published_at < day.end => {}
            _ => {
                unreleased.entry(INSTANCE).or_default().push(pull.id());
                unreleased
                    .entry(pull.repository.as_str())
                    .or_default()
                    .push(pull.id());
            }
        }
        unreleased.entry(pull.repository.as_str()).or_default();
    }
    let mut measured = quantiles(
        ("K5.median", "K5.p90"),
        &scopes(released.into_iter()),
        "no pull request was released on the day",
    );
    let mut counts: Vec<(&str, Vec<String>)> = unreleased.into_iter().collect();
    counts.sort_by_key(|(scope, _)| (*scope != INSTANCE, *scope));
    for (scope, pulls) in counts {
        #[allow(clippy::cast_precision_loss)]
        let value = pulls.len() as f64;
        measured.push(Measured::value("K5.unreleased", scope, value, pulls));
    }
    measured
}

/// The items the store's daily rollups count on the day, by series and repository.
struct Rollups(BTreeMap<(String, String), BTreeSet<String>>);

impl Rollups {
    /// The rollups of `day` grouped `by` repository; a rollup that does not decode is left out
    /// and kept for [`Store::check`].
    fn read(store: &Store, day: &Day) -> Self {
        let mut items: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for row in store.daily_series() {
            let data = row.data;
            if data.day == day.text && data.by == "repository" {
                items
                    .entry((data.series, data.key))
                    .or_default()
                    .extend(data.items);
            }
        }
        Self(items)
    }

    /// The items of `series`, by repository.
    fn of(&self, series: &str) -> BTreeMap<&str, &BTreeSet<String>> {
        self.0
            .iter()
            .filter(|((held, _), _)| held == series)
            .map(|((_, repository), items)| (repository.as_str(), items))
            .collect()
    }
}

/// `metric` for the instance and each repository: the items of the series `part` over those of
/// the series `whole` (and of `part` too, when `whole_holds_part`), times `scale`. A scope whose
/// whole is empty has no value; the instance's names `none`, a repository's is left out.
fn rollup_ratio(
    metric: &'static str,
    rollups: &Rollups,
    (part, whole): (&str, &str),
    whole_holds_part: bool,
    scale: f64,
    none: &str,
) -> Vec<Measured> {
    let part = rollups.of(part);
    let whole = rollups.of(whole);
    let repositories: BTreeSet<&str> = part.keys().chain(whole.keys()).copied().collect();
    let mut scoped: Vec<(String, BTreeSet<String>, BTreeSet<String>)> = Vec::new();
    let mut instance = (BTreeSet::new(), BTreeSet::new());
    for repository in repositories {
        let counted = part
            .get(repository)
            .map(|items| (*items).clone())
            .unwrap_or_default();
        let mut of = whole
            .get(repository)
            .map(|items| (*items).clone())
            .unwrap_or_default();
        if whole_holds_part {
            of.extend(counted.iter().cloned());
        }
        instance.0.extend(counted.iter().cloned());
        instance.1.extend(of.iter().cloned());
        scoped.push((repository.to_owned(), counted, of));
    }
    scoped.insert(0, (INSTANCE.to_owned(), instance.0, instance.1));
    let mut measured = Vec::new();
    for (scope, counted, of) in scoped {
        if of.is_empty() {
            if scope == INSTANCE {
                measured.push(Measured::none(metric, &scope, none));
            }
            continue;
        }
        #[allow(clippy::cast_precision_loss)]
        let value = scale * counted.len() as f64 / of.len() as f64;
        let sources = counted.union(&of).cloned().collect();
        measured.push(Measured::value(metric, &scope, value, sources));
    }
    measured
}

/// K6.ci_failed, K6.closed_unmerged and K6.quick_patches, for the instance and each repository.
fn k6(day: &Day, rollups: &Rollups, forge: Option<&Forge>) -> Vec<Measured> {
    let mut measured = rollup_ratio(
        "K6.ci_failed",
        rollups,
        ("ci_failed", "prs_merged"),
        false,
        1.0,
        "no pull request merged on the day",
    );
    measured.extend(rollup_ratio(
        "K6.closed_unmerged",
        rollups,
        ("prs_closed_unmerged", "prs_merged"),
        true,
        100.0,
        "no pull request closed on the day",
    ));
    const QUICK: &str = "K6.quick_patches";
    let Some(forge) = forge else {
        measured.push(Measured::none(
            QUICK,
            INSTANCE,
            "no complete snapshot in the store",
        ));
        return measured;
    };
    let mut samples = Vec::new();
    for (repository, releases) in &forge.tagged {
        let mut releases: Vec<&(OffsetDateTime, String)> = releases.iter().collect();
        releases.sort();
        for (at, (published_at, tag)) in releases.iter().enumerate() {
            if !day.holds(*published_at) {
                continue;
            }
            let quick = crate::rollup::patch_of(tag).is_some_and(|patch| patch > 0)
                && at
                    .checked_sub(1)
                    .and_then(|before| releases.get(before))
                    .is_some_and(|(before, _)| *published_at - *before <= Duration::hours(24));
            samples.push((
                repository.as_str(),
                format!("{repository}@{tag}"),
                if quick { 100.0 } else { 0.0 },
            ));
        }
    }
    let scoped = scopes(samples.into_iter());
    for (scope, samples) in scoped {
        if samples.is_empty() {
            measured.push(Measured::none(
                QUICK,
                &scope,
                "no release published on the day",
            ));
            continue;
        }
        let values: Vec<f64> = samples.iter().map(|(_, value)| *value).collect();
        #[allow(clippy::cast_precision_loss)]
        let value = values.iter().sum::<f64>() / values.len() as f64;
        let sources = samples.into_iter().map(|(id, _)| id).collect();
        measured.push(Measured::value(QUICK, &scope, value, sources));
    }
    measured
}

/// K12.fix_releases, for the instance and each repository.
fn k12(rollups: &Rollups) -> Vec<Measured> {
    rollup_ratio(
        "K12.fix_releases",
        rollups,
        ("releases_patch", "releases_feature"),
        false,
        1.0,
        "no feature release published on the day",
    )
}

/// Samples `(repository, evidence id, value)` by scope: the instance first, then each
/// repository in name order.
fn scopes<'a>(
    samples: impl Iterator<Item = (&'a str, String, f64)>,
) -> Vec<(String, Vec<(String, f64)>)> {
    let mut instance = Vec::new();
    let mut repositories: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    for (repository, id, value) in samples {
        instance.push((id.clone(), value));
        repositories
            .entry(repository.to_owned())
            .or_default()
            .push((id, value));
    }
    let mut scoped = vec![(INSTANCE.to_owned(), instance)];
    scoped.extend(repositories);
    scoped
}

/// The median and p90 of each scope's samples; the instance with none has no value.
fn quantiles(
    (median_id, p90_id): (&'static str, &'static str),
    scoped: &[(String, Vec<(String, f64)>)],
    none: &str,
) -> Vec<Measured> {
    let mut measured = Vec::new();
    for (scope, samples) in scoped {
        if samples.is_empty() {
            measured.push(Measured::none(median_id, scope, none));
            measured.push(Measured::none(p90_id, scope, none));
            continue;
        }
        let values: Vec<f64> = samples.iter().map(|(_, value)| *value).collect();
        let sources: Vec<String> = samples.iter().map(|(id, _)| id.clone()).collect();
        measured.push(Measured::value(
            median_id,
            scope,
            median(&values),
            sources.clone(),
        ));
        measured.push(Measured::value(p90_id, scope, p90(&values), sources));
    }
    measured
}

/// One merged pull request of the snapshot.
struct Pull {
    repository: String,
    number: i64,
    merged_at: OffsetDateTime,
}

impl Pull {
    fn id(&self) -> String {
        format!("{}#{}", self.repository, self.number)
    }
}

/// What the newest complete snapshot observed on the forge: its merged pull requests, each once,
/// and each repository's release instants.
struct Forge {
    pulls: Vec<Pull>,
    releases: BTreeMap<String, Vec<OffsetDateTime>>,
    /// Each repository's releases with their tags.
    tagged: BTreeMap<String, Vec<(OffsetDateTime, String)>>,
}

impl Forge {
    /// The newest complete snapshot's forge observations, or none when the store holds no
    /// complete snapshot.
    fn read(state: Option<&Path>) -> Result<Option<Self>> {
        let generated = Generated::new(crate::state::open_existing(state)?);
        let mut newest: Option<(OffsetDateTime, _)> = None;
        for row in generated.snapshots().map_err(|unmet| anyhow!("{unmet}"))? {
            if row.state != SnapshotState::Complete {
                continue;
            }
            let started = instant("a snapshot's start", &row.started_at.0)?;
            if newest.as_ref().is_none_or(|(at, _)| started > *at) {
                newest = Some((started, row.snapshot_id));
            }
        }
        let Some((_, snapshot)) = newest else {
            return Ok(None);
        };
        let mut releases: BTreeMap<String, Vec<OffsetDateTime>> = BTreeMap::new();
        let mut tagged: BTreeMap<String, Vec<(OffsetDateTime, String)>> = BTreeMap::new();
        for row in generated.releases().map_err(|unmet| anyhow!("{unmet}"))? {
            if row.snapshot_id == snapshot {
                let published_at = instant("a release's publication", &row.published_at.0)?;
                releases
                    .entry(row.repository.0.clone())
                    .or_default()
                    .push(published_at);
                tagged
                    .entry(row.repository.0)
                    .or_default()
                    .push((published_at, row.tag));
            }
        }
        let mut seen = BTreeSet::new();
        let mut pulls = Vec::new();
        for row in generated
            .merged_pull_requests()
            .map_err(|unmet| anyhow!("{unmet}"))?
        {
            if row.snapshot_id == snapshot && seen.insert((row.repository.0.clone(), row.number)) {
                pulls.push(Pull {
                    merged_at: instant("a pull request's merge", &row.merged_at.0)?,
                    repository: row.repository.0,
                    number: row.number,
                });
            }
        }
        pulls.sort_by(|one, other| {
            (&one.repository, one.number).cmp(&(&other.repository, other.number))
        });
        generated
            .ports
            .check()
            .map_err(|failure| anyhow!("read the newest complete snapshot: {failure}"))?;
        Ok(Some(Self {
            pulls,
            releases,
            tagged,
        }))
    }
}

fn instant(what: &str, text: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339)
        .with_context(|| format!("{what} {text:?} is not an RFC 3339 instant"))
}

// ---------------------------------------------------------------------------------------------
// The ports, and SetMetricTarget's behaviour
// ---------------------------------------------------------------------------------------------

/// The ports the metric commands run over: the store, and the behaviour of `SetMetricTarget`,
/// which the generated model leaves an obligation (a subject's state chooses between a move and
/// an update).
pub struct Ports {
    store: Store,
}

impl Ports {
    fn open(state: Option<&Path>) -> Result<Self> {
        Ok(Self {
            store: crate::state::open(state)?,
        })
    }
}

impl MetricStorage for Ports {
    fn get(&self, identity: &MetricId) -> Option<MetricSnapshot> {
        MetricStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: MetricSnapshot) {
        MetricStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &MetricId) {
        MetricStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<MetricSnapshot> {
        MetricStorage::list(&self.store)
    }
}

impl MetricReadingStorage for Ports {
    fn get(&self, identity: &MetricReadingId) -> Option<MetricReadingSnapshot> {
        MetricReadingStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: MetricReadingSnapshot) {
        MetricReadingStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &MetricReadingId) {
        MetricReadingStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<MetricReadingSnapshot> {
        MetricReadingStorage::list(&self.store)
    }
}

impl SetMetricTargetBehavior for Ports {
    /// The contract: `targeted` when the held metric is `Untargeted`, taking `target` to
    /// `Targeted`; `retargeted` when it is `Targeted`, updating it; `no-such-metric` for an id no
    /// metric carries. Both accepting outcomes set `target`, `direction`, `target_owner` and
    /// `target_set_at` from the input and emit `MetricTargetSet`.
    fn set_metric_target(
        &mut self,
        input: SetMetricTarget,
    ) -> Result<SetMetricTargetOutcome, UnmetObligation> {
        let Some(held) = MetricStorage::get(&self.store, &input.metric_id) else {
            return Ok(SetMetricTargetOutcome::NoSuchMetric {
                error: MetricNotFound {
                    metric_id: input.metric_id,
                },
            });
        };
        let was = held.state;
        let mut next = match held.refine() {
            AnyMetric::Untargeted(metric) => AnyMetric::Targeted(metric.target()),
            targeted @ AnyMetric::Targeted(_) => targeted,
        }
        .snapshot();
        next.data.target = Some(input.target.clone());
        next.data.direction = Some(input.direction);
        next.data.target_owner = Some(input.target_owner.clone());
        next.data.target_set_at = Some(input.target_set_at);
        MetricStorage::put(&mut self.store, next);
        let metric_target_set = MetricTargetSet {
            metric_id: input.metric_id,
            target: input.target,
            direction: input.direction,
            target_owner: input.target_owner,
        };
        Ok(match was {
            MetricState::Untargeted => SetMetricTargetOutcome::Targeted { metric_target_set },
            MetricState::Targeted => SetMetricTargetOutcome::Retargeted { metric_target_set },
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Names and helpers
// ---------------------------------------------------------------------------------------------

/// The metric's state as the specification spells it. Exhaustive, so a state the specification
/// adds fails to compile here rather than being written under a guess.
#[must_use]
pub const fn metric_state_name(state: MetricState) -> &'static str {
    match state {
        MetricState::Targeted => "Targeted",
        MetricState::Untargeted => "Untargeted",
    }
}

/// The metric's state the specification spells `name`.
#[must_use]
pub fn metric_state_named(name: &str) -> Option<MetricState> {
    match name {
        "Targeted" => Some(MetricState::Targeted),
        "Untargeted" => Some(MetricState::Untargeted),
        _ => None,
    }
}

/// The reading's state as the specification spells it.
#[must_use]
pub const fn reading_state_name(state: MetricReadingState) -> &'static str {
    match state {
        MetricReadingState::Recorded => "Recorded",
        MetricReadingState::Superseded => "Superseded",
    }
}

/// The reading's state the specification spells `name`.
#[must_use]
pub fn reading_state_named(name: &str) -> Option<MetricReadingState> {
    match name {
        "Recorded" => Some(MetricReadingState::Recorded),
        "Superseded" => Some(MetricReadingState::Superseded),
        _ => None,
    }
}

/// The direction as the specification spells it.
#[must_use]
pub const fn direction_name(direction: TargetDirection) -> &'static str {
    match direction {
        TargetDirection::AtMost => "AtMost",
        TargetDirection::AtLeast => "AtLeast",
    }
}

/// The direction the specification spells `name`.
#[must_use]
pub fn direction_named(name: &str) -> Option<TargetDirection> {
    match name {
        "AtMost" => Some(TargetDirection::AtMost),
        "AtLeast" => Some(TargetDirection::AtLeast),
        _ => None,
    }
}

/// A decimal as a view prints it: its text, or null.
fn decimal(value: Option<&Decimal>) -> Value {
    value.map_or(Value::Null, |value| Value::from(value.0.as_str()))
}

/// Refuses `--input-json`, which no handler reads yet, as `what` not implemented.
fn flags_only(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(_) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of the flag `name`, or an error naming it.
fn flag<T>(command: &str, name: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| anyhow!("{command}: {name} is missing"))
}

/// Whether the store kept every read and write the behaviour made.
fn kept(command: &str, store: &Store) -> Result<()> {
    store
        .check()
        .with_context(|| format!("{command}: the store did not keep it"))
}

/// The refusal `error` of `command`, as the one line the binary writes on standard error.
fn refused(command: &str, error: &str, detail: &str) -> Result<ExitCode> {
    bail!("{command}: {error}: {detail}")
}

fn no_metric(metric: &MetricId) -> String {
    format!("no metric has the id {:?}; define it first", metric.0)
}

fn already_recorded(reading: &MetricReadingId) -> String {
    format!(
        "the reading {:?} is there already, and is unchanged",
        reading.0
    )
}

#[cfg(test)]
mod tests {
    use super::{median, p90, rendered};

    #[test]
    fn the_median_and_the_nearest_rank_p90() {
        assert!((median(&[3.0, 1.0, 2.0]) - 2.0).abs() < f64::EPSILON);
        assert!((median(&[1.0, 2.5, 3.0, 14.0]) - 2.75).abs() < f64::EPSILON);
        assert!((p90(&[1.0, 2.5, 3.0, 14.0]) - 14.0).abs() < f64::EPSILON);
        let ten: Vec<f64> = (1..=10).map(f64::from).collect();
        assert!((p90(&ten) - 9.0).abs() < f64::EPSILON);
        assert!((p90(&[5.0]) - 5.0).abs() < f64::EPSILON);
    }

    #[test]
    fn values_are_rounded_to_three_decimals_without_trailing_zeros() {
        assert_eq!(rendered(0.75), "0.75");
        assert_eq!(rendered(100.0 / 3.0), "33.333");
        assert_eq!(rendered(14.0), "14");
        assert_eq!(rendered(0.0), "0");
    }
}
