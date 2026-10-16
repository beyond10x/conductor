// generated from conductor v1
// model digest 9a6703b0827f819c67385c1d2f48a0a266da464975a9c677ca4f1cc577e59c6d
// contract digest 4c9d1ece3a6a3f1887eb30feb4485b2fb67bd24b424ad0dcad73c377cf04299f
// do not edit: regenerate with `ess synthesize --layout crate`

//! Improvement — `conductor.improvement`.
//!
//! The measured self-improvement loop (docs/design/self-improvement.md § 2, § 5; story:kpi-spec). Its vocabulary is the operator's (story:metrics-naming, 2026-10-14): a **metric** is one measured quantity with its unit, its instrument and a series of readings over time; a **KPI** is a metric the operator set a target for, shown with that target and whether a reading meets it. Every KPI is a metric; not every metric is a KPI. A friction finding is what the loop detects (a KPI past its target, or a friction signature repeated) with the evidence ids it rests on; conductor-dev files it as a draft story naming the metric it should move, and a week after delivery the metric is read again and the finding is verified or closed.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// ComputeMetrics — `conductor.improvement.ComputeMetrics`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeMetrics {
    /// `day` — `String`.
    pub day: String,
}

/// FindingId — `conductor.improvement.FindingId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingId(pub String);

/// The states of `conductor.improvement.FrictionFinding`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `FrictionFinding<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrictionFindingState {
    /// `Closed`.
    Closed,
    /// `Filed`.
    Filed,
    /// `Open`.
    Open,
    /// `Verified`.
    Verified,
}

/// The states of `conductor.improvement.Metric`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Metric<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricState {
    /// `Targeted`.
    Targeted,
    /// `Untargeted`.
    Untargeted,
}

/// MetricId — `conductor.improvement.MetricId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricId(pub String);

/// The states of `conductor.improvement.MetricReading`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `MetricReading<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricReadingState {
    /// `Recorded`.
    Recorded,
    /// `Superseded`.
    Superseded,
}

/// MetricReadingId — `conductor.improvement.MetricReadingId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingId(pub String);

/// TargetDirection — `conductor.improvement.TargetDirection`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetDirection {
    /// `AtMost`.
    AtMost,
    /// `AtLeast`.
    AtLeast,
}

/// What FrictionFinding — `conductor.improvement.FrictionFinding` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`FrictionFinding<S>`], and at a boundary by [`FrictionFindingSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrictionFindingData {
    /// The identity: `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `signature` — `String`.
    pub signature: String,
    /// `evidence` — `List<String>`.
    pub evidence: Vec<String>,
    /// `metric_id` — `conductor.improvement.MetricId`.
    ///
    /// Carries `metric`: `conductor.improvement.FrictionFinding` references one `conductor.improvement.Metric`.
    pub metric_id: MetricId,
    /// `raised_at` — `Timestamp`.
    pub raised_at: crate::primitives::Timestamp,
    /// `story` — `Optional<String>`.
    pub story: Option<String>,
    /// `verdict` — `Optional<String>`.
    pub verdict: Option<String>,
}

/// The states of `conductor.improvement.FrictionFinding`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](friction_finding_state::Marker), so [`FrictionFinding<S>`](FrictionFinding) can only ever rest in a real state.
pub mod friction_finding_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Closed {}
        impl Sealed for super::Filed {}
        impl Sealed for super::Open {}
        impl Sealed for super::Verified {}
    }

    /// A declared state of `FrictionFinding`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::FrictionFindingState;
    }

    /// `Closed`. Terminal: an instance may rest here forever.
    pub struct Closed;

    impl Marker for Closed {
        const STATE: super::FrictionFindingState = super::FrictionFindingState::Closed;
    }

    /// `Filed`.
    pub struct Filed;

    impl Marker for Filed {
        const STATE: super::FrictionFindingState = super::FrictionFindingState::Filed;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::FrictionFindingState = super::FrictionFindingState::Open;
    }

    /// `Verified`. Terminal: an instance may rest here forever.
    pub struct Verified;

    impl Marker for Verified {
        const STATE: super::FrictionFindingState = super::FrictionFindingState::Verified;
    }
}

