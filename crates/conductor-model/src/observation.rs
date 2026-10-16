// generated from conductor v1
// model digest c7a400dd0778e5076d21a29762ae666f240b19f0559ada9a788ea1044ffa3f9a
// contract digest 09a75fd598f74b773a3cdd6f97222132becf654ce3ccd648ad0a31723f5935cb
// do not edit: regenerate with `ess synthesize --layout crate`

//! Observation — `conductor.observation`.
//!
//! What conductor sees at one moment: one snapshot per sense run, holding one observation per repository, open pull request, latest main-branch workflow run and live agent session. Every observation is read from the tool that owns the fact (git, GitHub, aep, the session lists) and is never edited after it is recorded. Design: docs/design/conductor.md § 4.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `conductor.observation.BlockerObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `BlockerObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockerObservationState {
    /// `Recorded`.
    Recorded,
}

/// The states of `conductor.observation.Board`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Board<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardState {
    /// `Published`.
    Published,
}

/// BoardId — `conductor.observation.BoardId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardId(pub crate::primitives::Uuid);

/// CollectGarbage — `conductor.observation.CollectGarbage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectGarbage {
    /// `dry_run` — `Optional<Boolean>`.
    pub dry_run: Option<bool>,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// The states of `conductor.observation.CollectorRunObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `CollectorRunObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorRunObservationState {
    /// `Recorded`.
    Recorded,
}

/// CollectorRunOutcome — `conductor.observation.CollectorRunOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectorRunOutcome {
    /// `Completed`.
    Completed,
    /// `Failed`.
    Failed,
    /// `OverBudget`.
    OverBudget,
}

/// CommitSha — `conductor.observation.CommitSha`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitSha(pub String);

/// The states of `conductor.observation.DailySeries`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `DailySeries<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DailySeriesState {
    /// `Recorded`.
    Recorded,
}

/// DailySeriesId — `conductor.observation.DailySeriesId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySeriesId(pub String);

/// DashboardMemory — `conductor.observation.DashboardMemory`.
///
/// Every value satisfies `store_handles >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `store_replays >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `store_replays_peak >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `store_bytes >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `memo_entries >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `memo_entries_peak_endpoint >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `memo_bytes >= 0` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardMemory {
    /// `rss_bytes` — `Optional<Integer>`.
    pub rss_bytes: Option<i64>,
    /// `store_handles` — `Integer`.
    pub store_handles: i64,
    /// `store_replays` — `Integer`.
    pub store_replays: i64,
    /// `store_replays_peak` — `Integer`.
    pub store_replays_peak: i64,
    /// `store_bytes` — `Integer`.
    pub store_bytes: i64,
    /// `memo_entries` — `Integer`.
    pub memo_entries: i64,
    /// `memo_entries_peak_endpoint` — `Integer`.
    pub memo_entries_peak_endpoint: i64,
    /// `memo_bytes` — `Integer`.
    pub memo_bytes: i64,
}

/// DashboardServe — `conductor.observation.DashboardServe`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardServe {
    /// `port` — `Optional<Integer>`.
    pub port: Option<i64>,
    /// `root` — `Optional<String>`.
    pub root: Option<String>,
}

/// EnvelopeProvider — `conductor.observation.EnvelopeProvider`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeProvider {
    /// `Systemd`.
    Systemd,
}

/// FailureClass — `conductor.observation.FailureClass`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// `NeedsLogin`.
    NeedsLogin,
    /// `UnreachableHost`.
    UnreachableHost,
    /// `TimedOut`.
    TimedOut,
    /// `OldStore`.
    OldStore,
    /// `Other`.
    Other,
}

/// GarbageOutcome — `conductor.observation.GarbageOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GarbageOutcome {
    /// `Reclaimable`.
    Reclaimable,
    /// `Removed`.
    Removed,
    /// `Refused`.
    Refused,
}

/// GarbageRow — `conductor.observation.GarbageRow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GarbageRow {
    /// `repository` — `Optional<String>`.
    pub repository: Option<String>,
    /// `what` — `String`.
    pub what: String,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `count` — `Optional<Integer>`.
    pub count: Option<i64>,
    /// `bytes` — `Optional<Integer>`.
    pub bytes: Option<i64>,
    /// `freed` — `Optional<Integer>`.
    pub freed: Option<i64>,
    /// `outcome` — `conductor.observation.GarbageOutcome`.
    pub outcome: GarbageOutcome,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
}

/// Harness — `conductor.observation.Harness`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    /// `Claude`.
    Claude,
    /// `Codex`.
    Codex,
}

/// HooksOutcome — `conductor.observation.HooksOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HooksOutcome {
    /// `Lagging`.
    Lagging,
    /// `Absent`.
    Absent,
    /// `Refreshed`.
    Refreshed,
    /// `Refused`.
    Refused,
}

/// HooksRow — `conductor.observation.HooksRow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HooksRow {
    /// `repository` — `String`.
    pub repository: String,
    /// `checkout` — `String`.
    pub checkout: String,
    /// `before` — `Optional<String>`.
    pub before: Option<String>,
    /// `after` — `Optional<String>`.
    pub after: Option<String>,
    /// `outcome` — `conductor.observation.HooksOutcome`.
    pub outcome: HooksOutcome,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
}

/// The states of `conductor.observation.MemberFailureObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `MemberFailureObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberFailureObservationState {
    /// `Recorded`.
    Recorded,
}

/// Mergeability — `conductor.observation.Mergeability`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mergeability {
    /// `Mergeable`.
    Mergeable,
    /// `Conflicting`.
    Conflicting,
    /// `Unknown`.
    Unknown,
}

/// The states of `conductor.observation.MergedPullRequestObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `MergedPullRequestObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergedPullRequestObservationState {
    /// `Recorded`.
    Recorded,
}

/// ObservationId — `conductor.observation.ObservationId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationId(pub crate::primitives::Uuid);

/// The states of `conductor.observation.PullRequestObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `PullRequestObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullRequestObservationState {
    /// `Recorded`.
    Recorded,
}

/// RefreshHooks — `conductor.observation.RefreshHooks`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshHooks {
    /// `apply` — `Optional<Boolean>`.
    pub apply: Option<bool>,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// The states of `conductor.observation.ReleaseObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ReleaseObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseObservationState {
    /// `Recorded`.
    Recorded,
}

/// RepositoryFigures — `conductor.observation.RepositoryFigures`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryFigures {
    /// `repository` — `String`.
    pub repository: String,
    /// `deviations` — `List<String>`.
    pub deviations: Vec<String>,
    /// `open_prs` — `Integer`.
    pub open_prs: i64,
    /// `failing_prs` — `Integer`.
    pub failing_prs: i64,
    /// `failing_checks` — `List<String>`.
    pub failing_checks: Vec<String>,
    /// `non_green_workflows` — `Integer`.
    pub non_green_workflows: i64,
    /// `open_blockers` — `Integer`.
    pub open_blockers: i64,
    /// `specifications_present` — `Integer`.
    pub specifications_present: i64,
}

/// RepositoryName — `conductor.observation.RepositoryName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryName(pub String);

/// The states of `conductor.observation.RepositoryObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `RepositoryObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryObservationState {
    /// `Recorded`.
    Recorded,
}

/// RepositoryShipped — `conductor.observation.RepositoryShipped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryShipped {
    /// `since` — `String`.
    pub since: String,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// ResourceUsage — `conductor.observation.ResourceUsage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUsage {
    /// `since` — `Optional<String>`.
    pub since: Option<String>,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// RunConclusion — `conductor.observation.RunConclusion`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunConclusion {
    /// `Success`.
    Success,
    /// `Failure`.
    Failure,
    /// `Cancelled`.
    Cancelled,
    /// `Skipped`.
    Skipped,
    /// `Neutral`.
    Neutral,
    /// `Pending`.
    Pending,
}

/// The states of `conductor.observation.SecretSyncObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SecretSyncObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretSyncObservationState {
    /// `Recorded`.
    Recorded,
}

/// The states of `conductor.observation.SessionObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SessionObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionObservationState {
    /// `Recorded`.
    Recorded,
}

/// SessionState — `conductor.observation.SessionState`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// `Busy`.
    Busy,
    /// `Idle`.
    Idle,
    /// `Waiting`.
    Waiting,
    /// `Background`.
    Background,
    /// `Unknown`.
    Unknown,
}

/// The states of `conductor.observation.Snapshot`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Snapshot<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotState {
    /// `Collecting`.
    Collecting,
    /// `Complete`.
    Complete,
    /// `Failed`.
    Failed,
}

/// SnapshotId — `conductor.observation.SnapshotId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotId(pub crate::primitives::Uuid);

/// The states of `conductor.observation.SpecificationObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `SpecificationObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecificationObservationState {
    /// `Recorded`.
    Recorded,
}

/// SpecificationPresence — `conductor.observation.SpecificationPresence`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecificationPresence {
    /// `Present`.
    Present,
    /// `OptedOut`.
    OptedOut,
    /// `Missing`.
    Missing,
    /// `NotInWorkspace`.
    NotInWorkspace,
}

/// Status — `conductor.observation.Status`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// StatusDispatch — `conductor.observation.StatusDispatch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusDispatch {
    /// `dispatch_id` — `String`.
    pub dispatch_id: String,
    /// `repository` — `String`.
    pub repository: String,
    /// `state` — `String`.
    pub state: String,
    /// `sent_at` — `String`.
    pub sent_at: String,
    /// `waits_on` — `Optional<String>`.
    pub waits_on: Option<String>,
    /// `brief` — `String`.
    pub brief: String,
}

/// StatusReport — `conductor.observation.StatusReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReport {
    /// `snapshot_id` — `Optional<String>`.
    pub snapshot_id: Option<String>,
    /// `snapshot_started_at` — `Optional<String>`.
    pub snapshot_started_at: Option<String>,
    /// `snapshot_age_seconds` — `Optional<Integer>`.
    pub snapshot_age_seconds: Option<i64>,
    /// `deviations` — `List<conductor.observation.RepositoryFigures>`.
    pub deviations: Vec<RepositoryFigures>,
    /// `sessions` — `List<conductor.observation.StatusSession>`.
    pub sessions: Vec<StatusSession>,
    /// `dispatches` — `List<conductor.observation.StatusDispatch>`.
    pub dispatches: Vec<StatusDispatch>,
    /// `waits` — `List<String>`.
    pub waits: Vec<String>,
    /// `requests` — `List<conductor.observation.StatusRequest>`.
    pub requests: Vec<StatusRequest>,
    /// `usage` — `Optional<String>`.
    pub usage: Option<String>,
    /// `unanswered_operator_items` — `Integer`.
    pub unanswered_operator_items: i64,
    /// `oldest_unanswered_request_id` — `Optional<String>`.
    pub oldest_unanswered_request_id: Option<String>,
    /// `oldest_unanswered_age_seconds` — `Optional<Integer>`.
    pub oldest_unanswered_age_seconds: Option<i64>,
}

/// StatusRequest — `conductor.observation.StatusRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusRequest {
    /// `kind` — `String`.
    pub kind: String,
    /// `request_id` — `String`.
    pub request_id: String,
    /// `repository` — `String`.
    pub repository: String,
    /// `state` — `String`.
    pub state: String,
    /// `since` — `String`.
    pub since: String,
    /// `summary` — `String`.
    pub summary: String,
}

/// StatusSession — `conductor.observation.StatusSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSession {
    /// `repository` — `Optional<String>`.
    pub repository: Option<String>,
    /// `session` — `String`.
    pub session: String,
    /// `harness` — `String`.
    pub harness: String,
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
    /// `activity` — `String`.
    pub activity: String,
}

