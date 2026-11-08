// generated from conductor v1
// contract digest 0be0922aec9b8f10c3509792e1441579a2719359e1cf48bcc0d06cbee2a292de
// do not edit: regenerate with `ess synthesize --layout crate`

//! Store — `conductor.store`.
//!
//! One shared database per deployment (design docs/design/architecture-next.md § 8, decision 13), opened through Eventlog's port: `sqlite` for a deployment that is one host, `postgres` for everything else. `daemon.store` names the one URL; the daemon hands it to every instance runtime it starts; every event names its instance; every UI and every operator reads the same records. No copy per instance, no sync, no restore. Today's eventlog tree is read once by the migration (`conductor store migrate`) and not opened again.
//! No command opens the store: § 2 and § 8 say the daemon opens it at start and hands its URL to every runtime, and neither lists that opening among the records the store keeps (decisions, dispatches, readings, findings, charters, hand-offs, snapshots, config changes, reconciliations). The `Store` row is the deployment's, not a command's.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Backend — `conductor.store.Backend`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// `Sqlite`.
    Sqlite,
    /// `Postgres`.
    Postgres,
}

/// The states of `conductor.store.Migration`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Migration<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationState {
    /// `Migrated`.
    Migrated,
}

/// The states of `conductor.store.Store`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Store<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreState {
    /// `Opened`.
    Opened,
}

/// StoreUrl — `conductor.store.StoreUrl`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUrl(pub String);

/// What Migration — `conductor.store.Migration` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Migration<S>`], and at a boundary by [`MigrationSnapshot::state`].
///
/// Every value satisfies `events_copied >= 0` — checked by [`MigrationData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationData {
    /// The identity: `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `source` — `String`.
    pub source: String,
    /// `store` — `conductor.store.StoreUrl`.
    ///
    /// Carries `store`: `conductor.store.Migration` references one `conductor.store.Store`.
    pub store: StoreUrl,
    /// `events_copied` — `Integer`.
    pub events_copied: i64,
    /// `migrated_at` — `Timestamp`.
    pub migrated_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
}

impl MigrationData {
    /// The first declared invariant of `conductor.store.Migration` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.events_copied)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("events_copied >= 0");
        }
        None
    }
}

/// The states of `conductor.store.Migration`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](migration_state::Marker), so [`Migration<S>`](Migration) can only ever rest in a real state.
pub mod migration_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Migrated {}
    }

    /// A declared state of `Migration`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MigrationState;
    }

    /// `Migrated`. Where a new instance starts.
    pub struct Migrated;

    impl Marker for Migrated {
        const STATE: super::MigrationState = super::MigrationState::Migrated;
    }
}

/// Migration — `conductor.store.Migration` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Migrated`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MigrationSnapshot`]
/// and [`MigrationSnapshot::refine`].
pub struct Migration<S: migration_state::Marker> {
    data: MigrationData,
    state: core::marker::PhantomData<S>,
}

impl<S: migration_state::Marker> Migration<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MigrationState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MigrationData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MigrationData {
        self.data
    }
}

