//! `conductor repository`: which repositories have their issues and pull requests processed
//! (design § 10, `story:repository-marks`).
//!
//! A repository's activity is computed when it is read, from the newest complete snapshot: Active
//! when its observation there has at least one real commit in 7 days and it is not archived,
//! Inactive otherwise. A `RepositoryMark`, set by the operator or by conductor with a reason,
//! overrides that computation. [`activity`] answers both, as
//! `conductor.direction.RepositoryActivityRow` rows, and writes nothing: no computed activity is
//! ever stored, so none can overwrite a mark.
//!
//! Each command records its outcome in the store under the state directory (`--state-dir`, else
//! `state/`; [`crate::state::dir`]) through the generated behaviour, and prints nothing. A refusal
//! the specification declares exits 1 with one line on standard error that names its error; a
//! repository is marked once (`mark-repository`), and changed after that by `activate-repository`
//! and `deactivate-repository`. `repository-marks` answers from those records: the marks only, as
//! the view declares them. `activity` ([`repository_activity`]) lists every repository's effective
//! activity. Input through `--input-json -` is not read yet.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use clap::ValueEnum as _;
use conductor_model::behaviour::{
    Generated, RepositoryMarkStorage, RepositoryObservationStorage, SnapshotStorage,
};
use conductor_model::direction::obligations::{
    ActivateRepositoryBehavior, DeactivateRepositoryBehavior, MarkRepositoryBehavior,
    RepositoryMarksQuery,
};
use conductor_model::direction::{
    ActivateRepository, ActivateRepositoryOutcome, Activity, DeactivateRepository,
    DeactivateRepositoryOutcome, DecidedBy, MarkRepository, MarkRepositoryOutcome, MarkedBy,
    MarkedRepository, RepositoryActivityRow, RepositoryMarkConflict, RepositoryMarkNotFound,
    RepositoryMarkSnapshot, RepositoryMarkState,
};
use conductor_model::observation::{SnapshotId, SnapshotSnapshot, SnapshotState};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{
    ActivateRepositoryArgs, DeactivateRepositoryArgs, Format, JsonInput, MarkRepositoryArgs,
    RepositoryActivityArgs, ViewArgs,
};
use crate::not_implemented;
use crate::store::Store;

/// Every repository's effective activity, by repository name: each repository the newest complete
/// snapshot observed, and each repository that holds a mark. A mark decides its repository's
/// activity; every other repository's is computed from its observation in that snapshot (the last
/// one recorded, should there be more than one): Active when `real_commits_7d` is at least 1 and it
/// is not archived. Without a complete snapshot only the marked repositories are answered.
///
/// The newest complete snapshot is the `Complete` one that started latest, and of equal starts the
/// one stored later; a start that does not read as RFC 3339 counts as earlier than any that does.
///
/// Reads the store through its storage ports and writes nothing. A record that does not read is
/// left out and kept on `store`, as every port keeps it: the caller answers it through
/// [`Store::check`] or [`crate::store::show`].
#[must_use]
pub fn activity(store: &Store) -> Vec<RepositoryActivityRow> {
    let mut effective = BTreeMap::new();
    if let Some(newest) = newest_complete(SnapshotStorage::list(store)) {
        for observation in RepositoryObservationStorage::list(store) {
            let data = observation.data;
            if data.snapshot_id != newest {
                continue;
            }
            let activity = if data.real_commits_7d >= 1 && !data.archived {
                Activity::Active
            } else {
                Activity::Inactive
            };
            effective.insert(
                data.repository.0.clone(),
                RepositoryActivityRow {
                    repository: MarkedRepository(data.repository.0),
                    activity,
                    decided_by: DecidedBy::Snapshot,
                    marked_by: None,
                    reason: None,
                },
            );
        }
    }
    for mark in RepositoryMarkStorage::list(store) {
        effective.insert(
            mark.data.repository.0.clone(),
            RepositoryActivityRow {
                repository: mark.data.repository,
                activity: activity_of(mark.state),
                decided_by: DecidedBy::Mark,
                marked_by: Some(mark.data.marked_by),
                reason: Some(mark.data.reason),
            },
        );
    }
    effective.into_values().collect()
}