/// StoreMigrate — `conductor.observation.StoreMigrate`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreMigrate {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// The states of `conductor.observation.UsageObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `UsageObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageObservationState {
    /// `Recorded`.
    Recorded,
}

/// ValidationResult — `conductor.observation.ValidationResult`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationResult {
    /// `Valid`.
    Valid,
    /// `Refused`.
    Refused,
    /// `NotRun`.
    NotRun,
}

/// Visibility — `conductor.observation.Visibility`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// `Public`.
    Public,
    /// `Private`.
    Private,
}

/// WatchRun — `conductor.observation.WatchRun`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchRun {
    /// `every` — `Optional<Integer>`.
    pub every: Option<i64>,
    /// `ci_every` — `Optional<Integer>`.
    pub ci_every: Option<i64>,
    /// `follow` — `Optional<Boolean>`.
    pub follow: Option<bool>,
}

/// The states of `conductor.observation.WorkflowRunObservation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `WorkflowRunObservation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowRunObservationState {
    /// `Recorded`.
    Recorded,
}

/// What BlockerObservation — `conductor.observation.BlockerObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`BlockerObservation<S>`], and at a boundary by [`BlockerObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.BlockerObservation` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `reference` — `String`.
    pub reference: String,
    /// `blocker_kind` — `String`.
    pub blocker_kind: String,
    /// `title` — `String`.
    pub title: String,
}

/// The states of `conductor.observation.BlockerObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](blocker_observation_state::Marker), so [`BlockerObservation<S>`](BlockerObservation) can only ever rest in a real state.
pub mod blocker_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `BlockerObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::BlockerObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::BlockerObservationState = super::BlockerObservationState::Recorded;
    }
}

/// BlockerObservation — `conductor.observation.BlockerObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`BlockerObservationSnapshot`]
/// and [`BlockerObservationSnapshot::refine`].
pub struct BlockerObservation<S: blocker_observation_state::Marker> {
    data: BlockerObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: blocker_observation_state::Marker> BlockerObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> BlockerObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &BlockerObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> BlockerObservationData {
        self.data
    }
}

impl BlockerObservation<blocker_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: BlockerObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.BlockerObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`BlockerObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: BlockerObservationState,
    /// What it holds.
    pub data: BlockerObservationData,
}

/// An `BlockerObservation` in whichever declared state it was found.
pub enum AnyBlockerObservation {
    /// Resting in `Recorded`.
    Recorded(BlockerObservation<blocker_observation_state::Recorded>),
}

impl BlockerObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `BlockerObservationState` cannot spell one.
    pub fn refine(self) -> AnyBlockerObservation {
        match self.state {
            BlockerObservationState::Recorded => AnyBlockerObservation::Recorded(BlockerObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyBlockerObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> BlockerObservationState {
        match self {
            Self::Recorded(_) => BlockerObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> BlockerObservationSnapshot {
        match self {
            Self::Recorded(instance) => BlockerObservationSnapshot {
                state: BlockerObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Board — `conductor.observation.Board` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Board<S>`], and at a boundary by [`BoardSnapshot::state`].
///
/// Every value satisfies `deviation_rows >= 0` — checked by [`BoardData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardData {
    /// The identity: `board_id` — `conductor.observation.BoardId`.
    pub board_id: BoardId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.Board` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
    /// `previous_snapshot_id` — `Optional<conductor.observation.SnapshotId>`.
    ///
    /// Carries `previous_snapshot`: `conductor.observation.Board` references one `conductor.observation.Snapshot`.
    pub previous_snapshot_id: Option<SnapshotId>,
    /// `deviation_rows` — `Integer`.
    pub deviation_rows: i64,
    /// `path` — `String`.
    pub path: String,
}

impl BoardData {
    /// The first declared invariant of `conductor.observation.Board` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.deviation_rows)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("deviation_rows >= 0");
        }
        None
    }
}

/// The states of `conductor.observation.Board`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](board_state::Marker), so [`Board<S>`](Board) can only ever rest in a real state.
pub mod board_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Published {}
    }

    /// A declared state of `Board`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::BoardState;
    }

    /// `Published`. Where a new instance starts.
    pub struct Published;

    impl Marker for Published {
        const STATE: super::BoardState = super::BoardState::Published;
    }
}

/// Board — `conductor.observation.Board` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Published`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`BoardSnapshot`]
/// and [`BoardSnapshot::refine`].
pub struct Board<S: board_state::Marker> {
    data: BoardData,
    state: core::marker::PhantomData<S>,
}

impl<S: board_state::Marker> Board<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> BoardState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &BoardData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> BoardData {
        self.data
    }
}