impl Migration<migration_state::Migrated> {
    /// A new instance, resting in `Migrated` — the only state the lifecycle starts one in.
    pub fn new(data: MigrationData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.store.Migration` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MigrationSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MigrationState,
    /// What it holds.
    pub data: MigrationData,
}

/// An `Migration` in whichever declared state it was found.
pub enum AnyMigration {
    /// Resting in `Migrated`.
    Migrated(Migration<migration_state::Migrated>),
}

impl MigrationSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MigrationState` cannot spell one.
    pub fn refine(self) -> AnyMigration {
        match self.state {
            MigrationState::Migrated => AnyMigration::Migrated(Migration {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMigration {
    /// The state, as the runtime value.
    pub fn state(&self) -> MigrationState {
        match self {
            Self::Migrated(_) => MigrationState::Migrated,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MigrationSnapshot {
        match self {
            Self::Migrated(instance) => MigrationSnapshot {
                state: MigrationState::Migrated,
                data: instance.into_data(),
            },
        }
    }
}

/// What Store — `conductor.store.Store` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Store<S>`], and at a boundary by [`StoreSnapshot::state`].
///
/// Every value satisfies `last_event_id >= 0` — checked by [`StoreData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreData {
    /// The identity: `url` — `conductor.store.StoreUrl`.
    pub url: StoreUrl,
    /// `backend` — `conductor.store.Backend`.
    pub backend: Backend,
    /// `opened_at` — `Timestamp`.
    pub opened_at: crate::primitives::Timestamp,
    /// `reachable` — `Boolean`.
    pub reachable: bool,
    /// `last_event_id` — `Integer`.
    pub last_event_id: i64,
    /// `instance_last_event_ids` — `Map<String, Integer>`.
    pub instance_last_event_ids: std::collections::BTreeMap<String, i64>,
}

impl StoreData {
    /// The first declared invariant of `conductor.store.Store` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.last_event_id)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("last_event_id >= 0");
        }
        None
    }
}

/// The states of `conductor.store.Store`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](store_state::Marker), so [`Store<S>`](Store) can only ever rest in a real state.
pub mod store_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Opened {}
    }

    /// A declared state of `Store`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::StoreState;
    }

    /// `Opened`. Where a new instance starts.
    pub struct Opened;

    impl Marker for Opened {
        const STATE: super::StoreState = super::StoreState::Opened;
    }
}

/// Store — `conductor.store.Store` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Opened`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`StoreSnapshot`]
/// and [`StoreSnapshot::refine`].
pub struct Store<S: store_state::Marker> {
    data: StoreData,
    state: core::marker::PhantomData<S>,
}

impl<S: store_state::Marker> Store<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> StoreState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &StoreData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> StoreData {
        self.data
    }
}

impl Store<store_state::Opened> {
    /// A new instance, resting in `Opened` — the only state the lifecycle starts one in.
    pub fn new(data: StoreData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.store.Store` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`StoreSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: StoreState,
    /// What it holds.
    pub data: StoreData,
}

/// An `Store` in whichever declared state it was found.
pub enum AnyStore {
    /// Resting in `Opened`.
    Opened(Store<store_state::Opened>),
}

impl StoreSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `StoreState` cannot spell one.
    pub fn refine(self) -> AnyStore {
        match self.state {
            StoreState::Opened => AnyStore::Opened(Store {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyStore {
    /// The state, as the runtime value.
    pub fn state(&self) -> StoreState {
        match self {
            Self::Opened(_) => StoreState::Opened,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> StoreSnapshot {
        match self {
            Self::Opened(instance) => StoreSnapshot {
                state: StoreState::Opened,
                data: instance.into_data(),
            },
        }
    }
}

/// Migrate an eventlog tree into the store — the input of `conductor.store.MigrateStore`.
///
/// Everything it can result in is [`MigrateStoreOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateStore {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `source` — `String`.
    pub source: String,
    /// `store` — `conductor.store.StoreUrl`.
    pub store: StoreUrl,
    /// `events_copied` — `Integer`.
    pub events_copied: i64,
    /// `migrated_at` — `Timestamp`.
    pub migrated_at: crate::primitives::Timestamp,
}

/// Everything `conductor.store.MigrateStore` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrateStoreOutcome {
    /// `not-human` — when `actor.kind != Human`.
    ///
    /// An agent ran the operator's one-time step; it is refused and nothing was copied.
    NotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: crate::operator::ActorNotHuman,
    },
    /// `negative-count` — when `events_copied < 0`.
    ///
    /// A count of events is never negative; nothing was recorded.
    NegativeCount {
        /// Why it was refused: `conductor.store.InvalidEventCount`.
        error: InvalidEventCount,
    },
    /// `already-migrated` — when, of the `conductor.store.Migration` rows satisfying `instance == input.instance`, at least one row is selected.
    ///
    /// The instance's tree is in the store already; it is not read again.
    AlreadyMigrated {
        /// Why it was refused: `conductor.store.AlreadyMigrated`.
        error: AlreadyMigrated,
    },
    /// `source-missing` — externally decided (no eventlog tree is at the source path).
    ///
    /// There is no tree to read; nothing was copied.
    SourceMissing {
        /// Why it was refused: `conductor.store.SourceMissing`.
        error: SourceMissing,
    },
    /// `migrated` — otherwise.
    ///
    /// The instance's tree is in the store, the count copied is on record, and the tree is not opened again.
    Migrated {
        /// The `conductor.store.StoreMigrated` this outcome publishes.
        store_migrated: StoreMigrated,
    },
}

/// StoreMigrated — the event `conductor.store.StoreMigrated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreMigrated {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `source` — `String`.
    pub source: String,
    /// `store` — `conductor.store.StoreUrl`.
    pub store: StoreUrl,
    /// `events_copied` — `Integer`.
    pub events_copied: i64,
    /// `migrated_at` — `Timestamp`.
    pub migrated_at: crate::primitives::Timestamp,
}

/// The declared error `conductor.store.AlreadyMigrated`.
///
/// The instance's tree was read into the store before; it is read once and not opened again, so nothing was copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlreadyMigrated {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
}

/// The declared error `conductor.store.InvalidEventCount`.
///
/// A count of events copied is never negative, so no migration was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidEventCount {
    /// `events_copied` — `Integer`.
    pub events_copied: i64,
}

/// The declared error `conductor.store.SourceMissing`.
///
/// No eventlog tree is at the source path, so nothing was copied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMissing {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `source` — `String`.
    pub source: String,
}

/// Store migrations — one row of the view `conductor.store.Migrations`.
///
/// Projects `conductor.store.Migration` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migrations {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `state` — `conductor.store.Migration.State`.
    pub state: MigrationState,
    /// `source` — `String`.
    pub source: String,
    /// `store` — `conductor.store.StoreUrl`.
    pub store: StoreUrl,
    /// `events_copied` — `Integer`.
    pub events_copied: i64,
    /// `migrated_at` — `Timestamp`.
    pub migrated_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
}

