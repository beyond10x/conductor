// generated from conductor v1
// contract digest 990a21ced6fe27f2052a755ba5a22f383ffd50d9d3f92b3324424d62542ace75
// do not edit: regenerate with `ess synthesize --layout crate`

//! Token — `conductor.token`.
//!
//! The token pool of a host (design docs/design/architecture-next.md § 5.3, decisions 10 and 11; story:host-token-pool): conductord's store holds one pool per host, shared by every instance it runs. An engineer who joins contributes their Claude token to it; the pool is a set of secrets, each with its owner, its seat and its state (`ok`, or `limited until <time>`); values never leave the secrets source but for the one start that leases them. The `claude-code` kind of any instance on the host leases a token per agent start (a token serves any number of agents at once, one lease each), resumes an agent on another token after a usage-limit stop, and names the token's owner on every agent record. A controller started for Remote Control on a named owner's token appears in that owner's Claude app.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// JoinDaemon — `conductor.token.JoinDaemon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinDaemon {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
    /// `seat` — `Optional<conductor.token.Seat>`.
    pub seat: Option<Seat>,
    /// `kind` — `Optional<conductor.config.ModelTokenKind>`.
    pub kind: Option<crate::config::ModelTokenKind>,
}

/// LeaveDaemon — `conductor.token.LeaveDaemon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveDaemon {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
}

/// The states of `conductor.token.PoolToken`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `PoolToken<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolTokenState {
    /// `Available`.
    Available,
    /// `Limited`.
    Limited,
    /// `Withdrawn`.
    Withdrawn,
}

/// RevealToken — `conductor.token.RevealToken`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealToken {
    /// `model_token` — `conductor.config.InstanceName`.
    pub model_token: crate::config::InstanceName,
}

/// Seat — `conductor.token.Seat`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat(pub String);

/// TokenId — `conductor.token.TokenId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenId(pub crate::primitives::Uuid);

/// The states of `conductor.token.TokenLease`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `TokenLease<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenLeaseState {
    /// `Held`.
    Held,
    /// `Released`.
    Released,
}

/// TokenLeaseId — `conductor.token.TokenLeaseId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLeaseId(pub crate::primitives::Uuid);

/// TokenState — `conductor.token.TokenState`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenState {
    /// `Ok`.
    Ok,
    /// `Limited`.
    Limited,
}

/// What PoolToken — `conductor.token.PoolToken` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`PoolToken<S>`], and at a boundary by [`PoolTokenSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolTokenData {
    /// The identity: `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
    /// `secret` — `conductor.secret.SecretName`.
    ///
    /// Carries `secret`: `conductor.token.PoolToken` references one `conductor.secret.Secret`.
    pub secret: crate::secret::SecretName,
    /// `token_state` — `conductor.token.TokenState`.
    pub token_state: TokenState,
    /// `limited_until` — `Optional<Timestamp>`.
    pub limited_until: Option<crate::primitives::Timestamp>,
    /// `contributed_at` — `Timestamp`.
    pub contributed_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `kind` — `Optional<conductor.config.ModelTokenKind>`.
    pub kind: Option<crate::config::ModelTokenKind>,
}

/// The states of `conductor.token.PoolToken`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](pool_token_state::Marker), so [`PoolToken<S>`](PoolToken) can only ever rest in a real state.
pub mod pool_token_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Available {}
        impl Sealed for super::Limited {}
        impl Sealed for super::Withdrawn {}
    }

    /// A declared state of `PoolToken`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::PoolTokenState;
    }

    /// `Available`. Where a new instance starts.
    pub struct Available;

    impl Marker for Available {
        const STATE: super::PoolTokenState = super::PoolTokenState::Available;
    }

    /// `Limited`.
    pub struct Limited;

    impl Marker for Limited {
        const STATE: super::PoolTokenState = super::PoolTokenState::Limited;
    }

    /// `Withdrawn`. Terminal: an instance may rest here forever.
    pub struct Withdrawn;

    impl Marker for Withdrawn {
        const STATE: super::PoolTokenState = super::PoolTokenState::Withdrawn;
    }
}

