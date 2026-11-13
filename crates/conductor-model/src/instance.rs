// generated from conductor v1
// contract digest 2fc2642808077fb28855408e63bbd2024df0cf9fb9146a58853be9bd124f1c4b
// do not edit: regenerate with `ess synthesize --layout crate`

//! Instance — `conductor.instance`.
//!
//! The instances conductord reconciles (design docs/design/architecture-next.md § 4): desired state from the config (name, placement, roles, settings), actual state from what runs (state, binary version, last seen, box), and one `Reconciliation` record per action the daemon takes (start, stop, restart, upgrade, nothing). Replaces the registry types of `conductor.host`.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `conductor.instance.Instance`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Instance<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    /// `Purged`.
    Purged,
    /// `Running`.
    Running,
    /// `Starting`.
    Starting,
    /// `Stopped`.
    Stopped,
    /// `Unreachable`.
    Unreachable,
    /// `Upgrading`.
    Upgrading,
}

/// ReconcileAction — `conductor.instance.ReconcileAction`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileAction {
    /// `Start`.
    Start,
    /// `Stop`.
    Stop,
    /// `Restart`.
    Restart,
    /// `Upgrade`.
    Upgrade,
    /// `Nothing`.
    Nothing,
    /// `Purge`.
    Purge,
}

/// ReconcileReason — `conductor.instance.ReconcileReason`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileReason {
    /// `ConfigChange`.
    ConfigChange,
    /// `LostRuntime`.
    LostRuntime,
    /// `Cadence`.
    Cadence,
    /// `OperatorCommand`.
    OperatorCommand,
    /// `RunBound`.
    RunBound,
}

/// ReconcileResult — `conductor.instance.ReconcileResult`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileResult {
    /// `Succeeded`.
    Succeeded,
    /// `Refused`.
    Refused,
    /// `RolledBack`.
    RolledBack,
}

/// The states of `conductor.instance.Reconciliation`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Reconciliation<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationState {
    /// `Recorded`.
    Recorded,
}

/// ReconciliationId — `conductor.instance.ReconciliationId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationId(pub crate::primitives::Uuid);

/// ServeDaemon — `conductor.instance.ServeDaemon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeDaemon {
    /// `install` — `Optional<Boolean>`.
    pub install: Option<bool>,
    /// `uninstall` — `Optional<Boolean>`.
    pub uninstall: Option<bool>,
    /// `daemon_config` — `Optional<String>`.
    pub daemon_config: Option<String>,
}

/// StartHostRole — `conductor.instance.StartHostRole`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartHostRole {
    /// `role` — `Optional<conductor.config.RoleName>`.
    pub role: Option<crate::config::RoleName>,
}

/// UpgradeInstances — `conductor.instance.UpgradeInstances`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeInstances {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
    /// `tag` — `String`.
    pub tag: String,
    /// `all` — `Optional<Boolean>`.
    pub all: Option<bool>,
}

/// What Instance — `conductor.instance.Instance` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Instance<S>`], and at a boundary by [`InstanceSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceData {
    /// The identity: `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `placement` — `conductor.placement.PlacementName`.
    ///
    /// Carries `placement`: `conductor.instance.Instance` references one `conductor.placement.Placement`.
    pub placement: crate::placement::PlacementName,
    /// `roles` — `List<conductor.config.RoleName>`.
    pub roles: Vec<crate::config::RoleName>,
    /// `binary_version` — `Optional<String>`.
    pub binary_version: Option<String>,
    /// `last_seen` — `Optional<Timestamp>`.
    pub last_seen: Option<crate::primitives::Timestamp>,
    /// `box_id` — `Optional<conductor.placement.BoxId>`.
    ///
    /// Carries `box`: `conductor.instance.Instance` references one `conductor.placement.Box`.
    pub box_id: Option<crate::placement::BoxId>,
    /// `sources` — `List<conductor.config.Source>`.
    pub sources: Vec<crate::config::Source>,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
    /// `changed_by` — `conductor.operator.Actor`.
    pub changed_by: crate::operator::Actor,
    /// `git_identity` — `Optional<conductor.config.GitIdentity>`.
    pub git_identity: Option<crate::config::GitIdentity>,
    /// `model_token_kind` — `Optional<conductor.config.ModelTokenKind>`.
    pub model_token_kind: Option<crate::config::ModelTokenKind>,
    /// `standing_rules` — `Optional<String>`.
    pub standing_rules: Option<String>,
    /// `forge_access` — `Optional<conductor.config.ForgeAccess>`.
    pub forge_access: Option<crate::config::ForgeAccess>,
}

