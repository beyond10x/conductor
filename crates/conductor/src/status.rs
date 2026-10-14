//! `conductor snapshot publish-board`: the board, `STATUS.md`, from one complete snapshot, and the
//! `snapshot boards` view. Filled by `story:status-board`.
//!
//! `publish-board` renders the snapshot `--snapshot-id` names, else the newest complete one, into
//! the file `--path` names, else `STATUS.md` in the instance's records directory, and records
//! `PublishBoard` with the number of deviation rows it wrote. The board compares the snapshot with
//! `--previous-snapshot-id`, else the newest complete snapshot started before it. Its sections, in
//! order:
//!
//! 1. the header: the command, the snapshot's id and its start time;
//! 2. **Waiting for review** (`story:review-limit`): one line per pull request that waits for a
//!    person's review ([`waiting_for_review`]: open, not draft, no failing check), oldest first,
//!    `<repo> #<n> <title> — <age>` at the snapshot's start; then, per repository with at least
//!    `controllers.max_open_reviews` of them (`0` is no limit), `<repo>: <k> wait for review
//!    (limit <n>): no new dispatch`, the line conductor holds that repository's dispatch on;
//! 3. **Deviations**, one row per repository that is not archived with at least one: a red `main`
//!    workflow, a pull request with a failing check, one open more than 14 days, unreleased
//!    commits, no checkout, a specification missing or refused, a Gates hook whose version
//!    differs from the `b10x-gates` on `PATH` (`stale gates hooks`, `story:gates-hooks-fresh`),
//!    a newest wave narrower than both the ready units its store offers and
//!    `controllers.max_subagents` whose wave page gives no reason (`narrow wave`,
//!    `story:wave-width`, [`narrow_wave`]);
//! 4. **Overview**, one row per repository that is not archived, in name order;
//! 5. **Specifications**, one row per repository with its specification's presence and
//!    validation;
//! 6. **Open blockers**, by repository and kind;
//! 7. **Sessions**;
//! 8. **Delta** against the previous snapshot: each repository whose deviations changed, with
//!    those added and removed and the change in five counts (open pull requests, those with a
//!    failing check, workflows that are not green, open blockers, specifications present);
//!    `no change` when none did.
//!
//! A snapshot that is not there, or not complete, is refused with the specification's error
//! (`SnapshotNotFound`, `SnapshotNotComplete`) before anything is written; so is a previous
//! snapshot that is not. The file is written whole (beside it, then renamed into place) before the
//! board is recorded, so a recorded board names a file that was written. `--deviation-rows` is
//! the count of the rows the board writes, so it is refused when given; `--input-json` is not read
//! yet and answers [`NotImplemented`](crate::NotImplemented).
//!
//! The view opens only an existing store ([`crate::state::open_existing`]) and creates nothing.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    BlockerObservationStorage, BoardStorage, ControllerStorage, Generated,
    PullRequestObservationStorage, RepositoryObservationStorage, SessionObservationStorage,
    SnapshotStorage, SpecificationObservationStorage, TryContext, WorkflowRunObservationStorage,
    unmet_context,
};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::{ControllerState, GuardDecisionId, MessageId, ResourceRequestId};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::obligations::{BoardsQuery, PublishBoardBehavior};
use conductor_model::observation::{
    BlockerObservationData, BoardId, BoardSnapshot, ObservationId, PublishBoard,
    PublishBoardOutcome, PullRequestObservationData, RepositoryFigures, RepositoryObservationData,
    RunConclusion, SessionObservationData, SnapshotId, SnapshotSnapshot, SnapshotState,
    SpecificationObservationData, SpecificationPresence, ValidationResult,
    WorkflowRunObservationData,
};
use conductor_model::primitives::Uuid;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use crate::cli::{PublishBoardArgs, ViewArgs};
use crate::not_implemented;
use crate::snapshot::{
    harness_name, session_state_name, specification_presence_name, validation_result_name,
};
use crate::store::Store;

