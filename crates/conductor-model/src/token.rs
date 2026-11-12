// generated from conductor v1
// contract digest 7a2d2151a617b034e5a12bae1525027cb9f09b0dd4f8f5435a8d7b89308daeba
// do not edit: regenerate with `ess synthesize --layout crate`

//! Token — `conductor.token`.
//!
//! The token pool of an instance (design docs/design/architecture-next.md § 5.3, decisions 10 and 11): an engineer who joins an instance contributes their Claude token; the pool is a set of secrets, each with its owner, its seat and its state (`ok`, or `limited until <time>`); values never leave the secrets source except as a sealed slot. The `claude-code` kind leases one token per agent start, resumes an agent on another token after a usage-limit stop, and names the token's owner on every agent record. A controller started for Remote Control on a named owner's token appears in that owner's Claude app.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// JoinDaemon — `conductor.token.JoinDaemon`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinDaemon {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
    /// `seat` — `Optional<conductor.token.Seat>`.
    pub seat: Option<Seat>,
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
    /// `Leased`.
    Leased,
    /// `Limited`.
    Limited,
    /// `Withdrawn`.
    Withdrawn,
}

/// RevealToken — `conductor.token.RevealToken`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevealToken {
    /// `token` — `Optional<String>`.
    pub token: Option<String>,
    /// `model_token` — `Optional<conductor.config.InstanceName>`.
    pub model_token: Option<crate::config::InstanceName>,
}

/// Seat — `conductor.token.Seat`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seat(pub String);

/// TokenId — `conductor.token.TokenId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenId(pub crate::primitives::Uuid);

