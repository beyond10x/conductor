//! `conductor snapshot`: one sense run, the observation commands it records with, and the views of
//! what it observed. Filled by `story:snapshot-driver`.
//!
//! [`take_in`] is the driver (design § 4). It starts a snapshot (`StartSnapshot`) at the current
//! time, with the bytes free on the file system holding `/`; runs each collector it is given, in
//! order, each recording through a [`Recorder`], which also gives it the state directory
//! ([`Recorder::state`]); and ends the snapshot with `CompleteSnapshot` when
//! every collector answered, or with `FailSnapshot` naming the first that did not, which stops the
//! run. A collector that panics did not answer: the panic is its failed answer. A failed snapshot
//! is kept with its reason, `<collector>: <error>`, which `snapshot snapshots` shows as
//! `failure_reason`; a command a collector ran past its bound ([`collect::run`]) is named there
//! with the bound. `snapshot start-snapshot` is the driver over
//! [`collect::registered`](crate::collect::registered) (design § 4).
//!
//! Once a snapshot is kept `Complete`, by the driver or by `complete-snapshot`, the store drops the
//! observations of the snapshots the instance's `retention.snapshots` does not keep
//! ([`Store::retain`], `story:observation-retention`); the `Snapshot` records stay.
//!
//! Every command runs its generated behaviour over the store, maps a refusal to exit 1 and one
//! line naming the specification's error, and is durable only when [`Store::check`] is `Ok` after
//! it. Every view runs the generated query over the existing store, which it never creates, and
//! answers through [`crate::store::show`]: every row it can read, then each record it cannot,
//! named on standard error, with exit 1.
//! The store is the one under the state directory (`--state-dir`, else `state/`;
//! [`crate::state::dir`]). An input given as `--input-json -` is not read yet: such a call answers
//! [`NotImplemented`](crate::NotImplemented).
//!
//! The group's other words live elsewhere: `publish-board` and `boards` in [`crate::status`].
//! `record-release`, `record-merged-pull-request`, `releases` and `merged-pull-requests` are
//! `story:release-digest`'s; `repository shipped` ([`crate::shipped`]) reads the last two.
//! [`take_from`] is [`take`] at a start its caller gives.

use std::any::Any;
use std::fmt::Display;
use std::io::{self, Write as _};
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    BlockerObservationStorage, Generated, MergedPullRequestObservationStorage,
    PullRequestObservationStorage, ReleaseObservationStorage, RepositoryObservationStorage,
    SessionObservationStorage, SnapshotStorage, SpecificationObservationStorage, TryContext,
    WorkflowRunObservationStorage, unmet_context,
};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::{GuardDecisionId, MessageId, ResourceRequestId};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::obligations::{
    BlockersQuery, CompleteSnapshotBehavior, FailSnapshotBehavior, MergedPullRequestsQuery,
    PullRequestsQuery, RecordBlockerBehavior, RecordMergedPullRequestBehavior,
    RecordPullRequestBehavior, RecordReleaseBehavior, RecordRepositoryBehavior,
    RecordSessionBehavior, RecordSpecificationBehavior, RecordWorkflowRunBehavior, ReleasesQuery,
    RepositoriesQuery, SessionsQuery, SnapshotsQuery, SpecificationsQuery, StartSnapshotBehavior,
    WorkflowRunsQuery,
};
use conductor_model::observation::{
    BlockerObservationSnapshot, BoardId, CommitSha, CompleteSnapshot, CompleteSnapshotOutcome,
    FailSnapshot, FailSnapshotOutcome, Harness, Mergeability, MergedPullRequestObservationSnapshot,
    ObservationId, PullRequestObservationSnapshot, RecordBlocker, RecordBlockerOutcome,
    RecordMergedPullRequest, RecordMergedPullRequestOutcome, RecordPullRequest,
    RecordPullRequestOutcome, RecordRelease, RecordReleaseOutcome, RecordRepository,
    RecordRepositoryOutcome, RecordSession, RecordSessionOutcome, RecordSpecification,
    RecordSpecificationOutcome, RecordWorkflowRun, RecordWorkflowRunOutcome,
    ReleaseObservationSnapshot, RepositoryName, RepositoryObservationSnapshot, RunConclusion,
    SessionObservationSnapshot, SessionState, SnapshotId, SnapshotSnapshot, SnapshotState,
    SpecificationObservationSnapshot, SpecificationPresence, StartSnapshot, StartSnapshotOutcome,
    ValidationResult, Visibility, WorkflowRunObservationSnapshot,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{
    CompleteSnapshotArgs, FailSnapshotArgs, JsonInput, RecordBlockerArgs,
    RecordMergedPullRequestArgs, RecordPullRequestArgs, RecordReleaseArgs, RecordRepositoryArgs,
    RecordSessionArgs, RecordSpecificationArgs, RecordWorkflowRunArgs, StartSnapshotArgs, ViewArgs,
};
use crate::collect::{self, Collector};
use crate::not_implemented;
use crate::store::Store;

/// The file system whose free bytes a snapshot records.
const DISK: &str = "/";

/// What one sense run ended as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The snapshot it started.
    pub snapshot_id: SnapshotId,
    /// `Complete` or `Failed`.
    pub state: SnapshotState,
    /// Why it failed: the first collector that did not answer, and its error. `None` when it
    /// completed.
    pub reason: Option<String>,
}

