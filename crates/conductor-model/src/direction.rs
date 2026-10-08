// generated from conductor v1
// model digest da7f223adaed415012785783afd834b78fee21f26d65fe9977c5813f8c3f9b33
// contract digest 2cad3a53f65176fd47da0cb4583a9f915aca1519194cf0b0d6ce4fcde6f0b33c
// do not edit: regenerate with `ess synthesize --layout crate`

//! Direction — `conductor.direction`.
//!
//! The goals conductor dispatches against, and which repositories it processes. A goal is proposed by conductor, confirmed or dropped by the operator, and marked met by conductor on the evidence it names; NORTHSTAR.md in conductor's records is the prose view. Each goal lists the repositories whose work serves it. A repository mark overrides computed activity (design § 10).
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Activity — `conductor.direction.Activity`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activity {
    /// `Active`.
    Active,
    /// `Inactive`.
    Inactive,
}

/// DecidedBy — `conductor.direction.DecidedBy`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecidedBy {
    /// `Mark`.
    Mark,
    /// `Snapshot`.
    Snapshot,
}

/// The states of `conductor.direction.Goal`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Goal<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalState {
    /// `Confirmed`.
    Confirmed,
    /// `Draft`.
    Draft,
    /// `Dropped`.
    Dropped,
    /// `Met`.
    Met,
}

/// GoalId — `conductor.direction.GoalId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalId(pub String);

/// The states of `conductor.direction.GoalServing`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `GoalServing<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalServingState {
    /// `Removed`.
    Removed,
    /// `Serving`.
    Serving,
}

/// MarkedBy — `conductor.direction.MarkedBy`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkedBy {
    /// `Conductor`.
    Conductor,
    /// `Operator`.
    Operator,
}

/// MarkedRepository — `conductor.direction.MarkedRepository`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkedRepository(pub String);

/// RepositoryActivity — `conductor.direction.RepositoryActivity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryActivity {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// RepositoryActivityRow — `conductor.direction.RepositoryActivityRow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryActivityRow {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `activity` — `conductor.direction.Activity`.
    pub activity: Activity,
    /// `decided_by` — `conductor.direction.DecidedBy`.
    pub decided_by: DecidedBy,
    /// `marked_by` — `Optional<conductor.direction.MarkedBy>`.
    pub marked_by: Option<MarkedBy>,
    /// `reason` — `Optional<String>`.
    pub reason: Option<String>,
}

/// The states of `conductor.direction.RepositoryMark`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `RepositoryMark<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryMarkState {
    /// `Active`.
    Active,
    /// `Inactive`.
    Inactive,
}

/// ServingId — `conductor.direction.ServingId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingId(pub crate::primitives::Uuid);

/// What Goal — `conductor.direction.Goal` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Goal<S>`], and at a boundary by [`GoalSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalData {
    /// The identity: `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `title` — `String`.
    pub title: String,
    /// `exit_evidence` — `String`.
    pub exit_evidence: String,
}

/// The states of `conductor.direction.Goal`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](goal_state::Marker), so [`Goal<S>`](Goal) can only ever rest in a real state.
pub mod goal_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Confirmed {}
        impl Sealed for super::Draft {}
        impl Sealed for super::Dropped {}
        impl Sealed for super::Met {}
    }

    /// A declared state of `Goal`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::GoalState;
    }

    /// `Confirmed`.
    pub struct Confirmed;

    impl Marker for Confirmed {
        const STATE: super::GoalState = super::GoalState::Confirmed;
    }

    /// `Draft`. Where a new instance starts.
    pub struct Draft;

    impl Marker for Draft {
        const STATE: super::GoalState = super::GoalState::Draft;
    }

    /// `Dropped`. Terminal: an instance may rest here forever.
    pub struct Dropped;

    impl Marker for Dropped {
        const STATE: super::GoalState = super::GoalState::Dropped;
    }

    /// `Met`. Terminal: an instance may rest here forever.
    pub struct Met;

    impl Marker for Met {
        const STATE: super::GoalState = super::GoalState::Met;
    }
}

/// Goal — `conductor.direction.Goal` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Draft`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`GoalSnapshot`]
/// and [`GoalSnapshot::refine`].
pub struct Goal<S: goal_state::Marker> {
    data: GoalData,
    state: core::marker::PhantomData<S>,
}

