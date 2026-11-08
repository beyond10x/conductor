// generated from conductor v1
// contract digest 925e95ab19211d2883a7b63b82a83c2503c557df74d9a01d0be9ddc0a4f90d6c
// do not edit: regenerate with `ess synthesize --layout crate`

//! Secret — `conductor.secret`.
//!
//! The config names secrets and never holds them (design docs/design/architecture-next.md § 9, decision 6): a `secret:<name>` is resolved by the daemon through the `secrets` crates, the OS keychain on a workstation and the Secrets service where conductord is deployed, and handed to a box as a sealed slot at box open (§ 6). The API lists names and sources, never values. The specification holds no secret value anywhere: no type, field, input, event, error or view below carries one.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Namespace — `conductor.secret.Namespace`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace(pub String);

/// The states of `conductor.secret.Secret`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Secret<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretState {
    /// `Stored`.
    Stored,
}

/// SecretName — `conductor.secret.SecretName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretName(pub String);

/// Slot — `conductor.secret.Slot`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// `secret` — `conductor.secret.SecretName`.
    pub secret: SecretName,
    /// `box` — `conductor.placement.BoxId`.
    pub r#box: crate::placement::BoxId,
    /// `slot_name` — `conductor.secret.SlotName`.
    pub slot_name: SlotName,
    /// `sealed_at` — `Timestamp`.
    pub sealed_at: crate::primitives::Timestamp,
}

/// SlotName — `conductor.secret.SlotName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotName(pub String);

/// Source — `conductor.secret.Source`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// `Keychain`.
    Keychain,
    /// `Remote`.
    Remote,
}

/// What Secret — `conductor.secret.Secret` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Secret<S>`], and at a boundary by [`SecretSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretData {
    /// The identity: `name` — `conductor.secret.SecretName`.
    pub name: SecretName,
    /// `source` — `conductor.secret.Source`.
    pub source: Source,
    /// `namespace` — `Optional<conductor.secret.Namespace>`.
    pub namespace: Option<Namespace>,
    /// `instances` — `List<conductor.config.InstanceName>`.
    ///
    /// Carries `instances`: `conductor.secret.Secret` references many `conductor.instance.Instance`.
    pub instances: Vec<crate::config::InstanceName>,
    /// `created_by` — `conductor.operator.Actor`.
    pub created_by: crate::operator::Actor,
    /// `created_at` — `Timestamp`.
    pub created_at: crate::primitives::Timestamp,
    /// `last_resolved_at` — `Optional<Timestamp>`.
    pub last_resolved_at: Option<crate::primitives::Timestamp>,
}

/// The states of `conductor.secret.Secret`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](secret_state::Marker), so [`Secret<S>`](Secret) can only ever rest in a real state.
pub mod secret_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Stored {}
    }

    /// A declared state of `Secret`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::SecretState;
    }

    /// `Stored`. Where a new instance starts.
    pub struct Stored;

    impl Marker for Stored {
        const STATE: super::SecretState = super::SecretState::Stored;
    }
}

/// Secret — `conductor.secret.Secret` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Stored`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`SecretSnapshot`]
/// and [`SecretSnapshot::refine`].
pub struct Secret<S: secret_state::Marker> {
    data: SecretData,
    state: core::marker::PhantomData<S>,
}

impl<S: secret_state::Marker> Secret<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> SecretState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &SecretData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> SecretData {
        self.data
    }
}