/// The driver over the store under the state directory `state`, which it opens, creating it when
/// it is not there: starts a snapshot now, with the bytes free on the file system holding `/`,
/// runs `collectors` in order, each given `state` through [`Recorder::state`], and ends the
/// snapshot `Complete` when every one answered, or `Failed` naming the first that did not, a
/// collector that panicked included.
///
/// # Errors
///
/// The store does not open, the free bytes cannot be read, a behaviour declares no outcome, or
/// the store did not keep a write; a snapshot started before that is left collecting, which is
/// never read as current.
pub fn take_in(state: &Path, collectors: &[Collector]) -> Result<Taken> {
    let state = absolute(state)?;
    let start = StartSnapshot {
        started_at: Timestamp(now()?),
        disk_free_bytes: disk_free_bytes(Path::new(DISK))?,
    };
    drive(crate::store::open(&state)?, Some(&state), start, collectors)
}

/// [`take_in`] over a store opened elsewhere, whose state directory the driver does not know: a
/// collector that asks for it ([`Recorder::state`]) fails the snapshot.
///
/// # Errors
///
/// As [`take_in`], but for opening the store.
pub fn take(store: Store, collectors: &[Collector]) -> Result<Taken> {
    let start = StartSnapshot {
        started_at: Timestamp(now()?),
        disk_free_bytes: disk_free_bytes(Path::new(DISK))?,
    };
    drive(store, None, start, collectors)
}

/// [`take`], started as `start` says rather than now: its time and free bytes, as
/// `start-snapshot --started-at --disk-free-bytes` give them.
///
/// # Errors
///
/// What [`take`] answers.
pub fn take_from(store: Store, start: StartSnapshot, collectors: &[Collector]) -> Result<Taken> {
    drive(store, None, start, collectors)
}

/// `state` as an absolute path: a collector runs commands in other directories (`git archive -o`
/// in a checkout), where a relative path would name another place.
fn absolute(state: &Path) -> Result<PathBuf> {
    std::path::absolute(state).with_context(|| {
        format!(
            "the state directory {} as an absolute path",
            state.display()
        )
    })
}

fn drive(
    store: Store,
    state: Option<&Path>,
    start: StartSnapshot,
    collectors: &[Collector],
) -> Result<Taken> {
    let mut generated = Generated::new(Ports { store });
    let snapshot_id = match generated
        .start_snapshot(start)
        .map_err(|unmet| anyhow!("snapshot start-snapshot: {unmet}"))?
    {
        StartSnapshotOutcome::Started { snapshot_started } => snapshot_started.snapshot_id,
        StartSnapshotOutcome::NegativeDisk { error } => {
            return Err(refused(
                "InvalidCount",
                format_args!("free disk {} is negative; no snapshot started", error.count),
            ));
        }
    };
    generated
        .ports
        .store
        .check()
        .context("keep the started snapshot")?;

    let mut reason = None;
    for collector in collectors {
        let answered = panic::catch_unwind(AssertUnwindSafe(|| {
            collector.collect(&mut Recorder {
                generated: &mut generated,
                snapshot: &snapshot_id,
                state,
            })
        }))
        .unwrap_or_else(|payload| Err(anyhow!("panicked: {}", panic_message(payload.as_ref()))));
        generated.ports.store.check().with_context(|| {
            format!(
                "keep what collector {} recorded; snapshot {} is left collecting",
                collector.name(),
                snapshot_id.0.0
            )
        })?;
        if let Err(error) = answered {
            reason = Some(format!("{}: {error:#}", collector.name()));
            break;
        }
    }

    let state = match &reason {
        None => {
            complete(&mut generated, snapshot_id.clone())?;
            SnapshotState::Complete
        }
        Some(reason) => {
            fail(&mut generated, snapshot_id.clone(), reason.clone())?;
            SnapshotState::Failed
        }
    };
    generated
        .ports
        .store
        .check()
        .context("keep the snapshot's end")?;
    if state == SnapshotState::Complete {
        retain(&generated.ports.store, &snapshot_id)?;
    }
    Ok(Taken {
        snapshot_id,
        state,
        reason,
    })
}

/// The complete snapshots whose observations the instance keeps: its `retention.snapshots`.
fn retention() -> usize {
    usize::try_from(crate::config::active().instance.retention.snapshots).unwrap_or(1)
}

/// [`Store::retain`] after `completed` was kept `Complete`, a failure naming it.
fn retain(store: &Store, completed: &SnapshotId) -> Result<()> {
    store.retain(retention()).with_context(|| {
        format!(
            "snapshot {} is complete; drop the observations of the snapshots retention does not \
             keep",
            completed.0.0
        )
    })?;
    Ok(())
}

/// What a panic said, when it said it as text.
fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("no message")
}

/// `CompleteSnapshot`, a refusal as an error.
fn complete(
    generated: &mut Generated<impl SnapshotStorage>,
    snapshot_id: SnapshotId,
) -> Result<()> {
    match generated
        .complete_snapshot(CompleteSnapshot { snapshot_id })
        .map_err(|unmet| anyhow!("{unmet}"))?
    {
        CompleteSnapshotOutcome::Completed { .. } => Ok(()),
        CompleteSnapshotOutcome::NoSuchSnapshot { error } => Err(not_found(&error.snapshot_id)),
        CompleteSnapshotOutcome::WrongState { error } => Err(not_collecting(&error.snapshot_id)),
    }
}