/// The `Complete` snapshot of `snapshots` that started latest; of equal starts, the later one in
/// the store's order. A start that does not read as RFC 3339 counts as earlier than any that does.
fn newest_complete(snapshots: Vec<SnapshotSnapshot>) -> Option<SnapshotId> {
    snapshots
        .into_iter()
        .enumerate()
        .filter(|(_, snapshot)| snapshot.state == SnapshotState::Complete)
        .max_by_key(|(stored, snapshot)| {
            (
                OffsetDateTime::parse(&snapshot.data.started_at.0, &Rfc3339).ok(),
                *stored,
            )
        })
        .map(|(_, snapshot)| snapshot.data.snapshot_id)
}

/// The activity a mark in `state` gives its repository.
fn activity_of(state: RepositoryMarkState) -> Activity {
    match state {
        RepositoryMarkState::Active => Activity::Active,
        RepositoryMarkState::Inactive => Activity::Inactive,
    }
}

/// `conductor repository mark-repository`: the command `conductor.direction.MarkRepository`, which
/// marks a repository the first time only.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the mark, or the
/// repository holds a mark already (`RepositoryAlreadyMarked`), which is left as it is.
pub fn mark_repository(state: Option<&Path>, args: MarkRepositoryArgs) -> Result<ExitCode> {
    const WHAT: &str = "repository mark-repository";
    flags_only(args.input_json, "repository mark-repository --input-json")?;
    let repository = required(args.repository, WHAT, "--repository")?;
    let activity = named(args.activity, WHAT, "--activity", activity_named)?;
    let marked_by = named(args.marked_by, WHAT, "--marked-by", marked_by_named)?;
    let reason = required(args.reason, WHAT, "--reason")?;
    let mut generated = record(state)?;
    let outcome = generated
        .mark_repository(MarkRepository {
            repository: MarkedRepository(repository),
            activity,
            marked_by,
            reason,
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated.ports.store.check().context("keep the mark")?;
    match outcome {
        MarkRepositoryOutcome::MarkedActive { .. }
        | MarkRepositoryOutcome::MarkedInactive { .. } => Ok(ExitCode::SUCCESS),
        MarkRepositoryOutcome::AlreadyMarked { error } => refused(
            WHAT,
            "RepositoryAlreadyMarked",
            &format!(
                "repository {:?} holds a mark already; change it with activate-repository or \
                 deactivate-repository",
                error.repository.0
            ),
        ),
    }
}

/// `conductor repository activate-repository`: the command
/// `conductor.direction.ActivateRepository`.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the mark, or the
/// repository has no mark (`RepositoryMarkNotFound`) or is marked active already
/// (`RepositoryMarkConflict`).
pub fn activate_repository(state: Option<&Path>, args: ActivateRepositoryArgs) -> Result<ExitCode> {
    const WHAT: &str = "repository activate-repository";
    flags_only(
        args.input_json,
        "repository activate-repository --input-json",
    )?;
    let repository = required(args.repository, WHAT, "--repository")?;
    let marked_by = named(args.marked_by, WHAT, "--marked-by", marked_by_named)?;
    let reason = required(args.reason, WHAT, "--reason")?;
    let mut generated = record(state)?;
    let outcome = generated
        .activate_repository(ActivateRepository {
            repository: MarkedRepository(repository),
            marked_by,
            reason,
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated
        .ports
        .store
        .check()
        .context("keep the activation")?;
    match outcome {
        ActivateRepositoryOutcome::Activated { .. } => Ok(ExitCode::SUCCESS),
        ActivateRepositoryOutcome::NoSuchMark { error } => not_found(WHAT, &error),
        ActivateRepositoryOutcome::WrongState { error } => {
            conflict(WHAT, &error, "is marked active already")
        }
    }
}

/// `conductor repository deactivate-repository`: the command
/// `conductor.direction.DeactivateRepository`.
///
/// # Errors
///
/// A flag is missing or `--input-json` is given, the store does not open or keep the mark, or the
/// repository has no mark (`RepositoryMarkNotFound`) or is marked inactive already
/// (`RepositoryMarkConflict`).
pub fn deactivate_repository(
    state: Option<&Path>,
    args: DeactivateRepositoryArgs,
) -> Result<ExitCode> {
    const WHAT: &str = "repository deactivate-repository";
    flags_only(
        args.input_json,
        "repository deactivate-repository --input-json",
    )?;
    let repository = required(args.repository, WHAT, "--repository")?;
    let marked_by = named(args.marked_by, WHAT, "--marked-by", marked_by_named)?;
    let reason = required(args.reason, WHAT, "--reason")?;
    let mut generated = record(state)?;
    let outcome = generated
        .deactivate_repository(DeactivateRepository {
            repository: MarkedRepository(repository),
            marked_by,
            reason,
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated
        .ports
        .store
        .check()
        .context("keep the deactivation")?;
    match outcome {
        DeactivateRepositoryOutcome::Deactivated { .. } => Ok(ExitCode::SUCCESS),
        DeactivateRepositoryOutcome::NoSuchMark { error } => not_found(WHAT, &error),
        DeactivateRepositoryOutcome::WrongState { error } => {
            conflict(WHAT, &error, "is marked inactive already")
        }
    }
}

/// `conductor repository repository-marks`: the view `conductor.direction.RepositoryMarks`,
/// served by the generated query over the existing store under the state directory
/// (`--state-dir`, else `state/`; [`crate::state::dir`]) and rendered as `--format` asks. A view
/// creates no store.
///
/// Lists every mark it can read, with who set it and why, and names each record it cannot on
/// standard error, after them, exiting 1 ([`crate::store::show`]). The view declares the marks
/// only: a repository without a mark, whose activity is computed, has no row.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn repository_marks(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/direction.yaml`).
    const COLUMNS: [&str; 4] = ["repository", "state", "marked_by", "reason"];

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .repository_marks()
        .map_err(|unmet| anyhow!("repository repository-marks: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.repository.0),
                Value::from(state_name(row.state)),
                Value::from(marked_by_name(row.marked_by)),
                Value::from(row.reason),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "repository marks",
        &view.render(&COLUMNS, &rows),
    )
}

/// `conductor repository activity [--format <format>]`: the binding's `repository-activity`
/// callable, whose result is a list of `conductor.direction.RepositoryActivityRow`. Lists every
/// repository's effective activity as [`activity`] computes it, by repository name, from the
/// existing store under the state directory (`--state-dir`, else `state/`;
/// [`crate::state::dir`]), rendered as `--format` asks (text by default). It creates no store.
///
/// Names each record it cannot read on standard error, after the rows it could, and exits 1
/// ([`crate::store::show`]).
///
/// # Errors
///
/// `--format` is not a view's format, there is no store or it does not open, a read of it fails
/// for a reason other than a record this build cannot read, or the rows cannot be written.
pub fn repository_activity(
    state: Option<&Path>,
    args: &RepositoryActivityArgs,
) -> Result<ExitCode> {
    /// The row type's fields, in the specification's order (`spec/domains/direction.yaml`).
    const COLUMNS: [&str; 5] = [
        "repository",
        "activity",
        "decided_by",
        "marked_by",
        "reason",
    ];
    const WHAT: &str = "repository activity";

    let view = ViewArgs {
        format: match args.format.as_deref() {
            None => Format::Text,
            Some(name) => Format::from_str(name, false).map_err(|_| {
                anyhow!("{WHAT}: --format {name:?} is not one of text, json, jsonl, markdown")
            })?,
        },
    };
    let store = crate::state::open_existing(state).context(WHAT)?;
    let rows: Vec<Vec<Value>> = activity(&store)
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.repository.0),
                Value::from(activity_name(row.activity)),
                Value::from(decided_by_name(row.decided_by)),
                row.marked_by
                    .map_or(Value::Null, |by| Value::from(marked_by_name(by))),
                row.reason.map_or(Value::Null, Value::from),
            ]
        })
        .collect();
    crate::store::show(&store, WHAT, &view.render(&COLUMNS, &rows))
}

/// What decided a repository's activity, by the specification's name for it.
fn decided_by_name(decided_by: DecidedBy) -> &'static str {
    match decided_by {
        DecidedBy::Mark => "Mark",
        DecidedBy::Snapshot => "Snapshot",
    }
}

/// The ports the repository commands run over: the store.
struct Ports {
    store: Store,
}

/// Opens the store a command records into, creating it when it is not there.
fn record(state: Option<&Path>) -> Result<Generated<Ports>> {
    Ok(Generated::new(Ports {
        store: crate::state::open(state)?,
    }))
}

impl RepositoryMarkStorage for Ports {
    fn get(&self, identity: &MarkedRepository) -> Option<RepositoryMarkSnapshot> {
        RepositoryMarkStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: RepositoryMarkSnapshot) {
        RepositoryMarkStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &MarkedRepository) {
        RepositoryMarkStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<RepositoryMarkSnapshot> {
        RepositoryMarkStorage::list(&self.store)
    }
}

/// Refuses `--input-json`, which no repository command reads yet.
fn flags_only(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(_) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of `flag`, which `what` cannot do without.
fn required(value: Option<String>, what: &str, flag: &str) -> Result<String> {
    match value {
        Some(value) => Ok(value),
        None => bail!("{what}: {flag} is required"),
    }
}

/// The enum value `flag` names, by the specification's spelling.
fn named<T>(
    value: Option<String>,
    what: &str,
    flag: &str,
    named: fn(&str) -> Option<T>,
) -> Result<T> {
    let name = required(value, what, flag)?;
    named(&name)
        .ok_or_else(|| anyhow!("{what}: {flag} {name:?} is not a value the specification names"))
}

/// Answers a refusal the specification declares: one line naming `error`, and exit 1.
fn refused(what: &str, error: &str, why: &str) -> Result<ExitCode> {
    bail!("{what}: refused: {error}: {why}")
}

fn not_found(what: &str, error: &RepositoryMarkNotFound) -> Result<ExitCode> {
    refused(
        what,
        "RepositoryMarkNotFound",
        &format!(
            "repository {:?} has no mark; mark it first",
            error.repository.0
        ),
    )
}

fn conflict(what: &str, error: &RepositoryMarkConflict, why: &str) -> Result<ExitCode> {
    refused(
        what,
        "RepositoryMarkConflict",
        &format!("repository {:?} {why}", error.repository.0),
    )
}

/// Each enum's names as the specification spells them: `$name` answers a value's name and
/// `$named` the value a name spells. The match is exhaustive, so a value the specification adds
/// fails to compile here rather than being written or shown under a guessed name.
macro_rules! names {
    ($($ty:ident: $name:ident, $named:ident = [$($variant:ident),+];)+) => {$(
        pub(crate) fn $name(value: $ty) -> &'static str {
            match value {
                $($ty::$variant => stringify!($variant),)+
            }
        }

        pub(crate) fn $named(name: &str) -> Option<$ty> {
            [$($ty::$variant),+].into_iter().find(|value| $name(*value) == name)
        }
    )+};
}

names! {
    RepositoryMarkState: state_name, state_named = [Active, Inactive];
    MarkedBy: marked_by_name, marked_by_named = [Conductor, Operator];
    Activity: activity_name, activity_named = [Active, Inactive];
}