impl Secret<secret_state::Stored> {
    /// A new instance, resting in `Stored` — the only state the lifecycle starts one in.
    pub fn new(data: SecretData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.secret.Secret` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`SecretSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: SecretState,
    /// What it holds.
    pub data: SecretData,
}

/// An `Secret` in whichever declared state it was found.
pub enum AnySecret {
    /// Resting in `Stored`.
    Stored(Secret<secret_state::Stored>),
}

impl SecretSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `SecretState` cannot spell one.
    pub fn refine(self) -> AnySecret {
        match self.state {
            SecretState::Stored => AnySecret::Stored(Secret {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnySecret {
    /// The state, as the runtime value.
    pub fn state(&self) -> SecretState {
        match self {
            Self::Stored(_) => SecretState::Stored,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> SecretSnapshot {
        match self {
            Self::Stored(instance) => SecretSnapshot {
                state: SecretState::Stored,
                data: instance.into_data(),
            },
        }
    }
}

/// Put a secret into the keychain — the input of `conductor.secret.PutSecret`.
///
/// Everything it can result in is [`PutSecretOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutSecret {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.secret.SecretName`.
    pub name: SecretName,
    /// `put_at` — `Timestamp`.
    pub put_at: crate::primitives::Timestamp,
}

/// Everything `conductor.secret.PutSecret` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PutSecretOutcome {
    /// `not-human` — when `actor.kind != Human`.
    ///
    /// An agent actor holds no credential and puts none; nothing was written.
    NotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: crate::operator::ActorNotHuman,
    },
    /// `unreachable` — externally decided (the OS keychain does not answer).
    ///
    /// Nothing was written to the keychain and nothing recorded.
    Unreachable {
        /// Why it was refused: `conductor.secret.SourceUnreachable`.
        error: SourceUnreachable,
    },
    /// `stored` — for an identity no record carries.
    ///
    /// The value is in the keychain under the name, and the secret is on record with its source and who put it.
    Stored {
        /// The `conductor.secret.SecretPut` this outcome publishes.
        secret_put: SecretPut,
    },
    /// `replaced` — otherwise.
    ///
    /// The value under the name is written again; the record is unchanged.
    Replaced {
        /// The `conductor.secret.SecretPut` this outcome publishes.
        secret_put: SecretPut,
    },
}

/// Resolve an instance's secrets at box open — the input of `conductor.secret.ResolveSecrets`.
///
/// Everything it can result in is [`ResolveSecretsOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveSecrets {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `box` — `conductor.placement.BoxId`.
    pub r#box: crate::placement::BoxId,
    /// `slots` — `List<conductor.secret.Slot>`.
    pub slots: Vec<Slot>,
    /// `resolved_at` — `Timestamp`.
    pub resolved_at: crate::primitives::Timestamp,
}

/// Everything `conductor.secret.ResolveSecrets` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveSecretsOutcome {
    /// `unreachable` — externally decided (the daemon's secrets source (the OS keychain or the Secrets service) does not answer).
    ///
    /// Nothing was resolved and no slot sealed; the box is not opened, and the daemon records why.
    Unreachable {
        /// Why it was refused: `conductor.secret.SourceUnreachable`.
        error: SourceUnreachable,
    },
    /// `missing` — externally decided (the secrets source holds no value under one or more of the names the slots carry).
    ///
    /// No slot was sealed; the refusal names every missing secret, and the box is not opened.
    Missing {
        /// Why it was refused: `conductor.secret.SecretMissing`.
        error: SecretMissing,
    },
    /// `resolved` — otherwise.
    ///
    /// Every named secret is sealed into the box as a slot; the record names the slots and never a value.
    Resolved {
        /// The `conductor.secret.SecretsResolved` this outcome publishes.
        secrets_resolved: SecretsResolved,
    },
}

/// SecretPut — the event `conductor.secret.SecretPut`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPut {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.secret.SecretName`.
    pub name: SecretName,
    /// `source` — `conductor.secret.Source`.
    pub source: Source,
    /// `put_at` — `Timestamp`.
    pub put_at: crate::primitives::Timestamp,
}

/// SecretsResolved — the event `conductor.secret.SecretsResolved`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretsResolved {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `box` — `conductor.placement.BoxId`.
    pub r#box: crate::placement::BoxId,
    /// `slots` — `List<conductor.secret.Slot>`.
    pub slots: Vec<Slot>,
    /// `resolved_at` — `Timestamp`.
    pub resolved_at: crate::primitives::Timestamp,
}