/// `FailSnapshot`, a refusal as an error.
fn fail(
    generated: &mut Generated<impl SnapshotStorage>,
    snapshot_id: SnapshotId,
    reason: String,
) -> Result<()> {
    match generated
        .fail_snapshot(FailSnapshot {
            snapshot_id,
            reason,
        })
        .map_err(|unmet| anyhow!("{unmet}"))?
    {
        FailSnapshotOutcome::Failed { .. } => Ok(()),
        FailSnapshotOutcome::NoSuchSnapshot { error } => Err(not_found(&error.snapshot_id)),
        FailSnapshotOutcome::WrongState { error } => Err(not_collecting(&error.snapshot_id)),
    }
}

/// What a collector records with: the generated observation commands, over the store a sense run
/// holds open, for the snapshot it fills.
///
/// Each method records into the snapshot this recorder holds, whatever snapshot its input names:
/// the input's `snapshot_id` is replaced by the recorder's before the command runs, so a collector
/// cannot file an observation under another snapshot. Each answers the identity the store
/// assigned the observation. A refusal the specification declares, or a write the store did not
/// keep, is an error naming it.
pub struct Recorder<'a> {
    generated: &'a mut Generated<Ports>,
    snapshot: &'a SnapshotId,
    /// The state directory the store is kept under; `None` when the driver was handed a store
    /// opened elsewhere ([`take`]).
    state: Option<&'a Path>,
}

impl Recorder<'_> {
    /// The snapshot being collected, which each observation names.
    #[must_use]
    pub fn snapshot(&self) -> &SnapshotId {
        self.snapshot
    }

    /// The state directory the store is kept under, an absolute path: where a collector keeps
    /// what it writes besides observations (`exports/`, `specs/`), never under the working
    /// directory.
    ///
    /// # Errors
    ///
    /// The snapshot is taken over a store opened elsewhere ([`take`]), whose state directory the
    /// driver does not know.
    pub fn state(&self) -> Result<&Path> {
        self.state.ok_or_else(|| {
            anyhow!(
                "this snapshot runs over a store opened without its state directory; take it \
                 with `snapshot::take_in`"
            )
        })
    }

    /// `RecordRepository`: one repository's state.
    ///
    /// # Errors
    ///
    /// `SnapshotNotFound`, `SnapshotNotCollecting`, or the store did not keep it.
    pub fn repository(&mut self, mut input: RecordRepository) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self.generated.record_repository(input).map_err(unmet)? {
            RecordRepositoryOutcome::Recorded {
                repository_recorded,
            } => repository_recorded.observation_id,
            RecordRepositoryOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordRepositoryOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
        };
        self.kept(recorded)
    }

    /// `RecordPullRequest`: one open pull request and its checks.
    ///
    /// # Errors
    ///
    /// `InvalidPullRequestNumber`, `InvalidCount`, `SnapshotNotFound`, `SnapshotNotCollecting`,
    /// or the store did not keep it.
    pub fn pull_request(&mut self, mut input: RecordPullRequest) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self.generated.record_pull_request(input).map_err(unmet)? {
            RecordPullRequestOutcome::Recorded {
                pull_request_recorded,
            } => pull_request_recorded.observation_id,
            RecordPullRequestOutcome::InvalidNumber { error } => {
                return Err(refused(
                    "InvalidPullRequestNumber",
                    format_args!("pull request number {} is not positive", error.number),
                ));
            }
            RecordPullRequestOutcome::NegativeChecks { error } => {
                return Err(invalid_count(error.count));
            }
            RecordPullRequestOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordPullRequestOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
        };
        self.kept(recorded)
    }

    /// `RecordWorkflowRun`: the latest `main` run of one workflow.
    ///
    /// # Errors
    ///
    /// `SnapshotNotFound`, `SnapshotNotCollecting`, or the store did not keep it.
    pub fn workflow_run(&mut self, mut input: RecordWorkflowRun) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self.generated.record_workflow_run(input).map_err(unmet)? {
            RecordWorkflowRunOutcome::Recorded {
                workflow_run_recorded,
            } => workflow_run_recorded.observation_id,
            RecordWorkflowRunOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordWorkflowRunOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
        };
        self.kept(recorded)
    }

    /// `RecordSession`: one live session.
    ///
    /// # Errors
    ///
    /// `SnapshotNotFound`, `SnapshotNotCollecting`, or the store did not keep it.
    pub fn session(&mut self, mut input: RecordSession) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self.generated.record_session(input).map_err(unmet)? {
            RecordSessionOutcome::Recorded { session_recorded } => session_recorded.observation_id,
            RecordSessionOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordSessionOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
        };
        self.kept(recorded)
    }

    /// `RecordBlocker`: one open blocker artifact.
    ///
    /// # Errors
    ///
    /// `SnapshotNotFound`, `SnapshotNotCollecting`, or the store did not keep it.
    pub fn blocker(&mut self, mut input: RecordBlocker) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self.generated.record_blocker(input).map_err(unmet)? {
            RecordBlockerOutcome::Recorded { blocker_recorded } => blocker_recorded.observation_id,
            RecordBlockerOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordBlockerOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
        };
        self.kept(recorded)
    }

    /// `RecordSpecification`: one repository's specification status.
    ///
    /// # Errors
    ///
    /// `SnapshotNotFound`, `SnapshotNotCollecting`, `InvalidCount`, or the store did not keep it.
    pub fn specification(&mut self, mut input: RecordSpecification) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self.generated.record_specification(input).map_err(unmet)? {
            RecordSpecificationOutcome::Recorded {
                specification_recorded,
            } => specification_recorded.observation_id,
            RecordSpecificationOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordSpecificationOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
            RecordSpecificationOutcome::NegativeRefusals { error } => {
                return Err(invalid_count(error.count));
            }
        };
        self.kept(recorded)
    }

    /// When the snapshot being collected started, as it is stored: the clock of a collector whose
    /// window is the snapshot's (`github`'s releases and merged pull requests).
    ///
    /// # Errors
    ///
    /// `SnapshotNotFound` when the store holds no readable snapshot of this id.
    pub fn started_at(&self) -> Result<Timestamp> {
        SnapshotStorage::get(&self.generated.ports, self.snapshot)
            .map(|held| held.data.started_at)
            .ok_or_else(|| not_found(self.snapshot))
    }

    /// `RecordRelease`: one published release.
    ///
    /// # Errors
    ///
    /// `SnapshotNotFound`, `SnapshotNotCollecting`, or the store did not keep it.
    pub fn release(&mut self, mut input: RecordRelease) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self.generated.record_release(input).map_err(unmet)? {
            RecordReleaseOutcome::Recorded { release_recorded } => release_recorded.observation_id,
            RecordReleaseOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordReleaseOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
        };
        self.kept(recorded)
    }

    /// `RecordMergedPullRequest`: one merged pull request.
    ///
    /// # Errors
    ///
    /// `InvalidPullRequestNumber`, `SnapshotNotFound`, `SnapshotNotCollecting`, or the store did
    /// not keep it.
    pub fn merged_pull_request(
        &mut self,
        mut input: RecordMergedPullRequest,
    ) -> Result<ObservationId> {
        input.snapshot_id = self.snapshot.clone();
        let recorded = match self
            .generated
            .record_merged_pull_request(input)
            .map_err(unmet)?
        {
            RecordMergedPullRequestOutcome::Recorded {
                merged_pull_request_recorded,
            } => merged_pull_request_recorded.observation_id,
            RecordMergedPullRequestOutcome::InvalidNumber { error } => {
                return Err(refused(
                    "InvalidPullRequestNumber",
                    format_args!("pull request number {} is not positive", error.number),
                ));
            }
            RecordMergedPullRequestOutcome::NoSuchSnapshot { error } => {
                return Err(not_found(&error.snapshot_id));
            }
            RecordMergedPullRequestOutcome::Closed { error } => {
                return Err(not_collecting(&error.snapshot_id));
            }
        };
        self.kept(recorded)
    }

    /// `recorded`, once the store has kept every write so far.
    fn kept(&self, recorded: ObservationId) -> Result<ObservationId> {
        self.generated
            .ports
            .store
            .check()
            .context("keep the observation")?;
        Ok(recorded)
    }
}

