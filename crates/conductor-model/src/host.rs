// generated from conductor v1
// model digest 8edad738c6b5689ba394ca170ba4f1bdba8e03a517cceb3adee4fb4b6566a0df
// contract digest b63ca81a4b13794d6772aed33575fa3e4e1ed6bca22c18a46e3a425a0ae59e38
// do not edit: regenerate with `ess synthesize --layout crate`

//! Host — `conductor.host`.
//!
//! The machine several conductor instances share (story:multi-instance-host), and the protocol its disk runs under (epic:disk-protocol, story:disk-protocol-spec). Each instance keeps its own records, state, cache and sessions; what they share is one disk, one CPU and memory budget, and one machine-wide namespace of harness session names. The host ledger holds every grant an instance's conductor or a `conductor build run` takes: a controller slot, a build slot, or a disk reservation for a run's expected peak, so two instances or two runs cannot each grant the same free space. A reclaim run frees what needs no decision (`conductor host reclaim`) and records each step with the bytes it freed, or what is left when it cannot reach the floor.
//! The host directory (story:instance-isolation) is `host/` beside the config file, one for every instance the file names (`~/.b10x/conductor/host/` beside the default file), never under an instance's `state`. It holds the host ledger's store, its lock `host.lock`, the run locks `host-runs/<grant>.lock`, the reclaim lock `host.reclaim.lock` and record `host.reclaim`, and the marker of the open fall of free disk `host.fall`, which names the item's instance: only that instance withdraws the item. A `--state-dir` does not move it. The ledger an instance kept in its `state` before is moved there once, at its first host command: every grant of its store, its run locks, the reclaim record and the fall's marker; `host.migrated` in the state then records that it was moved, and nothing is moved again.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// BuildRun — `conductor.host.BuildRun`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRun {
    /// `reserve` — `conductor.config.Gibibytes`.
    pub reserve: crate::config::Gibibytes,
    /// `repository` — `Optional<conductor.observation.RepositoryName>`.
    pub repository: Option<crate::observation::RepositoryName>,
    /// `command` — `List<String>`.
    pub command: Vec<String>,
}

/// BuildRunRefused — `conductor.host.BuildRunRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRunRefused {
    /// `bytes` — `Integer`.
    pub bytes: i64,
    /// `shortfall_bytes` — `Integer`.
    pub shortfall_bytes: i64,
    /// `left` — `List<conductor.host.LeftOver>`.
    pub left: Vec<LeftOver>,
}

/// GrantId — `conductor.host.GrantId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantId(pub String);

/// GrantKind — `conductor.host.GrantKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantKind {
    /// `ControllerSlot`.
    ControllerSlot,
    /// `BuildSlot`.
    BuildSlot,
    /// `Disk`.
    Disk,
}

/// The states of `conductor.host.HostGrant`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `HostGrant<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostGrantState {
    /// `Granted`.
    Granted,
    /// `Refused`.
    Refused,
    /// `Released`.
    Released,
    /// `Requested`.
    Requested,
    /// `Running`.
    Running,
    /// `Withdrawn`.
    Withdrawn,
}

/// LeftOver — `conductor.host.LeftOver`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeftOver {
    /// `path` — `String`.
    pub path: String,
    /// `bytes` — `Integer`.
    pub bytes: i64,
}

/// ReclaimDisk — `conductor.host.ReclaimDisk`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimDisk {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// ReclaimFellShort — `conductor.host.ReclaimFellShort`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimFellShort {
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
    /// `shortfall_bytes` — `Integer`.
    pub shortfall_bytes: i64,
    /// `left` — `List<conductor.host.LeftOver>`.
    pub left: Vec<LeftOver>,
}

/// ReclaimKind — `conductor.host.ReclaimKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimKind {
    /// `IdleTreeBuildOutput`.
    IdleTreeBuildOutput,
    /// `ArchiveBuildOutput`.
    ArchiveBuildOutput,
    /// `Cache`.
    Cache,
    /// `Snapshot`.
    Snapshot,
}

/// ReclaimResult — `conductor.host.ReclaimResult`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimResult {
    /// `Reclaimed`.
    Reclaimed,
    /// `FloorNotReached`.
    FloorNotReached,
}

/// The states of `conductor.host.ReclaimRun`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ReclaimRun<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReclaimRunState {
    /// `Recorded`.
    Recorded,
}

