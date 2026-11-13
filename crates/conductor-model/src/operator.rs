// generated from conductor v1
// contract digest 882143897862f23950c9f2c26ad3b57f23a8a0eb8334ddf254337d9e57836461
// do not edit: regenerate with `ess synthesize --layout crate`

//! Operator — `conductor.operator`.
//!
//! Who acts on conductor, told apart by kind (design docs/design/architecture-next.md § 3 "Actors", § 5.3, decisions 9, 11, 12 and 14). An `Actor` is a kind and the fields of that kind: `Human`, an operator (an Identity principal with grants); `Agent`, a program the runtime started in a role of an instance; `Daemon`, conductord or an instance runtime acting on its own account. Every command in every domain carries its `actor`; every event and every record derived from one carries the same; a grant belongs to a human actor only, and a command that needs one refuses every other kind by name, as a predicate over `actor.kind`. The instance cluster has several operators; an operator joins an instance and leaves it; the records projection writes an actor as `human:<name>`, `agent:<instance>/<role>/<name>` or `daemon`.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Actor — `conductor.operator.Actor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    /// `kind` — `conductor.operator.ActorKind`.
    pub kind: ActorKind,
    /// `operator` — `Optional<conductor.operator.OperatorName>`.
    pub operator: Option<OperatorName>,
    /// `agent` — `Optional<conductor.operator.AgentActor>`.
    pub agent: Option<AgentActor>,
}

/// ActorKind — `conductor.operator.ActorKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    /// `ControlPlane`.
    ControlPlane,
    /// `Human`.
    Human,
    /// `Agent`.
    Agent,
    /// `Daemon`.
    Daemon,
}

/// AgentActor — `conductor.operator.AgentActor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentActor {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `role` — `conductor.config.RoleName`.
    pub role: crate::config::RoleName,
    /// `name` — `conductor.agent.AgentName`.
    pub name: crate::agent::AgentName,
    /// `kind` — `conductor.agent.AgentKindName`.
    pub kind: crate::agent::AgentKindName,
}

/// Grant — `conductor.operator.Grant`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grant {
    /// `AnswerDecisions`.
    AnswerDecisions,
    /// `ChangeSettings`.
    ChangeSettings,
    /// `StartInstances`.
    StartInstances,
    /// `StopInstances`.
    StopInstances,
    /// `UpgradeInstances`.
    UpgradeInstances,
    /// `Purge`.
    Purge,
    /// `Join`.
    Join,
}

/// The states of `conductor.operator.Operator`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Operator<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorState {
    /// `Active`.
    Active,
    /// `Left`.
    Left,
}

/// OperatorKey — `conductor.operator.OperatorKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorKey {
    /// `operator` — `conductor.operator.OperatorName`.
    pub operator: OperatorName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
}

/// OperatorName — `conductor.operator.OperatorName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorName(pub String);

/// What Operator — `conductor.operator.Operator` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Operator<S>`], and at a boundary by [`OperatorSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorData {
    /// The identity: `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
    /// `grants` — `List<conductor.operator.Grant>`.
    pub grants: Vec<Grant>,
    /// `joined_at` — `Timestamp`.
    pub joined_at: crate::primitives::Timestamp,
    /// `joined_by` — `conductor.operator.Actor`.
    pub joined_by: Actor,
    /// `left_at` — `Optional<Timestamp>`.
    pub left_at: Option<crate::primitives::Timestamp>,
    /// `left_by` — `Optional<conductor.operator.Actor>`.
    pub left_by: Option<Actor>,
}

/// The states of `conductor.operator.Operator`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](operator_state::Marker), so [`Operator<S>`](Operator) can only ever rest in a real state.
pub mod operator_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Active {}
        impl Sealed for super::Left {}
    }

    /// A declared state of `Operator`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::OperatorState;
    }

    /// `Active`. Where a new instance starts.
    pub struct Active;

    impl Marker for Active {
        const STATE: super::OperatorState = super::OperatorState::Active;
    }

    /// `Left`.
    pub struct Left;

    impl Marker for Left {
        const STATE: super::OperatorState = super::OperatorState::Left;
    }
}