/// Runs one observation command, `observe`, over the store under the state directory `state`
/// (`--state-dir`, else `state/`), for `snapshot`, and writes the identity it assigned to standard
/// output.
///
/// # Errors
///
/// The store does not open, `observe` is refused, or the store did not keep the observation.
pub(crate) fn record(
    state: Option<&Path>,
    snapshot: &SnapshotId,
    observe: impl FnOnce(&mut Recorder<'_>) -> Result<ObservationId>,
) -> Result<ExitCode> {
    let dir = absolute(&crate::state::dir(state)?)?;
    let mut generated = Generated::new(Ports {
        store: crate::store::open(&dir)?,
    });
    let recorded = observe(&mut Recorder {
        generated: &mut generated,
        snapshot,
        state: Some(&dir),
    })?;
    line(&recorded.0.0)?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor snapshot start-snapshot`: the driver over the registered collectors
/// ([`collect::registered`]). `--started-at` and `--disk-free-bytes` replace the current time and
/// the measured free bytes.
///
/// Prints the snapshot's id. Exits 0 when it completed, and 1 with one line naming the collector
/// that did not answer when it failed.
///
/// # Errors
///
/// `InvalidCount` for negative free bytes, or what [`take`] answers.
pub fn start_snapshot(state: Option<&Path>, args: StartSnapshotArgs) -> Result<ExitCode> {
    const COMMAND: &str = "snapshot start-snapshot";
    no_input_json(args.input_json, "snapshot start-snapshot --input-json")?;
    let taken = run(COMMAND, || {
        let started_at = match args.started_at {
            Some(at) => at,
            None => now()?,
        };
        let disk_free_bytes = match args.disk_free_bytes {
            Some(bytes) => bytes,
            None => disk_free_bytes(Path::new(DISK))?,
        };
        let start = StartSnapshot {
            started_at: Timestamp(started_at),
            disk_free_bytes,
        };
        let dir = absolute(&crate::state::dir(state)?)?;
        let taken = drive(
            crate::store::open(&dir)?,
            Some(&dir),
            start,
            &collect::registered(),
        )?;
        line(&taken.snapshot_id.0.0)?;
        Ok(taken)
    })?;
    match taken.reason {
        None => Ok(ExitCode::SUCCESS),
        Some(reason) => {
            eprintln!(
                "conductor: {COMMAND}: snapshot {} failed: {}",
                taken.snapshot_id.0.0,
                one_line(&reason)
            );
            Ok(ExitCode::FAILURE)
        }
    }
}

/// `conductor snapshot record-repository`: the command `conductor.observation.RecordRepository`.
/// Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_repository(state: Option<&Path>, args: RecordRepositoryArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot record-repository --input-json")?;
    run("snapshot record-repository", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordRepository {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(given(args.repository, "repository")?),
            visibility: given_named(args.visibility, "visibility", visibility_named)?,
            archived: given(args.archived, "archived")?,
            local_checkout: given(args.local_checkout, "local-checkout")?,
            main_head: CommitSha(given(args.main_head, "main-head")?),
            main_committed_at: Timestamp(given(args.main_committed_at, "main-committed-at")?),
            main_commits_7d: given(args.main_commits_7d, "main-commits-7d")?,
            real_commits_7d: given(args.real_commits_7d, "real-commits-7d")?,
            behind_main: given(args.behind_main, "behind-main")?,
            dirty_files: given(args.dirty_files, "dirty-files")?,
            worktrees: given(args.worktrees, "worktrees")?,
            open_issues: given(args.open_issues, "open-issues")?,
            oldest_open_issue_at: args.oldest_open_issue_at.map(Timestamp),
            latest_release: args.latest_release,
            latest_release_at: args.latest_release_at.map(Timestamp),
            unreleased_commits: args.unreleased_commits,
            planning_store_version: args.planning_store_version,
            in_catalog: args.in_catalog,
        };
        record(state, &snapshot, |recorder| recorder.repository(input))
    })
}

/// `conductor snapshot record-pull-request`: the command
/// `conductor.observation.RecordPullRequest`. Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_pull_request(state: Option<&Path>, args: RecordPullRequestArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot record-pull-request --input-json")?;
    run("snapshot record-pull-request", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordPullRequest {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(given(args.repository, "repository")?),
            number: given(args.number, "number")?,
            title: given(args.title, "title")?,
            draft: given(args.draft, "draft")?,
            mergeability: given_named(args.mergeability, "mergeability", mergeability_named)?,
            failing_checks: given(args.failing_checks, "failing-checks")?,
            opened_at: Timestamp(given(args.opened_at, "opened-at")?),
            updated_at: Timestamp(given(args.updated_at, "updated-at")?),
        };
        record(state, &snapshot, |recorder| recorder.pull_request(input))
    })
}

/// `conductor snapshot record-release`: the command `conductor.observation.RecordRelease`.
/// Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_release(state: Option<&Path>, args: RecordReleaseArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot record-release --input-json")?;
    run("snapshot record-release", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordRelease {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(given(args.repository, "repository")?),
            tag: given(args.tag, "tag")?,
            published_at: Timestamp(given(args.published_at, "published-at")?),
            url: given(args.url, "url")?,
        };
        record(state, &snapshot, |recorder| recorder.release(input))
    })
}

/// `conductor snapshot record-merged-pull-request`: the command
/// `conductor.observation.RecordMergedPullRequest`. Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_merged_pull_request(
    state: Option<&Path>,
    args: RecordMergedPullRequestArgs,
) -> Result<ExitCode> {
    no_input_json(
        args.input_json,
        "snapshot record-merged-pull-request --input-json",
    )?;
    run("snapshot record-merged-pull-request", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordMergedPullRequest {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(given(args.repository, "repository")?),
            number: given(args.number, "number")?,
            title: given(args.title, "title")?,
            merged_at: Timestamp(given(args.merged_at, "merged-at")?),
            url: given(args.url, "url")?,
        };
        record(state, &snapshot, |recorder| {
            recorder.merged_pull_request(input)
        })
    })
}

/// `conductor snapshot releases`: the view `conductor.observation.Releases`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn releases(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 6] = [
        "observation_id",
        "snapshot_id",
        "repository",
        "tag",
        "published_at",
        "url",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .releases()
        .map_err(|unmet| anyhow!("snapshot releases: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.repository.0),
                Value::from(row.tag),
                Value::from(row.published_at.0),
                Value::from(row.url),
            ]
        })
        .collect();
    show(&generated, "releases", view, &COLUMNS, rows)
}

/// `conductor snapshot merged-pull-requests`: the view `conductor.observation.MergedPullRequests`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn merged_pull_requests(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 7] = [
        "observation_id",
        "snapshot_id",
        "repository",
        "number",
        "title",
        "merged_at",
        "url",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .merged_pull_requests()
        .map_err(|unmet| anyhow!("snapshot merged-pull-requests: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.repository.0),
                Value::from(row.number),
                Value::from(row.title),
                Value::from(row.merged_at.0),
                Value::from(row.url),
            ]
        })
        .collect();
    show(&generated, "merged pull requests", view, &COLUMNS, rows)
}

/// `conductor snapshot record-workflow-run`: the command
/// `conductor.observation.RecordWorkflowRun`. Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_workflow_run(state: Option<&Path>, args: RecordWorkflowRunArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot record-workflow-run --input-json")?;
    run("snapshot record-workflow-run", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordWorkflowRun {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(given(args.repository, "repository")?),
            workflow: given(args.workflow, "workflow")?,
            conclusion: given_named(args.conclusion, "conclusion", run_conclusion_named)?,
            run_at: Timestamp(given(args.run_at, "run-at")?),
        };
        record(state, &snapshot, |recorder| recorder.workflow_run(input))
    })
}

/// `conductor snapshot record-session`: the command `conductor.observation.RecordSession`.
/// Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_session(state: Option<&Path>, args: RecordSessionArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot record-session --input-json")?;
    run("snapshot record-session", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordSession {
            snapshot_id: snapshot.clone(),
            harness: given_named(args.harness, "harness", harness_named)?,
            session_ref: given(args.session_ref, "session-ref")?,
            name: args.name,
            cwd: given(args.cwd, "cwd")?,
            repository: args.repository.map(RepositoryName),
            role: args.role,
            activity: given_named(args.activity, "activity", session_state_named)?,
        };
        record(state, &snapshot, |recorder| recorder.session(input))
    })
}

/// `conductor snapshot record-blocker`: the command `conductor.observation.RecordBlocker`.
/// Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_blocker(state: Option<&Path>, args: RecordBlockerArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot record-blocker --input-json")?;
    run("snapshot record-blocker", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordBlocker {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(given(args.repository, "repository")?),
            reference: given(args.reference, "reference")?,
            blocker_kind: given(args.blocker_kind, "blocker-kind")?,
            title: given(args.title, "title")?,
        };
        record(state, &snapshot, |recorder| recorder.blocker(input))
    })
}

/// `conductor snapshot record-specification`: the command
/// `conductor.observation.RecordSpecification`. Prints the observation's id.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn record_specification(
    state: Option<&Path>,
    args: RecordSpecificationArgs,
) -> Result<ExitCode> {
    no_input_json(
        args.input_json,
        "snapshot record-specification --input-json",
    )?;
    run("snapshot record-specification", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let input = RecordSpecification {
            snapshot_id: snapshot.clone(),
            repository: RepositoryName(given(args.repository, "repository")?),
            presence: given_named(args.presence, "presence", specification_presence_named)?,
            path: args.path,
            format: args.format,
            required_ess: args.required_ess,
            validation: given_named(args.validation, "validation", validation_result_named)?,
            validation_refusals: given(args.validation_refusals, "validation-refusals")?,
            scenarios: args.scenarios,
            synthesis_refusals: args.synthesis_refusals,
            conformance_status: args.conformance_status,
        };
        record(state, &snapshot, |recorder| recorder.specification(input))
    })
}

/// `conductor snapshot complete-snapshot`: the command `conductor.observation.CompleteSnapshot`.
///
/// # Errors
///
/// `--snapshot-id` is missing, or the command is refused.
pub fn complete_snapshot(state: Option<&Path>, args: CompleteSnapshotArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot complete-snapshot --input-json")?;
    run("snapshot complete-snapshot", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let mut generated = Generated::new(crate::state::open(state)?);
        complete(&mut generated, snapshot.clone())?;
        generated
            .ports
            .check()
            .context("keep the completed snapshot")?;
        retain(&generated.ports, &snapshot)?;
        Ok(ExitCode::SUCCESS)
    })
}

/// `conductor snapshot fail-snapshot`: the command `conductor.observation.FailSnapshot`.
///
/// # Errors
///
/// A field is missing, or the command is refused.
pub fn fail_snapshot(state: Option<&Path>, args: FailSnapshotArgs) -> Result<ExitCode> {
    no_input_json(args.input_json, "snapshot fail-snapshot --input-json")?;
    run("snapshot fail-snapshot", || {
        let snapshot = snapshot_id(args.snapshot_id)?;
        let reason = given(args.reason, "reason")?;
        let mut generated = Generated::new(crate::state::open(state)?);
        fail(&mut generated, snapshot, reason)?;
        generated
            .ports
            .check()
            .context("keep the failed snapshot")?;
        Ok(ExitCode::SUCCESS)
    })
}

/// `conductor snapshot snapshots`: the view `conductor.observation.Snapshots`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn snapshots(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 5] = [
        "snapshot_id",
        "state",
        "started_at",
        "disk_free_bytes",
        "failure_reason",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .snapshots()
        .map_err(|unmet| anyhow!("snapshot snapshots: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.snapshot_id.0.0),
                Value::from(snapshot_state_name(row.state)),
                Value::from(row.started_at.0),
                Value::from(row.disk_free_bytes),
                Value::from(row.failure_reason),
            ]
        })
        .collect();
    show(&generated, "snapshots", view, &COLUMNS, rows)
}

/// `conductor snapshot repositories`: the view `conductor.observation.Repositories`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn repositories(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 20] = [
        "observation_id",
        "snapshot_id",
        "repository",
        "visibility",
        "archived",
        "local_checkout",
        "main_head",
        "main_committed_at",
        "main_commits_7d",
        "real_commits_7d",
        "behind_main",
        "dirty_files",
        "worktrees",
        "open_issues",
        "oldest_open_issue_at",
        "latest_release",
        "latest_release_at",
        "unreleased_commits",
        "planning_store_version",
        "in_catalog",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .repositories()
        .map_err(|unmet| anyhow!("snapshot repositories: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.repository.0),
                Value::from(visibility_name(row.visibility)),
                Value::from(row.archived),
                Value::from(row.local_checkout),
                Value::from(row.main_head.0),
                Value::from(row.main_committed_at.0),
                Value::from(row.main_commits_7d),
                Value::from(row.real_commits_7d),
                Value::from(row.behind_main),
                Value::from(row.dirty_files),
                Value::from(row.worktrees),
                Value::from(row.open_issues),
                Value::from(row.oldest_open_issue_at.map(|at| at.0)),
                Value::from(row.latest_release),
                Value::from(row.latest_release_at.map(|at| at.0)),
                Value::from(row.unreleased_commits),
                Value::from(row.planning_store_version),
                Value::from(row.in_catalog),
            ]
        })
        .collect();
    show(&generated, "repositories", view, &COLUMNS, rows)
}

/// `conductor snapshot pull-requests`: the view `conductor.observation.PullRequests`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn pull_requests(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 10] = [
        "observation_id",
        "snapshot_id",
        "repository",
        "number",
        "title",
        "draft",
        "mergeability",
        "failing_checks",
        "opened_at",
        "updated_at",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .pull_requests()
        .map_err(|unmet| anyhow!("snapshot pull-requests: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.repository.0),
                Value::from(row.number),
                Value::from(row.title),
                Value::from(row.draft),
                Value::from(mergeability_name(row.mergeability)),
                Value::from(row.failing_checks),
                Value::from(row.opened_at.0),
                Value::from(row.updated_at.0),
            ]
        })
        .collect();
    show(&generated, "pull requests", view, &COLUMNS, rows)
}

/// `conductor snapshot workflow-runs`: the view `conductor.observation.WorkflowRuns`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn workflow_runs(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 6] = [
        "observation_id",
        "snapshot_id",
        "repository",
        "workflow",
        "conclusion",
        "run_at",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .workflow_runs()
        .map_err(|unmet| anyhow!("snapshot workflow-runs: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.repository.0),
                Value::from(row.workflow),
                Value::from(run_conclusion_name(row.conclusion)),
                Value::from(row.run_at.0),
            ]
        })
        .collect();
    show(&generated, "workflow runs", view, &COLUMNS, rows)
}

/// `conductor snapshot sessions`: the view `conductor.observation.Sessions`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn sessions(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 9] = [
        "observation_id",
        "snapshot_id",
        "harness",
        "session_ref",
        "name",
        "cwd",
        "repository",
        "role",
        "activity",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .sessions()
        .map_err(|unmet| anyhow!("snapshot sessions: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(harness_name(row.harness)),
                Value::from(row.session_ref),
                Value::from(row.name),
                Value::from(row.cwd),
                Value::from(row.repository.map(|repository| repository.0)),
                Value::from(row.role),
                Value::from(session_state_name(row.activity)),
            ]
        })
        .collect();
    show(&generated, "sessions", view, &COLUMNS, rows)
}

/// `conductor snapshot blockers`: the view `conductor.observation.Blockers`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn blockers(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 6] = [
        "observation_id",
        "snapshot_id",
        "repository",
        "reference",
        "blocker_kind",
        "title",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .blockers()
        .map_err(|unmet| anyhow!("snapshot blockers: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.repository.0),
                Value::from(row.reference),
                Value::from(row.blocker_kind),
                Value::from(row.title),
            ]
        })
        .collect();
    show(&generated, "blockers", view, &COLUMNS, rows)
}

/// `conductor snapshot specifications`: the view `conductor.observation.Specifications`.
///
/// # Errors
///
/// There is no store, a read of it fails for a reason other than a record this build cannot read,
/// or the rows cannot be written to standard output.
pub fn specifications(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 12] = [
        "observation_id",
        "snapshot_id",
        "repository",
        "presence",
        "path",
        "format",
        "required_ess",
        "validation",
        "validation_refusals",
        "scenarios",
        "synthesis_refusals",
        "conformance_status",
    ];
    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .specifications()
        .map_err(|unmet| anyhow!("snapshot specifications: {unmet}"))?;
    let rows = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.observation_id.0.0),
                Value::from(row.snapshot_id.0.0),
                Value::from(row.repository.0),
                Value::from(specification_presence_name(row.presence)),
                Value::from(row.path),
                Value::from(row.format),
                Value::from(row.required_ess),
                Value::from(validation_result_name(row.validation)),
                Value::from(row.validation_refusals),
                Value::from(row.scenarios),
                Value::from(row.synthesis_refusals),
                Value::from(row.conformance_status),
            ]
        })
        .collect();
    show(&generated, "specifications", view, &COLUMNS, rows)
}

/// Answers a view: its `rows`, rendered as `--format` asks, through [`crate::store::show`]. Its
/// `columns` are the view's fields in the specification's order (`spec/domains/observation.yaml`).
fn show(
    generated: &Generated<Store>,
    what: &str,
    view: ViewArgs,
    columns: &[&str],
    rows: Vec<Vec<Value>>,
) -> Result<ExitCode> {
    crate::store::show(&generated.ports, what, &view.render(columns, &rows))
}

/// The ports the observation commands run over: the store, and the identities they assign.
pub(crate) struct Ports {
    store: Store,
}

/// Each storage port of [`Ports`] is the store's.
macro_rules! delegate {
    ($($port:ident: $id:ident => $snapshot:ident;)+) => {$(
        impl $port for Ports {
            fn get(&self, identity: &$id) -> Option<$snapshot> {
                $port::get(&self.store, identity)
            }

            fn put(&mut self, snapshot: $snapshot) {
                $port::put(&mut self.store, snapshot);
            }

            fn delete(&mut self, identity: &$id) {
                $port::delete(&mut self.store, identity);
            }

            fn list(&self) -> Vec<$snapshot> {
                $port::list(&self.store)
            }
        }
    )+};
}

delegate! {
    SnapshotStorage: SnapshotId => SnapshotSnapshot;
    RepositoryObservationStorage: ObservationId => RepositoryObservationSnapshot;
    PullRequestObservationStorage: ObservationId => PullRequestObservationSnapshot;
    ReleaseObservationStorage: ObservationId => ReleaseObservationSnapshot;
    MergedPullRequestObservationStorage: ObservationId => MergedPullRequestObservationSnapshot;
    WorkflowRunObservationStorage: ObservationId => WorkflowRunObservationSnapshot;
    SessionObservationStorage: ObservationId => SessionObservationSnapshot;
    BlockerObservationStorage: ObservationId => BlockerObservationSnapshot;
    SpecificationObservationStorage: ObservationId => SpecificationObservationSnapshot;
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
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Ok(ObservationId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Ok(SnapshotId(Uuid(eventlog_core::new_event_id())))
    }
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
    SnapshotState: snapshot_state_name, snapshot_state_named = [Collecting, Complete, Failed];
    Visibility: visibility_name, visibility_named = [Public, Private];
    Mergeability: mergeability_name, mergeability_named = [Mergeable, Conflicting, Unknown];
    RunConclusion: run_conclusion_name, run_conclusion_named =
        [Success, Failure, Cancelled, Skipped, Neutral, Pending];
    Harness: harness_name, harness_named = [Claude, Codex];
    SessionState: session_state_name, session_state_named =
        [Busy, Idle, Waiting, Background, Unknown];
    SpecificationPresence: specification_presence_name, specification_presence_named =
        [Present, OptedOut, Missing];
    ValidationResult: validation_result_name, validation_result_named = [Valid, Refused, NotRun];
}

/// A refusal the specification declares, as one line: the error's name, then what it says.
fn refused(error: &str, detail: impl Display) -> anyhow::Error {
    anyhow!("{error}: {detail}")
}

fn not_found(snapshot: &SnapshotId) -> anyhow::Error {
    refused(
        "SnapshotNotFound",
        format_args!("no snapshot has the id {}", snapshot.0.0),
    )
}

fn not_collecting(snapshot: &SnapshotId) -> anyhow::Error {
    refused(
        "SnapshotNotCollecting",
        format_args!(
            "snapshot {} is complete or failed, so it takes no more observations and cannot \
             change state",
            snapshot.0.0
        ),
    )
}

fn invalid_count(count: i64) -> anyhow::Error {
    refused("InvalidCount", format_args!("count {count} is negative"))
}

/// A behaviour that declares no outcome for its input.
fn unmet(unmet: UnmetObligation) -> anyhow::Error {
    anyhow!("{unmet}")
}

/// What `body` answers, an error named by the command it ran for.
fn run<T>(command: &'static str, body: impl FnOnce() -> Result<T>) -> Result<T> {
    body().context(command)
}

/// Answers [`NotImplemented`](crate::NotImplemented) for `what` when `--input-json -` was given.
fn no_input_json(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(JsonInput::Stdin) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of the flag `--<flag>`, which this command needs.
fn given<T>(value: Option<T>, flag: &str) -> Result<T> {
    value.ok_or_else(|| anyhow!("--{flag} is missing"))
}

/// The enum value `--<flag>` names, by the specification's spelling.
fn given_named<T>(value: Option<String>, flag: &str, named: fn(&str) -> Option<T>) -> Result<T> {
    let name = given(value, flag)?;
    named(&name).ok_or_else(|| anyhow!("--{flag} {name:?} is not a value the specification names"))
}

fn snapshot_id(value: Option<String>) -> Result<SnapshotId> {
    Ok(SnapshotId(Uuid(given(value, "snapshot-id")?)))
}

/// The current time, RFC 3339.
fn now() -> Result<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format the current time")
}

/// The bytes an unprivileged writer has free on the file system holding `path`, as `df` reads them
/// from `statvfs` (the crate forbids `unsafe`, and has no dependency that wraps the call).
fn disk_free_bytes(path: &Path) -> Result<i64> {
    let output = Command::new("df")
        .args(["-B1", "--output=avail"])
        .arg(path)
        .output()
        .with_context(|| format!("run `df {}`", path.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        bail!(
            "`df {}` exited {:?}: {}",
            path.display(),
            output.status.code(),
            one_line(&String::from_utf8_lossy(&output.stderr))
        );
    }
    stdout
        .lines()
        .last()
        .and_then(|line| line.trim().parse::<i64>().ok())
        .ok_or_else(|| anyhow!("`df {}` printed no byte count: {stdout:?}", path.display()))
}

/// Writes `text` and a line break to standard output.
fn line(text: &str) -> Result<()> {
    writeln!(io::stdout().lock(), "{text}").context("write to standard output")
}

/// `text` on one line: every line break a space.
fn one_line(text: &str) -> String {
    text.split(['\n', '\r'])
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