/// The states of `conductor.instance.Instance`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](instance_state::Marker), so [`Instance<S>`](Instance) can only ever rest in a real state.
pub mod instance_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Purged {}
        impl Sealed for super::Running {}
        impl Sealed for super::Starting {}
        impl Sealed for super::Stopped {}
        impl Sealed for super::Unreachable {}
        impl Sealed for super::Upgrading {}
    }

    /// A declared state of `Instance`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::InstanceState;
    }

    /// `Purged`. Terminal: an instance may rest here forever.
    pub struct Purged;

    impl Marker for Purged {
        const STATE: super::InstanceState = super::InstanceState::Purged;
    }

    /// `Running`.
    pub struct Running;

    impl Marker for Running {
        const STATE: super::InstanceState = super::InstanceState::Running;
    }

    /// `Starting`. Where a new instance starts.
    pub struct Starting;

    impl Marker for Starting {
        const STATE: super::InstanceState = super::InstanceState::Starting;
    }

    /// `Stopped`.
    pub struct Stopped;

    impl Marker for Stopped {
        const STATE: super::InstanceState = super::InstanceState::Stopped;
    }

    /// `Unreachable`.
    pub struct Unreachable;

    impl Marker for Unreachable {
        const STATE: super::InstanceState = super::InstanceState::Unreachable;
    }

    /// `Upgrading`.
    pub struct Upgrading;

    impl Marker for Upgrading {
        const STATE: super::InstanceState = super::InstanceState::Upgrading;
    }
}

/// Instance — `conductor.instance.Instance` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Starting`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`InstanceSnapshot`]
/// and [`InstanceSnapshot::refine`].
pub struct Instance<S: instance_state::Marker> {
    data: InstanceData,
    state: core::marker::PhantomData<S>,
}

impl<S: instance_state::Marker> Instance<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> InstanceState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &InstanceData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> InstanceData {
        self.data
    }
}