/// FrictionFinding — `conductor.improvement.FrictionFinding` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`FrictionFindingSnapshot`]
/// and [`FrictionFindingSnapshot::refine`].
pub struct FrictionFinding<S: friction_finding_state::Marker> {
    data: FrictionFindingData,
    state: core::marker::PhantomData<S>,
}

impl<S: friction_finding_state::Marker> FrictionFinding<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> FrictionFindingState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &FrictionFindingData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> FrictionFindingData {
        self.data
    }
}

impl FrictionFinding<friction_finding_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: FrictionFindingData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl FrictionFinding<friction_finding_state::Filed> {
    /// `verify` — `Filed` → `Verified`. Taken by the `verified` outcome of `conductor.improvement.VerifyFinding`.
    pub fn verify(self) -> FrictionFinding<friction_finding_state::Verified> {
        FrictionFinding {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `close` — `Filed` → `Closed`. Taken by the `closed` outcome of `conductor.improvement.VerifyFinding`.
    pub fn close(self) -> FrictionFinding<friction_finding_state::Closed> {
        FrictionFinding {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl FrictionFinding<friction_finding_state::Open> {
    /// `file` — `Open` → `Filed`. Taken by the `filed` outcome of `conductor.improvement.FileFinding`.
    pub fn file(self) -> FrictionFinding<friction_finding_state::Filed> {
        FrictionFinding {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.improvement.FrictionFinding` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`FrictionFindingSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrictionFindingSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: FrictionFindingState,
    /// What it holds.
    pub data: FrictionFindingData,
}

/// An `FrictionFinding` in whichever declared state it was found.
pub enum AnyFrictionFinding {
    /// Resting in `Closed`.
    Closed(FrictionFinding<friction_finding_state::Closed>),
    /// Resting in `Filed`.
    Filed(FrictionFinding<friction_finding_state::Filed>),
    /// Resting in `Open`.
    Open(FrictionFinding<friction_finding_state::Open>),
    /// Resting in `Verified`.
    Verified(FrictionFinding<friction_finding_state::Verified>),
}

impl FrictionFindingSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `FrictionFindingState` cannot spell one.
    pub fn refine(self) -> AnyFrictionFinding {
        match self.state {
            FrictionFindingState::Closed => AnyFrictionFinding::Closed(FrictionFinding {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            FrictionFindingState::Filed => AnyFrictionFinding::Filed(FrictionFinding {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            FrictionFindingState::Open => AnyFrictionFinding::Open(FrictionFinding {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            FrictionFindingState::Verified => AnyFrictionFinding::Verified(FrictionFinding {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyFrictionFinding {
    /// The state, as the runtime value.
    pub fn state(&self) -> FrictionFindingState {
        match self {
            Self::Closed(_) => FrictionFindingState::Closed,
            Self::Filed(_) => FrictionFindingState::Filed,
            Self::Open(_) => FrictionFindingState::Open,
            Self::Verified(_) => FrictionFindingState::Verified,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> FrictionFindingSnapshot {
        match self {
            Self::Closed(instance) => FrictionFindingSnapshot {
                state: FrictionFindingState::Closed,
                data: instance.into_data(),
            },
            Self::Filed(instance) => FrictionFindingSnapshot {
                state: FrictionFindingState::Filed,
                data: instance.into_data(),
            },
            Self::Open(instance) => FrictionFindingSnapshot {
                state: FrictionFindingState::Open,
                data: instance.into_data(),
            },
            Self::Verified(instance) => FrictionFindingSnapshot {
                state: FrictionFindingState::Verified,
                data: instance.into_data(),
            },
        }
    }
}

/// What Metric — `conductor.improvement.Metric` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Metric<S>`], and at a boundary by [`MetricSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricData {
    /// The identity: `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `name` — `String`.
    pub name: String,
    /// `unit` — `String`.
    pub unit: String,
    /// `instrument` — `String`.
    pub instrument: String,
    /// `definition` — `String`.
    pub definition: String,
    /// `baseline` — `Optional<Decimal>`.
    pub baseline: Option<crate::primitives::Decimal>,
    /// `baseline_period` — `Optional<String>`.
    pub baseline_period: Option<String>,
    /// `target` — `Optional<Decimal>`.
    pub target: Option<crate::primitives::Decimal>,
    /// `direction` — `Optional<conductor.improvement.TargetDirection>`.
    pub direction: Option<TargetDirection>,
    /// `target_owner` — `Optional<String>`.
    pub target_owner: Option<String>,
    /// `target_set_at` — `Optional<Timestamp>`.
    pub target_set_at: Option<crate::primitives::Timestamp>,
}

/// The states of `conductor.improvement.Metric`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](metric_state::Marker), so [`Metric<S>`](Metric) can only ever rest in a real state.
pub mod metric_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Targeted {}
        impl Sealed for super::Untargeted {}
    }

    /// A declared state of `Metric`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MetricState;
    }

    /// `Targeted`. Terminal: an instance may rest here forever.
    pub struct Targeted;

    impl Marker for Targeted {
        const STATE: super::MetricState = super::MetricState::Targeted;
    }

    /// `Untargeted`. Where a new instance starts.
    pub struct Untargeted;

    impl Marker for Untargeted {
        const STATE: super::MetricState = super::MetricState::Untargeted;
    }
}

/// Metric — `conductor.improvement.Metric` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Untargeted`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MetricSnapshot`]
/// and [`MetricSnapshot::refine`].
pub struct Metric<S: metric_state::Marker> {
    data: MetricData,
    state: core::marker::PhantomData<S>,
}

impl<S: metric_state::Marker> Metric<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MetricState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MetricData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MetricData {
        self.data
    }
}

impl Metric<metric_state::Untargeted> {
    /// A new instance, resting in `Untargeted` — the only state the lifecycle starts one in.
    pub fn new(data: MetricData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Metric<metric_state::Untargeted> {
    /// `target` — `Untargeted` → `Targeted`. Taken by the `targeted` outcome of `conductor.improvement.SetMetricTarget`.
    pub fn target(self) -> Metric<metric_state::Targeted> {
        Metric {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.improvement.Metric` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MetricSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MetricState,
    /// What it holds.
    pub data: MetricData,
}

/// An `Metric` in whichever declared state it was found.
pub enum AnyMetric {
    /// Resting in `Targeted`.
    Targeted(Metric<metric_state::Targeted>),
    /// Resting in `Untargeted`.
    Untargeted(Metric<metric_state::Untargeted>),
}

impl MetricSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MetricState` cannot spell one.
    pub fn refine(self) -> AnyMetric {
        match self.state {
            MetricState::Targeted => AnyMetric::Targeted(Metric {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MetricState::Untargeted => AnyMetric::Untargeted(Metric {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMetric {
    /// The state, as the runtime value.
    pub fn state(&self) -> MetricState {
        match self {
            Self::Targeted(_) => MetricState::Targeted,
            Self::Untargeted(_) => MetricState::Untargeted,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MetricSnapshot {
        match self {
            Self::Targeted(instance) => MetricSnapshot {
                state: MetricState::Targeted,
                data: instance.into_data(),
            },
            Self::Untargeted(instance) => MetricSnapshot {
                state: MetricState::Untargeted,
                data: instance.into_data(),
            },
        }
    }
}

/// What MetricReading — `conductor.improvement.MetricReading` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`MetricReading<S>`], and at a boundary by [`MetricReadingSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingData {
    /// The identity: `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
    /// `metric_id` — `conductor.improvement.MetricId`.
    ///
    /// Carries `readings`: `conductor.improvement.Metric` owns many `conductor.improvement.MetricReading`.
    pub metric_id: MetricId,
    /// `day` — `String`.
    pub day: String,
    /// `scope` — `String`.
    pub scope: String,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
    /// `sources` — `List<String>`.
    pub sources: Vec<String>,
    /// `target` — `Optional<Decimal>`.
    pub target: Option<crate::primitives::Decimal>,
    /// `met` — `Optional<Boolean>`.
    pub met: Option<bool>,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// The states of `conductor.improvement.MetricReading`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](metric_reading_state::Marker), so [`MetricReading<S>`](MetricReading) can only ever rest in a real state.
pub mod metric_reading_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
        impl Sealed for super::Superseded {}
    }

    /// A declared state of `MetricReading`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MetricReadingState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::MetricReadingState = super::MetricReadingState::Recorded;
    }

    /// `Superseded`. Terminal: an instance may rest here forever.
    pub struct Superseded;

    impl Marker for Superseded {
        const STATE: super::MetricReadingState = super::MetricReadingState::Superseded;
    }
}

/// MetricReading — `conductor.improvement.MetricReading` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MetricReadingSnapshot`]
/// and [`MetricReadingSnapshot::refine`].
pub struct MetricReading<S: metric_reading_state::Marker> {
    data: MetricReadingData,
    state: core::marker::PhantomData<S>,
}

impl<S: metric_reading_state::Marker> MetricReading<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MetricReadingState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MetricReadingData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MetricReadingData {
        self.data
    }
}

impl MetricReading<metric_reading_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: MetricReadingData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl MetricReading<metric_reading_state::Recorded> {
    /// `supersede` — `Recorded` → `Superseded`. Taken by the `superseded` outcome of `conductor.improvement.SupersedeMetricReading`.
    pub fn supersede(self) -> MetricReading<metric_reading_state::Superseded> {
        MetricReading {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.improvement.MetricReading` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MetricReadingSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MetricReadingState,
    /// What it holds.
    pub data: MetricReadingData,
}

/// An `MetricReading` in whichever declared state it was found.
pub enum AnyMetricReading {
    /// Resting in `Recorded`.
    Recorded(MetricReading<metric_reading_state::Recorded>),
    /// Resting in `Superseded`.
    Superseded(MetricReading<metric_reading_state::Superseded>),
}

impl MetricReadingSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MetricReadingState` cannot spell one.
    pub fn refine(self) -> AnyMetricReading {
        match self.state {
            MetricReadingState::Recorded => AnyMetricReading::Recorded(MetricReading {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MetricReadingState::Superseded => AnyMetricReading::Superseded(MetricReading {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMetricReading {
    /// The state, as the runtime value.
    pub fn state(&self) -> MetricReadingState {
        match self {
            Self::Recorded(_) => MetricReadingState::Recorded,
            Self::Superseded(_) => MetricReadingState::Superseded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MetricReadingSnapshot {
        match self {
            Self::Recorded(instance) => MetricReadingSnapshot {
                state: MetricReadingState::Recorded,
                data: instance.into_data(),
            },
            Self::Superseded(instance) => MetricReadingSnapshot {
                state: MetricReadingState::Superseded,
                data: instance.into_data(),
            },
        }
    }
}

/// Define a metric — the input of `conductor.improvement.DefineMetric`.
///
/// Everything it can result in is [`DefineMetricOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefineMetric {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `name` — `String`.
    pub name: String,
    /// `unit` — `String`.
    pub unit: String,
    /// `instrument` — `String`.
    pub instrument: String,
    /// `definition` — `String`.
    pub definition: String,
    /// `baseline` — `Optional<Decimal>`.
    pub baseline: Option<crate::primitives::Decimal>,
    /// `baseline_period` — `Optional<String>`.
    pub baseline_period: Option<String>,
}

/// Everything `conductor.improvement.DefineMetric` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefineMetricOutcome {
    /// `defined` — otherwise.
    ///
    /// The metric is defined without a target; its readings can be recorded.
    Defined {
        /// The `conductor.improvement.MetricDefined` this outcome publishes.
        metric_defined: MetricDefined,
    },
    /// `already-defined` — for an identity a record already carries.
    ///
    /// A metric already has this id; the held metric is unchanged.
    AlreadyDefined {
        /// Why it was refused: `conductor.improvement.MetricExists`.
        error: MetricExists,
    },
}

/// File a finding as a draft story — the input of `conductor.improvement.FileFinding`.
///
/// Everything it can result in is [`FileFindingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFinding {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `story` — `String`.
    pub story: String,
}

/// Everything `conductor.improvement.FileFinding` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileFindingOutcome {
    /// `filed` — otherwise.
    ///
    /// The finding is filed as the draft story, which names the metric it should move.
    Filed {
        /// The `conductor.improvement.FindingFiled` this outcome publishes.
        finding_filed: FindingFiled,
    },
    /// `no-such-finding` — for an identity no record carries.
    NoSuchFinding {
        /// Why it was refused: `conductor.improvement.FindingNotFound`.
        error: FindingNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only an open finding is filed.
    WrongState {
        /// Why it was refused: `conductor.improvement.FindingStateConflict`.
        error: FindingStateConflict,
    },
}

/// Raise a friction finding — the input of `conductor.improvement.RaiseFinding`.
///
/// Everything it can result in is [`RaiseFindingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaiseFinding {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `signature` — `String`.
    pub signature: String,
    /// `evidence` — `List<String>`.
    pub evidence: Vec<String>,
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `raised_at` — `Timestamp`.
    pub raised_at: crate::primitives::Timestamp,
}

/// Everything `conductor.improvement.RaiseFinding` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaiseFindingOutcome {
    /// `no-such-metric` — when no `conductor.improvement.Metric` carries the identity `input.metric_id` names.
    ///
    /// No metric has this id, and nothing was raised.
    NoSuchMetric {
        /// Why it was refused: `conductor.improvement.MetricNotFound`.
        error: MetricNotFound,
    },
    /// `already-raised` — for an identity a record already carries.
    ///
    /// A finding already has this id; the held finding is unchanged.
    AlreadyRaised {
        /// Why it was refused: `conductor.improvement.FindingExists`.
        error: FindingExists,
    },
    /// `raised` — otherwise.
    ///
    /// The friction is an open finding until conductor-dev files it as a draft story.
    Raised {
        /// The `conductor.improvement.FindingRaised` this outcome publishes.
        finding_raised: FindingRaised,
    },
}

/// Record a metric's reading — the input of `conductor.improvement.RecordMetricReading`.
///
/// Everything it can result in is [`RecordMetricReadingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordMetricReading {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `day` — `String`.
    pub day: String,
    /// `scope` — `String`.
    pub scope: String,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
    /// `sources` — `List<String>`.
    pub sources: Vec<String>,
    /// `target` — `Optional<Decimal>`.
    pub target: Option<crate::primitives::Decimal>,
    /// `met` — `Optional<Boolean>`.
    pub met: Option<bool>,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// Everything `conductor.improvement.RecordMetricReading` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordMetricReadingOutcome {
    /// `no-such-metric` — when no `conductor.improvement.Metric` carries the identity `input.metric_id` names.
    ///
    /// No metric has this id, and nothing was recorded.
    NoSuchMetric {
        /// Why it was refused: `conductor.improvement.MetricNotFound`.
        error: MetricNotFound,
    },
    /// `already-recorded` — for an identity a record already carries.
    ///
    /// The day's reading for the scope is there already, and is unchanged.
    AlreadyRecorded {
        /// Why it was refused: `conductor.improvement.MetricReadingExists`.
        error: MetricReadingExists,
    },
    /// `recorded` — otherwise.
    ///
    /// The metric's value for the scope and day is kept in its series.
    Recorded {
        /// The `conductor.improvement.MetricReadingRecorded` this outcome publishes.
        metric_reading_recorded: MetricReadingRecorded,
    },
}

/// Set a metric's target — the input of `conductor.improvement.SetMetricTarget`.
///
/// Everything it can result in is [`SetMetricTargetOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetMetricTarget {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `target` — `Decimal`.
    pub target: crate::primitives::Decimal,
    /// `direction` — `conductor.improvement.TargetDirection`.
    pub direction: TargetDirection,
    /// `target_owner` — `String`.
    pub target_owner: String,
    /// `target_set_at` — `Timestamp`.
    pub target_set_at: crate::primitives::Timestamp,
}

/// Everything `conductor.improvement.SetMetricTarget` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetMetricTargetOutcome {
    /// `targeted` — when the existing subject is in Untargeted.
    ///
    /// The metric has a target and is a KPI from now on.
    Targeted {
        /// The `conductor.improvement.MetricTargetSet` this outcome publishes.
        metric_target_set: MetricTargetSet,
    },
    /// `retargeted` — when the existing subject is in Targeted.
    ///
    /// The KPI's target is replaced by the one given.
    Retargeted {
        /// The `conductor.improvement.MetricTargetSet` this outcome publishes.
        metric_target_set: MetricTargetSet,
    },
    /// `no-such-metric` — for an identity no record carries.
    NoSuchMetric {
        /// Why it was refused: `conductor.improvement.MetricNotFound`.
        error: MetricNotFound,
    },
}

/// Supersede a metric's reading — the input of `conductor.improvement.SupersedeMetricReading`.
///
/// Everything it can result in is [`SupersedeMetricReadingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupersedeMetricReading {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
    /// `by` — `conductor.improvement.MetricReadingId`.
    pub by: MetricReadingId,
}

/// Everything `conductor.improvement.SupersedeMetricReading` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupersedeMetricReadingOutcome {
    /// `superseded` — otherwise.
    ///
    /// The reading is no longer its metric's newest for its scope.
    Superseded {
        /// The `conductor.improvement.MetricReadingSuperseded` this outcome publishes.
        metric_reading_superseded: MetricReadingSuperseded,
    },
    /// `no-such-reading` — for an identity no record carries.
    NoSuchReading {
        /// Why it was refused: `conductor.improvement.MetricReadingNotFound`.
        error: MetricReadingNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The reading is superseded already, and nothing moved.
    WrongState {
        /// Why it was refused: `conductor.improvement.MetricReadingStateConflict`.
        error: MetricReadingStateConflict,
    },
}

/// Verify a filed finding — the input of `conductor.improvement.VerifyFinding`.
///
/// Everything it can result in is [`VerifyFindingOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyFinding {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `moved` — `Boolean`.
    pub moved: bool,
    /// `verdict` — `String`.
    pub verdict: String,
}

/// Everything `conductor.improvement.VerifyFinding` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyFindingOutcome {
    /// `verified` — when `moved == true`.
    ///
    /// The metric moved; the finding's story did what it was filed for.
    Verified {
        /// The `conductor.improvement.FindingVerified` this outcome publishes.
        finding_verified: FindingVerified,
    },
    /// `closed` — otherwise.
    ///
    /// The metric did not move; the friction is raised again as a new finding.
    Closed {
        /// The `conductor.improvement.FindingClosed` this outcome publishes.
        finding_closed: FindingClosed,
    },
    /// `no-such-finding` — for an identity no record carries.
    NoSuchFinding {
        /// Why it was refused: `conductor.improvement.FindingNotFound`.
        error: FindingNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a filed finding is verified.
    WrongState {
        /// Why it was refused: `conductor.improvement.FindingStateConflict`.
        error: FindingStateConflict,
    },
}

/// FindingClosed — the event `conductor.improvement.FindingClosed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingClosed {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `verdict` — `String`.
    pub verdict: String,
}

/// FindingFiled — the event `conductor.improvement.FindingFiled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingFiled {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `story` — `String`.
    pub story: String,
}

/// FindingRaised — the event `conductor.improvement.FindingRaised`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingRaised {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `signature` — `String`.
    pub signature: String,
    /// `evidence` — `List<String>`.
    pub evidence: Vec<String>,
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
}

/// FindingVerified — the event `conductor.improvement.FindingVerified`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingVerified {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `verdict` — `String`.
    pub verdict: String,
}

/// MetricDefined — the event `conductor.improvement.MetricDefined`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricDefined {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `name` — `String`.
    pub name: String,
    /// `unit` — `String`.
    pub unit: String,
}

/// MetricReadingRecorded — the event `conductor.improvement.MetricReadingRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingRecorded {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `day` — `String`.
    pub day: String,
    /// `scope` — `String`.
    pub scope: String,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
}

/// MetricReadingSuperseded — the event `conductor.improvement.MetricReadingSuperseded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingSuperseded {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
    /// `by` — `conductor.improvement.MetricReadingId`.
    pub by: MetricReadingId,
}

/// MetricTargetSet — the event `conductor.improvement.MetricTargetSet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricTargetSet {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `target` — `Decimal`.
    pub target: crate::primitives::Decimal,
    /// `direction` — `conductor.improvement.TargetDirection`.
    pub direction: TargetDirection,
    /// `target_owner` — `String`.
    pub target_owner: String,
}

/// The declared error `conductor.improvement.FindingExists`.
///
/// A finding already has this id, so nothing was raised; the held finding is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingExists {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
}

/// The declared error `conductor.improvement.FindingNotFound`.
///
/// No finding has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingNotFound {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
}

/// The declared error `conductor.improvement.FindingStateConflict`.
///
/// The finding is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindingStateConflict {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
}

/// The declared error `conductor.improvement.MetricExists`.
///
/// A metric already has this id, so nothing was defined; the held metric is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricExists {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
}

/// The declared error `conductor.improvement.MetricNotFound`.
///
/// No metric has this id; define it first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricNotFound {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
}

/// The declared error `conductor.improvement.MetricReadingExists`.
///
/// The metric already has a reading for this scope and day, so nothing was recorded; the held reading is unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingExists {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
}

/// The declared error `conductor.improvement.MetricReadingNotFound`.
///
/// No reading has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingNotFound {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
}

/// The declared error `conductor.improvement.MetricReadingStateConflict`.
///
/// The reading is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadingStateConflict {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
}

/// Friction findings — one row of the view `conductor.improvement.Findings`.
///
/// Projects `conductor.improvement.FrictionFinding` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Findings {
    /// `finding_id` — `conductor.improvement.FindingId`.
    pub finding_id: FindingId,
    /// `state` — `conductor.improvement.FrictionFinding.State`.
    pub state: FrictionFindingState,
    /// `signature` — `String`.
    pub signature: String,
    /// `evidence` — `List<String>`.
    pub evidence: Vec<String>,
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `raised_at` — `Timestamp`.
    pub raised_at: crate::primitives::Timestamp,
    /// `story` — `Optional<String>`.
    pub story: Option<String>,
    /// `verdict` — `Optional<String>`.
    pub verdict: Option<String>,
}

/// KPI readings — one row of the view `conductor.improvement.KpiReadings`.
///
/// Projects `conductor.improvement.MetricReading` at `read_your_writes` consistency, containing instances where `(state == Recorded and scope == instance and defined(target))`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KpiReadings {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `scope` — `String`.
    pub scope: String,
    /// `day` — `String`.
    pub day: String,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
    /// `target` — `Optional<Decimal>`.
    pub target: Option<crate::primitives::Decimal>,
    /// `met` — `Optional<Boolean>`.
    pub met: Option<bool>,
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
}

/// KPIs — one row of the view `conductor.improvement.Kpis`.
///
/// Projects `conductor.improvement.Metric` at `read_your_writes` consistency, containing instances where `state == Targeted`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kpis {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `state` — `conductor.improvement.Metric.State`.
    pub state: MetricState,
    /// `name` — `String`.
    pub name: String,
    /// `unit` — `String`.
    pub unit: String,
    /// `instrument` — `String`.
    pub instrument: String,
    /// `baseline` — `Optional<Decimal>`.
    pub baseline: Option<crate::primitives::Decimal>,
    /// `target` — `Optional<Decimal>`.
    pub target: Option<crate::primitives::Decimal>,
    /// `direction` — `Optional<conductor.improvement.TargetDirection>`.
    pub direction: Option<TargetDirection>,
    /// `target_owner` — `Optional<String>`.
    pub target_owner: Option<String>,
    /// `target_set_at` — `Optional<Timestamp>`.
    pub target_set_at: Option<crate::primitives::Timestamp>,
}

/// Metric readings — one row of the view `conductor.improvement.MetricReadings`.
///
/// Projects `conductor.improvement.MetricReading` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricReadings {
    /// `reading_id` — `conductor.improvement.MetricReadingId`.
    pub reading_id: MetricReadingId,
    /// `state` — `conductor.improvement.MetricReading.State`.
    pub state: MetricReadingState,
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `day` — `String`.
    pub day: String,
    /// `scope` — `String`.
    pub scope: String,
    /// `value` — `Decimal`.
    pub value: crate::primitives::Decimal,
    /// `sources` — `List<String>`.
    pub sources: Vec<String>,
    /// `target` — `Optional<Decimal>`.
    pub target: Option<crate::primitives::Decimal>,
    /// `met` — `Optional<Boolean>`.
    pub met: Option<bool>,
    /// `recorded_at` — `Timestamp`.
    pub recorded_at: crate::primitives::Timestamp,
}

/// Metrics — one row of the view `conductor.improvement.Metrics`.
///
/// Projects `conductor.improvement.Metric` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metrics {
    /// `metric_id` — `conductor.improvement.MetricId`.
    pub metric_id: MetricId,
    /// `state` — `conductor.improvement.Metric.State`.
    pub state: MetricState,
    /// `name` — `String`.
    pub name: String,
    /// `unit` — `String`.
    pub unit: String,
    /// `instrument` — `String`.
    pub instrument: String,
    /// `definition` — `String`.
    pub definition: String,
    /// `baseline` — `Optional<Decimal>`.
    pub baseline: Option<crate::primitives::Decimal>,
    /// `baseline_period` — `Optional<String>`.
    pub baseline_period: Option<String>,
    /// `target` — `Optional<Decimal>`.
    pub target: Option<crate::primitives::Decimal>,
    /// `direction` — `Optional<conductor.improvement.TargetDirection>`.
    pub direction: Option<TargetDirection>,
    /// `target_owner` — `Optional<String>`.
    pub target_owner: Option<String>,
    /// `target_set_at` — `Optional<Timestamp>`.
    pub target_set_at: Option<crate::primitives::Timestamp>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.improvement.DefineMetric` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait DefineMetricBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.improvement.DefineMetric`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn define_metric(&mut self, input: super::DefineMetric) -> Result<super::DefineMetricOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.improvement.FileFinding` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait FileFindingBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.improvement.FileFinding`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn file_finding(&mut self, input: super::FileFinding) -> Result<super::FileFindingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.improvement.RaiseFinding` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RaiseFindingBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.improvement.RaiseFinding`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn raise_finding(&mut self, input: super::RaiseFinding) -> Result<super::RaiseFindingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.improvement.RecordMetricReading` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordMetricReadingBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.improvement.RecordMetricReading`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_metric_reading(&mut self, input: super::RecordMetricReading) -> Result<super::RecordMetricReadingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.improvement.SetMetricTarget` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a subject predicate choosing between a move and an update.
    ///
    /// Contract: given `conductor.improvement.SetMetricTarget` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `targeted` when the existing subject is in Untargeted, takes `target` of `conductor.improvement.Metric`, emits `conductor.improvement.MetricTargetSet`; `retargeted` when the existing subject is in Targeted, updates `conductor.improvement.Metric`, emits `conductor.improvement.MetricTargetSet`; `no-such-metric` for an identity no record carries, error `conductor.improvement.MetricNotFound`.
    pub trait SetMetricTargetBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.improvement.SetMetricTarget`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn set_metric_target(&mut self, input: super::SetMetricTarget) -> Result<super::SetMetricTargetOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.improvement.SupersedeMetricReading` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait SupersedeMetricReadingBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.improvement.SupersedeMetricReading`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn supersede_metric_reading(&mut self, input: super::SupersedeMetricReading) -> Result<super::SupersedeMetricReadingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.improvement.VerifyFinding` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait VerifyFindingBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.improvement.VerifyFinding`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn verify_finding(&mut self, input: super::VerifyFinding) -> Result<super::VerifyFindingOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.improvement.Findings` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait FindingsQuery {
        /// Serves `conductor.improvement.Findings` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn findings(&self) -> Result<Vec<super::Findings>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.improvement.KpiReadings` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait KpiReadingsQuery {
        /// Serves `conductor.improvement.KpiReadings` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn kpi_readings(&self) -> Result<Vec<super::KpiReadings>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.improvement.Kpis` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait KpisQuery {
        /// Serves `conductor.improvement.Kpis` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn kpis(&self) -> Result<Vec<super::Kpis>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.improvement.MetricReadings` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MetricReadingsQuery {
        /// Serves `conductor.improvement.MetricReadings` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn metric_readings(&self) -> Result<Vec<super::MetricReadings>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.improvement.Metrics` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MetricsQuery {
        /// Serves `conductor.improvement.Metrics` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn metrics(&self) -> Result<Vec<super::Metrics>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl SetMetricTargetBehavior for Unimplemented {
        fn set_metric_target(&mut self, _input: super::SetMetricTarget) -> Result<super::SetMetricTargetOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.improvement.SetMetricTarget" })
        }
    }
}