impl Board<board_state::Published> {
    /// A new instance, resting in `Published` — the only state the lifecycle starts one in.
    pub fn new(data: BoardData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.Board` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`BoardSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: BoardState,
    /// What it holds.
    pub data: BoardData,
}

/// An `Board` in whichever declared state it was found.
pub enum AnyBoard {
    /// Resting in `Published`.
    Published(Board<board_state::Published>),
}

impl BoardSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `BoardState` cannot spell one.
    pub fn refine(self) -> AnyBoard {
        match self.state {
            BoardState::Published => AnyBoard::Published(Board {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyBoard {
    /// The state, as the runtime value.
    pub fn state(&self) -> BoardState {
        match self {
            Self::Published(_) => BoardState::Published,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> BoardSnapshot {
        match self {
            Self::Published(instance) => BoardSnapshot {
                state: BoardState::Published,
                data: instance.into_data(),
            },
        }
    }
}

/// What CollectorRunObservation — `conductor.observation.CollectorRunObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`CollectorRunObservation<S>`], and at a boundary by [`CollectorRunObservationSnapshot::state`].
///
/// Every value satisfies `duration_ms >= 0` — checked by [`CollectorRunObservationData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorRunObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.CollectorRunObservation` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
    /// `collector` — `String`.
    pub collector: String,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `duration_ms` — `Integer`.
    pub duration_ms: i64,
    /// `outcome` — `conductor.observation.CollectorRunOutcome`.
    pub outcome: CollectorRunOutcome,
}

impl CollectorRunObservationData {
    /// The first declared invariant of `conductor.observation.CollectorRunObservation` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.duration_ms)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("duration_ms >= 0");
        }
        None
    }
}

/// The states of `conductor.observation.CollectorRunObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](collector_run_observation_state::Marker), so [`CollectorRunObservation<S>`](CollectorRunObservation) can only ever rest in a real state.
pub mod collector_run_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `CollectorRunObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::CollectorRunObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::CollectorRunObservationState = super::CollectorRunObservationState::Recorded;
    }
}

/// CollectorRunObservation — `conductor.observation.CollectorRunObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`CollectorRunObservationSnapshot`]
/// and [`CollectorRunObservationSnapshot::refine`].
pub struct CollectorRunObservation<S: collector_run_observation_state::Marker> {
    data: CollectorRunObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: collector_run_observation_state::Marker> CollectorRunObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> CollectorRunObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &CollectorRunObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> CollectorRunObservationData {
        self.data
    }
}

impl CollectorRunObservation<collector_run_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: CollectorRunObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.CollectorRunObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`CollectorRunObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorRunObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: CollectorRunObservationState,
    /// What it holds.
    pub data: CollectorRunObservationData,
}

/// An `CollectorRunObservation` in whichever declared state it was found.
pub enum AnyCollectorRunObservation {
    /// Resting in `Recorded`.
    Recorded(CollectorRunObservation<collector_run_observation_state::Recorded>),
}

impl CollectorRunObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `CollectorRunObservationState` cannot spell one.
    pub fn refine(self) -> AnyCollectorRunObservation {
        match self.state {
            CollectorRunObservationState::Recorded => AnyCollectorRunObservation::Recorded(CollectorRunObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyCollectorRunObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> CollectorRunObservationState {
        match self {
            Self::Recorded(_) => CollectorRunObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> CollectorRunObservationSnapshot {
        match self {
            Self::Recorded(instance) => CollectorRunObservationSnapshot {
                state: CollectorRunObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What DailySeries — `conductor.observation.DailySeries` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`DailySeries<S>`], and at a boundary by [`DailySeriesSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySeriesData {
    /// The identity: `daily_series_id` — `conductor.observation.DailySeriesId`.
    pub daily_series_id: DailySeriesId,
    /// `series` — `String`.
    pub series: String,
    /// `day` — `String`.
    pub day: String,
    /// `by` — `String`.
    pub by: String,
    /// `key` — `String`.
    pub key: String,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
    /// `items` — `List<String>`.
    pub items: Vec<String>,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.DailySeries` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
}

/// The states of `conductor.observation.DailySeries`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](daily_series_state::Marker), so [`DailySeries<S>`](DailySeries) can only ever rest in a real state.
pub mod daily_series_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `DailySeries`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DailySeriesState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::DailySeriesState = super::DailySeriesState::Recorded;
    }
}

/// DailySeries — `conductor.observation.DailySeries` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DailySeriesSnapshot`]
/// and [`DailySeriesSnapshot::refine`].
pub struct DailySeries<S: daily_series_state::Marker> {
    data: DailySeriesData,
    state: core::marker::PhantomData<S>,
}

impl<S: daily_series_state::Marker> DailySeries<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DailySeriesState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DailySeriesData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DailySeriesData {
        self.data
    }
}

impl DailySeries<daily_series_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: DailySeriesData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.DailySeries` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DailySeriesSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySeriesSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DailySeriesState,
    /// What it holds.
    pub data: DailySeriesData,
}

/// An `DailySeries` in whichever declared state it was found.
pub enum AnyDailySeries {
    /// Resting in `Recorded`.
    Recorded(DailySeries<daily_series_state::Recorded>),
}

impl DailySeriesSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DailySeriesState` cannot spell one.
    pub fn refine(self) -> AnyDailySeries {
        match self.state {
            DailySeriesState::Recorded => AnyDailySeries::Recorded(DailySeries {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDailySeries {
    /// The state, as the runtime value.
    pub fn state(&self) -> DailySeriesState {
        match self {
            Self::Recorded(_) => DailySeriesState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DailySeriesSnapshot {
        match self {
            Self::Recorded(instance) => DailySeriesSnapshot {
                state: DailySeriesState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What MemberFailureObservation — `conductor.observation.MemberFailureObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`MemberFailureObservation<S>`], and at a boundary by [`MemberFailureObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFailureObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.MemberFailureObservation` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
    /// `member` — `conductor.observation.RepositoryName`.
    pub member: RepositoryName,
    /// `collector` — `String`.
    pub collector: String,
    /// `error_line` — `String`.
    pub error_line: String,
}

/// The states of `conductor.observation.MemberFailureObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](member_failure_observation_state::Marker), so [`MemberFailureObservation<S>`](MemberFailureObservation) can only ever rest in a real state.
pub mod member_failure_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `MemberFailureObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MemberFailureObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::MemberFailureObservationState = super::MemberFailureObservationState::Recorded;
    }
}

/// MemberFailureObservation — `conductor.observation.MemberFailureObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MemberFailureObservationSnapshot`]
/// and [`MemberFailureObservationSnapshot::refine`].
pub struct MemberFailureObservation<S: member_failure_observation_state::Marker> {
    data: MemberFailureObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: member_failure_observation_state::Marker> MemberFailureObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MemberFailureObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MemberFailureObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MemberFailureObservationData {
        self.data
    }
}

impl MemberFailureObservation<member_failure_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: MemberFailureObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.MemberFailureObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MemberFailureObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFailureObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MemberFailureObservationState,
    /// What it holds.
    pub data: MemberFailureObservationData,
}

/// An `MemberFailureObservation` in whichever declared state it was found.
pub enum AnyMemberFailureObservation {
    /// Resting in `Recorded`.
    Recorded(MemberFailureObservation<member_failure_observation_state::Recorded>),
}

impl MemberFailureObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MemberFailureObservationState` cannot spell one.
    pub fn refine(self) -> AnyMemberFailureObservation {
        match self.state {
            MemberFailureObservationState::Recorded => AnyMemberFailureObservation::Recorded(MemberFailureObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMemberFailureObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> MemberFailureObservationState {
        match self {
            Self::Recorded(_) => MemberFailureObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MemberFailureObservationSnapshot {
        match self {
            Self::Recorded(instance) => MemberFailureObservationSnapshot {
                state: MemberFailureObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What MergedPullRequestObservation — `conductor.observation.MergedPullRequestObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`MergedPullRequestObservation<S>`], and at a boundary by [`MergedPullRequestObservationSnapshot::state`].
///
/// Every value satisfies `number > 0` — checked by [`MergedPullRequestObservationData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedPullRequestObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `merged_pull_requests`: `conductor.observation.Snapshot` owns many `conductor.observation.MergedPullRequestObservation`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
    /// `title` — `String`.
    pub title: String,
    /// `merged_at` — `Timestamp`.
    pub merged_at: crate::primitives::Timestamp,
    /// `url` — `String`.
    pub url: String,
}

impl MergedPullRequestObservationData {
    /// The first declared invariant of `conductor.observation.MergedPullRequestObservation` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.number)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("number > 0");
        }
        None
    }
}

/// The states of `conductor.observation.MergedPullRequestObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](merged_pull_request_observation_state::Marker), so [`MergedPullRequestObservation<S>`](MergedPullRequestObservation) can only ever rest in a real state.
pub mod merged_pull_request_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `MergedPullRequestObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MergedPullRequestObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::MergedPullRequestObservationState = super::MergedPullRequestObservationState::Recorded;
    }
}

/// MergedPullRequestObservation — `conductor.observation.MergedPullRequestObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MergedPullRequestObservationSnapshot`]
/// and [`MergedPullRequestObservationSnapshot::refine`].
pub struct MergedPullRequestObservation<S: merged_pull_request_observation_state::Marker> {
    data: MergedPullRequestObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: merged_pull_request_observation_state::Marker> MergedPullRequestObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MergedPullRequestObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MergedPullRequestObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MergedPullRequestObservationData {
        self.data
    }
}

impl MergedPullRequestObservation<merged_pull_request_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: MergedPullRequestObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.MergedPullRequestObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MergedPullRequestObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedPullRequestObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MergedPullRequestObservationState,
    /// What it holds.
    pub data: MergedPullRequestObservationData,
}

/// An `MergedPullRequestObservation` in whichever declared state it was found.
pub enum AnyMergedPullRequestObservation {
    /// Resting in `Recorded`.
    Recorded(MergedPullRequestObservation<merged_pull_request_observation_state::Recorded>),
}

impl MergedPullRequestObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MergedPullRequestObservationState` cannot spell one.
    pub fn refine(self) -> AnyMergedPullRequestObservation {
        match self.state {
            MergedPullRequestObservationState::Recorded => AnyMergedPullRequestObservation::Recorded(MergedPullRequestObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMergedPullRequestObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> MergedPullRequestObservationState {
        match self {
            Self::Recorded(_) => MergedPullRequestObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MergedPullRequestObservationSnapshot {
        match self {
            Self::Recorded(instance) => MergedPullRequestObservationSnapshot {
                state: MergedPullRequestObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What PullRequestObservation — `conductor.observation.PullRequestObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`PullRequestObservation<S>`], and at a boundary by [`PullRequestObservationSnapshot::state`].
///
/// Every value satisfies `number > 0` — checked by [`PullRequestObservationData::broken_invariant`].
/// Every value satisfies `failing_checks >= 0` — checked by [`PullRequestObservationData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `pull_requests`: `conductor.observation.Snapshot` owns many `conductor.observation.PullRequestObservation`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
    /// `title` — `String`.
    pub title: String,
    /// `draft` — `Boolean`.
    pub draft: bool,
    /// `mergeability` — `conductor.observation.Mergeability`.
    pub mergeability: Mergeability,
    /// `failing_checks` — `Integer`.
    pub failing_checks: i64,
    /// `failing_check_names` — `List<String>`.
    pub failing_check_names: Vec<String>,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
    /// `updated_at` — `Timestamp`.
    pub updated_at: crate::primitives::Timestamp,
    /// `url` — `Optional<String>`.
    pub url: Option<String>,
}

impl PullRequestObservationData {
    /// The first declared invariant of `conductor.observation.PullRequestObservation` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.number)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("number > 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.failing_checks)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("failing_checks >= 0");
        }
        None
    }
}

/// The states of `conductor.observation.PullRequestObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](pull_request_observation_state::Marker), so [`PullRequestObservation<S>`](PullRequestObservation) can only ever rest in a real state.
pub mod pull_request_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `PullRequestObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::PullRequestObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::PullRequestObservationState = super::PullRequestObservationState::Recorded;
    }
}

/// PullRequestObservation — `conductor.observation.PullRequestObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`PullRequestObservationSnapshot`]
/// and [`PullRequestObservationSnapshot::refine`].
pub struct PullRequestObservation<S: pull_request_observation_state::Marker> {
    data: PullRequestObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: pull_request_observation_state::Marker> PullRequestObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> PullRequestObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &PullRequestObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> PullRequestObservationData {
        self.data
    }
}

impl PullRequestObservation<pull_request_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: PullRequestObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.PullRequestObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`PullRequestObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: PullRequestObservationState,
    /// What it holds.
    pub data: PullRequestObservationData,
}

/// An `PullRequestObservation` in whichever declared state it was found.
pub enum AnyPullRequestObservation {
    /// Resting in `Recorded`.
    Recorded(PullRequestObservation<pull_request_observation_state::Recorded>),
}

impl PullRequestObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `PullRequestObservationState` cannot spell one.
    pub fn refine(self) -> AnyPullRequestObservation {
        match self.state {
            PullRequestObservationState::Recorded => AnyPullRequestObservation::Recorded(PullRequestObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyPullRequestObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> PullRequestObservationState {
        match self {
            Self::Recorded(_) => PullRequestObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> PullRequestObservationSnapshot {
        match self {
            Self::Recorded(instance) => PullRequestObservationSnapshot {
                state: PullRequestObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ReleaseObservation — `conductor.observation.ReleaseObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ReleaseObservation<S>`], and at a boundary by [`ReleaseObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `releases`: `conductor.observation.Snapshot` owns many `conductor.observation.ReleaseObservation`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `tag` — `String`.
    pub tag: String,
    /// `published_at` — `Timestamp`.
    pub published_at: crate::primitives::Timestamp,
    /// `url` — `String`.
    pub url: String,
}

/// The states of `conductor.observation.ReleaseObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](release_observation_state::Marker), so [`ReleaseObservation<S>`](ReleaseObservation) can only ever rest in a real state.
pub mod release_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ReleaseObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ReleaseObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ReleaseObservationState = super::ReleaseObservationState::Recorded;
    }
}

/// ReleaseObservation — `conductor.observation.ReleaseObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ReleaseObservationSnapshot`]
/// and [`ReleaseObservationSnapshot::refine`].
pub struct ReleaseObservation<S: release_observation_state::Marker> {
    data: ReleaseObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: release_observation_state::Marker> ReleaseObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ReleaseObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ReleaseObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ReleaseObservationData {
        self.data
    }
}

impl ReleaseObservation<release_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ReleaseObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.ReleaseObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ReleaseObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ReleaseObservationState,
    /// What it holds.
    pub data: ReleaseObservationData,
}

/// An `ReleaseObservation` in whichever declared state it was found.
pub enum AnyReleaseObservation {
    /// Resting in `Recorded`.
    Recorded(ReleaseObservation<release_observation_state::Recorded>),
}

impl ReleaseObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ReleaseObservationState` cannot spell one.
    pub fn refine(self) -> AnyReleaseObservation {
        match self.state {
            ReleaseObservationState::Recorded => AnyReleaseObservation::Recorded(ReleaseObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyReleaseObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> ReleaseObservationState {
        match self {
            Self::Recorded(_) => ReleaseObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ReleaseObservationSnapshot {
        match self {
            Self::Recorded(instance) => ReleaseObservationSnapshot {
                state: ReleaseObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What RepositoryObservation — `conductor.observation.RepositoryObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`RepositoryObservation<S>`], and at a boundary by [`RepositoryObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `repositories`: `conductor.observation.Snapshot` owns many `conductor.observation.RepositoryObservation`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `visibility` — `conductor.observation.Visibility`.
    pub visibility: Visibility,
    /// `archived` — `Boolean`.
    pub archived: bool,
    /// `local_checkout` — `Boolean`.
    pub local_checkout: bool,
    /// `main_head` — `conductor.observation.CommitSha`.
    pub main_head: CommitSha,
    /// `main_committed_at` — `Timestamp`.
    pub main_committed_at: crate::primitives::Timestamp,
    /// `main_commits_7d` — `Integer`.
    pub main_commits_7d: i64,
    /// `real_commits_7d` — `Integer`.
    pub real_commits_7d: i64,
    /// `behind_main` — `Integer`.
    pub behind_main: i64,
    /// `dirty_files` — `Integer`.
    pub dirty_files: i64,
    /// `worktrees` — `Integer`.
    pub worktrees: i64,
    /// `open_issues` — `Integer`.
    pub open_issues: i64,
    /// `oldest_open_issue_at` — `Optional<Timestamp>`.
    pub oldest_open_issue_at: Option<crate::primitives::Timestamp>,
    /// `latest_release` — `Optional<String>`.
    pub latest_release: Option<String>,
    /// `latest_release_at` — `Optional<Timestamp>`.
    pub latest_release_at: Option<crate::primitives::Timestamp>,
    /// `unreleased_commits` — `Optional<Integer>`.
    pub unreleased_commits: Option<i64>,
    /// `planning_store_version` — `Optional<String>`.
    pub planning_store_version: Option<String>,
    /// `in_catalog` — `Optional<Boolean>`.
    pub in_catalog: Option<bool>,
    /// `gates_hooks_version` — `Optional<String>`.
    pub gates_hooks_version: Option<String>,
    /// `gates_version` — `Optional<String>`.
    pub gates_version: Option<String>,
    /// `wave_width` — `Optional<Integer>`.
    pub wave_width: Option<i64>,
    /// `wave_ready` — `Optional<Integer>`.
    pub wave_ready: Option<i64>,
    /// `wave_reason` — `Optional<String>`.
    pub wave_reason: Option<String>,
}

/// The states of `conductor.observation.RepositoryObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](repository_observation_state::Marker), so [`RepositoryObservation<S>`](RepositoryObservation) can only ever rest in a real state.
pub mod repository_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `RepositoryObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::RepositoryObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::RepositoryObservationState = super::RepositoryObservationState::Recorded;
    }
}

/// RepositoryObservation — `conductor.observation.RepositoryObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`RepositoryObservationSnapshot`]
/// and [`RepositoryObservationSnapshot::refine`].
pub struct RepositoryObservation<S: repository_observation_state::Marker> {
    data: RepositoryObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: repository_observation_state::Marker> RepositoryObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> RepositoryObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &RepositoryObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> RepositoryObservationData {
        self.data
    }
}

impl RepositoryObservation<repository_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: RepositoryObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.RepositoryObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`RepositoryObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: RepositoryObservationState,
    /// What it holds.
    pub data: RepositoryObservationData,
}

/// An `RepositoryObservation` in whichever declared state it was found.
pub enum AnyRepositoryObservation {
    /// Resting in `Recorded`.
    Recorded(RepositoryObservation<repository_observation_state::Recorded>),
}

impl RepositoryObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `RepositoryObservationState` cannot spell one.
    pub fn refine(self) -> AnyRepositoryObservation {
        match self.state {
            RepositoryObservationState::Recorded => AnyRepositoryObservation::Recorded(RepositoryObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyRepositoryObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> RepositoryObservationState {
        match self {
            Self::Recorded(_) => RepositoryObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> RepositoryObservationSnapshot {
        match self {
            Self::Recorded(instance) => RepositoryObservationSnapshot {
                state: RepositoryObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What SecretSyncObservation — `conductor.observation.SecretSyncObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SecretSyncObservation<S>`], and at a boundary by [`SecretSyncObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSyncObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.SecretSyncObservation` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
    /// `sync` — `String`.
    pub sync: String,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `branch` — `String`.
    pub branch: String,
    /// `path` — `String`.
    pub path: String,
    /// `source_committed_at` — `Optional<Timestamp>`.
    pub source_committed_at: Option<crate::primitives::Timestamp>,
    /// `secrets_updated_at` — `Optional<Timestamp>`.
    pub secrets_updated_at: Option<crate::primitives::Timestamp>,
    /// `error_line` — `Optional<String>`.
    pub error_line: Option<String>,
}

/// The states of `conductor.observation.SecretSyncObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](secret_sync_observation_state::Marker), so [`SecretSyncObservation<S>`](SecretSyncObservation) can only ever rest in a real state.
pub mod secret_sync_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `SecretSyncObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SecretSyncObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::SecretSyncObservationState = super::SecretSyncObservationState::Recorded;
    }
}

/// SecretSyncObservation — `conductor.observation.SecretSyncObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SecretSyncObservationSnapshot`]
/// and [`SecretSyncObservationSnapshot::refine`].
pub struct SecretSyncObservation<S: secret_sync_observation_state::Marker> {
    data: SecretSyncObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: secret_sync_observation_state::Marker> SecretSyncObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SecretSyncObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SecretSyncObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SecretSyncObservationData {
        self.data
    }
}

impl SecretSyncObservation<secret_sync_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: SecretSyncObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.SecretSyncObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SecretSyncObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSyncObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SecretSyncObservationState,
    /// What it holds.
    pub data: SecretSyncObservationData,
}

/// An `SecretSyncObservation` in whichever declared state it was found.
pub enum AnySecretSyncObservation {
    /// Resting in `Recorded`.
    Recorded(SecretSyncObservation<secret_sync_observation_state::Recorded>),
}

impl SecretSyncObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SecretSyncObservationState` cannot spell one.
    pub fn refine(self) -> AnySecretSyncObservation {
        match self.state {
            SecretSyncObservationState::Recorded => AnySecretSyncObservation::Recorded(SecretSyncObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySecretSyncObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> SecretSyncObservationState {
        match self {
            Self::Recorded(_) => SecretSyncObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SecretSyncObservationSnapshot {
        match self {
            Self::Recorded(instance) => SecretSyncObservationSnapshot {
                state: SecretSyncObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What SessionObservation — `conductor.observation.SessionObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SessionObservation<S>`], and at a boundary by [`SessionObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `sessions`: `conductor.observation.Snapshot` owns many `conductor.observation.SessionObservation`.
    pub snapshot_id: SnapshotId,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: Harness,
    /// `session_ref` — `String`.
    pub session_ref: String,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `cwd` — `String`.
    pub cwd: String,
    /// `repository` — `Optional<conductor.observation.RepositoryName>`.
    pub repository: Option<RepositoryName>,
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
    /// `activity` — `conductor.observation.SessionState`.
    pub activity: SessionState,
    /// `envelope_provider` — `Optional<conductor.observation.EnvelopeProvider>`.
    pub envelope_provider: Option<EnvelopeProvider>,
    /// `envelope_cgroup` — `Optional<String>`.
    pub envelope_cgroup: Option<String>,
    /// `envelope_memory_max` — `Optional<Integer>`.
    pub envelope_memory_max: Option<i64>,
    /// `envelope_cpu_weight` — `Optional<Integer>`.
    pub envelope_cpu_weight: Option<i64>,
    /// `envelope_tasks_max` — `Optional<Integer>`.
    pub envelope_tasks_max: Option<i64>,
    /// `envelope_writable` — `Optional<String>`.
    pub envelope_writable: Option<String>,
    /// `envelope_unboxed` — `Optional<String>`.
    pub envelope_unboxed: Option<String>,
}

/// The states of `conductor.observation.SessionObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](session_observation_state::Marker), so [`SessionObservation<S>`](SessionObservation) can only ever rest in a real state.
pub mod session_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `SessionObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SessionObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::SessionObservationState = super::SessionObservationState::Recorded;
    }
}

/// SessionObservation — `conductor.observation.SessionObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SessionObservationSnapshot`]
/// and [`SessionObservationSnapshot::refine`].
pub struct SessionObservation<S: session_observation_state::Marker> {
    data: SessionObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: session_observation_state::Marker> SessionObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SessionObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SessionObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SessionObservationData {
        self.data
    }
}

impl SessionObservation<session_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: SessionObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.SessionObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SessionObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SessionObservationState,
    /// What it holds.
    pub data: SessionObservationData,
}

/// An `SessionObservation` in whichever declared state it was found.
pub enum AnySessionObservation {
    /// Resting in `Recorded`.
    Recorded(SessionObservation<session_observation_state::Recorded>),
}

impl SessionObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SessionObservationState` cannot spell one.
    pub fn refine(self) -> AnySessionObservation {
        match self.state {
            SessionObservationState::Recorded => AnySessionObservation::Recorded(SessionObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySessionObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> SessionObservationState {
        match self {
            Self::Recorded(_) => SessionObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SessionObservationSnapshot {
        match self {
            Self::Recorded(instance) => SessionObservationSnapshot {
                state: SessionObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Snapshot — `conductor.observation.Snapshot` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Snapshot<S>`], and at a boundary by [`SnapshotSnapshot::state`].
///
/// Every value satisfies `disk_free_bytes >= 0` — checked by [`SnapshotData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotData {
    /// The identity: `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `disk_free_bytes` — `Integer`.
    pub disk_free_bytes: i64,
    /// `failure_reason` — `Optional<String>`.
    pub failure_reason: Option<String>,
}

impl SnapshotData {
    /// The first declared invariant of `conductor.observation.Snapshot` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.disk_free_bytes)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("disk_free_bytes >= 0");
        }
        None
    }
}

/// The states of `conductor.observation.Snapshot`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](snapshot_state::Marker), so [`Snapshot<S>`](Snapshot) can only ever rest in a real state.
pub mod snapshot_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Collecting {}
        impl Sealed for super::Complete {}
        impl Sealed for super::Failed {}
    }

    /// A declared state of `Snapshot`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SnapshotState;
    }

    /// `Collecting`. Where a new instance starts.
    pub struct Collecting;

    impl Marker for Collecting {
        const STATE: super::SnapshotState = super::SnapshotState::Collecting;
    }

    /// `Complete`. Terminal: an instance may rest here forever.
    pub struct Complete;

    impl Marker for Complete {
        const STATE: super::SnapshotState = super::SnapshotState::Complete;
    }

    /// `Failed`. Terminal: an instance may rest here forever.
    pub struct Failed;

    impl Marker for Failed {
        const STATE: super::SnapshotState = super::SnapshotState::Failed;
    }
}

/// Snapshot — `conductor.observation.Snapshot` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Collecting`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SnapshotSnapshot`]
/// and [`SnapshotSnapshot::refine`].
pub struct Snapshot<S: snapshot_state::Marker> {
    data: SnapshotData,
    state: core::marker::PhantomData<S>,
}

impl<S: snapshot_state::Marker> Snapshot<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SnapshotState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SnapshotData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SnapshotData {
        self.data
    }
}

impl Snapshot<snapshot_state::Collecting> {
    /// A new instance, resting in `Collecting` — the only state the lifecycle starts one in.
    pub fn new(data: SnapshotData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Snapshot<snapshot_state::Collecting> {
    /// `complete` — `Collecting` → `Complete`. Taken by the `completed` outcome of `conductor.observation.CompleteSnapshot`.
    pub fn complete(self) -> Snapshot<snapshot_state::Complete> {
        Snapshot {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `fail` — `Collecting` → `Failed`. Taken by the `failed` outcome of `conductor.observation.FailSnapshot`.
    pub fn fail(self) -> Snapshot<snapshot_state::Failed> {
        Snapshot {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.Snapshot` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SnapshotSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SnapshotState,
    /// What it holds.
    pub data: SnapshotData,
}

/// An `Snapshot` in whichever declared state it was found.
pub enum AnySnapshot {
    /// Resting in `Collecting`.
    Collecting(Snapshot<snapshot_state::Collecting>),
    /// Resting in `Complete`.
    Complete(Snapshot<snapshot_state::Complete>),
    /// Resting in `Failed`.
    Failed(Snapshot<snapshot_state::Failed>),
}

impl SnapshotSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SnapshotState` cannot spell one.
    pub fn refine(self) -> AnySnapshot {
        match self.state {
            SnapshotState::Collecting => AnySnapshot::Collecting(Snapshot {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SnapshotState::Complete => AnySnapshot::Complete(Snapshot {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            SnapshotState::Failed => AnySnapshot::Failed(Snapshot {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySnapshot {
    /// The state, as the runtime value.
    pub fn state(&self) -> SnapshotState {
        match self {
            Self::Collecting(_) => SnapshotState::Collecting,
            Self::Complete(_) => SnapshotState::Complete,
            Self::Failed(_) => SnapshotState::Failed,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SnapshotSnapshot {
        match self {
            Self::Collecting(instance) => SnapshotSnapshot {
                state: SnapshotState::Collecting,
                data: instance.into_data(),
            },
            Self::Complete(instance) => SnapshotSnapshot {
                state: SnapshotState::Complete,
                data: instance.into_data(),
            },
            Self::Failed(instance) => SnapshotSnapshot {
                state: SnapshotState::Failed,
                data: instance.into_data(),
            },
        }
    }
}

/// What SpecificationObservation — `conductor.observation.SpecificationObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`SpecificationObservation<S>`], and at a boundary by [`SpecificationObservationSnapshot::state`].
///
/// Every value satisfies `validation_refusals >= 0` — checked by [`SpecificationObservationData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecificationObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.SpecificationObservation` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `presence` — `conductor.observation.SpecificationPresence`.
    pub presence: SpecificationPresence,
    /// `path` — `Optional<String>`.
    pub path: Option<String>,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
    /// `required_ess` — `Optional<String>`.
    pub required_ess: Option<String>,
    /// `validation` — `conductor.observation.ValidationResult`.
    pub validation: ValidationResult,
    /// `validation_refusals` — `Integer`.
    pub validation_refusals: i64,
    /// `scenarios` — `Optional<Integer>`.
    pub scenarios: Option<i64>,
    /// `synthesis_refusals` — `Optional<Integer>`.
    pub synthesis_refusals: Option<i64>,
    /// `conformance_status` — `Optional<String>`.
    pub conformance_status: Option<String>,
}

impl SpecificationObservationData {
    /// The first declared invariant of `conductor.observation.SpecificationObservation` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.validation_refusals)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("validation_refusals >= 0");
        }
        None
    }
}

/// The states of `conductor.observation.SpecificationObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](specification_observation_state::Marker), so [`SpecificationObservation<S>`](SpecificationObservation) can only ever rest in a real state.
pub mod specification_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `SpecificationObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SpecificationObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::SpecificationObservationState = super::SpecificationObservationState::Recorded;
    }
}

/// SpecificationObservation — `conductor.observation.SpecificationObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SpecificationObservationSnapshot`]
/// and [`SpecificationObservationSnapshot::refine`].
pub struct SpecificationObservation<S: specification_observation_state::Marker> {
    data: SpecificationObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: specification_observation_state::Marker> SpecificationObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SpecificationObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SpecificationObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SpecificationObservationData {
        self.data
    }
}

impl SpecificationObservation<specification_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: SpecificationObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.SpecificationObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SpecificationObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecificationObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SpecificationObservationState,
    /// What it holds.
    pub data: SpecificationObservationData,
}

/// An `SpecificationObservation` in whichever declared state it was found.
pub enum AnySpecificationObservation {
    /// Resting in `Recorded`.
    Recorded(SpecificationObservation<specification_observation_state::Recorded>),
}

impl SpecificationObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SpecificationObservationState` cannot spell one.
    pub fn refine(self) -> AnySpecificationObservation {
        match self.state {
            SpecificationObservationState::Recorded => AnySpecificationObservation::Recorded(SpecificationObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySpecificationObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> SpecificationObservationState {
        match self {
            Self::Recorded(_) => SpecificationObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SpecificationObservationSnapshot {
        match self {
            Self::Recorded(instance) => SpecificationObservationSnapshot {
                state: SpecificationObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What UsageObservation — `conductor.observation.UsageObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`UsageObservation<S>`], and at a boundary by [`UsageObservationSnapshot::state`].
///
/// Every value satisfies `used_percent >= 0` — checked by [`UsageObservationData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `snapshot`: `conductor.observation.UsageObservation` references one `conductor.observation.Snapshot`.
    pub snapshot_id: SnapshotId,
    /// `read_at` — `Timestamp`.
    pub read_at: crate::primitives::Timestamp,
    /// `account` — `String`.
    pub account: String,
    /// `active` — `Boolean`.
    pub active: bool,
    /// `health` — `String`.
    pub health: String,
    /// `window` — `String`.
    pub window: String,
    /// `used_percent` — `Integer`.
    pub used_percent: i64,
    /// `resets_at` — `Optional<Timestamp>`.
    pub resets_at: Option<crate::primitives::Timestamp>,
}

impl UsageObservationData {
    /// The first declared invariant of `conductor.observation.UsageObservation` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.used_percent)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("used_percent >= 0");
        }
        None
    }
}

/// The states of `conductor.observation.UsageObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](usage_observation_state::Marker), so [`UsageObservation<S>`](UsageObservation) can only ever rest in a real state.
pub mod usage_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `UsageObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::UsageObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::UsageObservationState = super::UsageObservationState::Recorded;
    }
}

/// UsageObservation — `conductor.observation.UsageObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`UsageObservationSnapshot`]
/// and [`UsageObservationSnapshot::refine`].
pub struct UsageObservation<S: usage_observation_state::Marker> {
    data: UsageObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: usage_observation_state::Marker> UsageObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> UsageObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &UsageObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> UsageObservationData {
        self.data
    }
}

impl UsageObservation<usage_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: UsageObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.UsageObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`UsageObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: UsageObservationState,
    /// What it holds.
    pub data: UsageObservationData,
}

/// An `UsageObservation` in whichever declared state it was found.
pub enum AnyUsageObservation {
    /// Resting in `Recorded`.
    Recorded(UsageObservation<usage_observation_state::Recorded>),
}

impl UsageObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `UsageObservationState` cannot spell one.
    pub fn refine(self) -> AnyUsageObservation {
        match self.state {
            UsageObservationState::Recorded => AnyUsageObservation::Recorded(UsageObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyUsageObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> UsageObservationState {
        match self {
            Self::Recorded(_) => UsageObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> UsageObservationSnapshot {
        match self {
            Self::Recorded(instance) => UsageObservationSnapshot {
                state: UsageObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What WorkflowRunObservation — `conductor.observation.WorkflowRunObservation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`WorkflowRunObservation<S>`], and at a boundary by [`WorkflowRunObservationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunObservationData {
    /// The identity: `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    ///
    /// Carries `workflow_runs`: `conductor.observation.Snapshot` owns many `conductor.observation.WorkflowRunObservation`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `workflow` — `String`.
    pub workflow: String,
    /// `conclusion` — `conductor.observation.RunConclusion`.
    pub conclusion: RunConclusion,
    /// `run_at` — `Timestamp`.
    pub run_at: crate::primitives::Timestamp,
}

/// The states of `conductor.observation.WorkflowRunObservation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](workflow_run_observation_state::Marker), so [`WorkflowRunObservation<S>`](WorkflowRunObservation) can only ever rest in a real state.
pub mod workflow_run_observation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `WorkflowRunObservation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::WorkflowRunObservationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::WorkflowRunObservationState = super::WorkflowRunObservationState::Recorded;
    }
}

/// WorkflowRunObservation — `conductor.observation.WorkflowRunObservation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`WorkflowRunObservationSnapshot`]
/// and [`WorkflowRunObservationSnapshot::refine`].
pub struct WorkflowRunObservation<S: workflow_run_observation_state::Marker> {
    data: WorkflowRunObservationData,
    state: core::marker::PhantomData<S>,
}

impl<S: workflow_run_observation_state::Marker> WorkflowRunObservation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> WorkflowRunObservationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &WorkflowRunObservationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> WorkflowRunObservationData {
        self.data
    }
}

impl WorkflowRunObservation<workflow_run_observation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: WorkflowRunObservationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.observation.WorkflowRunObservation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`WorkflowRunObservationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunObservationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: WorkflowRunObservationState,
    /// What it holds.
    pub data: WorkflowRunObservationData,
}

/// An `WorkflowRunObservation` in whichever declared state it was found.
pub enum AnyWorkflowRunObservation {
    /// Resting in `Recorded`.
    Recorded(WorkflowRunObservation<workflow_run_observation_state::Recorded>),
}

impl WorkflowRunObservationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `WorkflowRunObservationState` cannot spell one.
    pub fn refine(self) -> AnyWorkflowRunObservation {
        match self.state {
            WorkflowRunObservationState::Recorded => AnyWorkflowRunObservation::Recorded(WorkflowRunObservation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyWorkflowRunObservation {
    /// The state, as the runtime value.
    pub fn state(&self) -> WorkflowRunObservationState {
        match self {
            Self::Recorded(_) => WorkflowRunObservationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> WorkflowRunObservationSnapshot {
        match self {
            Self::Recorded(instance) => WorkflowRunObservationSnapshot {
                state: WorkflowRunObservationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// Complete a snapshot — the input of `conductor.observation.CompleteSnapshot`.
///
/// Everything it can result in is [`CompleteSnapshotOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompleteSnapshot {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// Everything `conductor.observation.CompleteSnapshot` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompleteSnapshotOutcome {
    /// `completed` — otherwise.
    ///
    /// Every source answered; the snapshot is the newest complete one.
    Completed {
        /// The `conductor.observation.SnapshotCompleted` this outcome publishes.
        snapshot_completed: SnapshotCompleted,
    },
    /// `no-such-snapshot` — for an identity no record carries.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The snapshot was already complete or failed.
    WrongState {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
}

/// Fail a snapshot — the input of `conductor.observation.FailSnapshot`.
///
/// Everything it can result in is [`FailSnapshotOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailSnapshot {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.observation.FailSnapshot` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailSnapshotOutcome {
    /// `failed` — otherwise.
    ///
    /// A source did not answer; the snapshot is kept but never read as the current state.
    Failed {
        /// The `conductor.observation.SnapshotFailed` this outcome publishes.
        snapshot_failed: SnapshotFailed,
    },
    /// `no-such-snapshot` — for an identity no record carries.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The snapshot was already complete or failed.
    WrongState {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
}

/// Publish the board — the input of `conductor.observation.PublishBoard`.
///
/// Everything it can result in is [`PublishBoardOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishBoard {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `previous_snapshot_id` — `Optional<conductor.observation.SnapshotId>`.
    pub previous_snapshot_id: Option<SnapshotId>,
    /// `deviation_rows` — `Integer`.
    pub deviation_rows: i64,
    /// `path` — `String`.
    pub path: String,
}

/// Everything `conductor.observation.PublishBoard` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishBoardOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id; nothing was published.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `not-complete` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Complete`.
    ///
    /// Only a complete snapshot is published; a failed or collecting one never is.
    NotComplete {
        /// Why it was refused: `conductor.observation.SnapshotNotComplete`.
        error: SnapshotNotComplete,
    },
    /// `negative-rows` — when `deviation_rows < 0`.
    ///
    /// A row count is never negative; nothing was published.
    NegativeRows {
        /// Why it was refused: `conductor.observation.InvalidCount`.
        error: InvalidCount,
    },
    /// `published` — otherwise.
    ///
    /// STATUS.md was written from the snapshot, with its delta against the previous one.
    Published {
        /// The `conductor.observation.BoardPublished` this outcome publishes.
        board_published: BoardPublished,
    },
}

/// Record a blocker — the input of `conductor.observation.RecordBlocker`.
///
/// Everything it can result in is [`RecordBlockerOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordBlocker {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `reference` — `String`.
    pub reference: String,
    /// `blocker_kind` — `String`.
    pub blocker_kind: String,
    /// `title` — `String`.
    pub title: String,
}

/// Everything `conductor.observation.RecordBlocker` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordBlockerOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The open blocker is recorded with the repository whose store holds it.
    Recorded {
        /// The `conductor.observation.BlockerRecorded` this outcome publishes.
        blocker_recorded: BlockerRecorded,
    },
}

/// Record a collector's run — the input of `conductor.observation.RecordCollectorRun`.
///
/// Everything it can result in is [`RecordCollectorRunOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCollectorRun {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `collector` — `String`.
    pub collector: String,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `duration_ms` — `Integer`.
    pub duration_ms: i64,
    /// `outcome` — `conductor.observation.CollectorRunOutcome`.
    pub outcome: CollectorRunOutcome,
}

/// Everything `conductor.observation.RecordCollectorRun` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordCollectorRunOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `negative-duration` — when `duration_ms < 0`.
    ///
    /// A duration is never negative; nothing was recorded.
    NegativeDuration {
        /// Why it was refused: `conductor.observation.InvalidCount`.
        error: InvalidCount,
    },
    /// `recorded` — otherwise.
    ///
    /// The collector's run, its start, duration and outcome, is recorded.
    Recorded {
        /// The `conductor.observation.CollectorRunRecorded` this outcome publishes.
        collector_run_recorded: CollectorRunRecorded,
    },
}

/// Record a daily rollup — the input of `conductor.observation.RecordDailySeries`.
///
/// Everything it can result in is [`RecordDailySeriesOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordDailySeries {
    /// `daily_series_id` — `conductor.observation.DailySeriesId`.
    pub daily_series_id: DailySeriesId,
    /// `series` — `String`.
    pub series: String,
    /// `day` — `String`.
    pub day: String,
    /// `by` — `String`.
    pub by: String,
    /// `key` — `String`.
    pub key: String,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
    /// `items` — `List<String>`.
    pub items: Vec<String>,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// Everything `conductor.observation.RecordDailySeries` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordDailySeriesOutcome {
    /// `recorded` — otherwise.
    ///
    /// The day's rollup of the series for the group is kept.
    Recorded {
        /// The `conductor.observation.DailySeriesRecorded` this outcome publishes.
        daily_series_recorded: DailySeriesRecorded,
    },
    /// `already-recorded` — for an identity a record already carries.
    ///
    /// The rollup is there already, and is unchanged.
    AlreadyRecorded {
        /// Why it was refused: `conductor.observation.DailySeriesExists`.
        error: DailySeriesExists,
    },
}

/// Record a member a collector could not read — the input of `conductor.observation.RecordMemberFailure`.
///
/// Everything it can result in is [`RecordMemberFailureOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordMemberFailure {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `member` — `conductor.observation.RepositoryName`.
    pub member: RepositoryName,
    /// `collector` — `String`.
    pub collector: String,
    /// `error_line` — `String`.
    pub error_line: String,
}

/// Everything `conductor.observation.RecordMemberFailure` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordMemberFailureOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The member the collector could not read, and why, is recorded; the snapshot goes on.
    Recorded {
        /// The `conductor.observation.MemberFailureRecorded` this outcome publishes.
        member_failure_recorded: MemberFailureRecorded,
    },
}

/// Record a merged pull request — the input of `conductor.observation.RecordMergedPullRequest`.
///
/// Everything it can result in is [`RecordMergedPullRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordMergedPullRequest {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
    /// `title` — `String`.
    pub title: String,
    /// `merged_at` — `Timestamp`.
    pub merged_at: crate::primitives::Timestamp,
    /// `url` — `String`.
    pub url: String,
}

/// Everything `conductor.observation.RecordMergedPullRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordMergedPullRequestOutcome {
    /// `invalid-number` — when `number <= 0`.
    ///
    /// A pull request number is positive; nothing was recorded.
    InvalidNumber {
        /// Why it was refused: `conductor.observation.InvalidPullRequestNumber`.
        error: InvalidPullRequestNumber,
    },
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The merged pull request, when it merged and its link are recorded.
    Recorded {
        /// The `conductor.observation.MergedPullRequestRecorded` this outcome publishes.
        merged_pull_request_recorded: MergedPullRequestRecorded,
    },
}

/// Record a pull request — the input of `conductor.observation.RecordPullRequest`.
///
/// Everything it can result in is [`RecordPullRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPullRequest {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
    /// `title` — `String`.
    pub title: String,
    /// `draft` — `Boolean`.
    pub draft: bool,
    /// `mergeability` — `conductor.observation.Mergeability`.
    pub mergeability: Mergeability,
    /// `failing_checks` — `Integer`.
    pub failing_checks: i64,
    /// `failing_check_names` — `List<String>`.
    pub failing_check_names: Vec<String>,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
    /// `updated_at` — `Timestamp`.
    pub updated_at: crate::primitives::Timestamp,
    /// `url` — `Optional<String>`.
    pub url: Option<String>,
}

/// Everything `conductor.observation.RecordPullRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordPullRequestOutcome {
    /// `invalid-number` — when `number <= 0`.
    ///
    /// A pull request number is positive; nothing was recorded.
    InvalidNumber {
        /// Why it was refused: `conductor.observation.InvalidPullRequestNumber`.
        error: InvalidPullRequestNumber,
    },
    /// `negative-checks` — when `failing_checks < 0`.
    ///
    /// A failing-check count is never negative; nothing was recorded.
    NegativeChecks {
        /// Why it was refused: `conductor.observation.InvalidCount`.
        error: InvalidCount,
    },
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The open pull request and its checks are recorded.
    Recorded {
        /// The `conductor.observation.PullRequestRecorded` this outcome publishes.
        pull_request_recorded: PullRequestRecorded,
    },
}

/// Record a release — the input of `conductor.observation.RecordRelease`.
///
/// Everything it can result in is [`RecordReleaseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRelease {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `tag` — `String`.
    pub tag: String,
    /// `published_at` — `Timestamp`.
    pub published_at: crate::primitives::Timestamp,
    /// `url` — `String`.
    pub url: String,
}

/// Everything `conductor.observation.RecordRelease` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordReleaseOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The published release, its date and its link are recorded.
    Recorded {
        /// The `conductor.observation.ReleaseRecorded` this outcome publishes.
        release_recorded: ReleaseRecorded,
    },
}

/// Record a repository — the input of `conductor.observation.RecordRepository`.
///
/// Everything it can result in is [`RecordRepositoryOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRepository {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `visibility` — `conductor.observation.Visibility`.
    pub visibility: Visibility,
    /// `archived` — `Boolean`.
    pub archived: bool,
    /// `local_checkout` — `Boolean`.
    pub local_checkout: bool,
    /// `main_head` — `conductor.observation.CommitSha`.
    pub main_head: CommitSha,
    /// `main_committed_at` — `Timestamp`.
    pub main_committed_at: crate::primitives::Timestamp,
    /// `main_commits_7d` — `Integer`.
    pub main_commits_7d: i64,
    /// `real_commits_7d` — `Integer`.
    pub real_commits_7d: i64,
    /// `behind_main` — `Integer`.
    pub behind_main: i64,
    /// `dirty_files` — `Integer`.
    pub dirty_files: i64,
    /// `worktrees` — `Integer`.
    pub worktrees: i64,
    /// `open_issues` — `Integer`.
    pub open_issues: i64,
    /// `oldest_open_issue_at` — `Optional<Timestamp>`.
    pub oldest_open_issue_at: Option<crate::primitives::Timestamp>,
    /// `latest_release` — `Optional<String>`.
    pub latest_release: Option<String>,
    /// `latest_release_at` — `Optional<Timestamp>`.
    pub latest_release_at: Option<crate::primitives::Timestamp>,
    /// `unreleased_commits` — `Optional<Integer>`.
    pub unreleased_commits: Option<i64>,
    /// `planning_store_version` — `Optional<String>`.
    pub planning_store_version: Option<String>,
    /// `in_catalog` — `Optional<Boolean>`.
    pub in_catalog: Option<bool>,
    /// `gates_hooks_version` — `Optional<String>`.
    pub gates_hooks_version: Option<String>,
    /// `gates_version` — `Optional<String>`.
    pub gates_version: Option<String>,
    /// `wave_width` — `Optional<Integer>`.
    pub wave_width: Option<i64>,
    /// `wave_ready` — `Optional<Integer>`.
    pub wave_ready: Option<i64>,
    /// `wave_reason` — `Optional<String>`.
    pub wave_reason: Option<String>,
}

/// Everything `conductor.observation.RecordRepository` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordRepositoryOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The repository's state at this snapshot is recorded.
    Recorded {
        /// The `conductor.observation.RepositoryRecorded` this outcome publishes.
        repository_recorded: RepositoryRecorded,
    },
}

/// Record a secret sync — the input of `conductor.observation.RecordSecretSync`.
///
/// Everything it can result in is [`RecordSecretSyncOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSecretSync {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `sync` — `String`.
    pub sync: String,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `branch` — `String`.
    pub branch: String,
    /// `path` — `String`.
    pub path: String,
    /// `source_committed_at` — `Optional<Timestamp>`.
    pub source_committed_at: Option<crate::primitives::Timestamp>,
    /// `secrets_updated_at` — `Optional<Timestamp>`.
    pub secrets_updated_at: Option<crate::primitives::Timestamp>,
    /// `error_line` — `Optional<String>`.
    pub error_line: Option<String>,
}

/// Everything `conductor.observation.RecordSecretSync` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordSecretSyncOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The sync's source commit and its secrets' oldest write are recorded.
    Recorded {
        /// The `conductor.observation.SecretSyncRecorded` this outcome publishes.
        secret_sync_recorded: SecretSyncRecorded,
    },
}

/// Record a session — the input of `conductor.observation.RecordSession`.
///
/// Everything it can result in is [`RecordSessionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSession {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: Harness,
    /// `session_ref` — `String`.
    pub session_ref: String,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `cwd` — `String`.
    pub cwd: String,
    /// `repository` — `Optional<conductor.observation.RepositoryName>`.
    pub repository: Option<RepositoryName>,
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
    /// `activity` — `conductor.observation.SessionState`.
    pub activity: SessionState,
    /// `envelope_provider` — `Optional<conductor.observation.EnvelopeProvider>`.
    pub envelope_provider: Option<EnvelopeProvider>,
    /// `envelope_cgroup` — `Optional<String>`.
    pub envelope_cgroup: Option<String>,
    /// `envelope_memory_max` — `Optional<Integer>`.
    pub envelope_memory_max: Option<i64>,
    /// `envelope_cpu_weight` — `Optional<Integer>`.
    pub envelope_cpu_weight: Option<i64>,
    /// `envelope_tasks_max` — `Optional<Integer>`.
    pub envelope_tasks_max: Option<i64>,
    /// `envelope_writable` — `Optional<String>`.
    pub envelope_writable: Option<String>,
    /// `envelope_unboxed` — `Optional<String>`.
    pub envelope_unboxed: Option<String>,
}

/// Everything `conductor.observation.RecordSession` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordSessionOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The live session, where it runs and which repository it is bound to are recorded.
    Recorded {
        /// The `conductor.observation.SessionRecorded` this outcome publishes.
        session_recorded: SessionRecorded,
    },
}

/// Record a specification status — the input of `conductor.observation.RecordSpecification`.
///
/// Everything it can result in is [`RecordSpecificationOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordSpecification {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `presence` — `conductor.observation.SpecificationPresence`.
    pub presence: SpecificationPresence,
    /// `path` — `Optional<String>`.
    pub path: Option<String>,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
    /// `required_ess` — `Optional<String>`.
    pub required_ess: Option<String>,
    /// `validation` — `conductor.observation.ValidationResult`.
    pub validation: ValidationResult,
    /// `validation_refusals` — `Integer`.
    pub validation_refusals: i64,
    /// `scenarios` — `Optional<Integer>`.
    pub scenarios: Option<i64>,
    /// `synthesis_refusals` — `Optional<Integer>`.
    pub synthesis_refusals: Option<i64>,
    /// `conformance_status` — `Optional<String>`.
    pub conformance_status: Option<String>,
}

/// Everything `conductor.observation.RecordSpecification` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordSpecificationOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `negative-refusals` — when `validation_refusals < 0`.
    ///
    /// A refusal count is never negative; nothing was recorded.
    NegativeRefusals {
        /// Why it was refused: `conductor.observation.InvalidCount`.
        error: InvalidCount,
    },
    /// `recorded` — otherwise.
    ///
    /// The repository's specification status at this snapshot is recorded.
    Recorded {
        /// The `conductor.observation.SpecificationRecorded` this outcome publishes.
        specification_recorded: SpecificationRecorded,
    },
}

/// Record one window of an account's usage — the input of `conductor.observation.RecordUsage`.
///
/// Everything it can result in is [`RecordUsageOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordUsage {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `read_at` — `Timestamp`.
    pub read_at: crate::primitives::Timestamp,
    /// `account` — `String`.
    pub account: String,
    /// `active` — `Boolean`.
    pub active: bool,
    /// `health` — `String`.
    pub health: String,
    /// `window` — `String`.
    pub window: String,
    /// `used_percent` — `Integer`.
    pub used_percent: i64,
    /// `resets_at` — `Optional<Timestamp>`.
    pub resets_at: Option<crate::primitives::Timestamp>,
}

/// Everything `conductor.observation.RecordUsage` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordUsageOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `negative-percent` — when `used_percent < 0`.
    ///
    /// A percent used is never negative; nothing was recorded.
    NegativePercent {
        /// Why it was refused: `conductor.observation.InvalidCount`.
        error: InvalidCount,
    },
    /// `recorded` — otherwise.
    ///
    /// The window of the account's usage, as the probe read it, is recorded.
    Recorded {
        /// The `conductor.observation.UsageRecorded` this outcome publishes.
        usage_recorded: UsageRecorded,
    },
}

/// Record a workflow run — the input of `conductor.observation.RecordWorkflowRun`.
///
/// Everything it can result in is [`RecordWorkflowRunOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordWorkflowRun {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `workflow` — `String`.
    pub workflow: String,
    /// `conclusion` — `conductor.observation.RunConclusion`.
    pub conclusion: RunConclusion,
    /// `run_at` — `Timestamp`.
    pub run_at: crate::primitives::Timestamp,
}

/// Everything `conductor.observation.RecordWorkflowRun` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordWorkflowRunOutcome {
    /// `no-such-snapshot` — when no `conductor.observation.Snapshot` carries the identity `input.snapshot_id` names.
    ///
    /// No snapshot has this id, and nothing was recorded.
    NoSuchSnapshot {
        /// Why it was refused: `conductor.observation.SnapshotNotFound`.
        error: SnapshotNotFound,
    },
    /// `closed` — when the `conductor.observation.Snapshot` that `input.snapshot_id` names satisfies `state != Collecting`.
    ///
    /// The snapshot is not collecting, and nothing was recorded.
    Closed {
        /// Why it was refused: `conductor.observation.SnapshotNotCollecting`.
        error: SnapshotNotCollecting,
    },
    /// `recorded` — otherwise.
    ///
    /// The latest main-branch run of one workflow is recorded.
    Recorded {
        /// The `conductor.observation.WorkflowRunRecorded` this outcome publishes.
        workflow_run_recorded: WorkflowRunRecorded,
    },
}

/// Revise a daily rollup — the input of `conductor.observation.ReviseDailySeries`.
///
/// Everything it can result in is [`ReviseDailySeriesOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviseDailySeries {
    /// `daily_series_id` — `conductor.observation.DailySeriesId`.
    pub daily_series_id: DailySeriesId,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
    /// `items` — `List<String>`.
    pub items: Vec<String>,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// Everything `conductor.observation.ReviseDailySeries` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviseDailySeriesOutcome {
    /// `revised` — when the existing subject is in Recorded.
    ///
    /// The rollup counts the items it kept and the new ones.
    Revised {
        /// The `conductor.observation.DailySeriesRevised` this outcome publishes.
        daily_series_revised: DailySeriesRevised,
    },
    /// `no-such-rollup` — for an identity no record carries.
    NoSuchRollup {
        /// Why it was refused: `conductor.observation.DailySeriesNotFound`.
        error: DailySeriesNotFound,
    },
}

/// Start a snapshot — the input of `conductor.observation.StartSnapshot`.
///
/// Everything it can result in is [`StartSnapshotOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartSnapshot {
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `disk_free_bytes` — `Integer`.
    pub disk_free_bytes: i64,
    /// `budget` — `Optional<Integer>`.
    pub budget: Option<i64>,
}

/// Everything `conductor.observation.StartSnapshot` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartSnapshotOutcome {
    /// `negative-disk` — when `disk_free_bytes < 0`.
    ///
    /// Free disk is never negative; no snapshot started.
    NegativeDisk {
        /// Why it was refused: `conductor.observation.InvalidCount`.
        error: InvalidCount,
    },
    /// `started` — otherwise.
    ///
    /// A snapshot is collecting.
    Started {
        /// The `conductor.observation.SnapshotStarted` this outcome publishes.
        snapshot_started: SnapshotStarted,
    },
}

/// BlockerRecorded — the event `conductor.observation.BlockerRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockerRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `reference` — `String`.
    pub reference: String,
}

/// BoardPublished — the event `conductor.observation.BoardPublished`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardPublished {
    /// `board_id` — `conductor.observation.BoardId`.
    pub board_id: BoardId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `deviation_rows` — `Integer`.
    pub deviation_rows: i64,
}

/// CollectorRunRecorded — the event `conductor.observation.CollectorRunRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorRunRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `collector` — `String`.
    pub collector: String,
    /// `duration_ms` — `Integer`.
    pub duration_ms: i64,
    /// `outcome` — `conductor.observation.CollectorRunOutcome`.
    pub outcome: CollectorRunOutcome,
}

/// DailySeriesRecorded — the event `conductor.observation.DailySeriesRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySeriesRecorded {
    /// `daily_series_id` — `conductor.observation.DailySeriesId`.
    pub daily_series_id: DailySeriesId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// DailySeriesRevised — the event `conductor.observation.DailySeriesRevised`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySeriesRevised {
    /// `daily_series_id` — `conductor.observation.DailySeriesId`.
    pub daily_series_id: DailySeriesId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// MemberFailureRecorded — the event `conductor.observation.MemberFailureRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFailureRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `member` — `conductor.observation.RepositoryName`.
    pub member: RepositoryName,
    /// `collector` — `String`.
    pub collector: String,
}

/// MergedPullRequestRecorded — the event `conductor.observation.MergedPullRequestRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedPullRequestRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
}

/// PullRequestRecorded — the event `conductor.observation.PullRequestRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
}

/// ReleaseRecorded — the event `conductor.observation.ReleaseRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `tag` — `String`.
    pub tag: String,
}

/// RepositoryRecorded — the event `conductor.observation.RepositoryRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
}