impl<S: goal_state::Marker> Goal<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> GoalState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &GoalData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> GoalData {
        self.data
    }
}

impl Goal<goal_state::Draft> {
    /// A new instance, resting in `Draft` — the only state the lifecycle starts one in.
    pub fn new(data: GoalData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Goal<goal_state::Confirmed> {
    /// `meet` — `Confirmed` → `Met`. Taken by the `met` outcome of `conductor.direction.MarkGoalMet`.
    pub fn meet(self) -> Goal<goal_state::Met> {
        Goal {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `drop` — `Confirmed` → `Dropped`. Taken by the `dropped` outcome of `conductor.direction.DropGoal`.
    pub fn drop(self) -> Goal<goal_state::Dropped> {
        Goal {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Goal<goal_state::Draft> {
    /// `confirm` — `Draft` → `Confirmed`. Taken by the `confirmed` outcome of `conductor.direction.ConfirmGoal`.
    pub fn confirm(self) -> Goal<goal_state::Confirmed> {
        Goal {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `drop` — `Draft` → `Dropped`. Taken by the `dropped` outcome of `conductor.direction.DropGoal`.
    pub fn drop(self) -> Goal<goal_state::Dropped> {
        Goal {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.direction.Goal` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`GoalSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: GoalState,
    /// What it holds.
    pub data: GoalData,
}

/// An `Goal` in whichever declared state it was found.
pub enum AnyGoal {
    /// Resting in `Confirmed`.
    Confirmed(Goal<goal_state::Confirmed>),
    /// Resting in `Draft`.
    Draft(Goal<goal_state::Draft>),
    /// Resting in `Dropped`.
    Dropped(Goal<goal_state::Dropped>),
    /// Resting in `Met`.
    Met(Goal<goal_state::Met>),
}

impl GoalSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `GoalState` cannot spell one.
    pub fn refine(self) -> AnyGoal {
        match self.state {
            GoalState::Confirmed => AnyGoal::Confirmed(Goal {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GoalState::Draft => AnyGoal::Draft(Goal {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GoalState::Dropped => AnyGoal::Dropped(Goal {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GoalState::Met => AnyGoal::Met(Goal {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyGoal {
    /// The state, as the runtime value.
    pub fn state(&self) -> GoalState {
        match self {
            Self::Confirmed(_) => GoalState::Confirmed,
            Self::Draft(_) => GoalState::Draft,
            Self::Dropped(_) => GoalState::Dropped,
            Self::Met(_) => GoalState::Met,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> GoalSnapshot {
        match self {
            Self::Confirmed(instance) => GoalSnapshot {
                state: GoalState::Confirmed,
                data: instance.into_data(),
            },
            Self::Draft(instance) => GoalSnapshot {
                state: GoalState::Draft,
                data: instance.into_data(),
            },
            Self::Dropped(instance) => GoalSnapshot {
                state: GoalState::Dropped,
                data: instance.into_data(),
            },
            Self::Met(instance) => GoalSnapshot {
                state: GoalState::Met,
                data: instance.into_data(),
            },
        }
    }
}

/// What GoalServing — `conductor.direction.GoalServing` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`GoalServing<S>`], and at a boundary by [`GoalServingSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalServingData {
    /// The identity: `serving_id` — `conductor.direction.ServingId`.
    pub serving_id: ServingId,
    /// `goal_id` — `conductor.direction.GoalId`.
    ///
    /// Carries `servings`: `conductor.direction.Goal` owns many `conductor.direction.GoalServing`.
    pub goal_id: GoalId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
}

/// The states of `conductor.direction.GoalServing`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](goal_serving_state::Marker), so [`GoalServing<S>`](GoalServing) can only ever rest in a real state.
pub mod goal_serving_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Removed {}
        impl Sealed for super::Serving {}
    }

    /// A declared state of `GoalServing`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::GoalServingState;
    }

    /// `Removed`. Terminal: an instance may rest here forever.
    pub struct Removed;

    impl Marker for Removed {
        const STATE: super::GoalServingState = super::GoalServingState::Removed;
    }

    /// `Serving`. Where a new instance starts.
    pub struct Serving;

    impl Marker for Serving {
        const STATE: super::GoalServingState = super::GoalServingState::Serving;
    }
}

/// GoalServing — `conductor.direction.GoalServing` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Serving`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`GoalServingSnapshot`]
/// and [`GoalServingSnapshot::refine`].
pub struct GoalServing<S: goal_serving_state::Marker> {
    data: GoalServingData,
    state: core::marker::PhantomData<S>,
}

impl<S: goal_serving_state::Marker> GoalServing<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> GoalServingState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &GoalServingData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> GoalServingData {
        self.data
    }
}

impl GoalServing<goal_serving_state::Serving> {
    /// A new instance, resting in `Serving` — the only state the lifecycle starts one in.
    pub fn new(data: GoalServingData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl GoalServing<goal_serving_state::Serving> {
    /// `remove` — `Serving` → `Removed`. Taken by the `removed` outcome of `conductor.direction.RemoveServing`.
    pub fn remove(self) -> GoalServing<goal_serving_state::Removed> {
        GoalServing {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.direction.GoalServing` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`GoalServingSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalServingSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: GoalServingState,
    /// What it holds.
    pub data: GoalServingData,
}

/// An `GoalServing` in whichever declared state it was found.
pub enum AnyGoalServing {
    /// Resting in `Removed`.
    Removed(GoalServing<goal_serving_state::Removed>),
    /// Resting in `Serving`.
    Serving(GoalServing<goal_serving_state::Serving>),
}

impl GoalServingSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `GoalServingState` cannot spell one.
    pub fn refine(self) -> AnyGoalServing {
        match self.state {
            GoalServingState::Removed => AnyGoalServing::Removed(GoalServing {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            GoalServingState::Serving => AnyGoalServing::Serving(GoalServing {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyGoalServing {
    /// The state, as the runtime value.
    pub fn state(&self) -> GoalServingState {
        match self {
            Self::Removed(_) => GoalServingState::Removed,
            Self::Serving(_) => GoalServingState::Serving,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> GoalServingSnapshot {
        match self {
            Self::Removed(instance) => GoalServingSnapshot {
                state: GoalServingState::Removed,
                data: instance.into_data(),
            },
            Self::Serving(instance) => GoalServingSnapshot {
                state: GoalServingState::Serving,
                data: instance.into_data(),
            },
        }
    }
}

/// What RepositoryMark — `conductor.direction.RepositoryMark` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`RepositoryMark<S>`], and at a boundary by [`RepositoryMarkSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMarkData {
    /// The identity: `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
    /// `reason` — `String`.
    pub reason: String,
}

/// The states of `conductor.direction.RepositoryMark`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](repository_mark_state::Marker), so [`RepositoryMark<S>`](RepositoryMark) can only ever rest in a real state.
pub mod repository_mark_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Active {}
        impl Sealed for super::Inactive {}
    }

    /// A declared state of `RepositoryMark`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::RepositoryMarkState;
    }

    /// `Active`.
    pub struct Active;

    impl Marker for Active {
        const STATE: super::RepositoryMarkState = super::RepositoryMarkState::Active;
    }

    /// `Inactive`. Where a new instance starts.
    pub struct Inactive;

    impl Marker for Inactive {
        const STATE: super::RepositoryMarkState = super::RepositoryMarkState::Inactive;
    }
}

/// RepositoryMark — `conductor.direction.RepositoryMark` — with its lifecycle state carried by the type.
///
/// Its constructors rest in the states a creation lands in: `Inactive`, `Active`.
/// And the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`RepositoryMarkSnapshot`]
/// and [`RepositoryMarkSnapshot::refine`].
pub struct RepositoryMark<S: repository_mark_state::Marker> {
    data: RepositoryMarkData,
    state: core::marker::PhantomData<S>,
}

impl<S: repository_mark_state::Marker> RepositoryMark<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> RepositoryMarkState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &RepositoryMarkData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> RepositoryMarkData {
        self.data
    }
}

impl RepositoryMark<repository_mark_state::Inactive> {
    /// A new instance, resting in `Inactive` — where a creation naming no `into:` lands it.
    pub fn new(data: RepositoryMarkData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl RepositoryMark<repository_mark_state::Active> {
    /// A new instance, resting in `Active` — where a creation declared `into: Active` lands it.
    pub fn new_active(data: RepositoryMarkData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl RepositoryMark<repository_mark_state::Active> {
    /// `deactivate` — `Active` → `Inactive`. Taken by the `deactivated` outcome of `conductor.direction.DeactivateRepository`.
    pub fn deactivate(self) -> RepositoryMark<repository_mark_state::Inactive> {
        RepositoryMark {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl RepositoryMark<repository_mark_state::Inactive> {
    /// `activate` — `Inactive` → `Active`. Taken by the `activated` outcome of `conductor.direction.ActivateRepository`.
    pub fn activate(self) -> RepositoryMark<repository_mark_state::Active> {
        RepositoryMark {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.direction.RepositoryMark` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`RepositoryMarkSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMarkSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: RepositoryMarkState,
    /// What it holds.
    pub data: RepositoryMarkData,
}

/// An `RepositoryMark` in whichever declared state it was found.
pub enum AnyRepositoryMark {
    /// Resting in `Active`.
    Active(RepositoryMark<repository_mark_state::Active>),
    /// Resting in `Inactive`.
    Inactive(RepositoryMark<repository_mark_state::Inactive>),
}

impl RepositoryMarkSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `RepositoryMarkState` cannot spell one.
    pub fn refine(self) -> AnyRepositoryMark {
        match self.state {
            RepositoryMarkState::Active => AnyRepositoryMark::Active(RepositoryMark {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            RepositoryMarkState::Inactive => AnyRepositoryMark::Inactive(RepositoryMark {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyRepositoryMark {
    /// The state, as the runtime value.
    pub fn state(&self) -> RepositoryMarkState {
        match self {
            Self::Active(_) => RepositoryMarkState::Active,
            Self::Inactive(_) => RepositoryMarkState::Inactive,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> RepositoryMarkSnapshot {
        match self {
            Self::Active(instance) => RepositoryMarkSnapshot {
                state: RepositoryMarkState::Active,
                data: instance.into_data(),
            },
            Self::Inactive(instance) => RepositoryMarkSnapshot {
                state: RepositoryMarkState::Inactive,
                data: instance.into_data(),
            },
        }
    }
}

/// Mark a repository active — the input of `conductor.direction.ActivateRepository`.
///
/// Everything it can result in is [`ActivateRepositoryOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivateRepository {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.direction.ActivateRepository` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivateRepositoryOutcome {
    /// `activated` — otherwise.
    ///
    /// The repository's issues and pull requests are processed from the next cycle.
    Activated {
        /// The `conductor.direction.RepositoryActivated` this outcome publishes.
        repository_activated: RepositoryActivated,
    },
    /// `no-such-mark` — for an identity no record carries.
    NoSuchMark {
        /// Why it was refused: `conductor.direction.RepositoryMarkNotFound`.
        error: RepositoryMarkNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The repository is already active.
    WrongState {
        /// Why it was refused: `conductor.direction.RepositoryMarkConflict`.
        error: RepositoryMarkConflict,
    },
}

/// Add a serving repository to a goal — the input of `conductor.direction.AddServing`.
///
/// Everything it can result in is [`AddServingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddServing {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
}

/// Everything `conductor.direction.AddServing` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddServingOutcome {
    /// `no-such-goal` — when no `conductor.direction.Goal` carries the identity `input.goal_id` names.
    ///
    /// No goal has this id, and nothing was added.
    NoSuchGoal {
        /// Why it was refused: `conductor.direction.GoalNotFound`.
        error: GoalNotFound,
    },
    /// `goal-closed` — when the `conductor.direction.Goal` that `input.goal_id` names satisfies `state in [Met, Dropped]`.
    ///
    /// A met or dropped goal no longer directs work, so it gains no serving repository.
    GoalClosed {
        /// Why it was refused: `conductor.direction.GoalStateConflict`.
        error: GoalStateConflict,
    },
    /// `added` — otherwise.
    ///
    /// The repository's work serves the goal.
    Added {
        /// The `conductor.direction.ServingAdded` this outcome publishes.
        serving_added: ServingAdded,
    },
}

/// Confirm a goal — the input of `conductor.direction.ConfirmGoal`.
///
/// Everything it can result in is [`ConfirmGoalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmGoal {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
}

/// Everything `conductor.direction.ConfirmGoal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmGoalOutcome {
    /// `invalid-goal-id` — when `(goal_id == "" or {utf8_bytes: goal_id} > 170)`.
    ///
    /// No goal can have this id, so nothing moved.
    InvalidGoalId {
        /// Why it was refused: `conductor.direction.InvalidGoalId`.
        error: InvalidGoalId,
    },
    /// `confirmed` — otherwise.
    ///
    /// The operator confirmed the goal; dispatches may serve it.
    Confirmed {
        /// The `conductor.direction.GoalConfirmed` this outcome publishes.
        goal_confirmed: GoalConfirmed,
    },
    /// `no-such-goal` — for an identity no record carries.
    NoSuchGoal {
        /// Why it was refused: `conductor.direction.GoalNotFound`.
        error: GoalNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The goal is not a draft, so nothing moved.
    WrongState {
        /// Why it was refused: `conductor.direction.GoalStateConflict`.
        error: GoalStateConflict,
    },
}

/// Mark a repository inactive — the input of `conductor.direction.DeactivateRepository`.
///
/// Everything it can result in is [`DeactivateRepositoryOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeactivateRepository {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.direction.DeactivateRepository` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeactivateRepositoryOutcome {
    /// `deactivated` — otherwise.
    ///
    /// The repository's issues and pull requests are left alone from the next cycle.
    Deactivated {
        /// The `conductor.direction.RepositoryDeactivated` this outcome publishes.
        repository_deactivated: RepositoryDeactivated,
    },
    /// `no-such-mark` — for an identity no record carries.
    NoSuchMark {
        /// Why it was refused: `conductor.direction.RepositoryMarkNotFound`.
        error: RepositoryMarkNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The repository is already inactive.
    WrongState {
        /// Why it was refused: `conductor.direction.RepositoryMarkConflict`.
        error: RepositoryMarkConflict,
    },
}

/// Drop a goal — the input of `conductor.direction.DropGoal`.
///
/// Everything it can result in is [`DropGoalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropGoal {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.direction.DropGoal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DropGoalOutcome {
    /// `invalid-goal-id` — when `(goal_id == "" or {utf8_bytes: goal_id} > 170)`.
    ///
    /// No goal can have this id, so nothing moved.
    InvalidGoalId {
        /// Why it was refused: `conductor.direction.InvalidGoalId`.
        error: InvalidGoalId,
    },
    /// `dropped` — otherwise.
    ///
    /// The goal no longer directs work.
    Dropped {
        /// The `conductor.direction.GoalDropped` this outcome publishes.
        goal_dropped: GoalDropped,
    },
    /// `no-such-goal` — for an identity no record carries.
    NoSuchGoal {
        /// Why it was refused: `conductor.direction.GoalNotFound`.
        error: GoalNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The goal is already met or dropped.
    WrongState {
        /// Why it was refused: `conductor.direction.GoalStateConflict`.
        error: GoalStateConflict,
    },
}

/// Mark a goal met — the input of `conductor.direction.MarkGoalMet`.
///
/// Everything it can result in is [`MarkGoalMetOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkGoalMet {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `evidence` — `String`.
    pub evidence: String,
}

/// Everything `conductor.direction.MarkGoalMet` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkGoalMetOutcome {
    /// `invalid-goal-id` — when `(goal_id == "" or {utf8_bytes: goal_id} > 170)`.
    ///
    /// No goal can have this id, so nothing moved.
    InvalidGoalId {
        /// Why it was refused: `conductor.direction.InvalidGoalId`.
        error: InvalidGoalId,
    },
    /// `met` — otherwise.
    ///
    /// The goal's exit evidence holds.
    Met {
        /// The `conductor.direction.GoalMet` this outcome publishes.
        goal_met: GoalMet,
    },
    /// `no-such-goal` — for an identity no record carries.
    NoSuchGoal {
        /// Why it was refused: `conductor.direction.GoalNotFound`.
        error: GoalNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a confirmed goal can be met.
    WrongState {
        /// Why it was refused: `conductor.direction.GoalStateConflict`.
        error: GoalStateConflict,
    },
}

/// Mark a repository for the first time — the input of `conductor.direction.MarkRepository`.
///
/// Everything it can result in is [`MarkRepositoryOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkRepository {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `activity` — `conductor.direction.Activity`.
    pub activity: Activity,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.direction.MarkRepository` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkRepositoryOutcome {
    /// `already-marked` — for an identity a record already carries.
    ///
    /// A repository is marked once; activate-repository or deactivate-repository changes its mark. Nothing was written.
    AlreadyMarked {
        /// Why it was refused: `conductor.direction.RepositoryAlreadyMarked`.
        error: RepositoryAlreadyMarked,
    },
    /// `marked-active` — when `activity == Active`.
    ///
    /// Conductor processes the repository's issues and pull requests.
    MarkedActive {
        /// The `conductor.direction.RepositoryMarked` this outcome publishes.
        repository_marked: RepositoryMarked,
    },
    /// `marked-inactive` — otherwise.
    ///
    /// Conductor leaves the repository's issues and pull requests alone.
    MarkedInactive {
        /// The `conductor.direction.RepositoryMarked` this outcome publishes.
        repository_marked: RepositoryMarked,
    },
}

/// Propose a goal — the input of `conductor.direction.ProposeGoal`.
///
/// Everything it can result in is [`ProposeGoalOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposeGoal {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `title` — `String`.
    pub title: String,
    /// `exit_evidence` — `String`.
    pub exit_evidence: String,
}

/// Everything `conductor.direction.ProposeGoal` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposeGoalOutcome {
    /// `invalid-goal-id` — when `(goal_id == "" or {utf8_bytes: goal_id} > 170)`.
    ///
    /// The store cannot keep a goal under this id, so nothing was proposed.
    InvalidGoalId {
        /// Why it was refused: `conductor.direction.InvalidGoalId`.
        error: InvalidGoalId,
    },
    /// `proposed` — otherwise.
    ///
    /// The goal is a draft until the operator confirms or drops it.
    Proposed {
        /// The `conductor.direction.GoalProposed` this outcome publishes.
        goal_proposed: GoalProposed,
    },
    /// `already-proposed` — for an identity a record already carries.
    ///
    /// A goal already has this id; the held goal is unchanged.
    AlreadyProposed {
        /// Why it was refused: `conductor.direction.GoalExists`.
        error: GoalExists,
    },
}

/// Remove a serving repository from a goal — the input of `conductor.direction.RemoveServing`.
///
/// Everything it can result in is [`RemoveServingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveServing {
    /// `serving_id` — `conductor.direction.ServingId`.
    pub serving_id: ServingId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.direction.RemoveServing` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveServingOutcome {
    /// `removed` — otherwise.
    ///
    /// The repository's work no longer serves the goal.
    Removed {
        /// The `conductor.direction.ServingRemoved` this outcome publishes.
        serving_removed: ServingRemoved,
    },
    /// `no-such-serving` — for an identity no record carries.
    NoSuchServing {
        /// Why it was refused: `conductor.direction.ServingNotFound`.
        error: ServingNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The repository was already removed from the goal.
    WrongState {
        /// Why it was refused: `conductor.direction.ServingStateConflict`.
        error: ServingStateConflict,
    },
}

/// GoalConfirmed — the event `conductor.direction.GoalConfirmed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalConfirmed {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
}

/// GoalDropped — the event `conductor.direction.GoalDropped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalDropped {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `reason` — `String`.
    pub reason: String,
}

/// GoalMet — the event `conductor.direction.GoalMet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalMet {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `evidence` — `String`.
    pub evidence: String,
}

/// GoalProposed — the event `conductor.direction.GoalProposed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalProposed {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `title` — `String`.
    pub title: String,
}

/// RepositoryActivated — the event `conductor.direction.RepositoryActivated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryActivated {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
}

/// RepositoryDeactivated — the event `conductor.direction.RepositoryDeactivated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryDeactivated {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
    /// `reason` — `String`.
    pub reason: String,
}

/// RepositoryMarked — the event `conductor.direction.RepositoryMarked`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMarked {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `activity` — `conductor.direction.Activity`.
    pub activity: Activity,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
}

/// ServingAdded — the event `conductor.direction.ServingAdded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingAdded {
    /// `serving_id` — `conductor.direction.ServingId`.
    pub serving_id: ServingId,
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
}

/// ServingRemoved — the event `conductor.direction.ServingRemoved`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingRemoved {
    /// `serving_id` — `conductor.direction.ServingId`.
    pub serving_id: ServingId,
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `conductor.direction.GoalExists`.
///
/// A goal already has this id, so nothing was proposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalExists {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
}

/// The declared error `conductor.direction.GoalNotFound`.
///
/// No goal has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalNotFound {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
}

/// The declared error `conductor.direction.GoalStateConflict`.
///
/// The goal is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalStateConflict {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
}

/// The declared error `conductor.direction.InvalidGoalId`.
///
/// The goal id is empty or longer than 170 UTF-8 bytes. The store keeps a goal under its id with every byte escaped to at most 3, and keeps at most 512, so no goal can have this id and nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidGoalId {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
}

/// The declared error `conductor.direction.RepositoryAlreadyMarked`.
///
/// The repository already holds a mark, so nothing was written; activate-repository or deactivate-repository changes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryAlreadyMarked {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
}

/// The declared error `conductor.direction.RepositoryMarkConflict`.
///
/// The repository already holds that mark, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMarkConflict {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
}

/// The declared error `conductor.direction.RepositoryMarkNotFound`.
///
/// The repository has no mark yet; mark it first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMarkNotFound {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
}

/// The declared error `conductor.direction.ServingNotFound`.
///
/// No serving repository has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingNotFound {
    /// `serving_id` — `conductor.direction.ServingId`.
    pub serving_id: ServingId,
}

/// The declared error `conductor.direction.ServingStateConflict`.
///
/// The repository was already removed from the goal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServingStateConflict {
    /// `serving_id` — `conductor.direction.ServingId`.
    pub serving_id: ServingId,
}

/// Serving repositories — one row of the view `conductor.direction.GoalServings`.
///
/// Projects `conductor.direction.GoalServing` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalServings {
    /// `serving_id` — `conductor.direction.ServingId`.
    pub serving_id: ServingId,
    /// `state` — `conductor.direction.GoalServing.State`.
    pub state: GoalServingState,
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
}

/// Goals — one row of the view `conductor.direction.Goals`.
///
/// Projects `conductor.direction.Goal` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Goals {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: GoalId,
    /// `state` — `conductor.direction.Goal.State`.
    pub state: GoalState,
    /// `title` — `String`.
    pub title: String,
    /// `exit_evidence` — `String`.
    pub exit_evidence: String,
}

/// Repository marks — one row of the view `conductor.direction.RepositoryMarks`.
///
/// Projects `conductor.direction.RepositoryMark` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMarks {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: MarkedRepository,
    /// `state` — `conductor.direction.RepositoryMark.State`.
    pub state: RepositoryMarkState,
    /// `marked_by` — `conductor.direction.MarkedBy`.
    pub marked_by: MarkedBy,
    /// `reason` — `String`.
    pub reason: String,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.direction.ActivateRepository` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ActivateRepositoryBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.ActivateRepository`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn activate_repository(&mut self, input: super::ActivateRepository) -> Result<super::ActivateRepositoryOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.AddServing` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AddServingBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.AddServing`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn add_serving(&mut self, input: super::AddServing) -> Result<super::AddServingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.ConfirmGoal` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ConfirmGoalBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.ConfirmGoal`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn confirm_goal(&mut self, input: super::ConfirmGoal) -> Result<super::ConfirmGoalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.DeactivateRepository` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait DeactivateRepositoryBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.DeactivateRepository`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn deactivate_repository(&mut self, input: super::DeactivateRepository) -> Result<super::DeactivateRepositoryOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.DropGoal` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait DropGoalBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.DropGoal`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn drop_goal(&mut self, input: super::DropGoal) -> Result<super::DropGoalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.MarkGoalMet` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait MarkGoalMetBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.MarkGoalMet`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn mark_goal_met(&mut self, input: super::MarkGoalMet) -> Result<super::MarkGoalMetOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.MarkRepository` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait MarkRepositoryBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.MarkRepository`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn mark_repository(&mut self, input: super::MarkRepository) -> Result<super::MarkRepositoryOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.ProposeGoal` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ProposeGoalBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.ProposeGoal`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn propose_goal(&mut self, input: super::ProposeGoal) -> Result<super::ProposeGoalOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.direction.RemoveServing` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RemoveServingBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.direction.RemoveServing`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn remove_serving(&mut self, input: super::RemoveServing) -> Result<super::RemoveServingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.direction.GoalServings` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait GoalServingsQuery {
        /// Serves `conductor.direction.GoalServings` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn goal_servings(&self) -> Result<Vec<super::GoalServings>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.direction.Goals` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait GoalsQuery {
        /// Serves `conductor.direction.Goals` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn goals(&self) -> Result<Vec<super::Goals>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.direction.RepositoryMarks` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RepositoryMarksQuery {
        /// Serves `conductor.direction.RepositoryMarks` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn repository_marks(&self) -> Result<Vec<super::RepositoryMarks>, crate::obligation::UnmetObligation>;
    }

}