impl Instance<instance_state::Starting> {
    /// A new instance, resting in `Starting` — the only state the lifecycle starts one in.
    pub fn new(data: InstanceData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Instance<instance_state::Running> {
    /// `come_up` — `Running` → `Running`. Taken by the `answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn come_up(self) -> Instance<instance_state::Running> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `lose` — `Running` → `Unreachable`. Taken by the `not-answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn lose(self) -> Instance<instance_state::Unreachable> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stop` — `Running` → `Stopped`. Taken by the `stopped` outcome of `conductor.instance.StopInstance`.
    pub fn stop(self) -> Instance<instance_state::Stopped> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `restart` — `Running` → `Starting`. Taken by the `restarted` outcome of `conductor.instance.RestartInstance`.
    pub fn restart(self) -> Instance<instance_state::Starting> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `upgrade` — `Running` → `Upgrading`. Taken by the `upgrading` outcome of `conductor.instance.UpgradeInstance`.
    pub fn upgrade(self) -> Instance<instance_state::Upgrading> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Instance<instance_state::Starting> {
    /// `come_up` — `Starting` → `Running`. Taken by the `answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn come_up(self) -> Instance<instance_state::Running> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `lose` — `Starting` → `Unreachable`. Taken by the `not-answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn lose(self) -> Instance<instance_state::Unreachable> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stop` — `Starting` → `Stopped`. Taken by the `stopped` outcome of `conductor.instance.StopInstance`.
    pub fn stop(self) -> Instance<instance_state::Stopped> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Instance<instance_state::Stopped> {
    /// `start` — `Stopped` → `Starting`. Taken by the `started` outcome of `conductor.instance.StartInstance`.
    pub fn start(self) -> Instance<instance_state::Starting> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `purge` — `Stopped` → `Purged`. Taken by the `purged` outcome of `conductor.instance.PurgeInstance`.
    pub fn purge(self) -> Instance<instance_state::Purged> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Instance<instance_state::Unreachable> {
    /// `come_up` — `Unreachable` → `Running`. Taken by the `answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn come_up(self) -> Instance<instance_state::Running> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `lose` — `Unreachable` → `Unreachable`. Taken by the `not-answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn lose(self) -> Instance<instance_state::Unreachable> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stop` — `Unreachable` → `Stopped`. Taken by the `stopped` outcome of `conductor.instance.StopInstance`.
    pub fn stop(self) -> Instance<instance_state::Stopped> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `restart` — `Unreachable` → `Starting`. Taken by the `restarted` outcome of `conductor.instance.RestartInstance`.
    pub fn restart(self) -> Instance<instance_state::Starting> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `roll_back` — `Unreachable` → `Starting`. Taken by the `rolled-back` outcome of `conductor.instance.UpgradeInstance`.
    pub fn roll_back(self) -> Instance<instance_state::Starting> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Instance<instance_state::Upgrading> {
    /// `come_up` — `Upgrading` → `Running`. Taken by the `answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn come_up(self) -> Instance<instance_state::Running> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `lose` — `Upgrading` → `Unreachable`. Taken by the `not-answering` outcome of `conductor.instance.ObserveInstance`.
    pub fn lose(self) -> Instance<instance_state::Unreachable> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stop` — `Upgrading` → `Stopped`. Taken by the `stopped` outcome of `conductor.instance.StopInstance`.
    pub fn stop(self) -> Instance<instance_state::Stopped> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `roll_back` — `Upgrading` → `Starting`. Taken by the `rolled-back` outcome of `conductor.instance.UpgradeInstance`.
    pub fn roll_back(self) -> Instance<instance_state::Starting> {
        Instance {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.instance.Instance` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`InstanceSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: InstanceState,
    /// What it holds.
    pub data: InstanceData,
}

/// An `Instance` in whichever declared state it was found.
pub enum AnyInstance {
    /// Resting in `Purged`.
    Purged(Instance<instance_state::Purged>),
    /// Resting in `Running`.
    Running(Instance<instance_state::Running>),
    /// Resting in `Starting`.
    Starting(Instance<instance_state::Starting>),
    /// Resting in `Stopped`.
    Stopped(Instance<instance_state::Stopped>),
    /// Resting in `Unreachable`.
    Unreachable(Instance<instance_state::Unreachable>),
    /// Resting in `Upgrading`.
    Upgrading(Instance<instance_state::Upgrading>),
}

impl InstanceSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `InstanceState` cannot spell one.
    pub fn refine(self) -> AnyInstance {
        match self.state {
            InstanceState::Purged => AnyInstance::Purged(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            InstanceState::Running => AnyInstance::Running(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            InstanceState::Starting => AnyInstance::Starting(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            InstanceState::Stopped => AnyInstance::Stopped(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            InstanceState::Unreachable => AnyInstance::Unreachable(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            InstanceState::Upgrading => AnyInstance::Upgrading(Instance {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyInstance {
    /// The state, as the runtime value.
    pub fn state(&self) -> InstanceState {
        match self {
            Self::Purged(_) => InstanceState::Purged,
            Self::Running(_) => InstanceState::Running,
            Self::Starting(_) => InstanceState::Starting,
            Self::Stopped(_) => InstanceState::Stopped,
            Self::Unreachable(_) => InstanceState::Unreachable,
            Self::Upgrading(_) => InstanceState::Upgrading,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> InstanceSnapshot {
        match self {
            Self::Purged(instance) => InstanceSnapshot {
                state: InstanceState::Purged,
                data: instance.into_data(),
            },
            Self::Running(instance) => InstanceSnapshot {
                state: InstanceState::Running,
                data: instance.into_data(),
            },
            Self::Starting(instance) => InstanceSnapshot {
                state: InstanceState::Starting,
                data: instance.into_data(),
            },
            Self::Stopped(instance) => InstanceSnapshot {
                state: InstanceState::Stopped,
                data: instance.into_data(),
            },
            Self::Unreachable(instance) => InstanceSnapshot {
                state: InstanceState::Unreachable,
                data: instance.into_data(),
            },
            Self::Upgrading(instance) => InstanceSnapshot {
                state: InstanceState::Upgrading,
                data: instance.into_data(),
            },
        }
    }
}

/// What Reconciliation — `conductor.instance.Reconciliation` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Reconciliation<S>`], and at a boundary by [`ReconciliationSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationData {
    /// The identity: `reconciliation_id` — `conductor.instance.ReconciliationId`.
    pub reconciliation_id: ReconciliationId,
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `subject`: `conductor.instance.Reconciliation` references one `conductor.instance.Instance`.
    pub instance: crate::config::InstanceName,
    /// `action` — `conductor.instance.ReconcileAction`.
    pub action: ReconcileAction,
    /// `reason` — `conductor.instance.ReconcileReason`.
    pub reason: ReconcileReason,
    /// `result` — `conductor.instance.ReconcileResult`.
    pub result: ReconcileResult,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// The states of `conductor.instance.Reconciliation`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](reconciliation_state::Marker), so [`Reconciliation<S>`](Reconciliation) can only ever rest in a real state.
pub mod reconciliation_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `Reconciliation`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ReconciliationState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ReconciliationState = super::ReconciliationState::Recorded;
    }
}

/// Reconciliation — `conductor.instance.Reconciliation` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ReconciliationSnapshot`]
/// and [`ReconciliationSnapshot::refine`].
pub struct Reconciliation<S: reconciliation_state::Marker> {
    data: ReconciliationData,
    state: core::marker::PhantomData<S>,
}

impl<S: reconciliation_state::Marker> Reconciliation<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ReconciliationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ReconciliationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ReconciliationData {
        self.data
    }
}

impl Reconciliation<reconciliation_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ReconciliationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.instance.Reconciliation` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ReconciliationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ReconciliationState,
    /// What it holds.
    pub data: ReconciliationData,
}

/// An `Reconciliation` in whichever declared state it was found.
pub enum AnyReconciliation {
    /// Resting in `Recorded`.
    Recorded(Reconciliation<reconciliation_state::Recorded>),
}

impl ReconciliationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ReconciliationState` cannot spell one.
    pub fn refine(self) -> AnyReconciliation {
        match self.state {
            ReconciliationState::Recorded => AnyReconciliation::Recorded(Reconciliation {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyReconciliation {
    /// The state, as the runtime value.
    pub fn state(&self) -> ReconciliationState {
        match self {
            Self::Recorded(_) => ReconciliationState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ReconciliationSnapshot {
        match self {
            Self::Recorded(instance) => ReconciliationSnapshot {
                state: ReconciliationState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// Observe an instance — the input of `conductor.instance.ObserveInstance`.
///
/// Everything it can result in is [`ObserveInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObserveInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `answering` — `Boolean`.
    pub answering: bool,
    /// `binary_version` — `Optional<String>`.
    pub binary_version: Option<String>,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.instance.ObserveInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObserveInstanceOutcome {
    /// `unknown` — for an identity no record carries.
    ///
    /// No record of the instance is kept; nothing was observed.
    Unknown {
        /// Why it was refused: `conductor.instance.UnknownInstance`.
        error: UnknownInstance,
    },
    /// `answering` — when `answering == true`.
    ///
    /// The runtime answers; the instance runs, with the version it reported and the time it answered.
    Answering {
        /// The `conductor.instance.InstanceAnswered` this outcome publishes.
        instance_answered: InstanceAnswered,
    },
    /// `not-answering` — otherwise.
    ///
    /// The runtime does not answer; the instance is a row with state Unreachable and the time it last answered.
    NotAnswering {
        /// The `conductor.instance.InstanceUnreachable` this outcome publishes.
        instance_unreachable: InstanceUnreachable,
    },
    /// `not-running` — from a state no declared move starts in.
    ///
    /// A stopped or purged instance has no runtime to observe; nothing changed.
    NotRunning {
        /// Why it was refused: `conductor.instance.InstanceStateConflict`.
        error: InstanceStateConflict,
    },
}

/// Purge an instance — the input of `conductor.instance.PurgeInstance`.
///
/// Everything it can result in is [`PurgeInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.instance.PurgeInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurgeInstanceOutcome {
    /// `actor-not-human` — when `actor.kind != Human`.
    ///
    /// A purge is a class H decision; an agent actor is refused by name and nothing is deleted.
    ActorNotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: crate::operator::ActorNotHuman,
    },
    /// `unknown` — for an identity no record carries.
    ///
    /// No record of the instance is kept; nothing was purged.
    Unknown {
        /// Why it was refused: `conductor.instance.UnknownInstance`.
        error: UnknownInstance,
    },
    /// `purged` — otherwise.
    ///
    /// The operator said yes; the instance's data is deleted and the instance is Purged.
    Purged {
        /// The `conductor.instance.InstancePurged` this outcome publishes.
        instance_purged: InstancePurged,
    },
    /// `not-stopped` — from a state no declared move starts in.
    ///
    /// Only a stopped instance is purged; nothing was deleted.
    NotStopped {
        /// Why it was refused: `conductor.instance.InstanceStateConflict`.
        error: InstanceStateConflict,
    },
}

/// Record a reconciliation — the input of `conductor.instance.Reconcile`.
///
/// Everything it can result in is [`ReconcileOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconcile {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `action` — `conductor.instance.ReconcileAction`.
    pub action: ReconcileAction,
    /// `reason` — `conductor.instance.ReconcileReason`.
    pub reason: ReconcileReason,
    /// `result` — `conductor.instance.ReconcileResult`.
    pub result: ReconcileResult,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.instance.Reconcile` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileOutcome {
    /// `recorded` — otherwise.
    ///
    /// The action is on record in the daemon's store with its reason and result.
    Recorded {
        /// The `conductor.instance.InstanceReconciled` this outcome publishes.
        instance_reconciled: InstanceReconciled,
    },
}

/// Restart an instance — the input of `conductor.instance.RestartInstance`.
///
/// Everything it can result in is [`RestartInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: crate::placement::PlacementName,
    /// `roles` — `List<conductor.config.RoleName>`.
    pub roles: Vec<crate::config::RoleName>,
    /// `sources` — `List<conductor.config.Source>`.
    pub sources: Vec<crate::config::Source>,
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: crate::placement::BoxId,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.instance.RestartInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartInstanceOutcome {
    /// `unknown` — for an identity no record carries.
    ///
    /// No record of the instance is kept; nothing was restarted.
    Unknown {
        /// Why it was refused: `conductor.instance.UnknownInstance`.
        error: UnknownInstance,
    },
    /// `placement-refused` — externally decided (substrate refused the box the placement asks for).
    ///
    /// The instance does not come back and nothing falls back; the daemon records why.
    PlacementRefused {
        /// Why it was refused: `conductor.instance.PlacementRefused`.
        error: PlacementRefused,
    },
    /// `restarted` — otherwise.
    ///
    /// The runtime is stopped and started again in a box at the placement the config names.
    Restarted {
        /// The `conductor.instance.InstanceRestarted` this outcome publishes.
        instance_restarted: InstanceRestarted,
    },
    /// `not-running` — from a state no declared move starts in.
    ///
    /// Only a running or unreachable instance is restarted; nothing changed.
    NotRunning {
        /// Why it was refused: `conductor.instance.InstanceStateConflict`.
        error: InstanceStateConflict,
    },
}

/// Start an instance — the input of `conductor.instance.StartInstance`.
///
/// Everything it can result in is [`StartInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: crate::placement::PlacementName,
    /// `roles` — `List<conductor.config.RoleName>`.
    pub roles: Vec<crate::config::RoleName>,
    /// `sources` — `List<conductor.config.Source>`.
    pub sources: Vec<crate::config::Source>,
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: crate::placement::BoxId,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.instance.StartInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartInstanceOutcome {
    /// `created` — for an identity no record carries.
    ///
    /// The config names the instance and nothing ran it; its box is open, its runtime starting.
    Created {
        /// The `conductor.instance.InstanceStarted` this outcome publishes.
        instance_started: InstanceStarted,
    },
    /// `started` — when the existing subject is in Stopped.
    ///
    /// The stopped instance's box is open again and its runtime starting.
    Started {
        /// The `conductor.instance.InstanceStarted` this outcome publishes.
        instance_started: InstanceStarted,
    },
    /// `already-started` — when the existing subject is in Running or Starting or Upgrading.
    ///
    /// The instance already runs or is coming up; nothing changed.
    AlreadyStarted,
    /// `not-in-config` — externally decided (the config names no instance of this name).
    ///
    /// Nothing was started; an instance runs only where the config names it.
    NotInConfig {
        /// Why it was refused: `conductor.instance.UnknownInstance`.
        error: UnknownInstance,
    },
    /// `placement-refused` — externally decided (substrate refused the box the placement asks for).
    ///
    /// The instance does not start and nothing falls back; the daemon records why.
    PlacementRefused {
        /// Why it was refused: `conductor.instance.PlacementRefused`.
        error: PlacementRefused,
    },
    /// `not-startable` — otherwise.
    ///
    /// An unreachable instance is restarted, not started, and a purged one is not started; nothing changed.
    NotStartable {
        /// Why it was refused: `conductor.instance.InstanceStateConflict`.
        error: InstanceStateConflict,
    },
}

/// Stop an instance — the input of `conductor.instance.StopInstance`.
///
/// Everything it can result in is [`StopInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.instance.StopInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopInstanceOutcome {
    /// `unknown` — for an identity no record carries.
    ///
    /// No record of the instance is kept; nothing was stopped.
    Unknown {
        /// Why it was refused: `conductor.instance.UnknownInstance`.
        error: UnknownInstance,
    },
    /// `stopped` — when the existing subject is in Running or Starting or Unreachable or Upgrading.
    ///
    /// The runtime is stopped and its box closed; the instance's records are kept.
    Stopped {
        /// The `conductor.instance.InstanceStopped` this outcome publishes.
        instance_stopped: InstanceStopped,
    },
    /// `already-stopped` — when the existing subject is in Stopped.
    ///
    /// The instance is stopped already; nothing changed.
    AlreadyStopped,
    /// `purged` — otherwise.
    ///
    /// A purged instance has nothing to stop; nothing changed.
    Purged {
        /// Why it was refused: `conductor.instance.InstanceStateConflict`.
        error: InstanceStateConflict,
    },
}

/// Upgrade an instance — the input of `conductor.instance.UpgradeInstance`.
///
/// Everything it can result in is [`UpgradeInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `binary_version` — `String`.
    pub binary_version: String,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.instance.UpgradeInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeInstanceOutcome {
    /// `unknown` — for an identity no record carries.
    ///
    /// No record of the instance is kept; nothing was upgraded.
    Unknown {
        /// Why it was refused: `conductor.instance.UnknownInstance`.
        error: UnknownInstance,
    },
    /// `upgrading` — when the existing subject is in Running.
    ///
    /// The new binary is installed and the runtime restarted on it; the instance runs again once it answers.
    Upgrading {
        /// The `conductor.instance.InstanceUpgrading` this outcome publishes.
        instance_upgrading: InstanceUpgrading,
    },
    /// `rolled-back` — when the existing subject is in Unreachable or Upgrading.
    ///
    /// The upgraded runtime did not come up; the previous binary is back and its runtime starting.
    RolledBack {
        /// The `conductor.instance.InstanceRolledBack` this outcome publishes.
        instance_rolled_back: InstanceRolledBack,
    },
    /// `not-running` — otherwise.
    ///
    /// Only a running instance is upgraded; nothing changed.
    NotRunning {
        /// Why it was refused: `conductor.instance.InstanceStateConflict`.
        error: InstanceStateConflict,
    },
}

/// InstanceAnswered — the event `conductor.instance.InstanceAnswered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceAnswered {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `binary_version` — `Optional<String>`.
    pub binary_version: Option<String>,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstancePurged — the event `conductor.instance.InstancePurged`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstancePurged {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstanceReconciled — the event `conductor.instance.InstanceReconciled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceReconciled {
    /// `reconciliation_id` — `conductor.instance.ReconciliationId`.
    pub reconciliation_id: ReconciliationId,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `action` — `conductor.instance.ReconcileAction`.
    pub action: ReconcileAction,
    /// `reason` — `conductor.instance.ReconcileReason`.
    pub reason: ReconcileReason,
    /// `result` — `conductor.instance.ReconcileResult`.
    pub result: ReconcileResult,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstanceRestarted — the event `conductor.instance.InstanceRestarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRestarted {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: crate::placement::PlacementName,
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: crate::placement::BoxId,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstanceRolledBack — the event `conductor.instance.InstanceRolledBack`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRolledBack {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `binary_version` — `String`.
    pub binary_version: String,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstanceStarted — the event `conductor.instance.InstanceStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceStarted {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: crate::placement::PlacementName,
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: crate::placement::BoxId,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstanceStopped — the event `conductor.instance.InstanceStopped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceStopped {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstanceUnreachable — the event `conductor.instance.InstanceUnreachable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceUnreachable {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// InstanceUpgrading — the event `conductor.instance.InstanceUpgrading`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceUpgrading {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `binary_version` — `String`.
    pub binary_version: String,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// The declared error `conductor.instance.InstanceStateConflict`.
///
/// The instance is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceStateConflict {
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
}

/// The declared error `conductor.instance.PlacementRefused`.
///
/// The placement could not give what the profile asks for; substrate's reason is named, the instance does not start, and nothing falls back (§ 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRefused {
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: crate::placement::PlacementName,
}

/// The declared error `conductor.instance.SupervisorNotRunning`.
///
/// No supervisor runs to take the question, so it was not delivered; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorNotRunning {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
}

/// The declared error `conductor.instance.UnknownInstance`.
///
/// The config names no instance of this name, or no record of it is kept; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownInstance {
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
}

/// One instance — one row of the view `conductor.instance.InstanceByName`.
///
/// Projects `conductor.instance.Instance` at `read_your_writes` consistency, containing instances where `name == param.name`.
/// Serving it is an implementation obligation — see the plan — because how a projection is kept
/// current is a storage decision the specification does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceByName {
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `state` — `conductor.instance.Instance.State`.
    pub state: InstanceState,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: crate::placement::PlacementName,
    /// `roles` — `List<conductor.config.RoleName>`.
    pub roles: Vec<crate::config::RoleName>,
    /// `binary_version` — `Optional<String>`.
    pub binary_version: Option<String>,
    /// `last_seen` — `Optional<Timestamp>`.
    pub last_seen: Option<crate::primitives::Timestamp>,
    /// `box_id` — `Optional<conductor.placement.BoxId>`.
    pub box_id: Option<crate::placement::BoxId>,
    /// `sources` — `List<conductor.config.Source>`.
    pub sources: Vec<crate::config::Source>,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
    /// `changed_by` — `conductor.operator.Actor`.
    pub changed_by: crate::operator::Actor,
    /// `git_identity` — `Optional<conductor.config.GitIdentity>`.
    pub git_identity: Option<crate::config::GitIdentity>,
    /// `model_token_kind` — `Optional<conductor.config.ModelTokenKind>`.
    pub model_token_kind: Option<crate::config::ModelTokenKind>,
    /// `standing_rules` — `Optional<String>`.
    pub standing_rules: Option<String>,
    /// `forge_access` — `Optional<conductor.config.ForgeAccess>`.
    pub forge_access: Option<crate::config::ForgeAccess>,
}

/// Instances — one row of the view `conductor.instance.Instances`.
///
/// Projects `conductor.instance.Instance` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instances {
    /// `name` — `conductor.config.InstanceName`.
    pub name: crate::config::InstanceName,
    /// `state` — `conductor.instance.Instance.State`.
    pub state: InstanceState,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: crate::placement::PlacementName,
    /// `roles` — `List<conductor.config.RoleName>`.
    pub roles: Vec<crate::config::RoleName>,
    /// `binary_version` — `Optional<String>`.
    pub binary_version: Option<String>,
    /// `last_seen` — `Optional<Timestamp>`.
    pub last_seen: Option<crate::primitives::Timestamp>,
    /// `box_id` — `Optional<conductor.placement.BoxId>`.
    pub box_id: Option<crate::placement::BoxId>,
    /// `sources` — `List<conductor.config.Source>`.
    pub sources: Vec<crate::config::Source>,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
    /// `changed_by` — `conductor.operator.Actor`.
    pub changed_by: crate::operator::Actor,
}

/// Reconciliations — one row of the view `conductor.instance.Reconciliations`.
///
/// Projects `conductor.instance.Reconciliation` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciliations {
    /// `reconciliation_id` — `conductor.instance.ReconciliationId`.
    pub reconciliation_id: ReconciliationId,
    /// `state` — `conductor.instance.Reconciliation.State`.
    pub state: ReconciliationState,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `action` — `conductor.instance.ReconcileAction`.
    pub action: ReconcileAction,
    /// `reason` — `conductor.instance.ReconcileReason`.
    pub reason: ReconcileReason,
    /// `result` — `conductor.instance.ReconcileResult`.
    pub result: ReconcileResult,
    /// `detail` — `Optional<String>`.
    pub detail: Option<String>,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.instance.ObserveInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ObserveInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.instance.ObserveInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn observe_instance(&mut self, input: super::ObserveInstance) -> Result<super::ObserveInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.instance.PurgeInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PurgeInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.instance.PurgeInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn purge_instance(&mut self, input: super::PurgeInstance) -> Result<super::PurgeInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.instance.Reconcile` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReconcileBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.instance.Reconcile`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn reconcile(&mut self, input: super::Reconcile) -> Result<super::ReconcileOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.instance.RestartInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RestartInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.instance.RestartInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn restart_instance(&mut self, input: super::RestartInstance) -> Result<super::RestartInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.instance.StartInstance` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by `when_subject_state:` beside `external:` in one command.
    ///
    /// Contract: given `conductor.instance.StartInstance` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `created` for an identity no record carries, creates `conductor.instance.Instance`, emits `conductor.instance.InstanceStarted`; `started` when the existing subject is in Stopped, takes `start` of `conductor.instance.Instance`, emits `conductor.instance.InstanceStarted`; `already-started` when the existing subject is in Running or Starting or Upgrading, preserves `conductor.instance.Instance`; `not-in-config` externally decided (the config names no instance of this name), error `conductor.instance.UnknownInstance`; `placement-refused` externally decided (substrate refused the box the placement asks for), error `conductor.instance.PlacementRefused`; `not-startable` otherwise, error `conductor.instance.InstanceStateConflict`.
    pub trait StartInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.instance.StartInstance`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn start_instance(&mut self, input: super::StartInstance) -> Result<super::StartInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.instance.StopInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait StopInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.instance.StopInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn stop_instance(&mut self, input: super::StopInstance) -> Result<super::StopInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.instance.UpgradeInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait UpgradeInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.instance.UpgradeInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn upgrade_instance(&mut self, input: super::UpgradeInstance) -> Result<super::UpgradeInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.instance.InstanceByName` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a view parameter (`params:`), which a generated query does not apply.
    ///
    /// Contract: a query answering `conductor.instance.InstanceByName` with rows projected from `conductor.instance.Instance` at `read_your_writes` consistency, containing instances where `name == param.name`.
    pub trait InstanceByNameQuery {
        /// Serves `conductor.instance.InstanceByName` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn instance_by_name(&self, name: crate::config::InstanceName) -> Result<Vec<super::InstanceByName>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.instance.Instances` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait InstancesQuery {
        /// Serves `conductor.instance.Instances` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn instances(&self) -> Result<Vec<super::Instances>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.instance.Reconciliations` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ReconciliationsQuery {
        /// Serves `conductor.instance.Reconciliations` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn reconciliations(&self) -> Result<Vec<super::Reconciliations>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl StartInstanceBehavior for Unimplemented {
        fn start_instance(&mut self, _input: super::StartInstance) -> Result<super::StartInstanceOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.instance.StartInstance" })
        }
    }

    impl InstanceByNameQuery for Unimplemented {
        fn instance_by_name(&self, _name: crate::config::InstanceName) -> Result<Vec<super::InstanceByName>, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "view query", source: "conductor.instance.InstanceByName" })
        }
    }
}