/// SecretSyncRecorded — the event `conductor.observation.SecretSyncRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSyncRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `sync` — `String`.
    pub sync: String,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
}

/// SessionRecorded — the event `conductor.observation.SessionRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: Harness,
    /// `session_ref` — `String`.
    pub session_ref: String,
}

/// SnapshotCompleted — the event `conductor.observation.SnapshotCompleted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotCompleted {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// SnapshotFailed — the event `conductor.observation.SnapshotFailed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotFailed {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `reason` — `String`.
    pub reason: String,
}

/// SnapshotStarted — the event `conductor.observation.SnapshotStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotStarted {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
}

/// SpecificationRecorded — the event `conductor.observation.SpecificationRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecificationRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `presence` — `conductor.observation.SpecificationPresence`.
    pub presence: SpecificationPresence,
    /// `validation` — `conductor.observation.ValidationResult`.
    pub validation: ValidationResult,
}

/// UsageRecorded — the event `conductor.observation.UsageRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `account` — `String`.
    pub account: String,
    /// `window` — `String`.
    pub window: String,
    /// `used_percent` — `Integer`.
    pub used_percent: i64,
}

/// WorkflowRunRecorded — the event `conductor.observation.WorkflowRunRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRunRecorded {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `workflow` — `String`.
    pub workflow: String,
}