/// Store status — one row of the view `conductor.store.StoreStatus`.
///
/// Projects `conductor.store.Store` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreStatus {
    /// `url` — `conductor.store.StoreUrl`.
    pub url: StoreUrl,
    /// `state` — `conductor.store.Store.State`.
    pub state: StoreState,
    /// `backend` — `conductor.store.Backend`.
    pub backend: Backend,
    /// `reachable` — `Boolean`.
    pub reachable: bool,
    /// `last_event_id` — `Integer`.
    pub last_event_id: i64,
    /// `instance_last_event_ids` — `Map<String, Integer>`.
    pub instance_last_event_ids: std::collections::BTreeMap<String, i64>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.store.MigrateStore` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a guard over the rows a selector selects (`when_related: {entity, where, exists | count | forall}`), in `already-migrated`.
    ///
    /// Contract: given `conductor.store.MigrateStore` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `not-human` when `actor.kind != Human`, error `conductor.operator.ActorNotHuman`; `negative-count` when `events_copied < 0`, error `conductor.store.InvalidEventCount`; `already-migrated` when, of the `conductor.store.Migration` rows satisfying `instance == input.instance`, at least one row is selected, error `conductor.store.AlreadyMigrated`; `source-missing` externally decided (no eventlog tree is at the source path), error `conductor.store.SourceMissing`; `migrated` otherwise, creates `conductor.store.Migration`, emits `conductor.store.StoreMigrated`.
    pub trait MigrateStoreBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.store.MigrateStore`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn migrate_store(&mut self, input: super::MigrateStore) -> Result<super::MigrateStoreOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.store.Migrations` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MigrationsQuery {
        /// Serves `conductor.store.Migrations` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn migrations(&self) -> Result<Vec<super::Migrations>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.store.StoreStatus` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait StoreStatusQuery {
        /// Serves `conductor.store.StoreStatus` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn store_status(&self) -> Result<Vec<super::StoreStatus>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl MigrateStoreBehavior for Unimplemented {
        fn migrate_store(&mut self, _input: super::MigrateStore) -> Result<super::MigrateStoreOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.store.MigrateStore" })
        }
    }
}