/// Operator — `conductor.operator.Operator` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Active`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`OperatorSnapshot`]
/// and [`OperatorSnapshot::refine`].
pub struct Operator<S: operator_state::Marker> {
    data: OperatorData,
    state: core::marker::PhantomData<S>,
}

impl<S: operator_state::Marker> Operator<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> OperatorState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &OperatorData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> OperatorData {
        self.data
    }
}

impl Operator<operator_state::Active> {
    /// A new instance, resting in `Active` — the only state the lifecycle starts one in.
    pub fn new(data: OperatorData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Operator<operator_state::Active> {
    /// `leave` — `Active` → `Left`. Taken by the `left` outcome of `conductor.operator.LeaveInstance`.
    pub fn leave(self) -> Operator<operator_state::Left> {
        Operator {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Operator<operator_state::Left> {
    /// `rejoin` — `Left` → `Active`. Taken by the `rejoined` outcome of `conductor.operator.JoinInstance`.
    pub fn rejoin(self) -> Operator<operator_state::Active> {
        Operator {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.operator.Operator` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`OperatorSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: OperatorState,
    /// What it holds.
    pub data: OperatorData,
}

/// An `Operator` in whichever declared state it was found.
pub enum AnyOperator {
    /// Resting in `Active`.
    Active(Operator<operator_state::Active>),
    /// Resting in `Left`.
    Left(Operator<operator_state::Left>),
}

impl OperatorSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `OperatorState` cannot spell one.
    pub fn refine(self) -> AnyOperator {
        match self.state {
            OperatorState::Active => AnyOperator::Active(Operator {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            OperatorState::Left => AnyOperator::Left(Operator {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyOperator {
    /// The state, as the runtime value.
    pub fn state(&self) -> OperatorState {
        match self {
            Self::Active(_) => OperatorState::Active,
            Self::Left(_) => OperatorState::Left,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> OperatorSnapshot {
        match self {
            Self::Active(instance) => OperatorSnapshot {
                state: OperatorState::Active,
                data: instance.into_data(),
            },
            Self::Left(instance) => OperatorSnapshot {
                state: OperatorState::Left,
                data: instance.into_data(),
            },
        }
    }
}

/// Join an instance — the input of `conductor.operator.JoinInstance`.
///
/// Everything it can result in is [`JoinInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: Actor,
    /// `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
    /// `grants` — `List<conductor.operator.Grant>`.
    pub grants: Vec<Grant>,
    /// `joined_at` — `Timestamp`.
    pub joined_at: crate::primitives::Timestamp,
}

/// Everything `conductor.operator.JoinInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinInstanceOutcome {
    /// `actor-not-human` — when `actor.kind != Human`.
    ///
    /// An agent cannot join an instance; nothing was written.
    ActorNotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: ActorNotHuman,
    },
    /// `joined` — for an identity no record carries.
    ///
    /// The engineer is an active operator of the instance, with the grants given.
    Joined {
        /// The `conductor.operator.OperatorJoined` this outcome publishes.
        operator_joined: OperatorJoined,
    },
    /// `rejoined` — otherwise.
    ///
    /// An operator who had left joins again and is active, with the grants given now.
    Rejoined {
        /// The `conductor.operator.OperatorJoined` this outcome publishes.
        operator_joined: OperatorJoined,
    },
    /// `already-joined` — from a state no declared move starts in.
    ///
    /// The operator is active on the instance already; the record is unchanged.
    AlreadyJoined {
        /// Why it was refused: `conductor.operator.AlreadyJoined`.
        error: AlreadyJoined,
    },
}

/// Leave an instance — the input of `conductor.operator.LeaveInstance`.
///
/// Everything it can result in is [`LeaveInstanceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveInstance {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: Actor,
    /// `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
    /// `left_at` — `Timestamp`.
    pub left_at: crate::primitives::Timestamp,
}

/// Everything `conductor.operator.LeaveInstance` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaveInstanceOutcome {
    /// `actor-not-human` — when `actor.kind != Human`.
    ///
    /// An agent cannot make an operator leave; nothing was written.
    ActorNotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: ActorNotHuman,
    },
    /// `left` — otherwise.
    ///
    /// The operator has left the instance; the record stays, `Left`, with who and when.
    Left {
        /// The `conductor.operator.OperatorLeft` this outcome publishes.
        operator_left: OperatorLeft,
    },
    /// `not-joined` — from a state no declared move starts in.
    ///
    /// The operator had left the instance already; the record is unchanged.
    NotJoined {
        /// Why it was refused: `conductor.operator.NotJoined`.
        error: NotJoined,
    },
    /// `unknown-operator` — for an identity no record carries.
    ///
    /// The engineer never joined the instance; nothing was written.
    UnknownOperator {
        /// Why it was refused: `conductor.operator.NotJoined`.
        error: NotJoined,
    },
}

/// OperatorJoined — the event `conductor.operator.OperatorJoined`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorJoined {
    /// `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
    /// `grants` — `List<conductor.operator.Grant>`.
    pub grants: Vec<Grant>,
    /// `joined_at` — `Timestamp`.
    pub joined_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: Actor,
}

/// OperatorLeft — the event `conductor.operator.OperatorLeft`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorLeft {
    /// `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
    /// `left_at` — `Timestamp`.
    pub left_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: Actor,
}

/// The declared error `conductor.operator.ActorNotHuman`.
///
/// The command needs a grant, a grant belongs to a human actor only, and the actor that issued it is an agent or the daemon. Nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorNotHuman {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: Actor,
}

/// The declared error `conductor.operator.AlreadyJoined`.
///
/// The operator is an active operator of the instance already; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadyJoined {
    /// `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
}

/// The declared error `conductor.operator.GrantMissing`.
///
/// The operator is a person, and holds no grant for the write asked. Nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantMissing {
    /// `operator` — `conductor.operator.OperatorName`.
    pub operator: OperatorName,
    /// `grant` — `conductor.operator.Grant`.
    pub grant: Grant,
}

/// The declared error `conductor.operator.NotJoined`.
///
/// The operator is not an active operator of the instance, so there is nothing to leave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotJoined {
    /// `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
}

/// Operators — one row of the view `conductor.operator.Operators`.
///
/// Projects `conductor.operator.Operator` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operators {
    /// `key` — `conductor.operator.OperatorKey`.
    pub key: OperatorKey,
    /// `state` — `conductor.operator.Operator.State`.
    pub state: OperatorState,
    /// `grants` — `List<conductor.operator.Grant>`.
    pub grants: Vec<Grant>,
    /// `joined_at` — `Timestamp`.
    pub joined_at: crate::primitives::Timestamp,
    /// `joined_by` — `conductor.operator.Actor`.
    pub joined_by: Actor,
    /// `left_at` — `Optional<Timestamp>`.
    pub left_at: Option<crate::primitives::Timestamp>,
    /// `left_by` — `Optional<conductor.operator.Actor>`.
    pub left_by: Option<Actor>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.operator.JoinInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait JoinInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.operator.JoinInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn join_instance(&mut self, input: super::JoinInstance) -> Result<super::JoinInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.operator.LeaveInstance` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait LeaveInstanceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.operator.LeaveInstance`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn leave_instance(&mut self, input: super::LeaveInstance) -> Result<super::LeaveInstanceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.operator.Operators` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait OperatorsQuery {
        /// Serves `conductor.operator.Operators` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn operators(&self) -> Result<Vec<super::Operators>, crate::obligation::UnmetObligation>;
    }

}