/// The declared error `conductor.observation.DailySeriesExists`.
///
/// A rollup already has this id; it is revised, not recorded again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySeriesExists {
    /// `daily_series_id` — `conductor.observation.DailySeriesId`.
    pub daily_series_id: DailySeriesId,
}

/// The declared error `conductor.observation.DailySeriesNotFound`.
///
/// No rollup has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailySeriesNotFound {
    /// `daily_series_id` — `conductor.observation.DailySeriesId`.
    pub daily_series_id: DailySeriesId,
}

/// The declared error `conductor.observation.InvalidCount`.
///
/// A count was negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidCount {
    /// `count` — `Integer`.
    pub count: i64,
}

/// The declared error `conductor.observation.InvalidPullRequestNumber`.
///
/// A pull request number was not positive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPullRequestNumber {
    /// `number` — `Integer`.
    pub number: i64,
}

/// The declared error `conductor.observation.SnapshotNotCollecting`.
///
/// The snapshot is complete or failed, so it takes no more observations and cannot change state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotNotCollecting {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// The declared error `conductor.observation.SnapshotNotComplete`.
///
/// The snapshot is collecting or failed, so no board is published from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotNotComplete {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// The declared error `conductor.observation.SnapshotNotFound`.
///
/// No snapshot has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotNotFound {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
}