/// ReclaimRunId — `conductor.host.ReclaimRunId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimRunId(pub String);

/// ReclaimStep — `conductor.host.ReclaimStep`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimStep {
    /// `what` — `String`.
    pub what: String,
    /// `kind` — `conductor.host.ReclaimKind`.
    pub kind: ReclaimKind,
    /// `bytes` — `Integer`.
    pub bytes: i64,
}

/// ReleaseReason — `conductor.host.ReleaseReason`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseReason {
    /// `Exited`.
    Exited,
    /// `Killed`.
    Killed,
    /// `ProcessGroupGone`.
    ProcessGroupGone,
}

/// ShowGrants — `conductor.host.ShowGrants`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowGrants {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// What HostGrant — `conductor.host.HostGrant` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`HostGrant<S>`], and at a boundary by [`HostGrantSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostGrantData {
    /// The identity: `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `repository` — `Optional<conductor.observation.RepositoryName>`.
    pub repository: Option<crate::observation::RepositoryName>,
    /// `kind` — `conductor.host.GrantKind`.
    pub kind: GrantKind,
    /// `bytes` — `Optional<Integer>`.
    pub bytes: Option<i64>,
    /// `peak_bytes` — `Optional<Integer>`.
    pub peak_bytes: Option<i64>,
    /// `command` — `Optional<String>`.
    pub command: Option<String>,
    /// `process_group` — `Optional<Integer>`.
    pub process_group: Option<i64>,
    /// `session_name` — `Optional<conductor.config.SessionName>`.
    pub session_name: Option<crate::config::SessionName>,
    /// `reclaimed_bytes` — `Optional<Integer>`.
    pub reclaimed_bytes: Option<i64>,
    /// `shortfall_bytes` — `Optional<Integer>`.
    pub shortfall_bytes: Option<i64>,
    /// `requested_at` — `Timestamp`.
    pub requested_at: crate::primitives::Timestamp,
    /// `started_at` — `Optional<Timestamp>`.
    pub started_at: Option<crate::primitives::Timestamp>,
    /// `released_at` — `Optional<Timestamp>`.
    pub released_at: Option<crate::primitives::Timestamp>,
    /// `release_reason` — `Optional<conductor.host.ReleaseReason>`.
    pub release_reason: Option<ReleaseReason>,
}

/// The states of `conductor.host.HostGrant`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](host_grant_state::Marker), so [`HostGrant<S>`](HostGrant) can only ever rest in a real state.
pub mod host_grant_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Granted {}
        impl Sealed for super::Refused {}
        impl Sealed for super::Released {}
        impl Sealed for super::Requested {}
        impl Sealed for super::Running {}
        impl Sealed for super::Withdrawn {}
    }

    /// A declared state of `HostGrant`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::HostGrantState;
    }

    /// `Granted`.
    pub struct Granted;

    impl Marker for Granted {
        const STATE: super::HostGrantState = super::HostGrantState::Granted;
    }

    /// `Refused`. Terminal: an instance may rest here forever.
    pub struct Refused;

    impl Marker for Refused {
        const STATE: super::HostGrantState = super::HostGrantState::Refused;
    }

    /// `Released`. Terminal: an instance may rest here forever.
    pub struct Released;

    impl Marker for Released {
        const STATE: super::HostGrantState = super::HostGrantState::Released;
    }

    /// `Requested`. Where a new instance starts.
    pub struct Requested;

    impl Marker for Requested {
        const STATE: super::HostGrantState = super::HostGrantState::Requested;
    }

    /// `Running`.
    pub struct Running;

    impl Marker for Running {
        const STATE: super::HostGrantState = super::HostGrantState::Running;
    }

    /// `Withdrawn`. Terminal: an instance may rest here forever.
    pub struct Withdrawn;

    impl Marker for Withdrawn {
        const STATE: super::HostGrantState = super::HostGrantState::Withdrawn;
    }
}

/// HostGrant — `conductor.host.HostGrant` — with its lifecycle state carried by the type.
///
/// Its constructors rest in the states a creation lands in: `Requested`, `Granted`.
/// And the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`HostGrantSnapshot`]
/// and [`HostGrantSnapshot::refine`].
pub struct HostGrant<S: host_grant_state::Marker> {
    data: HostGrantData,
    state: core::marker::PhantomData<S>,
}