/// The states of `conductor.token.TokenPool`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `TokenPool<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenPoolState {
    /// `Open`.
    Open,
}

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
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `tokens`: `conductor.token.TokenPool` owns many `conductor.token.PoolToken`.
    pub instance: crate::config::InstanceName,
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
    /// `agent` — `Optional<conductor.agent.AgentName>`.
    ///
    /// Carries `agent`: `conductor.token.PoolToken` references one `conductor.agent.Agent`.
    pub agent: Option<crate::agent::AgentName>,
    /// `contributed_at` — `Timestamp`.
    pub contributed_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
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
        impl Sealed for super::Leased {}
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

    /// `Leased`.
    pub struct Leased;

    impl Marker for Leased {
        const STATE: super::PoolTokenState = super::PoolTokenState::Leased;
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
    /// `lease` — `Available` → `Leased`. Taken by the `leased` outcome of `conductor.token.LeaseToken`.
    pub fn lease(self) -> PoolToken<pool_token_state::Leased> {
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

impl PoolToken<pool_token_state::Leased> {
    /// `release` — `Leased` → `Available`. Taken by the `released` outcome of `conductor.token.ReleaseToken`.
    pub fn release(self) -> PoolToken<pool_token_state::Available> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `limit` — `Leased` → `Limited`. Taken by the `limited` outcome of `conductor.token.MarkTokenLimited`.
    pub fn limit(self) -> PoolToken<pool_token_state::Limited> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `withdraw` — `Leased` → `Withdrawn`. Taken by the `withdrawn` outcome of `conductor.token.WithdrawToken`.
    pub fn withdraw(self) -> PoolToken<pool_token_state::Withdrawn> {
        PoolToken {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl PoolToken<pool_token_state::Limited> {
    /// `lease` — `Limited` → `Leased`. Taken by the `leased` outcome of `conductor.token.LeaseToken`.
    pub fn lease(self) -> PoolToken<pool_token_state::Leased> {
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
    /// Resting in `Leased`.
    Leased(PoolToken<pool_token_state::Leased>),
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
            PoolTokenState::Leased => AnyPoolToken::Leased(PoolToken {
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
            Self::Leased(_) => PoolTokenState::Leased,
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
            Self::Leased(instance) => PoolTokenSnapshot {
                state: PoolTokenState::Leased,
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

/// What TokenPool — `conductor.token.TokenPool` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`TokenPool<S>`], and at a boundary by [`TokenPoolSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPoolData {
    /// The identity: `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `instance`: `conductor.token.TokenPool` references one `conductor.instance.Instance`.
    pub instance: crate::config::InstanceName,
}

/// The states of `conductor.token.TokenPool`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](token_pool_state::Marker), so [`TokenPool<S>`](TokenPool) can only ever rest in a real state.
pub mod token_pool_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Open {}
    }

    /// A declared state of `TokenPool`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::TokenPoolState;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::TokenPoolState = super::TokenPoolState::Open;
    }
}

/// TokenPool — `conductor.token.TokenPool` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`TokenPoolSnapshot`]
/// and [`TokenPoolSnapshot::refine`].
pub struct TokenPool<S: token_pool_state::Marker> {
    data: TokenPoolData,
    state: core::marker::PhantomData<S>,
}

impl<S: token_pool_state::Marker> TokenPool<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> TokenPoolState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &TokenPoolData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> TokenPoolData {
        self.data
    }
}

impl TokenPool<token_pool_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: TokenPoolData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.token.TokenPool` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`TokenPoolSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPoolSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: TokenPoolState,
    /// What it holds.
    pub data: TokenPoolData,
}

/// An `TokenPool` in whichever declared state it was found.
pub enum AnyTokenPool {
    /// Resting in `Open`.
    Open(TokenPool<token_pool_state::Open>),
}

impl TokenPoolSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `TokenPoolState` cannot spell one.
    pub fn refine(self) -> AnyTokenPool {
        match self.state {
            TokenPoolState::Open => AnyTokenPool::Open(TokenPool {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyTokenPool {
    /// The state, as the runtime value.
    pub fn state(&self) -> TokenPoolState {
        match self {
            Self::Open(_) => TokenPoolState::Open,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> TokenPoolSnapshot {
        match self {
            Self::Open(instance) => TokenPoolSnapshot {
                state: TokenPoolState::Open,
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
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
    /// `secret` — `conductor.secret.SecretName`.
    pub secret: crate::secret::SecretName,
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
    /// The token is in the pool, Available and Ok, with its owner and seat.
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
    /// `agent` — `conductor.agent.AgentName`.
    pub agent: crate::agent::AgentName,
    /// `preferred_owner` — `Optional<conductor.operator.OperatorName>`.
    pub preferred_owner: Option<crate::operator::OperatorName>,
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
    /// `none-available` — externally decided (the runtime's pick found no Available token in the instance's pool).
    ///
    /// The pool holds no Available token, so the agent does not start on one; nothing changed.
    NoneAvailable {
        /// Why it was refused: `conductor.token.NoTokenAvailable`.
        error: NoTokenAvailable,
    },
    /// `leased` — otherwise.
    ///
    /// The agent runs on the token; the event names the token's owner and seat, so the agent's record names them.
    Leased {
        /// The `conductor.token.TokenLeased` this outcome publishes.
        token_leased: TokenLeased,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The token named is neither Available nor Limited (it is leased or withdrawn); nothing changed.
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

/// Release a token as limited — the input of `conductor.token.MarkTokenLimited`.
///
/// Everything it can result in is [`MarkTokenLimitedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkTokenLimited {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
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
    /// The token is Limited until the time given and holds no agent; the agent it held is named to be resumed on another.
    Limited {
        /// The `conductor.token.TokenLimited` this outcome publishes.
        token_limited: TokenLimited,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The token is not Leased, so no agent on it hit a limit; nothing changed.
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

/// Release a leased token — the input of `conductor.token.ReleaseToken`.
///
/// Everything it can result in is [`ReleaseTokenOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseToken {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
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
    /// The token is Available again and holds no agent.
    Released {
        /// The `conductor.token.TokenReleased` this outcome publishes.
        token_released: TokenReleased,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The token is not Leased, so there is nothing to release; nothing changed.
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
    /// The token is out of the pool; the agent that held it, if any, is named so it is resumed on another token.
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
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
    /// `secret` — `conductor.secret.SecretName`.
    pub secret: crate::secret::SecretName,
    /// `contributed_at` — `Timestamp`.
    pub contributed_at: crate::primitives::Timestamp,
}

/// TokenLeased — the event `conductor.token.TokenLeased`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenLeased {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
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
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `agent` — `Optional<conductor.agent.AgentName>`.
    pub agent: Option<crate::agent::AgentName>,
    /// `released_at` — `Timestamp`.
    pub released_at: crate::primitives::Timestamp,
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

/// The declared error `conductor.token.NoTokenAvailable`.
///
/// No token of the instance's pool is Available (every one is leased, limited or withdrawn), so no agent can start on one; nothing changed.
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
/// The token is not in a state the command acts from (a lease of a token that is not Available, a release or a limit of one that is not Leased, a withdrawal of one already withdrawn); nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenStateConflict {
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
}

/// Tokens of the pool — one row of the view `conductor.token.Tokens`.
///
/// Projects `conductor.token.PoolToken` at `read_your_writes` consistency, containing instances where `instance == param.instance`.
/// Serving it is an implementation obligation — see the plan — because how a projection is kept
/// current is a storage decision the specification does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    /// `token_id` — `conductor.token.TokenId`.
    pub token_id: TokenId,
    /// `state` — `conductor.token.PoolToken.State`.
    pub state: PoolTokenState,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `owner` — `conductor.operator.OperatorName`.
    pub owner: crate::operator::OperatorName,
    /// `seat` — `conductor.token.Seat`.
    pub seat: Seat,
    /// `token_state` — `conductor.token.TokenState`.
    pub token_state: TokenState,
    /// `limited_until` — `Optional<Timestamp>`.
    pub limited_until: Option<crate::primitives::Timestamp>,
    /// `agent` — `Optional<conductor.agent.AgentName>`.
    pub agent: Option<crate::agent::AgentName>,
    /// `contributed_at` — `Timestamp`.
    pub contributed_at: crate::primitives::Timestamp,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
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

    /// The query `conductor.token.Tokens` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a view parameter (`params:`), which a generated query does not apply.
    ///
    /// Contract: a query answering `conductor.token.Tokens` with rows projected from `conductor.token.PoolToken` at `read_your_writes` consistency, containing instances where `instance == param.instance`.
    pub trait TokensQuery {
        /// Serves `conductor.token.Tokens` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn tokens(&self, instance: crate::config::InstanceName) -> Result<Vec<super::Tokens>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl TokensQuery for Unimplemented {
        fn tokens(&self, _instance: crate::config::InstanceName) -> Result<Vec<super::Tokens>, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "view query", source: "conductor.token.Tokens" })
        }
    }
}