/// Blockers — one row of the view `conductor.observation.Blockers`.
///
/// Projects `conductor.observation.BlockerObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blockers {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `reference` — `String`.
    pub reference: String,
    /// `blocker_kind` — `String`.
    pub blocker_kind: String,
    /// `title` — `String`.
    pub title: String,
}

/// Boards — one row of the view `conductor.observation.Boards`.
///
/// Projects `conductor.observation.Board` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boards {
    /// `board_id` — `conductor.observation.BoardId`.
    pub board_id: BoardId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `previous_snapshot_id` — `Optional<conductor.observation.SnapshotId>`.
    pub previous_snapshot_id: Option<SnapshotId>,
    /// `deviation_rows` — `Integer`.
    pub deviation_rows: i64,
    /// `path` — `String`.
    pub path: String,
}

/// Collector runs — one row of the view `conductor.observation.CollectorRuns`.
///
/// Projects `conductor.observation.CollectorRunObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectorRuns {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `collector` — `String`.
    pub collector: String,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `duration_ms` — `Integer`.
    pub duration_ms: i64,
    /// `outcome` — `conductor.observation.CollectorRunOutcome`.
    pub outcome: CollectorRunOutcome,
}

/// Members a collector could not read — one row of the view `conductor.observation.MemberFailures`.
///
/// Projects `conductor.observation.MemberFailureObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberFailures {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `member` — `conductor.observation.RepositoryName`.
    pub member: RepositoryName,
    /// `collector` — `String`.
    pub collector: String,
    /// `error_line` — `String`.
    pub error_line: String,
}