impl<S: host_grant_state::Marker> HostGrant<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> HostGrantState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &HostGrantData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> HostGrantData {
        self.data
    }
}

impl HostGrant<host_grant_state::Requested> {
    /// A new instance, resting in `Requested` — where a creation naming no `into:` lands it.
    pub fn new(data: HostGrantData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl HostGrant<host_grant_state::Granted> {
    /// A new instance, resting in `Granted` — where a creation declared `into: Granted` lands it.
    pub fn new_granted(data: HostGrantData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl HostGrant<host_grant_state::Granted> {
    /// `start` — `Granted` → `Running`. Taken by the `started` outcome of `conductor.host.StartRun`.
    pub fn start(self) -> HostGrant<host_grant_state::Running> {
        HostGrant {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `release` — `Granted` → `Released`. Taken by the `released` outcome of `conductor.host.ReleaseGrant`.
    pub fn release(self) -> HostGrant<host_grant_state::Released> {
        HostGrant {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl HostGrant<host_grant_state::Requested> {
    /// `grant` — `Requested` → `Granted`. Taken by the `granted-after-reclaim` outcome of `conductor.host.ReserveDisk`, the `granted` outcome of `conductor.host.ReserveDisk`.
    pub fn grant(self) -> HostGrant<host_grant_state::Granted> {
        HostGrant {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `refuse` — `Requested` → `Refused`. Taken by the `refused` outcome of `conductor.host.ReserveDisk`.
    pub fn refuse(self) -> HostGrant<host_grant_state::Refused> {
        HostGrant {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `withdraw` — `Requested` → `Withdrawn`. Taken by the `withdrawn` outcome of `conductor.host.WithdrawDiskRequest`.
    pub fn withdraw(self) -> HostGrant<host_grant_state::Withdrawn> {
        HostGrant {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl HostGrant<host_grant_state::Running> {
    /// `release` — `Running` → `Released`. Taken by the `released` outcome of `conductor.host.ReleaseGrant`.
    pub fn release(self) -> HostGrant<host_grant_state::Released> {
        HostGrant {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.host.HostGrant` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`HostGrantSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostGrantSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: HostGrantState,
    /// What it holds.
    pub data: HostGrantData,
}

/// An `HostGrant` in whichever declared state it was found.
pub enum AnyHostGrant {
    /// Resting in `Granted`.
    Granted(HostGrant<host_grant_state::Granted>),
    /// Resting in `Refused`.
    Refused(HostGrant<host_grant_state::Refused>),
    /// Resting in `Released`.
    Released(HostGrant<host_grant_state::Released>),
    /// Resting in `Requested`.
    Requested(HostGrant<host_grant_state::Requested>),
    /// Resting in `Running`.
    Running(HostGrant<host_grant_state::Running>),
    /// Resting in `Withdrawn`.
    Withdrawn(HostGrant<host_grant_state::Withdrawn>),
}

impl HostGrantSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `HostGrantState` cannot spell one.
    pub fn refine(self) -> AnyHostGrant {
        match self.state {
            HostGrantState::Granted => AnyHostGrant::Granted(HostGrant {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            HostGrantState::Refused => AnyHostGrant::Refused(HostGrant {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            HostGrantState::Released => AnyHostGrant::Released(HostGrant {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            HostGrantState::Requested => AnyHostGrant::Requested(HostGrant {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            HostGrantState::Running => AnyHostGrant::Running(HostGrant {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            HostGrantState::Withdrawn => AnyHostGrant::Withdrawn(HostGrant {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyHostGrant {
    /// The state, as the runtime value.
    pub fn state(&self) -> HostGrantState {
        match self {
            Self::Granted(_) => HostGrantState::Granted,
            Self::Refused(_) => HostGrantState::Refused,
            Self::Released(_) => HostGrantState::Released,
            Self::Requested(_) => HostGrantState::Requested,
            Self::Running(_) => HostGrantState::Running,
            Self::Withdrawn(_) => HostGrantState::Withdrawn,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> HostGrantSnapshot {
        match self {
            Self::Granted(instance) => HostGrantSnapshot {
                state: HostGrantState::Granted,
                data: instance.into_data(),
            },
            Self::Refused(instance) => HostGrantSnapshot {
                state: HostGrantState::Refused,
                data: instance.into_data(),
            },
            Self::Released(instance) => HostGrantSnapshot {
                state: HostGrantState::Released,
                data: instance.into_data(),
            },
            Self::Requested(instance) => HostGrantSnapshot {
                state: HostGrantState::Requested,
                data: instance.into_data(),
            },
            Self::Running(instance) => HostGrantSnapshot {
                state: HostGrantState::Running,
                data: instance.into_data(),
            },
            Self::Withdrawn(instance) => HostGrantSnapshot {
                state: HostGrantState::Withdrawn,
                data: instance.into_data(),
            },
        }
    }
}

/// What ReclaimRun — `conductor.host.ReclaimRun` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ReclaimRun<S>`], and at a boundary by [`ReclaimRunSnapshot::state`].
///
/// Every value satisfies `reclaimed_bytes >= 0` — checked by [`ReclaimRunData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimRunData {
    /// The identity: `reclaim_run_id` — `conductor.host.ReclaimRunId`.
    pub reclaim_run_id: ReclaimRunId,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `result` — `conductor.host.ReclaimResult`.
    pub result: ReclaimResult,
    /// `steps` — `List<conductor.host.ReclaimStep>`.
    pub steps: Vec<ReclaimStep>,
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
    /// `shortfall_bytes` — `Optional<Integer>`.
    pub shortfall_bytes: Option<i64>,
    /// `left` — `List<conductor.host.LeftOver>`.
    pub left: Vec<LeftOver>,
    /// `ran_at` — `Timestamp`.
    pub ran_at: crate::primitives::Timestamp,
}

impl ReclaimRunData {
    /// The first declared invariant of `conductor.host.ReclaimRun` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.reclaimed_bytes)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("reclaimed_bytes >= 0");
        }
        None
    }
}

/// The states of `conductor.host.ReclaimRun`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](reclaim_run_state::Marker), so [`ReclaimRun<S>`](ReclaimRun) can only ever rest in a real state.
pub mod reclaim_run_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ReclaimRun`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ReclaimRunState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ReclaimRunState = super::ReclaimRunState::Recorded;
    }
}

/// ReclaimRun — `conductor.host.ReclaimRun` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ReclaimRunSnapshot`]
/// and [`ReclaimRunSnapshot::refine`].
pub struct ReclaimRun<S: reclaim_run_state::Marker> {
    data: ReclaimRunData,
    state: core::marker::PhantomData<S>,
}

impl<S: reclaim_run_state::Marker> ReclaimRun<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ReclaimRunState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ReclaimRunData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ReclaimRunData {
        self.data
    }
}

impl ReclaimRun<reclaim_run_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ReclaimRunData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.host.ReclaimRun` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ReclaimRunSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimRunSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ReclaimRunState,
    /// What it holds.
    pub data: ReclaimRunData,
}

/// An `ReclaimRun` in whichever declared state it was found.
pub enum AnyReclaimRun {
    /// Resting in `Recorded`.
    Recorded(ReclaimRun<reclaim_run_state::Recorded>),
}

impl ReclaimRunSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ReclaimRunState` cannot spell one.
    pub fn refine(self) -> AnyReclaimRun {
        match self.state {
            ReclaimRunState::Recorded => AnyReclaimRun::Recorded(ReclaimRun {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyReclaimRun {
    /// The state, as the runtime value.
    pub fn state(&self) -> ReclaimRunState {
        match self {
            Self::Recorded(_) => ReclaimRunState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ReclaimRunSnapshot {
        match self {
            Self::Recorded(instance) => ReclaimRunSnapshot {
                state: ReclaimRunState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// Record a reclaim run — the input of `conductor.host.Reclaim`.
///
/// Everything it can result in is [`ReclaimOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reclaim {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `steps` — `List<conductor.host.ReclaimStep>`.
    pub steps: Vec<ReclaimStep>,
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
    /// `shortfall_bytes` — `Integer`.
    pub shortfall_bytes: i64,
    /// `left` — `List<conductor.host.LeftOver>`.
    pub left: Vec<LeftOver>,
    /// `ran_at` — `Timestamp`.
    pub ran_at: crate::primitives::Timestamp,
}

/// Everything `conductor.host.Reclaim` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReclaimOutcome {
    /// `invalid-size` — when `(reclaimed_bytes < 0 or shortfall_bytes < 0)`.
    ///
    /// No count is negative; nothing was recorded.
    InvalidSize {
        /// Why it was refused: `conductor.host.InvalidSize`.
        error: InvalidSize,
    },
    /// `floor-not-reached` — when `shortfall_bytes > 0`.
    ///
    /// What needs no decision is freed and the floor is still not reached; what is left is listed by path and size, `conductor host reclaim` exits 1, and the watch reports `disk floor not reached: <n>G short`.
    FloorNotReached {
        /// The `conductor.host.ReclaimFloorNotReached` this outcome publishes.
        reclaim_floor_not_reached: ReclaimFloorNotReached,
    },
    /// `reclaimed` — otherwise.
    ///
    /// Free disk is back over the floor; each step is printed with the bytes it freed.
    Reclaimed {
        /// The `conductor.host.DiskReclaimed` this outcome publishes.
        disk_reclaimed: DiskReclaimed,
    },
}

/// Record a run's peak — the input of `conductor.host.RecordPeak`.
///
/// Everything it can result in is [`RecordPeakOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPeak {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `peak_bytes` — `Integer`.
    pub peak_bytes: i64,
}

/// Everything `conductor.host.RecordPeak` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordPeakOutcome {
    /// `invalid-size` — when `peak_bytes < 0`.
    ///
    /// A measured size is not negative; nothing was recorded.
    InvalidSize {
        /// Why it was refused: `conductor.host.InvalidSize`.
        error: InvalidSize,
    },
    /// `recorded` — when the existing subject is in Running.
    ///
    /// The grant carries the peak; the Grants view shows it beside the bytes reserved.
    Recorded {
        /// The `conductor.host.PeakRecorded` this outcome publishes.
        peak_recorded: PeakRecorded,
    },
    /// `no-such-grant` — for an identity no record carries.
    NoSuchGrant {
        /// Why it was refused: `conductor.host.HostGrantNotFound`.
        error: HostGrantNotFound,
    },
    /// `not-running` — otherwise.
    ///
    /// Only a running grant records a peak.
    NotRunning {
        /// Why it was refused: `conductor.host.HostGrantStateConflict`.
        error: HostGrantStateConflict,
    },
}

/// Release a host grant — the input of `conductor.host.ReleaseGrant`.
///
/// Everything it can result in is [`ReleaseGrantOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseGrant {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `reason` — `Optional<conductor.host.ReleaseReason>`.
    pub reason: Option<ReleaseReason>,
    /// `released_at` — `Timestamp`.
    pub released_at: crate::primitives::Timestamp,
}

/// Everything `conductor.host.ReleaseGrant` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseGrantOutcome {
    /// `released` — otherwise.
    ///
    /// The slot or the space is free for any instance and any run again.
    Released {
        /// The `conductor.host.HostGrantReleased` this outcome publishes.
        host_grant_released: HostGrantReleased,
    },
    /// `no-such-grant` — for an identity no record carries.
    ///
    /// No grant with that id.
    NoSuchGrant {
        /// Why it was refused: `conductor.host.HostGrantNotFound`.
        error: HostGrantNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The grant was released before, or refused; nothing changes.
    WrongState {
        /// Why it was refused: `conductor.host.HostGrantStateConflict`.
        error: HostGrantStateConflict,
    },
}

/// Request disk for a command — the input of `conductor.host.RequestDisk`.
///
/// Everything it can result in is [`RequestDiskOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestDisk {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `repository` — `Optional<conductor.observation.RepositoryName>`.
    pub repository: Option<crate::observation::RepositoryName>,
    /// `session_name` — `Optional<conductor.config.SessionName>`.
    pub session_name: Option<crate::config::SessionName>,
    /// `bytes` — `Integer`.
    pub bytes: i64,
    /// `command` — `String`.
    pub command: String,
    /// `requested_at` — `Timestamp`.
    pub requested_at: crate::primitives::Timestamp,
}

/// Everything `conductor.host.RequestDisk` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestDiskOutcome {
    /// `invalid-size` — when `bytes <= 0`.
    ///
    /// A reservation asks for a positive size; nothing was requested.
    InvalidSize {
        /// Why it was refused: `conductor.host.InvalidSize`.
        error: InvalidSize,
    },
    /// `requested` — otherwise.
    ///
    /// The request counts against the host while the ledger answers it.
    Requested {
        /// The `conductor.host.DiskRequested` this outcome publishes.
        disk_requested: DiskRequested,
    },
}

/// Answer a disk request — the input of `conductor.host.ReserveDisk`.
///
/// Everything it can result in is [`ReserveDiskOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReserveDisk {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `shortfall_bytes` — `Integer`.
    pub shortfall_bytes: i64,
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
}

/// Everything `conductor.host.ReserveDisk` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReserveDiskOutcome {
    /// `invalid-size` — when `(shortfall_bytes < 0 or reclaimed_bytes < 0)`.
    ///
    /// No count is negative; the request stays open.
    InvalidSize {
        /// Why it was refused: `conductor.host.InvalidSize`.
        error: InvalidSize,
    },
    /// `refused` — when `shortfall_bytes > 0`.
    ///
    /// Reclaim could not reach the size; the refusal names the shortfall, the command never runs, and `conductor build run` exits 2.
    Refused {
        /// The `conductor.host.DiskReservationRefused` this outcome publishes.
        disk_reservation_refused: DiskReservationRefused,
    },
    /// `granted-after-reclaim` — when `reclaimed_bytes > 0`.
    ///
    /// It did not fit; a reclaim run freed the reclaimed bytes, and then it did.
    GrantedAfterReclaim {
        /// The `conductor.host.DiskReserved` this outcome publishes.
        disk_reserved: DiskReserved,
    },
    /// `granted` — otherwise.
    ///
    /// It fits; the bytes count against the host until the grant is released.
    Granted {
        /// The `conductor.host.DiskReserved` this outcome publishes.
        disk_reserved: DiskReserved,
    },
    /// `no-such-grant` — for an identity no record carries.
    NoSuchGrant {
        /// Why it was refused: `conductor.host.HostGrantNotFound`.
        error: HostGrantNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The request was answered already. Only RequestDisk creates a grant in Requested, so a slot, created granted, is answered here too.
    WrongState {
        /// Why it was refused: `conductor.host.HostGrantStateConflict`.
        error: HostGrantStateConflict,
    },
}

/// Start the reserved run — the input of `conductor.host.StartRun`.
///
/// Everything it can result in is [`StartRunOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartRun {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `process_group` — `Integer`.
    pub process_group: i64,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
}

/// Everything `conductor.host.StartRun` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartRunOutcome {
    /// `not-a-disk-grant` — when the existing subject's stored fields satisfy `kind != Disk`.
    ///
    /// A slot runs no command; nothing changes.
    NotADiskGrant {
        /// Why it was refused: `conductor.host.NotADiskGrant`.
        error: NotADiskGrant,
    },
    /// `started` — otherwise.
    ///
    /// The reservation holds while the process group lives.
    Started {
        /// The `conductor.host.RunStarted` this outcome publishes.
        run_started: RunStarted,
    },
    /// `no-such-grant` — for an identity no record carries.
    NoSuchGrant {
        /// Why it was refused: `conductor.host.HostGrantNotFound`.
        error: HostGrantNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a granted reservation starts its run, once.
    WrongState {
        /// Why it was refused: `conductor.host.HostGrantStateConflict`.
        error: HostGrantStateConflict,
    },
}

/// Take a host grant — the input of `conductor.host.TakeHostGrant`.
///
/// Everything it can result in is [`TakeHostGrantOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TakeHostGrant {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `kind` — `conductor.host.GrantKind`.
    pub kind: GrantKind,
    /// `bytes` — `Optional<Integer>`.
    pub bytes: Option<i64>,
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
    /// `requested_at` — `Timestamp`.
    pub requested_at: crate::primitives::Timestamp,
}

/// Everything `conductor.host.TakeHostGrant` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakeHostGrantOutcome {
    /// `disk-through-reservation` — when `kind == Disk`.
    ///
    /// A disk grant is a reservation for a command; nothing was taken.
    DiskThroughReservation {
        /// Why it was refused: `conductor.host.DiskThroughReservation`.
        error: DiskThroughReservation,
    },
    /// `granted` — otherwise.
    ///
    /// The slot counts against the host until it is released.
    Granted {
        /// The `conductor.host.HostGrantTaken` this outcome publishes.
        host_grant_taken: HostGrantTaken,
    },
}

/// Withdraw a waiting disk request — the input of `conductor.host.WithdrawDiskRequest`.
///
/// Everything it can result in is [`WithdrawDiskRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawDiskRequest {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `withdrawn_at` — `Timestamp`.
    pub withdrawn_at: crate::primitives::Timestamp,
}

/// Everything `conductor.host.WithdrawDiskRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WithdrawDiskRequestOutcome {
    /// `withdrawn` — otherwise.
    ///
    /// The request leaves the queue; the requests behind it move up.
    Withdrawn {
        /// The `conductor.host.DiskRequestWithdrawn` this outcome publishes.
        disk_request_withdrawn: DiskRequestWithdrawn,
    },
    /// `no-such-grant` — for an identity no record carries.
    ///
    /// No grant with that id.
    NoSuchGrant {
        /// Why it was refused: `conductor.host.HostGrantNotFound`.
        error: HostGrantNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a request still waiting is withdrawn; an answered one is released or was refused.
    WrongState {
        /// Why it was refused: `conductor.host.HostGrantStateConflict`.
        error: HostGrantStateConflict,
    },
}

/// DiskReclaimed — the event `conductor.host.DiskReclaimed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskReclaimed {
    /// `reclaim_run_id` — `conductor.host.ReclaimRunId`.
    pub reclaim_run_id: ReclaimRunId,
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
}

/// DiskRequestWithdrawn — the event `conductor.host.DiskRequestWithdrawn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskRequestWithdrawn {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
}

/// DiskRequested — the event `conductor.host.DiskRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskRequested {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `bytes` — `Integer`.
    pub bytes: i64,
}

/// DiskReservationRefused — the event `conductor.host.DiskReservationRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskReservationRefused {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `bytes` — `Optional<Integer>`.
    pub bytes: Option<i64>,
    /// `shortfall_bytes` — `Integer`.
    pub shortfall_bytes: i64,
}

/// DiskReserved — the event `conductor.host.DiskReserved`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskReserved {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `bytes` — `Optional<Integer>`.
    pub bytes: Option<i64>,
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
}

/// HostGrantReleased — the event `conductor.host.HostGrantReleased`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostGrantReleased {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `reason` — `Optional<conductor.host.ReleaseReason>`.
    pub reason: Option<ReleaseReason>,
}

/// HostGrantTaken — the event `conductor.host.HostGrantTaken`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostGrantTaken {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `kind` — `conductor.host.GrantKind`.
    pub kind: GrantKind,
}

/// PeakRecorded — the event `conductor.host.PeakRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeakRecorded {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `peak_bytes` — `Integer`.
    pub peak_bytes: i64,
}

/// ReclaimFloorNotReached — the event `conductor.host.ReclaimFloorNotReached`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimFloorNotReached {
    /// `reclaim_run_id` — `conductor.host.ReclaimRunId`.
    pub reclaim_run_id: ReclaimRunId,
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
    /// `shortfall_bytes` — `Integer`.
    pub shortfall_bytes: i64,
}

/// RunStarted — the event `conductor.host.RunStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStarted {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `process_group` — `Integer`.
    pub process_group: i64,
}

/// The declared error `conductor.host.DiskThroughReservation`.
///
/// Disk is taken through a disk reservation, not as a slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskThroughReservation {
    /// `kind` — `conductor.host.GrantKind`.
    pub kind: GrantKind,
}

/// The declared error `conductor.host.HostGrantNotFound`.
///
/// No host grant has that id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostGrantNotFound {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
}

/// The declared error `conductor.host.HostGrantStateConflict`.
///
/// The host grant is not in a state the command moves it from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostGrantStateConflict {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
}

/// The declared error `conductor.host.InvalidSize`.
///
/// A size is negative, or a reservation asks for nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSize {
    /// `bytes` — `Integer`.
    pub bytes: i64,
}

/// The declared error `conductor.host.NotADiskGrant`.
///
/// The host grant is a slot, not a disk reservation; it runs no command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotADiskGrant {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
}

/// Host grants — one row of the view `conductor.host.Grants`.
///
/// Projects `conductor.host.HostGrant` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grants {
    /// `grant_id` — `conductor.host.GrantId`.
    pub grant_id: GrantId,
    /// `state` — `conductor.host.HostGrant.State`.
    pub state: HostGrantState,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `repository` — `Optional<conductor.observation.RepositoryName>`.
    pub repository: Option<crate::observation::RepositoryName>,
    /// `kind` — `conductor.host.GrantKind`.
    pub kind: GrantKind,
    /// `bytes` — `Optional<Integer>`.
    pub bytes: Option<i64>,
    /// `peak_bytes` — `Optional<Integer>`.
    pub peak_bytes: Option<i64>,
    /// `command` — `Optional<String>`.
    pub command: Option<String>,
    /// `process_group` — `Optional<Integer>`.
    pub process_group: Option<i64>,
    /// `session_name` — `Optional<conductor.config.SessionName>`.
    pub session_name: Option<crate::config::SessionName>,
    /// `reclaimed_bytes` — `Optional<Integer>`.
    pub reclaimed_bytes: Option<i64>,
    /// `shortfall_bytes` — `Optional<Integer>`.
    pub shortfall_bytes: Option<i64>,
    /// `requested_at` — `Timestamp`.
    pub requested_at: crate::primitives::Timestamp,
    /// `started_at` — `Optional<Timestamp>`.
    pub started_at: Option<crate::primitives::Timestamp>,
    /// `released_at` — `Optional<Timestamp>`.
    pub released_at: Option<crate::primitives::Timestamp>,
    /// `release_reason` — `Optional<conductor.host.ReleaseReason>`.
    pub release_reason: Option<ReleaseReason>,
}

/// Reclaim runs — one row of the view `conductor.host.ReclaimRuns`.
///
/// Projects `conductor.host.ReclaimRun` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimRuns {
    /// `reclaim_run_id` — `conductor.host.ReclaimRunId`.
    pub reclaim_run_id: ReclaimRunId,
    /// `state` — `conductor.host.ReclaimRun.State`.
    pub state: ReclaimRunState,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `result` — `conductor.host.ReclaimResult`.
    pub result: ReclaimResult,
    /// `steps` — `List<conductor.host.ReclaimStep>`.
    pub steps: Vec<ReclaimStep>,
    /// `reclaimed_bytes` — `Integer`.
    pub reclaimed_bytes: i64,
    /// `shortfall_bytes` — `Optional<Integer>`.
    pub shortfall_bytes: Option<i64>,
    /// `left` — `List<conductor.host.LeftOver>`.
    pub left: Vec<LeftOver>,
    /// `ran_at` — `Timestamp`.
    pub ran_at: crate::primitives::Timestamp,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.host.Reclaim` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReclaimBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.Reclaim`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn reclaim(&mut self, input: super::Reclaim) -> Result<super::ReclaimOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.host.RecordPeak` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordPeakBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.RecordPeak`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_peak(&mut self, input: super::RecordPeak) -> Result<super::RecordPeakOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.host.ReleaseGrant` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReleaseGrantBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.ReleaseGrant`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn release_grant(&mut self, input: super::ReleaseGrant) -> Result<super::ReleaseGrantOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.host.RequestDisk` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RequestDiskBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.RequestDisk`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn request_disk(&mut self, input: super::RequestDisk) -> Result<super::RequestDiskOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.host.ReserveDisk` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReserveDiskBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.ReserveDisk`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn reserve_disk(&mut self, input: super::ReserveDisk) -> Result<super::ReserveDiskOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.host.StartRun` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait StartRunBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.StartRun`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn start_run(&mut self, input: super::StartRun) -> Result<super::StartRunOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.host.TakeHostGrant` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait TakeHostGrantBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.TakeHostGrant`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn take_host_grant(&mut self, input: super::TakeHostGrant) -> Result<super::TakeHostGrantOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.host.WithdrawDiskRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait WithdrawDiskRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.host.WithdrawDiskRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn withdraw_disk_request(&mut self, input: super::WithdrawDiskRequest) -> Result<super::WithdrawDiskRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.host.Grants` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait GrantsQuery {
        /// Serves `conductor.host.Grants` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn grants(&self) -> Result<Vec<super::Grants>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.host.ReclaimRuns` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ReclaimRunsQuery {
        /// Serves `conductor.host.ReclaimRuns` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn reclaim_runs(&self) -> Result<Vec<super::ReclaimRuns>, crate::obligation::UnmetObligation>;
    }

}