/// PoolToken — `conductor.token.PoolToken` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Available`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`PoolTokenSnapshot`]
/// and [`PoolTokenSnapshot::refine`].
pub struct PoolToken<S: pool_token_state::Marker> {
    data: PoolTokenData,
    state: core::marker::PhantomData<S>,
}

impl<S: pool_token_state::Marker> PoolToken<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> PoolTokenState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &PoolTokenData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> PoolTokenData {
        self.data
    }
}

impl PoolToken<pool_token_state::Available> {
    /// A new instance, resting in `Available` — the only state the lifecycle starts one in.
    pub fn new(data: PoolTokenData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl PoolToken<pool_token_state::Available> {
    /// `limit` — `Available` → `Limited`. Taken by the `limited` outcome of `conductor.token.MarkTokenLimited`.
    pub fn limit(self) -> PoolToken<pool_token_state::Limited> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `withdraw` — `Available` → `Withdrawn`. Taken by the `withdrawn` outcome of `conductor.token.WithdrawToken`.
    pub fn withdraw(self) -> PoolToken<pool_token_state::Withdrawn> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl PoolToken<pool_token_state::Limited> {
    /// `limit` — `Limited` → `Limited`. Taken by the `limited` outcome of `conductor.token.MarkTokenLimited`.
    pub fn limit(self) -> PoolToken<pool_token_state::Limited> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `restore` — `Limited` → `Available`. Taken by the `restored` outcome of `conductor.token.RestoreToken`.
    pub fn restore(self) -> PoolToken<pool_token_state::Available> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `withdraw` — `Limited` → `Withdrawn`. Taken by the `withdrawn` outcome of `conductor.token.WithdrawToken`.
    pub fn withdraw(self) -> PoolToken<pool_token_state::Withdrawn> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.token.PoolToken` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`PoolTokenSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolTokenSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: PoolTokenState,
    /// What it holds.
    pub data: PoolTokenData,
}

/// An `PoolToken` in whichever declared state it was found.
pub enum AnyPoolToken {
    /// Resting in `Available`.
    Available(PoolToken<pool_token_state::Available>),
    /// Resting in `Limited`.
    Limited(PoolToken<pool_token_state::Limited>),
    /// Resting in `Withdrawn`.
    Withdrawn(PoolToken<pool_token_state::Withdrawn>),
}

impl PoolTokenSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `PoolTokenState` cannot spell one.
    pub fn refine(self) -> AnyPoolToken {
        match self.state {
            PoolTokenState::Available => AnyPoolToken::Available(PoolToken {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            PoolTokenState::Limited => AnyPoolToken::Limited(PoolToken {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            PoolTokenState::Withdrawn => AnyPoolToken::Withdrawn(PoolToken {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyPoolToken {
    /// The state, as the runtime value.
    pub fn state(&self) -> PoolTokenState {
        match self {
            Self::Available(_) => PoolTokenState::Available,
            Self::Limited(_) => PoolTokenState::Limited,
            Self::Withdrawn(_) => PoolTokenState::Withdrawn,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> PoolTokenSnapshot {
        match self {
            Self::Available(instance) => PoolTokenSnapshot {
                state: PoolTokenState::Available,
                data: instance.into_data(),
            },
            Self::Limited(instance) => PoolTokenSnapshot {
                state: PoolTokenState::Limited,
                data: instance.into_data(),
            },
            Self::Withdrawn(instance) => PoolTokenSnapshot {
                state: PoolTokenState::Withdrawn,
                data: instance.into_data(),
            },
        }
    }
}

/// What TokenLease — `conductor.token.TokenLease` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`TokenLease<S>`], and at a boundary by [`TokenLeaseSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLeaseData {
    /// The identity: `lease_id` — `conductor.token.TokenLeaseId`.
    pub lease_id: TokenLeaseId,
    /// `token_id` — `conductor.token.TokenId`.
    ///
    /// Carries `leases`: `conductor.token.PoolToken` owns many `conductor.token.TokenLease`.
    pub token_id: TokenId,
    /// `agent` — `conductor.agent.AgentName`.
    ///
    /// Carries `agent`: `conductor.token.TokenLease` references one `conductor.agent.Agent`.
    pub agent: crate::agent::AgentName,
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `instance`: `conductor.token.TokenLease` references one `conductor.instance.Instance`.
    pub instance: crate::config::InstanceName,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `preferred_owner` — `Optional<conductor.operator.OperatorName>`.
    pub preferred_owner: Option<crate::operator::OperatorName>,
    /// `leased_at` — `Timestamp`.
    pub leased_at: crate::primitives::Timestamp,
    /// `released_at` — `Optional<Timestamp>`.
    pub released_at: Option<crate::primitives::Timestamp>,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `runtime` — `Optional<String>`.
    pub runtime: Option<String>,
}

/// The states of `conductor.token.TokenLease`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](token_lease_state::Marker), so [`TokenLease<S>`](TokenLease) can only ever rest in a real state.
pub mod token_lease_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Held {}
        impl Sealed for super::Released {}
    }

    /// A declared state of `TokenLease`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::TokenLeaseState;
    }

    /// `Held`. Where a new instance starts.
    pub struct Held;

    impl Marker for Held {
        const STATE: super::TokenLeaseState = super::TokenLeaseState::Held;
    }

    /// `Released`. Terminal: an instance may rest here forever.
    pub struct Released;

    impl Marker for Released {
        const STATE: super::TokenLeaseState = super::TokenLeaseState::Released;
    }
}

/// TokenLease — `conductor.token.TokenLease` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Held`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`TokenLeaseSnapshot`]
/// and [`TokenLeaseSnapshot::refine`].
pub struct TokenLease<S: token_lease_state::Marker> {
    data: TokenLeaseData,
    state: core::marker::PhantomData<S>,
}

impl<S: token_lease_state::Marker> TokenLease<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> TokenLeaseState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &TokenLeaseData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> TokenLeaseData {
        self.data
    }
}

impl TokenLease<token_lease_state::Held> {
    /// A new instance, resting in `Held` — the only state the lifecycle starts one in.
    pub fn new(data: TokenLeaseData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl TokenLease<token_lease_state::Held> {
    /// `release` — `Held` → `Released`. Taken by the `released` outcome of `conductor.token.ReleaseToken`.
    pub fn release(self) -> TokenLease<token_lease_state::Released> {
        TokenLease {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.token.TokenLease` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`TokenLeaseSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLeaseSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: TokenLeaseState,
    /// What it holds.
    pub data: TokenLeaseData,
}

/// An `TokenLease` in whichever declared state it was found.
pub enum AnyTokenLease {
    /// Resting in `Held`.
    Held(TokenLease<token_lease_state::Held>),
    /// Resting in `Released`.
    Released(TokenLease<token_lease_state::Released>),
}

impl TokenLeaseSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `TokenLeaseState` cannot spell one.
    pub fn refine(self) -> AnyTokenLease {
        match self.state {
            TokenLeaseState::Held => AnyTokenLease::Held(TokenLease {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            TokenLeaseState::Released => AnyTokenLease::Released(TokenLease {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyTokenLease {
    /// The state, as the runtime value.
    pub fn state(&self) -> TokenLeaseState {
        match self {
            Self::Held(_) => TokenLeaseState::Held,
            Self::Released(_) => TokenLeaseState::Released,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> TokenLeaseSnapshot {
        match self {
            Self::Held(instance) => TokenLeaseSnapshot {
                state: TokenLeaseState::Held,
                data: instance.into_data(),
            },
            Self::Released(instance) => TokenLeaseSnapshot {
                state: TokenLeaseState::Released,
                data: instance.into_data(),
            },
        }
    }
}

/// Contribute a token to the pool — the input of `conductor.token.ContributeToken`.
///
/// Everything it can result in is [`ContributeTokenOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContributeToken {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
    /// `secret` — `conductor.secret.SecretName`.
    pub secret: crate::secret::SecretName,
    /// `kind` — `conductor.config.ModelTokenKind`.
    pub kind: crate::config::ModelTokenKind,
    /// `contributed_at` — `Timestamp`.
    pub contributed_at: crate::primitives::Timestamp,
}

/// Everything `conductor.token.ContributeToken` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContributeTokenOutcome {
    /// `actor-not-human` — when `actor.kind != Human`.
    ///
    /// An agent tried to contribute a token; only a person contributes one, and nothing was written.
    ActorNotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: crate::operator::ActorNotHuman,
    },
    /// `contributed` — otherwise.
    ///
    /// The token is in the host's pool, Available and Ok, with its owner, seat and kind.
    Contributed {
        /// The `conductor.token.TokenContributed` this outcome publishes.
        token_contributed: TokenContributed,
    },
}

/// Lease a token for an agent start — the input of `conductor.token.LeaseToken`.
///
/// Everything it can result in is [`LeaseTokenOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseToken {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `agent` — `conductor.agent.AgentName`.
    pub agent: crate::agent::AgentName,
    /// `preferred_owner` — `Optional<conductor.operator.OperatorName>`.
    pub preferred_owner: Option<crate::operator::OperatorName>,
    /// `runtime` — `Optional<String>`.
    pub runtime: Option<String>,
    /// `leased_at` — `Timestamp`.
    pub leased_at: crate::primitives::Timestamp,
}

/// Everything `conductor.token.LeaseToken` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseTokenOutcome {
    /// `none-available` — externally decided (the runtime's pick found no token with headroom in the host's pool).
    ///
    /// The host's pool holds no token with headroom, so the agent does not start on one; nothing changed.
    NoneAvailable {
        /// Why it was refused: `conductor.token.NoTokenAvailable`.
        error: NoTokenAvailable,
    },
    /// `unknown-token` — when no `conductor.token.PoolToken` carries the identity `input.token_id` names.
    ///
    /// The token named is not in the pool; nothing changed.
    UnknownToken {
        /// Why it was refused: `conductor.token.TokenStateConflict`.
        error: TokenStateConflict,
    },
    /// `token-withdrawn` — when the `conductor.token.PoolToken` that `input.token_id` names satisfies `state in [Withdrawn]`.
    ///
    /// The token named is withdrawn; nothing changed.
    TokenWithdrawn {
        /// Why it was refused: `conductor.token.TokenStateConflict`.
        error: TokenStateConflict,
    },
    /// `leased` — otherwise.
    ///
    /// The agent runs on the token under a lease of its own; the event names the token's owner, so the agent's record names them.
    Leased {
        /// The `conductor.token.TokenLeased` this outcome publishes.
        token_leased: TokenLeased,
    },
}

/// Mark a token limited — the input of `conductor.token.MarkTokenLimited`.
///
/// Everything it can result in is [`MarkTokenLimitedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkTokenLimited {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `agent` — `Optional<conductor.agent.AgentName>`.
    pub agent: Option<crate::agent::AgentName>,
    /// `limited_until` — `Timestamp`.
    pub limited_until: crate::primitives::Timestamp,
    /// `limited_at` — `Timestamp`.
    pub limited_at: crate::primitives::Timestamp,
}

/// Everything `conductor.token.MarkTokenLimited` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkTokenLimitedOutcome {
    /// `limited` — otherwise.
    ///
    /// The token is Limited until the time given; the agent that hit the limit is named to be resumed on another.
    Limited {
        /// The `conductor.token.TokenLimited` this outcome publishes.
        token_limited: TokenLimited,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The token is withdrawn, so no agent on it hit a limit; nothing changed.
    WrongState {
        /// Why it was refused: `conductor.token.TokenStateConflict`.
        error: TokenStateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// Release a token's lease — the input of `conductor.token.ReleaseToken`.
///
/// Everything it can result in is [`ReleaseTokenOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseToken {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `lease_id` — `conductor.token.TokenLeaseId`.
    pub lease_id: TokenLeaseId,
    /// `released_at` — `Timestamp`.
    pub released_at: crate::primitives::Timestamp,
}

/// Everything `conductor.token.ReleaseToken` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseTokenOutcome {
    /// `released` — otherwise.
    ///
    /// The lease is released; its token serves its other leases and new ones as before.
    Released {
        /// The `conductor.token.TokenReleased` this outcome publishes.
        token_released: TokenReleased,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The lease was released already, so there is nothing to release; nothing changed.
    WrongState {
        /// Why it was refused: `conductor.token.LeaseStateConflict`.
        error: LeaseStateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// Restore a limited token — the input of `conductor.token.RestoreToken`.
///
/// Everything it can result in is [`RestoreTokenOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreToken {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `restored_at` — `Timestamp`.
    pub restored_at: crate::primitives::Timestamp,
}

/// Everything `conductor.token.RestoreToken` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreTokenOutcome {
    /// `restored` — otherwise.
    ///
    /// The token is Available and Ok again.
    Restored {
        /// The `conductor.token.TokenRestored` this outcome publishes.
        token_restored: TokenRestored,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The token is not Limited; nothing changed.
    WrongState {
        /// Why it was refused: `conductor.token.TokenStateConflict`.
        error: TokenStateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// Withdraw a token from the pool — the input of `conductor.token.WithdrawToken`.
///
/// Everything it can result in is [`WithdrawTokenOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawToken {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `withdrawn_at` — `Timestamp`.
    pub withdrawn_at: crate::primitives::Timestamp,
}

/// Everything `conductor.token.WithdrawToken` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WithdrawTokenOutcome {
    /// `actor-not-human` — when `actor.kind != Human`.
    ///
    /// An agent tried to withdraw a token; only a person withdraws one, and the token is unchanged.
    ActorNotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: crate::operator::ActorNotHuman,
    },
    /// `not-owner` — externally decided (the person who asked is not the token's owner).
    ///
    /// Someone other than the token's owner asked to withdraw it; the token is unchanged.
    NotOwner {
        /// Why it was refused: `conductor.token.NotTokenOwner`.
        error: NotTokenOwner,
    },
    /// `withdrawn` — otherwise.
    ///
    /// The token is out of the pool; the agents on its leases are resumed on another token.
    Withdrawn {
        /// The `conductor.token.TokenWithdrawn` this outcome publishes.
        token_withdrawn: TokenWithdrawn,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The token was withdrawn already, and nothing changed.
    WrongState {
        /// Why it was refused: `conductor.token.TokenStateConflict`.
        error: TokenStateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// TokenContributed — the event `conductor.token.TokenContributed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenContributed {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
    /// `secret` — `conductor.secret.SecretName`.
    pub secret: crate::secret::SecretName,
    /// `kind` — `conductor.config.ModelTokenKind`.
    pub kind: crate::config::ModelTokenKind,
    /// `contributed_at` — `Timestamp`.
    pub contributed_at: crate::primitives::Timestamp,
}

/// TokenLeased — the event `conductor.token.TokenLeased`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLeased {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `lease_id` — `conductor.token.TokenLeaseId`.
    pub lease_id: TokenLeaseId,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `agent` — `conductor.agent.AgentName`.
    pub agent: crate::agent::AgentName,
    /// `preferred_owner` — `Optional<conductor.operator.OperatorName>`.
    pub preferred_owner: Option<crate::operator::OperatorName>,
    /// `leased_at` — `Timestamp`.
    pub leased_at: crate::primitives::Timestamp,
}

/// TokenLimited — the event `conductor.token.TokenLimited`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLimited {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `agent` — `Optional<conductor.agent.AgentName>`.
    pub agent: Option<crate::agent::AgentName>,
    /// `limited_until` — `Timestamp`.
    pub limited_until: crate::primitives::Timestamp,
    /// `limited_at` — `Timestamp`.
    pub limited_at: crate::primitives::Timestamp,
}

/// TokenReleased — the event `conductor.token.TokenReleased`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenReleased {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `lease_id` — `conductor.token.TokenLeaseId`.
    pub lease_id: TokenLeaseId,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `agent` — `conductor.agent.AgentName`.
    pub agent: crate::agent::AgentName,
    /// `released_at` — `Timestamp`.
    pub released_at: crate::primitives::Timestamp,
}

/// TokenRestored — the event `conductor.token.TokenRestored`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRestored {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `restored_at` — `Timestamp`.
    pub restored_at: crate::primitives::Timestamp,
}

/// TokenWithdrawn — the event `conductor.token.TokenWithdrawn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenWithdrawn {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `withdrawn_at` — `Timestamp`.
    pub withdrawn_at: crate::primitives::Timestamp,
}

/// The declared error `conductor.token.LeaseStateConflict`.
///
/// The lease was released already, so there is nothing to release; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseStateConflict {
    /// `lease_id` — `conductor.token.TokenLeaseId`.
    pub lease_id: TokenLeaseId,
}

/// The declared error `conductor.token.NoTokenAvailable`.
///
/// No token of the host's pool has headroom (every one is limited or withdrawn, or the owner the start asked for has none), so the agent of the instance does not start on one; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoTokenAvailable {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
}

/// The declared error `conductor.token.NotTokenOwner`.
///
/// The person who asked is not the token's owner, and only the owner withdraws a token; nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotTokenOwner {
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
}

/// The declared error `conductor.token.TokenStateConflict`.
///
/// The token is not in a state the command acts from (a lease of a token that is withdrawn or not in the pool, a limit of one that is withdrawn, a restore of one that is not Limited, a withdrawal of one already withdrawn); nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenStateConflict {
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
}

/// Leases of the pool — one row of the view `conductor.token.TokenLeases`.
///
/// Projects `conductor.token.TokenLease` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLeases {
    /// `lease_id` — `conductor.token.TokenLeaseId`.
    pub lease_id: TokenLeaseId,
    /// `state` — `conductor.token.TokenLease.State`.
    pub state: TokenLeaseState,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `agent` — `conductor.agent.AgentName`.
    pub agent: crate::agent::AgentName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `leased_at` — `Timestamp`.
    pub leased_at: crate::primitives::Timestamp,
    /// `released_at` — `Optional<Timestamp>`.
    pub released_at: Option<crate::primitives::Timestamp>,
}

/// Tokens of the pool — one row of the view `conductor.token.Tokens`.
///
/// Projects `conductor.token.PoolToken` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `state` — `conductor.token.PoolToken.State`.
    pub state: PoolTokenState,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
    /// `token_state` — `conductor.token.TokenState`.
    pub token_state: TokenState,
    /// `limited_until` — `Optional<Timestamp>`.
    pub limited_until: Option<crate::primitives::Timestamp>,
    /// `contributed_at` — `Timestamp`.
    pub contributed_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `kind` — `Optional<conductor.config.ModelTokenKind>`.
    pub kind: Option<crate::config::ModelTokenKind>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.token.ContributeToken` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ContributeTokenBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.token.ContributeToken`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn contribute_token(&mut self, input: super::ContributeToken) -> Result<super::ContributeTokenOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.token.LeaseToken` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait LeaseTokenBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.token.LeaseToken`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn lease_token(&mut self, input: super::LeaseToken) -> Result<super::LeaseTokenOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.token.MarkTokenLimited` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait MarkTokenLimitedBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.token.MarkTokenLimited`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn mark_token_limited(&mut self, input: super::MarkTokenLimited) -> Result<super::MarkTokenLimitedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.token.ReleaseToken` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReleaseTokenBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.token.ReleaseToken`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn release_token(&mut self, input: super::ReleaseToken) -> Result<super::ReleaseTokenOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.token.RestoreToken` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RestoreTokenBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.token.RestoreToken`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn restore_token(&mut self, input: super::RestoreToken) -> Result<super::RestoreTokenOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.token.WithdrawToken` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait WithdrawTokenBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.token.WithdrawToken`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn withdraw_token(&mut self, input: super::WithdrawToken) -> Result<super::WithdrawTokenOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.token.TokenLeases` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait TokenLeasesQuery {
        /// Serves `conductor.token.TokenLeases` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn token_leases(&self) -> Result<Vec<super::TokenLeases>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.token.Tokens` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait TokensQuery {
        /// Serves `conductor.token.Tokens` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn tokens(&self) -> Result<Vec<super::Tokens>, crate::obligation::UnmetObligation>;
    }

}
