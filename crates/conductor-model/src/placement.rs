// generated from conductor v1
// contract digest 925e95ab19211d2883a7b63b82a83c2503c557df74d9a01d0be9ddc0a4f90d6c
// do not edit: regenerate with `ess synthesize --layout crate`

//! Placement — `conductor.placement`.
//!
//! Where an instance runs: a Substrate box, opened by a substrate daemon on this host or by a remote one named by URL with a bearer, through one of the adapters `systemd`, `linux` (bubblewrap), `oci` or `pod` (design docs/design/architecture-next.md § 6, decision 3). A placement carries the box's profile: read-only roots, the writable workspace, limits, egress and the secret slots. Conductor implements no sandbox: conductord asks substrate to open the box with the profile, and a refusal is recorded as a box refusal with substrate's own reason, by name, and nothing falls back (ADR 0109, quoted in § 6). Replaces `confinement.provider`, `confinement.mode` and the systemd provider's places (§ 3).
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Adapter — `conductor.placement.Adapter`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adapter {
    /// `Systemd`.
    Systemd,
    /// `Linux`.
    Linux,
    /// `Oci`.
    Oci,
    /// `Pod`.
    Pod,
}

/// The states of `conductor.placement.Box`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Box<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxState {
    /// `Closed`.
    Closed,
    /// `Opened`.
    Opened,
}

/// BoxId — `conductor.placement.BoxId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxId(pub crate::primitives::Uuid);

/// BoxLimits — `conductor.placement.BoxLimits`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxLimits {
    /// `limits` — `conductor.config.Limits`.
    pub limits: crate::config::Limits,
    /// `storage_max` — `Optional<conductor.config.Gibibytes>`.
    pub storage_max: Option<crate::config::Gibibytes>,
}

/// BoxProfile — `conductor.placement.BoxProfile`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxProfile {
    /// `read_only_roots` — `List<String>`.
    pub read_only_roots: Vec<String>,
    /// `workspace` — `String`.
    pub workspace: String,
    /// `limits` — `conductor.placement.BoxLimits`.
    pub limits: BoxLimits,
    /// `egress` — `conductor.placement.Egress`.
    pub egress: Egress,
    /// `secret_slots` — `List<conductor.secret.SecretName>`.
    pub secret_slots: Vec<crate::secret::SecretName>,
}

/// The states of `conductor.placement.BoxRefusal`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `BoxRefusal<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxRefusalState {
    /// `Refused`.
    Refused,
}

/// Daemon — `conductor.placement.Daemon`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Daemon {
    /// Tagged `Local` — `conductor.placement.LocalDaemon`.
    Local(LocalDaemon),
    /// Tagged `Remote` — `conductor.placement.RemoteDaemon`.
    Remote(RemoteDaemon),
}

/// Egress — `conductor.placement.Egress`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Egress {
    /// `forge_hosts` — `List<String>`.
    pub forge_hosts: Vec<String>,
    /// `model_host` — `String`.
    pub model_host: String,
}

/// LocalDaemon — `conductor.placement.LocalDaemon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDaemon {
    /// `socket` — `String`.
    pub socket: String,
}

/// The states of `conductor.placement.Placement`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Placement<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementState {
    /// `Configured`.
    Configured,
}

/// PlacementName — `conductor.placement.PlacementName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementName(pub String);

/// RefusalReason — `conductor.placement.RefusalReason`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusalReason(pub String);

/// RemoteDaemon — `conductor.placement.RemoteDaemon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDaemon {
    /// `url` — `String`.
    pub url: String,
    /// `bearer` — `conductor.secret.SecretName`.
    pub bearer: crate::secret::SecretName,
}

/// What Box — `conductor.placement.Box` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Box<S>`], and at a boundary by [`BoxSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxData {
    /// The identity: `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `placement` — `conductor.placement.PlacementName`.
    ///
    /// Carries `placement`: `conductor.placement.Box` references one `conductor.placement.Placement`.
    pub placement: PlacementName,
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `instance`: `conductor.placement.Box` references one `conductor.instance.Instance`.
    pub instance: crate::config::InstanceName,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
    /// `closed_at` — `Optional<Timestamp>`.
    pub closed_at: Option<crate::primitives::Timestamp>,
    /// `closed_by` — `Optional<conductor.operator.Actor>`.
    pub closed_by: Option<crate::operator::Actor>,
}

