// generated from conductor v1
// model digest da2650ca246276e52671fae1b8f0cb94808d7ac6df9237ba08bb0953984621f7
// contract digest 40f96bb6aa267ca9c7e4701e8fd647464238ddcc0603cc26e1357adc1d4f1e9b
// do not edit: regenerate with `ess synthesize --layout crate`

//! Session — `conductor.session`.
//!
//! The one conductor of an instance (story:single-conductor-lease). The lease, kept in the store under the instance's state directory, names the session id and the process id of the conductor session that holds it. Every conductor session started with the instance's settings claims it at its start, through the settings' SessionStart hook; a claim while another live process holds it is refused and names that holder, and a claim over a holder whose process is gone takes the lease over and records whom it took it from. Design: docs/design/conductor.md § 7.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `conductor.session.Lease`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Lease<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    /// `Held`.
    Held,
}

/// SessionId — `conductor.session.SessionId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);

/// What Lease — `conductor.session.Lease` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Lease<S>`], and at a boundary by [`LeaseSnapshot::state`].
///
/// Every value satisfies `pid > 0` — checked by [`LeaseData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseData {
    /// The identity: `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_at` — `Timestamp`.
    pub taken_at: crate::primitives::Timestamp,
    /// `taken_over_session_id` — `Optional<conductor.session.SessionId>`.
    pub taken_over_session_id: Option<SessionId>,
    /// `taken_over_pid` — `Optional<Integer>`.
    pub taken_over_pid: Option<i64>,
}

impl LeaseData {
    /// The first declared invariant of `conductor.session.Lease` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.pid)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("pid > 0");
        }
        None
    }
}

/// The states of `conductor.session.Lease`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](lease_state::Marker), so [`Lease<S>`](Lease) can only ever rest in a real state.
pub mod lease_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Held {}
    }

    /// A declared state of `Lease`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::LeaseState;
    }

    /// `Held`. Where a new instance starts.
    pub struct Held;

    impl Marker for Held {
        const STATE: super::LeaseState = super::LeaseState::Held;
    }
}

/// Lease — `conductor.session.Lease` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Held`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`LeaseSnapshot`]
/// and [`LeaseSnapshot::refine`].
pub struct Lease<S: lease_state::Marker> {
    data: LeaseData,
    state: core::marker::PhantomData<S>,
}

impl<S: lease_state::Marker> Lease<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> LeaseState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &LeaseData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> LeaseData {
        self.data
    }
}

impl Lease<lease_state::Held> {
    /// A new instance, resting in `Held` — the only state the lifecycle starts one in.
    pub fn new(data: LeaseData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.session.Lease` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`LeaseSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: LeaseState,
    /// What it holds.
    pub data: LeaseData,
}

/// An `Lease` in whichever declared state it was found.
pub enum AnyLease {
    /// Resting in `Held`.
    Held(Lease<lease_state::Held>),
}

impl LeaseSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `LeaseState` cannot spell one.
    pub fn refine(self) -> AnyLease {
        match self.state {
            LeaseState::Held => AnyLease::Held(Lease {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyLease {
    /// The state, as the runtime value.
    pub fn state(&self) -> LeaseState {
        match self {
            Self::Held(_) => LeaseState::Held,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> LeaseSnapshot {
        match self {
            Self::Held(instance) => LeaseSnapshot {
                state: LeaseState::Held,
                data: instance.into_data(),
            },
        }
    }
}

/// Claim the conductor lease — the input of `conductor.session.ClaimLease`.
///
/// Everything it can result in is [`ClaimLeaseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimLease {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_at` — `Timestamp`.
    pub taken_at: crate::primitives::Timestamp,
}

/// Everything `conductor.session.ClaimLease` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimLeaseOutcome {
    /// `taken` — when, of the `conductor.session.Lease` rows satisfying `instance == input.instance`, no row is selected.
    ///
    /// No session held the instance's lease; the claiming one holds it now.
    Taken {
        /// The `conductor.session.LeaseTaken` this outcome publishes.
        lease_taken: LeaseTaken,
    },
    /// `renewed` — when, of the `conductor.session.Lease` rows satisfying `(instance == input.instance and pid == input.pid)`, at least one row is selected.
    ///
    /// The holder's own process claims again (a resumed, cleared or compacted session); it keeps the lease under the session id it gives now.
    Renewed {
        /// The `conductor.session.LeaseTaken` this outcome publishes.
        lease_taken: LeaseTaken,
    },
    /// `held` — externally decided (the process the lease names lives and is not the claiming one).
    ///
    /// A second conductor is refused, told who holds the lease, and must stop; the lease is unchanged.
    Held {
        /// Why it was refused: `conductor.session.LeaseHeld`.
        error: LeaseHeld,
    },
    /// `taken-over` — otherwise.
    ///
    /// The holder's process is gone, so the claiming session takes the lease and the lease records whom it was taken from.
    TakenOver {
        /// The `conductor.session.LeaseTakenOver` this outcome publishes.
        lease_taken_over: LeaseTakenOver,
    },
}

/// LeaseTaken — the event `conductor.session.LeaseTaken`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseTaken {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
}

/// LeaseTakenOver — the event `conductor.session.LeaseTakenOver`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseTakenOver {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_over_session_id` — `conductor.session.SessionId`.
    pub taken_over_session_id: SessionId,
    /// `taken_over_pid` — `Integer`.
    pub taken_over_pid: i64,
}

/// The declared error `conductor.session.LeaseHeld`.
///
/// Another session's live process holds the instance's lease: the claiming session is a second conductor and must stop. Nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseHeld {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
}

/// Conductor leases — one row of the view `conductor.session.Leases`.
///
/// Projects `conductor.session.Lease` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leases {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `state` — `conductor.session.Lease.State`.
    pub state: LeaseState,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_at` — `Timestamp`.
    pub taken_at: crate::primitives::Timestamp,
    /// `taken_over_session_id` — `Optional<conductor.session.SessionId>`.
    pub taken_over_session_id: Option<SessionId>,
    /// `taken_over_pid` — `Optional<Integer>`.
    pub taken_over_pid: Option<i64>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.session.ClaimLease` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by an unknown identity, which reaches no declared outcome (neither `unknown_instance:` nor `wrong_state:`).
    ///
    /// Contract: given `conductor.session.ClaimLease` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `taken` when, of the `conductor.session.Lease` rows satisfying `instance == input.instance`, no row is selected, creates `conductor.session.Lease`, emits `conductor.session.LeaseTaken`; `renewed` when, of the `conductor.session.Lease` rows satisfying `(instance == input.instance and pid == input.pid)`, at least one row is selected, updates `conductor.session.Lease`, emits `conductor.session.LeaseTaken`; `held` externally decided (the process the lease names lives and is not the claiming one), error `conductor.session.LeaseHeld`; `taken-over` otherwise, updates `conductor.session.Lease`, emits `conductor.session.LeaseTakenOver`.
    pub trait ClaimLeaseBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.session.ClaimLease`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn claim_lease(&mut self, input: super::ClaimLease) -> Result<super::ClaimLeaseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.session.Leases` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait LeasesQuery {
        /// Serves `conductor.session.Leases` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn leases(&self) -> Result<Vec<super::Leases>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl ClaimLeaseBehavior for Unimplemented {
        fn claim_lease(&mut self, _input: super::ClaimLease) -> Result<super::ClaimLeaseOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.session.ClaimLease" })
        }
    }
}