/// Merged pull requests — one row of the view `conductor.observation.MergedPullRequests`.
///
/// Projects `conductor.observation.MergedPullRequestObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedPullRequests {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
    /// `title` — `String`.
    pub title: String,
    /// `merged_at` — `Timestamp`.
    pub merged_at: crate::primitives::Timestamp,
    /// `url` — `String`.
    pub url: String,
}

/// Pull requests — one row of the view `conductor.observation.PullRequests`.
///
/// Projects `conductor.observation.PullRequestObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequests {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `number` — `Integer`.
    pub number: i64,
    /// `title` — `String`.
    pub title: String,
    /// `draft` — `Boolean`.
    pub draft: bool,
    /// `mergeability` — `conductor.observation.Mergeability`.
    pub mergeability: Mergeability,
    /// `failing_checks` — `Integer`.
    pub failing_checks: i64,
    /// `failing_check_names` — `List<String>`.
    pub failing_check_names: Vec<String>,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
    /// `updated_at` — `Timestamp`.
    pub updated_at: crate::primitives::Timestamp,
    /// `url` — `Optional<String>`.
    pub url: Option<String>,
}

/// Releases — one row of the view `conductor.observation.Releases`.
///
/// Projects `conductor.observation.ReleaseObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Releases {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `tag` — `String`.
    pub tag: String,
    /// `published_at` — `Timestamp`.
    pub published_at: crate::primitives::Timestamp,
    /// `url` — `String`.
    pub url: String,
}