/// The command, as the board's header and every refusal name it.
const COMMAND: &str = "snapshot publish-board";

/// The board's file in the instance's records directory, without `--path`.
const BOARD_FILE: &str = "STATUS.md";

/// A pull request open longer than this many days is a deviation.
const STALE_DAYS: i64 = 14;

/// The deviations a repository can show, in the order the board lists them.
const RED_MAIN: &str = "red main";
const FAILING_PR: &str = "failing PR";
const STALE_PR: &str = "PR older than 14 days";
const UNRELEASED: &str = "unreleased commits";
const NO_CHECKOUT: &str = "no checkout";
const SPECIFICATION_MISSING: &str = "specification missing";
const SPECIFICATION_REFUSED: &str = "specification refused";
const STALE_GATES_HOOKS: &str = "stale gates hooks";
const NARROW_WAVE: &str = "narrow wave";

/// `conductor snapshot publish-board`: the command `conductor.observation.PublishBoard`. Writes
/// the board and prints the id of the board it recorded.
///
/// # Errors
///
/// `--input-json` is given ([`NotImplemented`](crate::NotImplemented)) or `--deviation-rows` is;
/// no snapshot is complete and none is named; the specification refuses the snapshot or the
/// previous one (`SnapshotNotFound`, `SnapshotNotComplete`); the store does not open, read or keep
/// the board; or the file cannot be written.
pub fn publish_board(state: Option<&Path>, args: PublishBoardArgs) -> Result<ExitCode> {
    if args.input_json.is_some() {
        return not_implemented("snapshot publish-board --input-json");
    }
    if let Some(rows) = args.deviation_rows {
        bail!(
            "{COMMAND}: --deviation-rows {rows} is not taken: the board counts the deviation rows \
             it writes"
        );
    }
    let store = crate::state::open(state)?;
    let snapshots = SnapshotStorage::list(&store);

    let current = match args.snapshot_id {
        Some(id) => complete_one(&snapshots, &SnapshotId(Uuid(id)))?,
        None => newest_complete(&snapshots, None).ok_or_else(|| {
            anyhow!(
                "{COMMAND}: no snapshot is complete, so there is no board to publish; take one \
                 with `conductor snapshot start-snapshot`"
            )
        })?,
    };
    let previous = match args.previous_snapshot_id {
        Some(id) => Some(complete_one(&snapshots, &SnapshotId(Uuid(id)))?),
        None => newest_complete(&snapshots, Some(current)),
    };
    let path = match args.path {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(&crate::config::active().instance.records).join(BOARD_FILE),
    };

    let now = observed(&store, current);
    let before = previous.map(|held| observed(&store, held));
    let running = running_controllers(&store);
    store
        .check()
        .with_context(|| format!("{COMMAND}: read the snapshot's observations"))?;
    let limit = crate::config::active()
        .instance
        .controllers
        .max_open_reviews;
    let (board, deviation_rows) = render(&now, before.as_ref(), &running, limit);

    write_board(&path, &board)?;
    let input = PublishBoard {
        snapshot_id: now.snapshot_id.clone(),
        previous_snapshot_id: before.as_ref().map(|held| held.snapshot_id.clone()),
        deviation_rows,
        path: path.to_string_lossy().into_owned(),
    };
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .publish_board(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    let board_id = match outcome {
        PublishBoardOutcome::Published { board_published } => board_published.board_id,
        PublishBoardOutcome::NoSuchSnapshot { error } => return Err(not_found(&error.snapshot_id)),
        PublishBoardOutcome::NotComplete { error } => {
            return Err(not_complete(&error.snapshot_id));
        }
        PublishBoardOutcome::NegativeRows { error } => {
            bail!("{COMMAND}: InvalidCount: count {} is negative", error.count);
        }
    };
    generated
        .ports
        .store
        .check()
        .with_context(|| format!("{COMMAND}: keep the board published to {}", path.display()))?;
    writeln!(io::stdout().lock(), "{}", board_id.0.0).context("write to standard output")?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor snapshot boards`: the view `conductor.observation.Boards`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn boards(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 5] = [
        "board_id",
        "snapshot_id",
        "previous_snapshot_id",
        "deviation_rows",
        "path",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .boards()
        .map_err(|unmet| anyhow!("snapshot boards: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.board_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.previous_snapshot_id.map(|id| id.0.0)),
                Value::from(row.deviation_rows),
                Value::from(row.path),
            ]
        })
        .collect();
    crate::store::show(&generated.ports, "boards", &view.render(&COLUMNS, &rows))
}

// ---------------------------------------------------------------------------------------------
// Which snapshots
// ---------------------------------------------------------------------------------------------

/// The snapshot `id` of `snapshots`, refused unless it is there and complete.
fn complete_one<'a>(
    snapshots: &'a [SnapshotSnapshot],
    id: &SnapshotId,
) -> Result<&'a SnapshotSnapshot> {
    let held = snapshots
        .iter()
        .find(|held| held.data.snapshot_id == *id)
        .ok_or_else(|| not_found(id))?;
    if held.state == SnapshotState::Complete {
        Ok(held)
    } else {
        Err(not_complete(id))
    }
}

/// The complete snapshot of `snapshots` that started last; with `before`, the one that started
/// last before it.
pub(crate) fn newest_complete<'a>(
    snapshots: &'a [SnapshotSnapshot],
    before: Option<&SnapshotSnapshot>,
) -> Option<&'a SnapshotSnapshot> {
    snapshots
        .iter()
        .filter(|held| held.state == SnapshotState::Complete)
        .filter(|held| {
            before.is_none_or(|current| {
                held.data.snapshot_id != current.data.snapshot_id
                    && order(&held.data.started_at.0, &current.data.started_at.0) == Ordering::Less
            })
        })
        .max_by(|left, right| order(&left.data.started_at.0, &right.data.started_at.0))
}

/// Two start times in time order; as text where either is not RFC 3339.
fn order(left: &str, right: &str) -> Ordering {
    match (instant(left), instant(right)) {
        (Some(left), Some(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

fn instant(text: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(text, &Rfc3339).ok()
}

// ---------------------------------------------------------------------------------------------
// What one snapshot observed
// ---------------------------------------------------------------------------------------------

/// The observations of one snapshot.
struct Observed {
    snapshot_id: SnapshotId,
    started_at: String,
    repositories: Vec<RepositoryObservationData>,
    pull_requests: Vec<PullRequestObservationData>,
    workflow_runs: Vec<WorkflowRunObservationData>,
    specifications: Vec<SpecificationObservationData>,
    blockers: Vec<BlockerObservationData>,
    sessions: Vec<SessionObservationData>,
}

/// What `store` holds of the snapshot `held`.
fn observed(store: &Store, held: &SnapshotSnapshot) -> Observed {
    let id = &held.data.snapshot_id;
    Observed {
        snapshot_id: id.clone(),
        started_at: held.data.started_at.0.clone(),
        repositories: RepositoryObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
            .collect(),
        pull_requests: PullRequestObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
            .collect(),
        workflow_runs: WorkflowRunObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
            .collect(),
        specifications: SpecificationObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
            .collect(),
        blockers: BlockerObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
            .collect(),
        sessions: SessionObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
            .collect(),
    }
}

/// The session name of each repository's running controller, by repository.
fn running_controllers(store: &Store) -> BTreeMap<String, String> {
    ControllerStorage::list(store)
        .into_iter()
        .filter(|held| held.state == ControllerState::Running)
        .map(|held| (held.data.repository.0, held.data.session_name.0))
        .collect()
}

/// The five counts the delta compares, for one repository.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Counts {
    open_prs: i64,
    failing_prs: i64,
    non_green_workflows: i64,
    open_blockers: i64,
    specifications_present: i64,
}

/// One repository as the board shows it: its deviations, in the order the board lists them, and
/// its counts.
#[derive(Debug, Clone, Default)]
struct Figures {
    deviations: Vec<&'static str>,
    counts: Counts,
}

/// The figures of each repository of the snapshot `held` in `store` that is not archived, in name
/// order, as `conductor.observation.RepositoryFigures`: the rows the dashboard's `/api/board`
/// answers (`story:dashboard-overview`), worked out by the one [`figures`] the board's deviations
/// and delta are, so the two never disagree.
pub(crate) fn repository_figures(store: &Store, held: &SnapshotSnapshot) -> Vec<RepositoryFigures> {
    figures(&observed(store, held))
        .into_iter()
        .map(|(repository, figures)| RepositoryFigures {
            repository,
            deviations: figures
                .deviations
                .iter()
                .map(|deviation| (*deviation).to_owned())
                .collect(),
            open_prs: figures.counts.open_prs,
            failing_prs: figures.counts.failing_prs,
            non_green_workflows: figures.counts.non_green_workflows,
            open_blockers: figures.counts.open_blockers,
            specifications_present: figures.counts.specifications_present,
        })
        .collect()
}

/// How many repositories of the snapshot `held` in `store` the board names `stale gates hooks`
/// (`story:gates-hooks-fresh`): the watch's count.
pub(crate) fn stale_gates_hooks(store: &Store, held: &SnapshotSnapshot) -> usize {
    figures(&observed(store, held))
        .values()
        .filter(|figures| figures.deviations.contains(&STALE_GATES_HOOKS))
        .count()
}

/// The figures of each repository of `observed` that is not archived, by name: the board's
/// deviations and delta, and [`repository_figures`].
fn figures(observed: &Observed) -> BTreeMap<String, Figures> {
    let started = instant(&observed.started_at);
    let max_subagents = crate::config::active().instance.controllers.max_subagents;
    let mut out = BTreeMap::new();
    for repository in observed.repositories.iter().filter(|held| !held.archived) {
        let name = &repository.repository.0;
        let pulls: Vec<&PullRequestObservationData> = observed
            .pull_requests
            .iter()
            .filter(|pull| pull.repository.0 == *name)
            .collect();
        let runs: Vec<&WorkflowRunObservationData> = observed
            .workflow_runs
            .iter()
            .filter(|run| run.repository.0 == *name)
            .collect();
        let specification = observed
            .specifications
            .iter()
            .find(|held| held.repository.0 == *name);
        let failing = pulls.iter().filter(|pull| pull.failing_checks > 0).count();

        let mut deviations = Vec::new();
        if runs
            .iter()
            .any(|run| run.conclusion == RunConclusion::Failure)
        {
            deviations.push(RED_MAIN);
        }
        if failing > 0 {
            deviations.push(FAILING_PR);
        }
        if pulls.iter().any(|pull| stale(&pull.opened_at.0, started)) {
            deviations.push(STALE_PR);
        }
        if repository
            .unreleased_commits
            .is_some_and(|commits| commits > 0)
        {
            deviations.push(UNRELEASED);
        }
        if !repository.local_checkout {
            deviations.push(NO_CHECKOUT);
        }
        if let Some(specification) = specification {
            if specification.presence == SpecificationPresence::Missing {
                deviations.push(SPECIFICATION_MISSING);
            }
            if specification.validation == ValidationResult::Refused {
                deviations.push(SPECIFICATION_REFUSED);
            }
        }
        if crate::gates::stale(
            repository.gates_hooks_version.as_deref(),
            repository.gates_version.as_deref(),
        ) {
            deviations.push(STALE_GATES_HOOKS);
        }
        if narrow_wave(repository, max_subagents) {
            deviations.push(NARROW_WAVE);
        }

        let counts = Counts {
            open_prs: count(pulls.len()),
            failing_prs: count(failing),
            non_green_workflows: count(
                runs.iter()
                    .filter(|run| run.conclusion != RunConclusion::Success)
                    .count(),
            ),
            open_blockers: count(
                observed
                    .blockers
                    .iter()
                    .filter(|blocker| blocker.repository.0 == *name)
                    .count(),
            ),
            specifications_present: i64::from(
                specification.is_some_and(|held| held.presence == SpecificationPresence::Present),
            ),
        };
        out.insert(name.clone(), Figures { deviations, counts });
    }
    out
}

/// Whether `repository`'s newest wave is narrow (`story:wave-width`): its width is under both the
/// ready units its store offered and `max_subagents`, and its wave page gives no reason. Never
/// when the width or the ready units were not read.
fn narrow_wave(repository: &RepositoryObservationData, max_subagents: i64) -> bool {
    match (repository.wave_width, repository.wave_ready) {
        (Some(width), Some(ready)) => {
            width < ready && width < max_subagents && repository.wave_reason.is_none()
        }
        _ => false,
    }
}

/// Whether a pull request opened at `opened_at` had been open longer than [`STALE_DAYS`] when the
/// snapshot started; never when either time does not read.
fn stale(opened_at: &str, started: Option<OffsetDateTime>) -> bool {
    match (started, instant(opened_at)) {
        (Some(started), Some(opened)) => started - opened > Duration::days(STALE_DAYS),
        _ => false,
    }
}

fn count(items: usize) -> i64 {
    i64::try_from(items).unwrap_or(i64::MAX)
}

// ---------------------------------------------------------------------------------------------
// Waiting for review (story:review-limit)
// ---------------------------------------------------------------------------------------------

/// The pull requests of `pulls` that wait for a person's review: open (a snapshot observes only
/// open ones), not draft, with no failing check; oldest first, then by repository and number.
pub(crate) fn waiting_for_review(
    pulls: &[PullRequestObservationData],
) -> Vec<PullRequestObservationData> {
    let mut waiting: Vec<PullRequestObservationData> = pulls
        .iter()
        .filter(|pull| !pull.draft && pull.failing_checks == 0)
        .cloned()
        .collect();
    waiting.sort_by(|left, right| {
        order(&left.opened_at.0, &right.opened_at.0).then_with(|| {
            (&left.repository.0, left.number).cmp(&(&right.repository.0, right.number))
        })
    });
    waiting
}

/// The pull requests of the snapshot `held` in `store` that wait for review, oldest first
/// ([`waiting_for_review`]): the dashboard's `/api/reviews` and the watch's count read these, so
/// they never disagree with the board.
pub(crate) fn waiting_in(
    store: &Store,
    held: &SnapshotSnapshot,
) -> Vec<PullRequestObservationData> {
    let id = &held.data.snapshot_id;
    let pulls: Vec<PullRequestObservationData> = PullRequestObservationStorage::list(store)
        .into_iter()
        .map(|held| held.data)
        .filter(|data| data.snapshot_id == *id)
        .collect();
    waiting_for_review(&pulls)
}

/// How many of `waiting` each repository holds, by name, for each repository at or over
/// `limit`; none when `limit` is `0`, no limit.
pub(crate) fn review_full(
    waiting: &[PullRequestObservationData],
    limit: i64,
) -> BTreeMap<String, i64> {
    if limit <= 0 {
        return BTreeMap::new();
    }
    let mut counts: BTreeMap<String, i64> = BTreeMap::new();
    for pull in waiting {
        *counts.entry(pull.repository.0.clone()).or_default() += 1;
    }
    counts.retain(|_, waiting| *waiting >= limit);
    counts
}

/// The **Waiting for review** section of `now` under the limit `limit`: one line per pull request
/// that waits, oldest first, then one per repository at or over the limit; `None.` when none
/// waits.
fn reviews(now: &Observed, limit: i64) -> String {
    let waiting = waiting_for_review(&now.pull_requests);
    if waiting.is_empty() {
        return "None.\n\n".to_owned();
    }
    let started = instant(&now.started_at);
    let mut out = String::new();
    for pull in &waiting {
        out.push_str(&format!(
            "- {} #{} {} — {}\n",
            pull.repository.0,
            pull.number,
            pull.title.replace(['\n', '\r'], " "),
            age(&pull.opened_at.0, started)
        ));
    }
    for (repository, held) in review_full(&waiting, limit) {
        out.push_str(&format!(
            "- {repository}: {held} wait for review (limit {limit}): no new dispatch\n"
        ));
    }
    out.push('\n');
    out
}

/// How long a pull request opened at `opened_at` had been open when the snapshot started, in its
/// largest whole unit: `7d`, `5h`, `40m`; the time as written when either does not read.
fn age(opened_at: &str, started: Option<OffsetDateTime>) -> String {
    let (Some(started), Some(opened)) = (started, instant(opened_at)) else {
        return opened_at.to_owned();
    };
    let open = started - opened;
    if open.whole_days() >= 1 {
        format!("{}d", open.whole_days())
    } else if open.whole_hours() >= 1 {
        format!("{}h", open.whole_hours())
    } else {
        format!("{}m", open.whole_minutes().max(0))
    }
}

// ---------------------------------------------------------------------------------------------
// The board
// ---------------------------------------------------------------------------------------------

/// The board of `now`, compared with `before`, its review limit `max_open_reviews`, and the
/// number of its deviation rows.
fn render(
    now: &Observed,
    before: Option<&Observed>,
    running: &BTreeMap<String, String>,
    max_open_reviews: i64,
) -> (String, i64) {
    let current = figures(now);
    let mut board = String::new();

    board.push_str("# Status\n\n");
    board.push_str(&format!(
        "Written by `conductor {COMMAND}` from snapshot {}, started {}.\n\n",
        now.snapshot_id.0.0, now.started_at
    ));

    board.push_str("## Waiting for review\n\n");
    board.push_str(&reviews(now, max_open_reviews));

    board.push_str("## Deviations\n\n");
    let deviations: Vec<Vec<String>> = current
        .iter()
        .filter(|(_, figures)| !figures.deviations.is_empty())
        .map(|(name, figures)| vec![name.clone(), figures.deviations.join(", ")])
        .collect();
    let deviation_rows = count(deviations.len());
    board.push_str(&table(&["repository", "deviations"], &deviations));

    board.push_str("## Overview\n\n");
    let overview: Vec<Vec<String>> = now
        .repositories
        .iter()
        .filter(|held| !held.archived)
        .map(|held| overview_row(now, held, running))
        .collect::<BTreeMap<String, Vec<String>>>()
        .into_values()
        .collect();
    board.push_str(&table(
        &[
            "repository",
            "open issues",
            "open PRs",
            "last release",
            "released",
            "commits after it",
            "red main workflows",
            "controller",
        ],
        &overview,
    ));

    board.push_str("## Specifications\n\n");
    let mut specifications: Vec<&SpecificationObservationData> =
        now.specifications.iter().collect();
    specifications.sort_by(|left, right| left.repository.0.cmp(&right.repository.0));
    let specifications: Vec<Vec<String>> = specifications
        .into_iter()
        .map(|held| {
            vec![
                held.repository.0.clone(),
                specification_presence_name(held.presence).to_owned(),
                validation_result_name(held.validation).to_owned(),
                held.validation_refusals.to_string(),
            ]
        })
        .collect();
    board.push_str(&table(
        &["repository", "presence", "validation", "refusals"],
        &specifications,
    ));

    board.push_str("## Open blockers\n\n");
    let mut blockers: Vec<&BlockerObservationData> = now.blockers.iter().collect();
    blockers.sort_by(|left, right| {
        (&left.repository.0, &left.blocker_kind, &left.reference).cmp(&(
            &right.repository.0,
            &right.blocker_kind,
            &right.reference,
        ))
    });
    let blockers: Vec<Vec<String>> = blockers
        .into_iter()
        .map(|held| {
            vec![
                held.repository.0.clone(),
                held.blocker_kind.clone(),
                held.reference.clone(),
                held.title.clone(),
            ]
        })
        .collect();
    board.push_str(&table(
        &["repository", "kind", "reference", "title"],
        &blockers,
    ));

    board.push_str("## Sessions\n\n");
    let mut sessions: Vec<Vec<String>> = now
        .sessions
        .iter()
        .map(|held| {
            vec![
                held.repository
                    .as_ref()
                    .map_or_else(String::new, |repository| repository.0.clone()),
                held.name
                    .clone()
                    .unwrap_or_else(|| held.session_ref.clone()),
                harness_name(held.harness).to_owned(),
                held.role.clone().unwrap_or_default(),
                session_state_name(held.activity).to_owned(),
            ]
        })
        .collect();
    sessions.sort();
    board.push_str(&table(
        &["repository", "session", "harness", "role", "activity"],
        &sessions,
    ));

    board.push_str("## Delta\n\n");
    board.push_str(&delta(&current, before));

    (board, deviation_rows)
}

/// One repository's row of the overview, keyed by its name.
fn overview_row(
    now: &Observed,
    held: &RepositoryObservationData,
    running: &BTreeMap<String, String>,
) -> (String, Vec<String>) {
    let name = &held.repository.0;
    let pulls: Vec<&PullRequestObservationData> = now
        .pull_requests
        .iter()
        .filter(|pull| pull.repository.0 == *name)
        .collect();
    let failing = pulls.iter().filter(|pull| pull.failing_checks > 0).count();
    let red = now
        .workflow_runs
        .iter()
        .filter(|run| run.repository.0 == *name)
        .filter(|run| run.conclusion == RunConclusion::Failure)
        .count();
    let row = vec![
        name.clone(),
        held.open_issues.to_string(),
        format!("{} ({failing} failing)", pulls.len()),
        held.latest_release
            .clone()
            .unwrap_or_else(|| "none".to_owned()),
        held.latest_release_at
            .as_ref()
            .map_or_else(String::new, |at| release_date(&at.0)),
        held.unreleased_commits
            .map_or_else(String::new, |commits| commits.to_string()),
        red.to_string(),
        running
            .get(name)
            .cloned()
            .unwrap_or_else(|| "none".to_owned()),
    ];
    (name.clone(), row)
}

/// The date of an RFC 3339 time, `2026-10-01` of `2026-10-01T09:00:00Z`; the whole text when it
/// is shorter.
fn release_date(at: &str) -> String {
    at.get(..10).unwrap_or(at).to_owned()
}

/// The delta of `current` against `before`: one row per repository whose deviations changed,
/// with those added and removed and the change in each count; `no change` when none changed.
fn delta(current: &BTreeMap<String, Figures>, before: Option<&Observed>) -> String {
    let Some(before) = before else {
        return "No earlier complete snapshot to compare with.\n".to_owned();
    };
    let previous = figures(before);
    let mut out = format!(
        "Against snapshot {}, started {}.\n\n",
        before.snapshot_id.0.0, before.started_at
    );
    let names: BTreeSet<&String> = current.keys().chain(previous.keys()).collect();
    let empty = Figures::default();
    let mut rows = Vec::new();
    for name in names {
        let now = current.get(name).unwrap_or(&empty);
        let then = previous.get(name).unwrap_or(&empty);
        let added: Vec<String> = now
            .deviations
            .iter()
            .filter(|deviation| !then.deviations.contains(*deviation))
            .map(|deviation| format!("+{deviation}"))
            .collect();
        let removed: Vec<String> = then
            .deviations
            .iter()
            .filter(|deviation| !now.deviations.contains(*deviation))
            .map(|deviation| format!("-{deviation}"))
            .collect();
        if added.is_empty() && removed.is_empty() {
            continue;
        }
        rows.push(vec![
            name.clone(),
            added.join(", "),
            removed.join(", "),
            signed(now.counts.open_prs - then.counts.open_prs),
            signed(now.counts.failing_prs - then.counts.failing_prs),
            signed(now.counts.non_green_workflows - then.counts.non_green_workflows),
            signed(now.counts.open_blockers - then.counts.open_blockers),
            signed(now.counts.specifications_present - then.counts.specifications_present),
        ]);
    }
    if rows.is_empty() {
        out.push_str("no change\n");
        return out;
    }
    out.push_str(&table(
        &[
            "repository",
            "added",
            "removed",
            "open PRs",
            "failing PRs",
            "non-green workflows",
            "open blockers",
            "specifications present",
        ],
        &rows,
    ));
    out
}

/// A change in a count: `+1`, `0`, `-1`.
fn signed(change: i64) -> String {
    if change > 0 {
        format!("+{change}")
    } else {
        change.to_string()
    }
}

/// A Markdown table of `rows` under `header`, and a blank line; `None.` when there are no rows.
fn table(header: &[&str], rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return "None.\n\n".to_owned();
    }
    let mut out = String::new();
    out.push_str(&line(header.iter().map(|name| (*name).to_owned())));
    out.push_str(&line(header.iter().map(|_| "---".to_owned())));
    for row in rows {
        out.push_str(&line(row.iter().map(String::as_str).map(cell)));
    }
    out.push('\n');
    out
}

/// One row of a Markdown table.
fn line(cells: impl Iterator<Item = String>) -> String {
    let cells: Vec<String> = cells.collect();
    format!("| {} |\n", cells.join(" | "))
}

/// `value` as one table cell: `\` and `|` escaped, every line break a space.
fn cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace(['\n', '\r'], " ")
}

/// Writes `board` to `path` whole: beside it first, then renamed into place, creating the
/// directory it lies in.
fn write_board(path: &Path, board: &str) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("{COMMAND}: create {}", parent.display()))?;
    }
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    fs::write(&partial, board)
        .with_context(|| format!("{COMMAND}: write {}", partial.display()))?;
    fs::rename(&partial, path)
        .with_context(|| format!("{COMMAND}: put the board in place at {}", path.display()))
}

// ---------------------------------------------------------------------------------------------
// Refusals and ports
// ---------------------------------------------------------------------------------------------

fn not_found(snapshot: &SnapshotId) -> anyhow::Error {
    anyhow!(
        "{COMMAND}: SnapshotNotFound: no snapshot has the id {}",
        snapshot.0.0
    )
}

fn not_complete(snapshot: &SnapshotId) -> anyhow::Error {
    anyhow!(
        "{COMMAND}: SnapshotNotComplete: snapshot {} is collecting or failed, so no board is \
         published from it",
        snapshot.0.0
    )
}

/// The ports `PublishBoard` runs over: the store, and the board id it assigns.
struct Ports {
    store: Store,
}

impl BoardStorage for Ports {
    fn get(&self, identity: &BoardId) -> Option<BoardSnapshot> {
        BoardStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: BoardSnapshot) {
        BoardStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &BoardId) {
        BoardStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<BoardSnapshot> {
        BoardStorage::list(&self.store)
    }
}

impl SnapshotStorage for Ports {
    fn get(&self, identity: &SnapshotId) -> Option<SnapshotSnapshot> {
        SnapshotStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: SnapshotSnapshot) {
        SnapshotStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &SnapshotId) {
        SnapshotStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<SnapshotSnapshot> {
        SnapshotStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Ok(BoardId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }

    fn try_generate_conductor_host_grant_id(
        &mut self,
    ) -> Result<conductor_model::host::GrantId, UnmetObligation> {
        Err(unmet_context("conductor.host.GrantId"))
    }

    fn try_generate_conductor_host_reclaim_run_id(
        &mut self,
    ) -> Result<conductor_model::host::ReclaimRunId, UnmetObligation> {
        Err(unmet_context("conductor.host.ReclaimRunId"))
    }
}