/// The declared error `conductor.secret.SecretMissing`.
///
/// The secrets source holds no value under one or more of the names the instance's effective config writes as `secret:<name>`, so no slot was sealed and the box is not opened; the refusal names every missing name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretMissing {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `box` — `conductor.placement.BoxId`.
    pub r#box: crate::placement::BoxId,
    /// `names` — `List<conductor.secret.SecretName>`.
    pub names: Vec<SecretName>,
}

/// The declared error `conductor.secret.SourceUnreachable`.
///
/// The secrets source (the OS keychain, or the Secrets service) did not answer, so nothing was read or written; the refusal names which source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnreachable {
    /// `source` — `conductor.secret.Source`.
    pub source: Source,
}

/// Secrets — one row of the view `conductor.secret.Secrets`.
///
/// Projects `conductor.secret.Secret` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secrets {
    /// `name` — `conductor.secret.SecretName`.
    pub name: SecretName,
    /// `state` — `conductor.secret.Secret.State`.
    pub state: SecretState,
    /// `source` — `conductor.secret.Source`.
    pub source: Source,
    /// `namespace` — `Optional<conductor.secret.Namespace>`.
    pub namespace: Option<Namespace>,
    /// `instances` — `List<conductor.config.InstanceName>`.
    pub instances: Vec<crate::config::InstanceName>,
    /// `created_by` — `conductor.operator.Actor`.
    pub created_by: crate::operator::Actor,
    /// `created_at` — `Timestamp`.
    pub created_at: crate::primitives::Timestamp,
    /// `last_resolved_at` — `Optional<Timestamp>`.
    pub last_resolved_at: Option<crate::primitives::Timestamp>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.secret.PutSecret` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by `creates:` leaving the required field `instances` of `conductor.secret.Secret` undetermined, in `stored`.
    ///
    /// Contract: given `conductor.secret.PutSecret` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `not-human` when `actor.kind != Human`, error `conductor.operator.ActorNotHuman`; `unreachable` externally decided (the OS keychain does not answer), error `conductor.secret.SourceUnreachable`; `stored` for an identity no record carries, creates `conductor.secret.Secret`, emits `conductor.secret.SecretPut`; `replaced` otherwise, updates `conductor.secret.Secret`, emits `conductor.secret.SecretPut`.
    pub trait PutSecretBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.secret.PutSecret`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn put_secret(&mut self, input: super::PutSecret) -> Result<super::PutSecretOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.secret.ResolveSecrets` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by the fields of error `conductor.secret.SourceUnreachable`, which the specification gives no source, in `unreachable`.
    ///
    /// Contract: given `conductor.secret.ResolveSecrets` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `unreachable` externally decided (the daemon's secrets source (the OS keychain or the Secrets service) does not answer), error `conductor.secret.SourceUnreachable`; `missing` externally decided (the secrets source holds no value under one or more of the names the slots carry), error `conductor.secret.SecretMissing`; `resolved` otherwise, emits `conductor.secret.SecretsResolved`.
    pub trait ResolveSecretsBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.secret.ResolveSecrets`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn resolve_secrets(&mut self, input: super::ResolveSecrets) -> Result<super::ResolveSecretsOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.secret.Secrets` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait SecretsQuery {
        /// Serves `conductor.secret.Secrets` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn secrets(&self) -> Result<Vec<super::Secrets>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl PutSecretBehavior for Unimplemented {
        fn put_secret(&mut self, _input: super::PutSecret) -> Result<super::PutSecretOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.secret.PutSecret" })
        }
    }

    impl ResolveSecretsBehavior for Unimplemented {
        fn resolve_secrets(&mut self, _input: super::ResolveSecrets) -> Result<super::ResolveSecretsOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.secret.ResolveSecrets" })
        }
    }
}