/// Repositories — one row of the view `conductor.observation.Repositories`.
///
/// Projects `conductor.observation.RepositoryObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repositories {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `visibility` — `conductor.observation.Visibility`.
    pub visibility: Visibility,
    /// `archived` — `Boolean`.
    pub archived: bool,
    /// `local_checkout` — `Boolean`.
    pub local_checkout: bool,
    /// `main_head` — `conductor.observation.CommitSha`.
    pub main_head: CommitSha,
    /// `main_committed_at` — `Timestamp`.
    pub main_committed_at: crate::primitives::Timestamp,
    /// `main_commits_7d` — `Integer`.
    pub main_commits_7d: i64,
    /// `real_commits_7d` — `Integer`.
    pub real_commits_7d: i64,
    /// `behind_main` — `Integer`.
    pub behind_main: i64,
    /// `dirty_files` — `Integer`.
    pub dirty_files: i64,
    /// `worktrees` — `Integer`.
    pub worktrees: i64,
    /// `open_issues` — `Integer`.
    pub open_issues: i64,
    /// `oldest_open_issue_at` — `Optional<Timestamp>`.
    pub oldest_open_issue_at: Option<crate::primitives::Timestamp>,
    /// `latest_release` — `Optional<String>`.
    pub latest_release: Option<String>,
    /// `latest_release_at` — `Optional<Timestamp>`.
    pub latest_release_at: Option<crate::primitives::Timestamp>,
    /// `unreleased_commits` — `Optional<Integer>`.
    pub unreleased_commits: Option<i64>,
    /// `planning_store_version` — `Optional<String>`.
    pub planning_store_version: Option<String>,
    /// `in_catalog` — `Optional<Boolean>`.
    pub in_catalog: Option<bool>,
    /// `gates_hooks_version` — `Optional<String>`.
    pub gates_hooks_version: Option<String>,
    /// `gates_version` — `Optional<String>`.
    pub gates_version: Option<String>,
    /// `wave_width` — `Optional<Integer>`.
    pub wave_width: Option<i64>,
    /// `wave_ready` — `Optional<Integer>`.
    pub wave_ready: Option<i64>,
    /// `wave_reason` — `Optional<String>`.
    pub wave_reason: Option<String>,
}

/// Secret syncs — one row of the view `conductor.observation.SecretSyncs`.
///
/// Projects `conductor.observation.SecretSyncObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSyncs {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `sync` — `String`.
    pub sync: String,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `branch` — `String`.
    pub branch: String,
    /// `path` — `String`.
    pub path: String,
    /// `source_committed_at` — `Optional<Timestamp>`.
    pub source_committed_at: Option<crate::primitives::Timestamp>,
    /// `secrets_updated_at` — `Optional<Timestamp>`.
    pub secrets_updated_at: Option<crate::primitives::Timestamp>,
    /// `error_line` — `Optional<String>`.
    pub error_line: Option<String>,
}

/// Sessions — one row of the view `conductor.observation.Sessions`.
///
/// Projects `conductor.observation.SessionObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sessions {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: Harness,
    /// `session_ref` — `String`.
    pub session_ref: String,
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `cwd` — `String`.
    pub cwd: String,
    /// `repository` — `Optional<conductor.observation.RepositoryName>`.
    pub repository: Option<RepositoryName>,
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
    /// `activity` — `conductor.observation.SessionState`.
    pub activity: SessionState,
    /// `envelope_provider` — `Optional<conductor.observation.EnvelopeProvider>`.
    pub envelope_provider: Option<EnvelopeProvider>,
    /// `envelope_cgroup` — `Optional<String>`.
    pub envelope_cgroup: Option<String>,
    /// `envelope_memory_max` — `Optional<Integer>`.
    pub envelope_memory_max: Option<i64>,
    /// `envelope_cpu_weight` — `Optional<Integer>`.
    pub envelope_cpu_weight: Option<i64>,
    /// `envelope_tasks_max` — `Optional<Integer>`.
    pub envelope_tasks_max: Option<i64>,
    /// `envelope_writable` — `Optional<String>`.
    pub envelope_writable: Option<String>,
    /// `envelope_unboxed` — `Optional<String>`.
    pub envelope_unboxed: Option<String>,
}

/// Snapshots — one row of the view `conductor.observation.Snapshots`.
///
/// Projects `conductor.observation.Snapshot` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshots {
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `state` — `conductor.observation.Snapshot.State`.
    pub state: SnapshotState,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `disk_free_bytes` — `Integer`.
    pub disk_free_bytes: i64,
    /// `failure_reason` — `Optional<String>`.
    pub failure_reason: Option<String>,
}

/// Specifications — one row of the view `conductor.observation.Specifications`.
///
/// Projects `conductor.observation.SpecificationObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Specifications {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `presence` — `conductor.observation.SpecificationPresence`.
    pub presence: SpecificationPresence,
    /// `path` — `Optional<String>`.
    pub path: Option<String>,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
    /// `required_ess` — `Optional<String>`.
    pub required_ess: Option<String>,
    /// `validation` — `conductor.observation.ValidationResult`.
    pub validation: ValidationResult,
    /// `validation_refusals` — `Integer`.
    pub validation_refusals: i64,
    /// `scenarios` — `Optional<Integer>`.
    pub scenarios: Option<i64>,
    /// `synthesis_refusals` — `Optional<Integer>`.
    pub synthesis_refusals: Option<i64>,
    /// `conformance_status` — `Optional<String>`.
    pub conformance_status: Option<String>,
}

/// Usage readings — one row of the view `conductor.observation.UsageReadings`.
///
/// Projects `conductor.observation.UsageObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageReadings {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `read_at` — `Timestamp`.
    pub read_at: crate::primitives::Timestamp,
    /// `account` — `String`.
    pub account: String,
    /// `active` — `Boolean`.
    pub active: bool,
    /// `health` — `String`.
    pub health: String,
    /// `window` — `String`.
    pub window: String,
    /// `used_percent` — `Integer`.
    pub used_percent: i64,
    /// `resets_at` — `Optional<Timestamp>`.
    pub resets_at: Option<crate::primitives::Timestamp>,
}

/// Workflow runs — one row of the view `conductor.observation.WorkflowRuns`.
///
/// Projects `conductor.observation.WorkflowRunObservation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowRuns {
    /// `observation_id` — `conductor.observation.ObservationId`.
    pub observation_id: ObservationId,
    /// `snapshot_id` — `conductor.observation.SnapshotId`.
    pub snapshot_id: SnapshotId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: RepositoryName,
    /// `workflow` — `String`.
    pub workflow: String,
    /// `conclusion` — `conductor.observation.RunConclusion`.
    pub conclusion: RunConclusion,
    /// `run_at` — `Timestamp`.
    pub run_at: crate::primitives::Timestamp,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.observation.CompleteSnapshot` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CompleteSnapshotBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.CompleteSnapshot`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn complete_snapshot(&mut self, input: super::CompleteSnapshot) -> Result<super::CompleteSnapshotOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.FailSnapshot` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FailSnapshotBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.FailSnapshot`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn fail_snapshot(&mut self, input: super::FailSnapshot) -> Result<super::FailSnapshotOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.PublishBoard` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PublishBoardBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.PublishBoard`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn publish_board(&mut self, input: super::PublishBoard) -> Result<super::PublishBoardOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordBlocker` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordBlockerBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordBlocker`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_blocker(&mut self, input: super::RecordBlocker) -> Result<super::RecordBlockerOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordCollectorRun` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordCollectorRunBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordCollectorRun`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_collector_run(&mut self, input: super::RecordCollectorRun) -> Result<super::RecordCollectorRunOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordDailySeries` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordDailySeriesBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordDailySeries`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_daily_series(&mut self, input: super::RecordDailySeries) -> Result<super::RecordDailySeriesOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordMemberFailure` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordMemberFailureBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordMemberFailure`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_member_failure(&mut self, input: super::RecordMemberFailure) -> Result<super::RecordMemberFailureOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordMergedPullRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordMergedPullRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordMergedPullRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_merged_pull_request(&mut self, input: super::RecordMergedPullRequest) -> Result<super::RecordMergedPullRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordPullRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordPullRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordPullRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_pull_request(&mut self, input: super::RecordPullRequest) -> Result<super::RecordPullRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordRelease` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordReleaseBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordRelease`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_release(&mut self, input: super::RecordRelease) -> Result<super::RecordReleaseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordRepository` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordRepositoryBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordRepository`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_repository(&mut self, input: super::RecordRepository) -> Result<super::RecordRepositoryOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordSecretSync` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordSecretSyncBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordSecretSync`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_secret_sync(&mut self, input: super::RecordSecretSync) -> Result<super::RecordSecretSyncOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordSession` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordSessionBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordSession`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_session(&mut self, input: super::RecordSession) -> Result<super::RecordSessionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordSpecification` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordSpecificationBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordSpecification`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_specification(&mut self, input: super::RecordSpecification) -> Result<super::RecordSpecificationOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordUsage` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordUsageBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordUsage`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_usage(&mut self, input: super::RecordUsage) -> Result<super::RecordUsageOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.RecordWorkflowRun` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordWorkflowRunBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.RecordWorkflowRun`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_workflow_run(&mut self, input: super::RecordWorkflowRun) -> Result<super::RecordWorkflowRunOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.ReviseDailySeries` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReviseDailySeriesBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.ReviseDailySeries`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn revise_daily_series(&mut self, input: super::ReviseDailySeries) -> Result<super::ReviseDailySeriesOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.observation.StartSnapshot` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait StartSnapshotBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.observation.StartSnapshot`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn start_snapshot(&mut self, input: super::StartSnapshot) -> Result<super::StartSnapshotOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.Blockers` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait BlockersQuery {
        /// Serves `conductor.observation.Blockers` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn blockers(&self) -> Result<Vec<super::Blockers>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.Boards` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait BoardsQuery {
        /// Serves `conductor.observation.Boards` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn boards(&self) -> Result<Vec<super::Boards>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.CollectorRuns` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait CollectorRunsQuery {
        /// Serves `conductor.observation.CollectorRuns` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn collector_runs(&self) -> Result<Vec<super::CollectorRuns>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.MemberFailures` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MemberFailuresQuery {
        /// Serves `conductor.observation.MemberFailures` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn member_failures(&self) -> Result<Vec<super::MemberFailures>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.MergedPullRequests` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MergedPullRequestsQuery {
        /// Serves `conductor.observation.MergedPullRequests` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn merged_pull_requests(&self) -> Result<Vec<super::MergedPullRequests>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.PullRequests` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait PullRequestsQuery {
        /// Serves `conductor.observation.PullRequests` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn pull_requests(&self) -> Result<Vec<super::PullRequests>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.Releases` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ReleasesQuery {
        /// Serves `conductor.observation.Releases` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn releases(&self) -> Result<Vec<super::Releases>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.Repositories` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RepositoriesQuery {
        /// Serves `conductor.observation.Repositories` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn repositories(&self) -> Result<Vec<super::Repositories>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.SecretSyncs` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SecretSyncsQuery {
        /// Serves `conductor.observation.SecretSyncs` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn secret_syncs(&self) -> Result<Vec<super::SecretSyncs>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.Sessions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SessionsQuery {
        /// Serves `conductor.observation.Sessions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn sessions(&self) -> Result<Vec<super::Sessions>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.Snapshots` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SnapshotsQuery {
        /// Serves `conductor.observation.Snapshots` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn snapshots(&self) -> Result<Vec<super::Snapshots>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.Specifications` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SpecificationsQuery {
        /// Serves `conductor.observation.Specifications` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn specifications(&self) -> Result<Vec<super::Specifications>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.UsageReadings` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait UsageReadingsQuery {
        /// Serves `conductor.observation.UsageReadings` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn usage_readings(&self) -> Result<Vec<super::UsageReadings>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.observation.WorkflowRuns` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait WorkflowRunsQuery {
        /// Serves `conductor.observation.WorkflowRuns` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn workflow_runs(&self) -> Result<Vec<super::WorkflowRuns>, crate::obligation::UnmetObligation>;
    }

}