/// The states of `conductor.placement.Box`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](r#box_state::Marker), so [`Box<S>`](Box) can only ever rest in a real state.
pub mod r#box_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Closed {}
        impl Sealed for super::Opened {}
    }

    /// A declared state of `Box`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::BoxState;
    }

    /// `Closed`. Terminal: an instance may rest here forever.
    pub struct Closed;

    impl Marker for Closed {
        const STATE: super::BoxState = super::BoxState::Closed;
    }

    /// `Opened`. Where a new instance starts.
    pub struct Opened;

    impl Marker for Opened {
        const STATE: super::BoxState = super::BoxState::Opened;
    }
}

/// Box — `conductor.placement.Box` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Opened`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`BoxSnapshot`]
/// and [`BoxSnapshot::refine`].
pub struct Box<S: r#box_state::Marker> {
    data: BoxData,
    state: core::marker::PhantomData<S>,
}

impl<S: r#box_state::Marker> Box<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> BoxState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &BoxData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> BoxData {
        self.data
    }
}

impl Box<r#box_state::Opened> {
    /// A new instance, resting in `Opened` — the only state the lifecycle starts one in.
    pub fn new(data: BoxData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Box<r#box_state::Opened> {
    /// `close` — `Opened` → `Closed`. Taken by the `closed` outcome of `conductor.placement.CloseBox`.
    pub fn close(self) -> Box<r#box_state::Closed> {
        Box {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.placement.Box` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`BoxSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: BoxState,
    /// What it holds.
    pub data: BoxData,
}

/// An `Box` in whichever declared state it was found.
pub enum AnyBox {
    /// Resting in `Closed`.
    Closed(Box<r#box_state::Closed>),
    /// Resting in `Opened`.
    Opened(Box<r#box_state::Opened>),
}

impl BoxSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `BoxState` cannot spell one.
    pub fn refine(self) -> AnyBox {
        match self.state {
            BoxState::Closed => AnyBox::Closed(Box {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            BoxState::Opened => AnyBox::Opened(Box {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyBox {
    /// The state, as the runtime value.
    pub fn state(&self) -> BoxState {
        match self {
            Self::Closed(_) => BoxState::Closed,
            Self::Opened(_) => BoxState::Opened,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> BoxSnapshot {
        match self {
            Self::Closed(instance) => BoxSnapshot {
                state: BoxState::Closed,
                data: instance.into_data(),
            },
            Self::Opened(instance) => BoxSnapshot {
                state: BoxState::Opened,
                data: instance.into_data(),
            },
        }
    }
}

/// What BoxRefusal — `conductor.placement.BoxRefusal` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`BoxRefusal<S>`], and at a boundary by [`BoxRefusalSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxRefusalData {
    /// The identity: `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `placement` — `conductor.placement.PlacementName`.
    ///
    /// Carries `placement`: `conductor.placement.BoxRefusal` references one `conductor.placement.Placement`.
    pub placement: PlacementName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `reason` — `conductor.placement.RefusalReason`.
    pub reason: RefusalReason,
    /// `refused_at` — `Timestamp`.
    pub refused_at: crate::primitives::Timestamp,
}

/// The states of `conductor.placement.BoxRefusal`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](box_refusal_state::Marker), so [`BoxRefusal<S>`](BoxRefusal) can only ever rest in a real state.
pub mod box_refusal_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Refused {}
    }

    /// A declared state of `BoxRefusal`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::BoxRefusalState;
    }

    /// `Refused`. Where a new instance starts.
    pub struct Refused;

    impl Marker for Refused {
        const STATE: super::BoxRefusalState = super::BoxRefusalState::Refused;
    }
}

/// BoxRefusal — `conductor.placement.BoxRefusal` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Refused`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`BoxRefusalSnapshot`]
/// and [`BoxRefusalSnapshot::refine`].
pub struct BoxRefusal<S: box_refusal_state::Marker> {
    data: BoxRefusalData,
    state: core::marker::PhantomData<S>,
}

impl<S: box_refusal_state::Marker> BoxRefusal<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> BoxRefusalState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &BoxRefusalData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> BoxRefusalData {
        self.data
    }
}

impl BoxRefusal<box_refusal_state::Refused> {
    /// A new instance, resting in `Refused` — the only state the lifecycle starts one in.
    pub fn new(data: BoxRefusalData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.placement.BoxRefusal` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`BoxRefusalSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxRefusalSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: BoxRefusalState,
    /// What it holds.
    pub data: BoxRefusalData,
}

/// An `BoxRefusal` in whichever declared state it was found.
pub enum AnyBoxRefusal {
    /// Resting in `Refused`.
    Refused(BoxRefusal<box_refusal_state::Refused>),
}

impl BoxRefusalSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `BoxRefusalState` cannot spell one.
    pub fn refine(self) -> AnyBoxRefusal {
        match self.state {
            BoxRefusalState::Refused => AnyBoxRefusal::Refused(BoxRefusal {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyBoxRefusal {
    /// The state, as the runtime value.
    pub fn state(&self) -> BoxRefusalState {
        match self {
            Self::Refused(_) => BoxRefusalState::Refused,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> BoxRefusalSnapshot {
        match self {
            Self::Refused(instance) => BoxRefusalSnapshot {
                state: BoxRefusalState::Refused,
                data: instance.into_data(),
            },
        }
    }
}

/// What Placement — `conductor.placement.Placement` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Placement<S>`], and at a boundary by [`PlacementSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementData {
    /// The identity: `name` — `conductor.placement.PlacementName`.
    pub name: PlacementName,
    /// `daemon` — `conductor.placement.Daemon`.
    pub daemon: Daemon,
    /// `adapter` — `conductor.placement.Adapter`.
    pub adapter: Adapter,
    /// `profile` — `conductor.placement.BoxProfile`.
    pub profile: BoxProfile,
}

/// The states of `conductor.placement.Placement`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](placement_state::Marker), so [`Placement<S>`](Placement) can only ever rest in a real state.
pub mod placement_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Configured {}
    }

    /// A declared state of `Placement`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::PlacementState;
    }

    /// `Configured`. Where a new instance starts.
    pub struct Configured;

    impl Marker for Configured {
        const STATE: super::PlacementState = super::PlacementState::Configured;
    }
}

/// Placement — `conductor.placement.Placement` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Configured`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`PlacementSnapshot`]
/// and [`PlacementSnapshot::refine`].
pub struct Placement<S: placement_state::Marker> {
    data: PlacementData,
    state: core::marker::PhantomData<S>,
}

impl<S: placement_state::Marker> Placement<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> PlacementState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &PlacementData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> PlacementData {
        self.data
    }
}

impl Placement<placement_state::Configured> {
    /// A new instance, resting in `Configured` — the only state the lifecycle starts one in.
    pub fn new(data: PlacementData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.placement.Placement` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`PlacementSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: PlacementState,
    /// What it holds.
    pub data: PlacementData,
}

/// An `Placement` in whichever declared state it was found.
pub enum AnyPlacement {
    /// Resting in `Configured`.
    Configured(Placement<placement_state::Configured>),
}

impl PlacementSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `PlacementState` cannot spell one.
    pub fn refine(self) -> AnyPlacement {
        match self.state {
            PlacementState::Configured => AnyPlacement::Configured(Placement {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyPlacement {
    /// The state, as the runtime value.
    pub fn state(&self) -> PlacementState {
        match self {
            Self::Configured(_) => PlacementState::Configured,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> PlacementSnapshot {
        match self {
            Self::Configured(instance) => PlacementSnapshot {
                state: PlacementState::Configured,
                data: instance.into_data(),
            },
        }
    }
}

/// Close a box — the input of `conductor.placement.CloseBox`.
///
/// conductord closes an open box, as when an instance is gone from the config (§ 4).
///
/// Everything it can result in is [`CloseBoxOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseBox {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.placement.CloseBox` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CloseBoxOutcome {
    /// `closed` — otherwise.
    ///
    /// The box is closed.
    Closed {
        /// The `conductor.placement.BoxClosed` this outcome publishes.
        box_closed: BoxClosed,
    },
    /// `no-such-box` — for an identity no record carries.
    ///
    /// No box has this id; nothing was closed.
    NoSuchBox {
        /// Why it was refused: `conductor.placement.BoxNotFound`.
        error: BoxNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The box is already Closed, so nothing was closed.
    WrongState {
        /// Why it was refused: `conductor.placement.BoxStateConflict`.
        error: BoxStateConflict,
    },
}

/// Open a box — the input of `conductor.placement.OpenBox`.
///
/// conductord asks the placement's substrate daemon to open a box for an instance with the placement's profile, the secrets resolved into its slots (§ 6, § 9).
///
/// Everything it can result in is [`OpenBoxOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenBox {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: PlacementName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `at` — `Timestamp`.
    pub at: crate::primitives::Timestamp,
}

/// Everything `conductor.placement.OpenBox` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenBoxOutcome {
    /// `refused` — externally decided (substrate cannot give what the placement's profile asks for and refuses by name (`exec.sandbox-unavailable`)).
    ///
    /// Substrate refused the box; the refusal is on record with substrate's reason, the instance does not start and nothing falls back.
    Refused {
        /// The `conductor.placement.BoxRefused` this outcome publishes.
        box_refused: BoxRefused,
    },
    /// `adapter-refused` — externally decided (substrate's host policy refuses the placement's adapter, and substrate names the refusal).
    ///
    /// The host policy refused the adapter; the refusal is on record with the name substrate gave it, passed through, and nothing falls back.
    AdapterRefused {
        /// The `conductor.placement.BoxRefused` this outcome publishes.
        box_refused: BoxRefused,
    },
    /// `opened` — otherwise.
    ///
    /// The box is open at the placement with the placement's profile.
    Opened {
        /// The `conductor.placement.BoxOpened` this outcome publishes.
        box_opened: BoxOpened,
    },
}

/// BoxClosed — the event `conductor.placement.BoxClosed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxClosed {
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `closed_at` — `Timestamp`.
    pub closed_at: crate::primitives::Timestamp,
}

/// BoxOpened — the event `conductor.placement.BoxOpened`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxOpened {
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: PlacementName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
}

/// BoxRefused — the event `conductor.placement.BoxRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxRefused {
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: PlacementName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `reason` — `conductor.placement.RefusalReason`.
    pub reason: RefusalReason,
    /// `refused_at` — `Timestamp`.
    pub refused_at: crate::primitives::Timestamp,
}

/// The declared error `conductor.placement.BoxNotFound`.
///
/// No box has this id, so nothing was closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxNotFound {
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
}

/// The declared error `conductor.placement.BoxStateConflict`.
///
/// The box is already closed, so nothing was closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxStateConflict {
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
}

/// Box refusals — one row of the view `conductor.placement.BoxRefusals`.
///
/// Projects `conductor.placement.BoxRefusal` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxRefusals {
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `state` — `conductor.placement.BoxRefusal.State`.
    pub state: BoxRefusalState,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: PlacementName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `reason` — `conductor.placement.RefusalReason`.
    pub reason: RefusalReason,
    /// `refused_at` — `Timestamp`.
    pub refused_at: crate::primitives::Timestamp,
}

/// Boxes — one row of the view `conductor.placement.Boxes`.
///
/// Projects `conductor.placement.Box` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Boxes {
    /// `box_id` — `conductor.placement.BoxId`.
    pub box_id: BoxId,
    /// `state` — `conductor.placement.Box.State`.
    pub state: BoxState,
    /// `placement` — `conductor.placement.PlacementName`.
    pub placement: PlacementName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
    /// `closed_at` — `Optional<Timestamp>`.
    pub closed_at: Option<crate::primitives::Timestamp>,
    /// `closed_by` — `Optional<conductor.operator.Actor>`.
    pub closed_by: Option<crate::operator::Actor>,
}

/// Placements — one row of the view `conductor.placement.Placements`.
///
/// Projects `conductor.placement.Placement` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placements {
    /// `name` — `conductor.placement.PlacementName`.
    pub name: PlacementName,
    /// `state` — `conductor.placement.Placement.State`.
    pub state: PlacementState,
    /// `daemon` — `conductor.placement.Daemon`.
    pub daemon: Daemon,
    /// `adapter` — `conductor.placement.Adapter`.
    pub adapter: Adapter,
    /// `profile` — `conductor.placement.BoxProfile`.
    pub profile: BoxProfile,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.placement.CloseBox` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CloseBoxBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.placement.CloseBox`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn close_box(&mut self, input: super::CloseBox) -> Result<super::CloseBoxOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.placement.OpenBox` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait OpenBoxBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.placement.OpenBox`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn open_box(&mut self, input: super::OpenBox) -> Result<super::OpenBoxOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.placement.BoxRefusals` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait BoxRefusalsQuery {
        /// Serves `conductor.placement.BoxRefusals` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn box_refusals(&self) -> Result<Vec<super::BoxRefusals>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.placement.Boxes` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait BoxesQuery {
        /// Serves `conductor.placement.Boxes` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn boxes(&self) -> Result<Vec<super::Boxes>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.placement.Placements` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait PlacementsQuery {
        /// Serves `conductor.placement.Placements` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn placements(&self) -> Result<Vec<super::Placements>, crate::obligation::UnmetObligation>;
    }

}
