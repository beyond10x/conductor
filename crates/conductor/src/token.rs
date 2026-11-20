//! The token pool of a host (`story:token-pool`, `story:host-token-pool`, design
//! architecture-next § 5.3, § 17, decisions 10 and 11): an engineer who joins contributes their
//! Claude token to the pool conductord holds for its host, and the agents of the `claude-code`
//! kind of every instance on the host lease from it.
//!
//! - The pool is `conductor.token.PoolToken` records in the daemon's tenant of the deployment's
//!   store, each with its owner, seat, state and the name of the secret that holds its value; the
//!   value is in the daemon's secrets source only ([`contribute`]). A token names no instance.
//! - Each agent start holds a lease of its own, a `conductor.token.TokenLease` naming the token,
//!   the agent and its instance; `(instance, agent)` finds an agent's held lease ([`held`]), so a
//!   token serves several agents at once (class C decision of `story:host-token-pool`).
//! - [`pick`] is the lease rule: a named owner's own tokens only, else every token of the pool
//!   (decision K4: no fallback); among the tokens with headroom (`Available`, or `Limited` with
//!   `limited_until` past) the one with the fewest live leases, ties by `token_id`. A Limited
//!   token whose `limited_until` is ahead is never taken, and no probe of the account's headroom
//!   lets it (`story:limit-aware-token-reassignment`). A token never holds more live leases than its
//!   kind's cap ([`Caps`]; Decision (operator, 2026-11-18): 4 sessions per `subscription_token`
//!   by default, no cap for an `api_key`).
//! - A lease that raced a withdrawal or another lease past the cap is released again and the pick
//!   runs anew ([`lease`]): no agent starts on a withdrawn token, and [`withdraw`] releases every
//!   lease of the token it withdrew, so whichever of the two wrote last sees the other.
//! - The value reaches the harness once, at its start, in its process's environment
//!   ([`apply_value`]): [`OAUTH_VARIABLE`] for a `subscription_token`, [`API_KEY_VARIABLE`] for an
//!   `api_key`, the session's settings copy naming no [`HELPER_KEY`]. No command reveals a pool
//!   token, and no log, record, argument, settings file or box spec carries it.
//! - A runtime whose process can open the daemon's store leases there. A runtime in a box, which
//!   mounts no daemon database, leases, releases and marks limited through conductord over the
//!   dial-in aperture it already has ([`hold_dial`], [`crate::daemon::dial`]): one request per
//!   connection, opened with [`crate::instance::POOL_HELLO`], its instance and the token the
//!   daemon minted for its start. conductord takes the instance from that admission, never from
//!   the request ([`pool_call`]).
//! - A usage limit of an agent marks the token it holds Limited until the reset its line named
//!   ([`on_limit`]), through conductord for a runtime in a box, and answers whether another token
//!   has headroom now, else the first instant a token of the pool resets ([`first_reset`]): the
//!   loop resumes the agent at once, or at that instant.
//! - A runtime in a box probes that route once it starts, until conductord answers
//!   ([`probe_pool`], call `probe`): conductord keeps the start of each instance that probed
//!   ([`probed`]), and an upgrade to a release that probes ([`PROBE_SINCE`]) counts as answered
//!   only once its new start did (`story:upgrade-checks-daemon-protocols`).
//! - Each lease names the runtime start that took it ([`runtime_tag`]); conductord releases the
//!   leases of a start that ended once the instance runs on another ([`reap`]).
//! - `conductor join` and `conductor leave` ([`join`], [`leave`]) are clients of the daemon's
//!   `POST /v1/join` and `POST /v1/leave`, or of `POST /v1/instances/{name}/join` and `leave`,
//!   which also join or leave that instance as its operator ([`join_route`], [`leave_route`]);
//!   `GET /v1/tokens` ([`rows`]) lists owners, seats and states, never a value nor the secret's
//!   name. Only a token's owner withdraws it (decision K5); each agent on a withdrawn token is
//!   named in its instance's `<state>/run/tokens-withdrawn.json`, and the loop ends it and resumes
//!   it on another token.
//!
//! With no token in the pool (no daemon store, or nothing contributed) every start is today's,
//! byte for byte.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::agent::AgentName;
use conductor_model::behaviour::{
    ExternalCommand, Generated, PoolTokenStorage, TokenLeaseStorage, TryContext, unmet_context,
};
use conductor_model::config::{
    DaemonOperator, Instance, InstanceName, ModelTokenKind, OperatorGrant, Role,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::operator::obligations::{
    JoinInstanceBehavior as _, LeaveInstanceBehavior as _,
};
use conductor_model::operator::{
    Actor, ActorKind, Grant, JoinInstance, JoinInstanceOutcome, LeaveInstance,
    LeaveInstanceOutcome, OperatorKey, OperatorName,
};
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::secret::{PutSecret, PutSecretOutcome, SecretName};
use conductor_model::token::obligations::{
    ContributeTokenBehavior as _, LeaseTokenBehavior as _, MarkTokenLimitedBehavior as _,
    ReleaseTokenBehavior as _, RestoreTokenBehavior as _, WithdrawTokenBehavior as _,
};
use conductor_model::token::{
    ContributeToken, ContributeTokenOutcome, LeaseToken, LeaseTokenOutcome, MarkTokenLimited,
    MarkTokenLimitedOutcome, NoTokenAvailable, PoolTokenData, PoolTokenSnapshot, PoolTokenState,
    ReleaseToken, ReleaseTokenOutcome, RestoreToken, Seat, TokenId, TokenLeaseId,
    TokenLeaseSnapshot, TokenLeaseState, WithdrawToken, WithdrawTokenOutcome,
};
use secrets_core::storage::SecretValue;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{JoinArgs, LeaveArgs, TokenRevealArgs};
use crate::confine::Launch;
use crate::store::Store;

/// How long a token stays Limited after a usage-limit stop whose text named no reset: Claude's
/// usage window.
pub const LIMIT_WINDOW: time::Duration = time::Duration::hours(5);

/// The furthest a usage limit holds a token from now: Claude's weekly window and an hour's margin.
/// A later end, whoever gave it, is clamped to it (story:host-token-pool).
pub const LIMIT_CEILING: time::Duration = time::Duration::hours(7 * 24 + 1);

/// The file under the instance's `<state>/run/` naming the agents on a withdrawn token, which
/// the loop resumes on another.
pub const WITHDRAWN_FILE: &str = "tokens-withdrawn.json";

/// The settings key the harness asks for its credential with.
pub const HELPER_KEY: &str = "apiKeyHelper";

/// The variable of the harness's process environment it reads the long-lived token of a
/// subscription login from. A [`HELPER_KEY`] in its settings outranks it.
pub const OAUTH_VARIABLE: &str = "CLAUDE_CODE_OAUTH_TOKEN";

/// The variable of the harness's process environment it reads an API key from. A [`HELPER_KEY`]
/// in its settings outranks it.
pub const API_KEY_VARIABLE: &str = "ANTHROPIC_API_KEY";

/// The variable a runtime names its start in for the processes it starts (a fingerprint of the
/// token the daemon minted for that start, [`runtime_tag`], never the token): each lease they take
/// names it, so conductord releases the leases of a start that ended ([`reap`]).
pub const RUNTIME_VARIABLE: &str = "CONDUCTOR_RUNTIME_START";

/// How many times [`lease`] picks anew after a lease that raced a withdrawal or another lease.
const LEASE_ATTEMPTS: usize = 8;

// ---------------------------------------------------------------------------------------------
// Ports
// ---------------------------------------------------------------------------------------------

/// The store the pool's commands run on: the daemon's tenant.
pub struct Ports {
    /// The daemon's store.
    pub store: Store,
}

impl PoolTokenStorage for Ports {
    fn get(&self, identity: &TokenId) -> Option<PoolTokenSnapshot> {
        PoolTokenStorage::get(&self.store, identity)
    }
    fn put(&mut self, snapshot: PoolTokenSnapshot) {
        PoolTokenStorage::put(&mut self.store, snapshot);
    }
    fn delete(&mut self, identity: &TokenId) {
        PoolTokenStorage::delete(&mut self.store, identity);
    }
    fn list(&self) -> Vec<PoolTokenSnapshot> {
        PoolTokenStorage::list(&self.store)
    }
}

impl TokenLeaseStorage for Ports {
    fn get(&self, identity: &TokenLeaseId) -> Option<TokenLeaseSnapshot> {
        TokenLeaseStorage::get(&self.store, identity)
    }
    fn put(&mut self, snapshot: TokenLeaseSnapshot) {
        TokenLeaseStorage::put(&mut self.store, snapshot);
    }
    fn delete(&mut self, identity: &TokenLeaseId) {
        TokenLeaseStorage::delete(&mut self.store, identity);
    }
    fn list(&self) -> Vec<TokenLeaseSnapshot> {
        TokenLeaseStorage::list(&self.store)
    }
}

macro_rules! unmet {
    ($($method:ident -> $type:ty, $name:literal;)*) => {
        $(fn $method(&mut self) -> Result<$type, UnmetObligation> {
            Err(unmet_context($name))
        })*
    };
}

impl TryContext for Ports {
    unmet! {
        try_generate_conductor_config_revision_id -> conductor_model::config::RevisionId, "conductor.config.RevisionId";
        try_generate_conductor_direction_serving_id -> conductor_model::direction::ServingId, "conductor.direction.ServingId";
        try_generate_conductor_dispatch_guard_decision_id -> conductor_model::dispatch::GuardDecisionId, "conductor.dispatch.GuardDecisionId";
        try_generate_conductor_dispatch_message_id -> conductor_model::dispatch::MessageId, "conductor.dispatch.MessageId";
        try_generate_conductor_dispatch_resource_request_id -> conductor_model::dispatch::ResourceRequestId, "conductor.dispatch.ResourceRequestId";
        try_generate_conductor_host_grant_id -> conductor_model::host::GrantId, "conductor.host.GrantId";
        try_generate_conductor_host_reclaim_run_id -> conductor_model::host::ReclaimRunId, "conductor.host.ReclaimRunId";
        try_generate_conductor_instance_reconciliation_id -> conductor_model::instance::ReconciliationId, "conductor.instance.ReconciliationId";
        try_generate_conductor_observation_board_id -> conductor_model::observation::BoardId, "conductor.observation.BoardId";
        try_generate_conductor_observation_observation_id -> conductor_model::observation::ObservationId, "conductor.observation.ObservationId";
        try_generate_conductor_observation_snapshot_id -> conductor_model::observation::SnapshotId, "conductor.observation.SnapshotId";
        try_generate_conductor_placement_box_id -> conductor_model::placement::BoxId, "conductor.placement.BoxId";
        try_generate_conductor_placement_refusal_reason -> conductor_model::placement::RefusalReason, "conductor.placement.RefusalReason";
        try_generate_conductor_session_delivery_id -> conductor_model::session::DeliveryId, "conductor.session.DeliveryId";
        try_generate_conductor_session_tick_id -> conductor_model::session::TickId, "conductor.session.TickId";
        try_generate_conductor_tool_tool_call_id -> conductor_model::tool::ToolCallId, "conductor.tool.ToolCallId";
        try_generate_string -> String, "String";
    }

    fn try_generate_conductor_token_token_id(&mut self) -> Result<TokenId, UnmetObligation> {
        Ok(TokenId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_token_token_lease_id(
        &mut self,
    ) -> Result<TokenLeaseId, UnmetObligation> {
        Ok(TokenLeaseId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_external(
        &mut self,
        command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        match (command, outcome) {
            // The pick answered before the command ran ([`lease`]).
            (ExternalCommand::ConductorTokenLeaseToken(_), "none-available") => Ok(false),
            // Decision K5: only the owner withdraws a token.
            (ExternalCommand::ConductorTokenWithdrawToken(input), "not-owner") => {
                Ok(PoolTokenStorage::get(&self.store, &input.token_id)
                    .is_some_and(|token| input.actor.operator.as_ref() != Some(&token.data.owner)))
            }
            (command, _) => Err(unmet_context(command.name())),
        }
    }
}

fn unmet(unmet: UnmetObligation) -> anyhow::Error {
    anyhow!("{unmet}")
}

/// The daemon's store at `url`, opened.
fn open_store(url: &str) -> Result<Store> {
    crate::store::open_url(url, crate::daemon::TENANT)
        .map_err(|error| anyhow!("daemon.store: {error:#}"))
}

// ---------------------------------------------------------------------------------------------
// The pool
// ---------------------------------------------------------------------------------------------

/// The token an agent runs on: its lease, the token's id, owner, seat, the secret that holds its
/// value and its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    /// The lease.
    pub lease_id: TokenLeaseId,
    /// The token.
    pub token_id: TokenId,
    /// Its owner, whom the agent's record names.
    pub owner: OperatorName,
    /// Its seat.
    pub seat: Seat,
    /// The secret that holds its value.
    pub secret: SecretName,
    /// Its kind ([`kind_of`]).
    pub kind: ModelTokenKind,
}

/// The most live leases one token of each kind holds (Decision (operator, 2026-11-18)): 0 is no
/// cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// `daemon.subscription_token_sessions`.
    pub subscription_token: u64,
    /// `daemon.api_key_sessions`.
    pub api_key: u64,
}

impl Default for Caps {
    fn default() -> Self {
        Self {
            subscription_token: crate::config::SUBSCRIPTION_TOKEN_SESSIONS.unsigned_abs(),
            api_key: 0,
        }
    }
}

impl Caps {
    /// The caps `daemon` names.
    #[must_use]
    pub fn of(daemon: &conductor_model::config::Daemon) -> Self {
        Self {
            subscription_token: daemon.subscription_token_sessions.unsigned_abs(),
            api_key: daemon.api_key_sessions.unsigned_abs(),
        }
    }

    /// The cap of a token of `kind`, `None` where it has none.
    #[must_use]
    pub const fn cap(&self, kind: ModelTokenKind) -> Option<u64> {
        let cap = match kind {
            ModelTokenKind::SubscriptionToken => self.subscription_token,
            ModelTokenKind::ApiKey => self.api_key,
        };
        if cap == 0 { None } else { Some(cap) }
    }
}

/// The kind of `token` (`story:pool-token-kind-files-source`): a token recorded before the kind
/// existed is an `ApiKey`, the route every pool token took then.
#[must_use]
pub fn kind_of(token: &PoolTokenData) -> ModelTokenKind {
    token.kind.unwrap_or(ModelTokenKind::ApiKey)
}

fn instant(at: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(at, &Rfc3339).ok()
}

fn now() -> Result<Timestamp> {
    crate::records::now()
}

/// The token the lease rule picks for an agent (module documentation), given each token's live
/// leases; `None` when no token of the pool is admitted.
#[must_use]
pub fn pick(
    tokens: &[PoolTokenSnapshot],
    live: &dyn Fn(&TokenId) -> u64,
    caps: Caps,
    owner: Option<&OperatorName>,
    now: OffsetDateTime,
) -> Option<TokenId> {
    let past = |token: &PoolTokenSnapshot| {
        token
            .data
            .limited_until
            .as_ref()
            .and_then(|at| instant(&at.0))
            .is_none_or(|until| until <= now)
    };
    tokens
        .iter()
        .filter(|token| owner.is_none_or(|owner| token.data.owner == *owner))
        .filter(|token| {
            caps.cap(kind_of(&token.data))
                .is_none_or(|cap| live(&token.data.token_id) < cap)
        })
        .filter(|token| match token.state {
            PoolTokenState::Available => true,
            PoolTokenState::Limited => past(token),
            PoolTokenState::Withdrawn => false,
        })
        .min_by(|a, b| {
            (live(&a.data.token_id), &a.data.token_id.0.0)
                .cmp(&(live(&b.data.token_id), &b.data.token_id.0.0))
        })
        .map(|token| token.data.token_id.clone())
}

/// The first instant after `now` a Limited token of `tokens` resets, its `limited_until`; none
/// when no token is Limited past `now` (`story:limit-aware-token-reassignment`). A session no
/// token has headroom for is resumed then, ahead of its own token's reset when another resets
/// first.
#[must_use]
pub fn first_reset(tokens: &[PoolTokenSnapshot], now: OffsetDateTime) -> Option<OffsetDateTime> {
    tokens
        .iter()
        .filter(|token| token.state == PoolTokenState::Limited)
        .filter_map(|token| {
            token
                .data
                .limited_until
                .as_ref()
                .and_then(|at| instant(&at.0))
        })
        .filter(|until| *until > now)
        .min()
}

/// The tokens of the host's pool that are not withdrawn.
#[must_use]
pub fn pool(store: &Store) -> Vec<PoolTokenSnapshot> {
    store
        .pool_tokens()
        .into_iter()
        .filter(|token| token.state != PoolTokenState::Withdrawn)
        .collect()
}

/// The lease `agent` of `instance` holds, if any.
#[must_use]
pub fn held(
    store: &Store,
    instance: &InstanceName,
    agent: &AgentName,
) -> Option<TokenLeaseSnapshot> {
    store.token_leases().into_iter().find(|lease| {
        lease.state == TokenLeaseState::Held
            && lease.data.instance == *instance
            && lease.data.agent == *agent
    })
}

/// The held leases of `token_id`.
fn leases_of(store: &Store, token_id: &TokenId) -> Vec<TokenLeaseSnapshot> {
    store
        .token_leases()
        .into_iter()
        .filter(|lease| lease.state == TokenLeaseState::Held && lease.data.token_id == *token_id)
        .collect()
}

fn lease_of(lease: &TokenLeaseSnapshot, token: &PoolTokenSnapshot) -> Lease {
    Lease {
        lease_id: lease.data.lease_id.clone(),
        token_id: token.data.token_id.clone(),
        owner: token.data.owner.clone(),
        seat: token.data.seat.clone(),
        secret: token.data.secret.clone(),
        kind: kind_of(&token.data),
    }
}

/// One agent start that asks for a lease: its instance, its agent, the owner it asks for, the
/// actor it runs as, and the runtime start it belongs to.
#[derive(Debug, Clone)]
pub struct Start {
    /// The instance whose agent starts.
    pub instance: InstanceName,
    /// The agent.
    pub agent: AgentName,
    /// The owner it asks for (`token_owner`).
    pub owner: Option<OperatorName>,
    /// Who leases.
    pub actor: Actor,
    /// The runtime start ([`runtime_tag`]).
    pub runtime: Option<String>,
}

/// `LeaseToken` for `start` on the token [`pick`] picks, in the store at `url`: an agent that
/// holds a lease on a token that is not withdrawn keeps it; one held on a withdrawn token is
/// released first. A lease that raced a withdrawal, or that put its token past its cap, is
/// released again and the pick runs anew, at most [`LEASE_ATTEMPTS`] times.
///
/// # Errors
///
/// The store cannot be written, the generated behaviour cannot run, or the pool kept moving.
pub fn lease(url: &str, caps: Caps, start: &Start) -> Result<Result<Lease, NoTokenAvailable>> {
    for _ in 0..LEASE_ATTEMPTS {
        let store = open_store(url)?;
        if let Some(lease) = held(&store, &start.instance, &start.agent) {
            match PoolTokenStorage::get(&store, &lease.data.token_id) {
                Some(token) if token.state != PoolTokenState::Withdrawn => {
                    return Ok(Ok(lease_of(&lease, &token)));
                }
                _ => {
                    release_lease(open_store(url)?, &lease.data.lease_id, start.actor.clone())?;
                    continue;
                }
            }
        }
        let tokens = pool(&store);
        let live = store.live_leases();
        let Some(token_id) = pick(
            &tokens,
            &|id| live.get(&id.0.0).copied().unwrap_or(0),
            caps,
            start.owner.as_ref(),
            OffsetDateTime::now_utc(),
        ) else {
            return Ok(Err(NoTokenAvailable {
                instance: start.instance.clone(),
            }));
        };
        let Some(picked) = tokens
            .into_iter()
            .find(|token| token.data.token_id == token_id)
        else {
            continue;
        };
        if picked.state == PoolTokenState::Limited {
            let mut generated = Generated::new(Ports { store });
            generated
                .restore_token(RestoreToken {
                    actor: start.actor.clone(),
                    token_id: token_id.clone(),
                    restored_at: now()?,
                })
                .map_err(unmet)?;
            if generated.ports.store.check().is_err() {
                continue;
            }
        }
        let mut generated = Generated::new(Ports {
            store: open_store(url)?,
        });
        let outcome = generated
            .lease_token(LeaseToken {
                actor: start.actor.clone(),
                instance: start.instance.clone(),
                token_id: token_id.clone(),
                owner: picked.data.owner.clone(),
                agent: start.agent.clone(),
                preferred_owner: start.owner.clone(),
                runtime: start.runtime.clone(),
                leased_at: now()?,
            })
            .map_err(unmet)?;
        if generated.ports.store.check().is_err() {
            continue;
        }
        let LeaseTokenOutcome::Leased { token_leased } = outcome else {
            continue;
        };
        // Compare after the write: a withdrawal or another lease that wrote first is seen here,
        // one that writes later sees this lease ([`withdraw`]).
        let after = open_store(url)?;
        let token = PoolTokenStorage::get(&after, &token_id);
        let live = after.live_leases().get(&token_id.0.0).copied().unwrap_or(0);
        let over = caps
            .cap(kind_of(&picked.data))
            .is_some_and(|cap| live > cap);
        match token {
            Some(token) if token.state != PoolTokenState::Withdrawn && !over => {
                return Ok(Ok(Lease {
                    lease_id: token_leased.lease_id,
                    token_id,
                    owner: token.data.owner.clone(),
                    seat: token.data.seat.clone(),
                    secret: token.data.secret.clone(),
                    kind: kind_of(&token.data),
                }));
            }
            _ => {
                release_lease(after, &token_leased.lease_id, start.actor.clone())?;
            }
        }
    }
    bail!(
        "the host's token pool kept moving under {} lease attempts; nothing was started",
        LEASE_ATTEMPTS
    )
}

/// `ReleaseToken` of the lease `lease_id`.
fn release_lease(store: Store, lease_id: &TokenLeaseId, actor: Actor) -> Result<bool> {
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .release_token(ReleaseToken {
            actor,
            lease_id: lease_id.clone(),
            released_at: now()?,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the release: {error}"))?;
    Ok(matches!(outcome, ReleaseTokenOutcome::Released { .. }))
}

/// `ReleaseToken` of the lease `agent` of `instance` holds, if any; answers its token.
///
/// # Errors
///
/// The store cannot be written.
pub fn release(
    url: &str,
    instance: &InstanceName,
    agent: &AgentName,
    actor: Actor,
) -> Result<Option<TokenId>> {
    let store = open_store(url)?;
    let Some(lease) = held(&store, instance, agent) else {
        return Ok(None);
    };
    Ok(release_lease(store, &lease.data.lease_id, actor)?.then_some(lease.data.token_id))
}

/// The end of a usage limit: `until`, the reset the limit's text named
/// (`story:limit-resume-at-reset`), else [`LIMIT_WINDOW`] from `now`; never later than
/// [`LIMIT_CEILING`] from `now`.
#[must_use]
pub fn limit_end(until: Option<OffsetDateTime>, now: OffsetDateTime) -> OffsetDateTime {
    until.unwrap_or(now + LIMIT_WINDOW).min(now + LIMIT_CEILING)
}

/// `MarkTokenLimited` of the token `agent` of `instance` holds a lease on, until [`limit_end`],
/// and the lease released; answers the token as it was. An agent that holds no lease limits no
/// token.
///
/// # Errors
///
/// The store cannot be written.
pub fn limit(
    url: &str,
    instance: &InstanceName,
    agent: &AgentName,
    actor: Actor,
    until: Option<OffsetDateTime>,
) -> Result<Option<PoolTokenData>> {
    let store = open_store(url)?;
    let Some(lease) = held(&store, instance, agent) else {
        return Ok(None);
    };
    let Some(token) = PoolTokenStorage::get(&store, &lease.data.token_id) else {
        return Ok(None);
    };
    let until = limit_end(until, OffsetDateTime::now_utc())
        .format(&Rfc3339)
        .context("format the end of a usage limit")?;
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .mark_token_limited(MarkTokenLimited {
            actor: actor.clone(),
            token_id: lease.data.token_id.clone(),
            agent: Some(agent.clone()),
            limited_until: Timestamp(until),
            limited_at: now()?,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the limit: {error}"))?;
    release_lease(open_store(url)?, &lease.data.lease_id, actor)?;
    Ok(matches!(outcome, MarkTokenLimitedOutcome::Limited { .. }).then_some(token.data))
}

/// `WithdrawToken` of `token_id` by `actor`, then `ReleaseToken` of each lease held on it;
/// answers the outcome and the `(instance, agent)` of each lease released.
///
/// # Errors
///
/// The store cannot be written.
pub fn withdraw(
    url: &str,
    token_id: &TokenId,
    actor: Actor,
) -> Result<(WithdrawTokenOutcome, Vec<(InstanceName, AgentName)>)> {
    let mut generated = Generated::new(Ports {
        store: open_store(url)?,
    });
    let outcome = generated
        .withdraw_token(WithdrawToken {
            actor: actor.clone(),
            token_id: token_id.clone(),
            withdrawn_at: now()?,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the withdrawal: {error}"))?;
    let mut agents = Vec::new();
    if matches!(outcome, WithdrawTokenOutcome::Withdrawn { .. }) {
        // Read after the withdrawal's write: a lease that wrote first is released here; one that
        // writes later sees the token withdrawn ([`lease`]).
        for lease in leases_of(&open_store(url)?, token_id) {
            if release_lease(open_store(url)?, &lease.data.lease_id, actor.clone())? {
                agents.push((lease.data.instance, lease.data.agent));
            }
        }
    }
    Ok((outcome, agents))
}

/// Releases each held lease of `instance` that names a runtime start other than `current`; with
/// `current` `None`, each that names any. A lease that names no start (one a `conductor session
/// start` outside a runtime took) is kept. Answers the agents released.
///
/// # Errors
///
/// The store cannot be written.
pub fn reap(url: &str, instance: &InstanceName, current: Option<&str>) -> Result<Vec<AgentName>> {
    let store = open_store(url)?;
    let ended: Vec<TokenLeaseSnapshot> = store
        .token_leases()
        .into_iter()
        .filter(|lease| {
            lease.state == TokenLeaseState::Held
                && lease.data.instance == *instance
                && lease
                    .data
                    .runtime
                    .as_deref()
                    .is_some_and(|runtime| Some(runtime) != current)
        })
        .collect();
    let mut agents = Vec::new();
    for lease in ended {
        if release_lease(
            open_store(url)?,
            &lease.data.lease_id,
            crate::actor::daemon(),
        )? {
            agents.push(lease.data.agent);
        }
    }
    Ok(agents)
}

/// The runtime start a token the daemon minted for it names: the first 16 hexadecimal digits of
/// its SHA-256, which tell two starts apart and reveal nothing of the token.
#[must_use]
pub fn runtime_tag(token: &str) -> String {
    ring::digest::digest(&ring::digest::SHA256, token.as_bytes())
        .as_ref()
        .iter()
        .take(8)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// conductord's reaping at each publication: for each instance it routes to a runtime, by the
/// [`runtime_tag`] of the token it minted for that runtime's start, the leases of every other
/// start of that instance are released ([`reap`]). Opens no store while none exists; says each
/// release on one line, without a value.
pub fn reap_routes(store: &str, routes: &BTreeMap<String, String>) {
    let Some(opened) = existing_store(store) else {
        return;
    };
    let mut ended: Vec<String> = opened
        .token_leases()
        .into_iter()
        .filter(|lease| lease.state == TokenLeaseState::Held)
        .filter(|lease| {
            routes
                .get(&lease.data.instance.0)
                .is_some_and(|tag| lease.data.runtime.as_deref().is_some_and(|ran| ran != tag))
        })
        .map(|lease| lease.data.instance.0)
        .collect();
    drop(opened);
    ended.sort();
    ended.dedup();
    for instance in ended {
        let tag = routes.get(&instance).map(String::as_str);
        match reap(store, &InstanceName(instance.clone()), tag) {
            Ok(agents) if !agents.is_empty() => println!(
                "conductord: {instance}: released {} lease(s) of a runtime start that ended",
                agents.len()
            ),
            Ok(_) => {}
            Err(error) => eprintln!("conductord: {instance}: reap the token leases: {error:#}"),
        }
    }
}

/// One segment of a secret name the `secrets` crates keep, from `text`.
fn segment(text: &str) -> String {
    let word: String = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let word = word.trim_start_matches(['.', '_', '-']).to_owned();
    if word.is_empty() {
        "x".to_owned()
    } else {
        word
    }
}

/// The secret a token of the host's pool is kept under: `claude-token/<id>`.
fn secret_name() -> SecretName {
    SecretName(format!(
        "claude-token/{}",
        segment(&eventlog_core::new_event_id())
    ))
}

/// `ContributeToken`: `value` into the secrets source under a name of its own, then the token in
/// the host's pool, Available and Ok, with its owner, seat and kind. Answers the outcome, or why
/// the source refused the value. Under the `files` source the value is the file of that name in
/// its directory, owner-only (`story:pool-token-kind-files-source`), and no secret is on record.
///
/// # Errors
///
/// The store cannot be written.
pub fn contribute(
    mut store: Store,
    secrets: &crate::secret::Secrets,
    contribution: &Contribution,
    value: SecretValue,
) -> Result<Result<ContributeTokenOutcome, String>> {
    let at = now()?;
    let actor = &contribution.actor;
    if actor.kind != ActorKind::Human {
        return Ok(Err(format!(
            "ActorNotHuman: {}",
            crate::actor::not_human(actor)
        )));
    }
    let name = secret_name();
    match secrets.write_pool_token(&name, &value) {
        Ok(true) => {
            let recorded = contribute_token(store, contribution, name.clone(), at);
            if recorded.is_err() {
                let _ = secrets.remove_pool_token(&name);
            }
            return recorded.map(Ok);
        }
        Ok(false) => {}
        Err(refusal) => return Ok(Err(refusal)),
    }
    let put = secrets.put(
        &mut store,
        &PutSecret {
            actor: actor.clone(),
            name: name.clone(),
            put_at: at.clone(),
            replace: false,
        },
        value,
    )?;
    match put {
        PutSecretOutcome::Stored { .. } | PutSecretOutcome::Replaced { .. } => {}
        PutSecretOutcome::Unreachable { error } => {
            return Ok(Err(format!(
                "SourceUnreachable: the {:?} source did not answer; nothing was written",
                error.source
            )));
        }
        PutSecretOutcome::Exists { error } => {
            return Ok(Err(format!("SecretExists: {}", error.name.0)));
        }
        PutSecretOutcome::ReadOnly { error } => {
            return Ok(Err(format!(
                "SourceReadOnly: the {} source is never written; nothing was written",
                crate::secret::source_word(error.source)
            )));
        }
        PutSecretOutcome::NotHuman { error } => {
            return Ok(Err(format!(
                "ActorNotHuman: {}",
                crate::actor::not_human(&error.actor)
            )));
        }
    }
    contribute_token(store, contribution, name, at).map(Ok)
}

/// Who contributes which token: the input of [`contribute`] but the value.
#[derive(Debug, Clone)]
pub struct Contribution {
    /// Who contributes it, a person.
    pub actor: Actor,
    /// Its owner.
    pub owner: OperatorName,
    /// Its seat.
    pub seat: Seat,
    /// Its kind.
    pub kind: ModelTokenKind,
}

/// The `ContributeToken` of [`contribute`], once the value is in the source under `secret`.
fn contribute_token(
    store: Store,
    contribution: &Contribution,
    secret: SecretName,
    at: Timestamp,
) -> Result<ContributeTokenOutcome> {
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .contribute_token(ContributeToken {
            actor: contribution.actor.clone(),
            owner: contribution.owner.clone(),
            seat: contribution.seat.clone(),
            secret,
            kind: contribution.kind,
            contributed_at: at,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the token: {error}"))?;
    Ok(outcome)
}

/// One row of `GET /v1/tokens` (`conductor.token.Tokens`), with its live leases: never a value,
/// nor the name of the secret that holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenRow {
    /// The token's id.
    pub token_id: String,
    /// `Available`, `Limited` or `Withdrawn`.
    pub state: &'static str,
    /// Its owner.
    pub owner: String,
    /// Its seat.
    pub seat: String,
    /// `Ok` or `Limited`.
    pub token_state: &'static str,
    /// Until when it is limited.
    pub limited_until: Option<String>,
    /// When it was contributed.
    pub contributed_at: String,
    /// Who last moved it.
    pub actor: String,
    /// `SubscriptionToken` or `ApiKey` ([`kind_of`]).
    pub kind: &'static str,
    /// How many agents run on it now.
    pub leases: u64,
}

/// The rows of the host's pool, withdrawn tokens included, by contribution time.
#[must_use]
pub fn rows(store: &Store) -> Vec<TokenRow> {
    let live = store.live_leases();
    let mut rows: Vec<TokenRow> = store
        .pool_tokens()
        .into_iter()
        .map(|token| {
            let data = token.data;
            TokenRow {
                kind: crate::store::token_kind_name(kind_of(&data)),
                leases: live.get(&data.token_id.0.0).copied().unwrap_or(0),
                token_id: data.token_id.0.0,
                state: crate::store::token_state_name(token.state),
                owner: data.owner.0,
                seat: data.seat.0,
                token_state: crate::store::usage_name(data.token_state),
                limited_until: data.limited_until.map(|at| at.0),
                contributed_at: data.contributed_at.0,
                actor: crate::actor::written(&data.actor),
            }
        })
        .collect();
    rows.sort_by(|a, b| (&a.contributed_at, &a.token_id).cmp(&(&b.contributed_at, &b.token_id)));
    rows
}

/// One row of `GET /v1/tokens/leases` (`conductor.token.TokenLeases`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeaseRow {
    /// The lease's id.
    pub lease_id: String,
    /// `Held` or `Released`.
    pub state: &'static str,
    /// The token.
    pub token_id: String,
    /// The agent on it.
    pub agent: String,
    /// The agent's instance.
    pub instance: String,
    /// The token's owner.
    pub owner: String,
    /// When it was leased.
    pub leased_at: String,
    /// When it was released.
    pub released_at: Option<String>,
}

/// The leases of the host's pool, by lease time.
#[must_use]
pub fn lease_rows(store: &Store) -> Vec<LeaseRow> {
    let mut rows: Vec<LeaseRow> = store
        .token_leases()
        .into_iter()
        .map(|lease| LeaseRow {
            state: crate::store::lease_state_name(lease.state),
            lease_id: lease.data.lease_id.0.0,
            token_id: lease.data.token_id.0.0,
            agent: lease.data.agent.0,
            instance: lease.data.instance.0,
            owner: lease.data.owner.0,
            leased_at: lease.data.leased_at.0,
            released_at: lease.data.released_at.map(|at| at.0),
        })
        .collect();
    rows.sort_by(|a, b| (&a.leased_at, &a.lease_id).cmp(&(&b.leased_at, &b.lease_id)));
    rows
}

// ---------------------------------------------------------------------------------------------
// The runtime's side: the claude-code kind
// ---------------------------------------------------------------------------------------------

/// The daemon section of the config file this process read, if it read one.
fn daemon_config() -> Option<conductor_model::config::Daemon> {
    let file = crate::config::config_file()?;
    let environment = crate::config::Environment::from_process().ok()?;
    crate::config::load(Some(file), &environment)
        .ok()
        .map(|loaded| loaded.config.daemon)
}

/// The daemon's store at `url`, when its database is there: a SQLite file that exists, or a
/// Postgres URL. Never creates one.
fn existing_store(url: &str) -> Option<Store> {
    match crate::store::Location::parse(url).ok()? {
        crate::store::Location::Sqlite(path) if !path.is_file() => None,
        _ => crate::store::open_url(url, crate::daemon::TENANT).ok(),
    }
}

/// The daemon's store URL as the runtime of this process sees it, with the caps of its pool:
/// none without a config file or a database.
fn runtime_pool() -> Option<(String, Caps)> {
    let daemon = daemon_config()?;
    existing_store(&daemon.store.0)?;
    Some((daemon.store.0.clone(), Caps::of(&daemon)))
}

/// The daemon's store as the runtime of this process sees it: none without a config file or a
/// database.
#[must_use]
pub fn runtime_store() -> Option<Store> {
    existing_store(&daemon_config()?.store.0)
}

/// How a runtime in a box reaches conductord's pool: the dial-in address, its instance and the
/// token the daemon minted for its start. Held in this process's memory only.
struct Dial {
    address: String,
    instance: String,
    token: String,
}

static DIAL: Mutex<Option<Dial>> = Mutex::new(None);

/// The runtime start of this process, [`runtime_tag`] of its token: set by the runtime at its
/// start, read from [`RUNTIME_VARIABLE`] in a process it started.
static RUNTIME: OnceLock<String> = OnceLock::new();

/// Keeps how this process, a runtime in a box, reaches conductord's pool
/// ([`crate::instance::instance_serve`]): every lease, release and limit of its agents then goes
/// through conductord ([`pool_call`]), never a store of its own.
pub fn hold_dial(address: &str, instance: &str, token: &str) {
    *DIAL.lock().unwrap_or_else(PoisonError::into_inner) = Some(Dial {
        address: address.to_owned(),
        instance: instance.to_owned(),
        token: token.to_owned(),
    });
}

/// Keeps the runtime start of this process, from the token the daemon minted for it.
pub fn hold_runtime(token: &str) {
    let _ = RUNTIME.set(runtime_tag(token));
}

/// The runtime start this process belongs to, if any.
#[must_use]
pub fn runtime() -> Option<String> {
    RUNTIME.get().cloned().or_else(|| {
        std::env::var(RUNTIME_VARIABLE)
            .ok()
            .filter(|tag| !tag.is_empty() && tag.chars().all(|c| c.is_ascii_hexdigit()))
    })
}

/// The first release whose runtime in a box probes conductord's pool at its start
/// ([`probe_pool`]); an upgrade to it or a later one is answered only once the new start probed
/// ([`probed`], `story:upgrade-checks-daemon-protocols`).
pub const PROBE_SINCE: &str = "1.17.0";

/// How long a runtime waits between two probes of the pool that were not answered.
const PROBE_EVERY: Duration = Duration::from_secs(5);

/// Each instance whose runtime probed the pool, with the start that did ([`runtime_tag`]): kept
/// in conductord's memory, which answered the probe.
static PROBED: Mutex<BTreeMap<String, String>> = Mutex::new(BTreeMap::new());

/// Keeps that the start `runtime` of `instance` reached the pool.
fn record_probe(instance: &str, runtime: &str) {
    PROBED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(instance.to_owned(), runtime.to_owned());
}

/// The start of `instance` ([`runtime_tag`]) that last probed conductord's pool, if one did.
#[must_use]
pub fn probed(instance: &str) -> Option<String> {
    PROBED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(instance)
        .cloned()
}

/// One probe of conductord's pool from this runtime, as `agent`: `None` when this process holds
/// no dial ([`hold_dial`]); else whether conductord answered it, a lease's route, admission and
/// pool all reached.
#[must_use]
pub fn probe_pool(agent: &str) -> Option<Result<(), String>> {
    let answer = dial_call(&json!({"call": "probe", "agent": agent}))?;
    Some(answer.and_then(|answer| {
        if answer["probed"] == true || answer["empty"] == true {
            Ok(())
        } else {
            Err(answer["error"].as_str().map_or_else(
                || "the host's token pool: conductord answered no probe".to_owned(),
                |error| format!("the host's token pool: {error}"),
            ))
        }
    }))
}

/// Probes conductord's pool on a thread of its own, every [`PROBE_EVERY`] until it answers, as
/// `agent`; says on standard error, the runtime's log, how the first probe and the one that was
/// answered went. Nothing when this process holds no dial.
pub fn keep_probing(agent: String) {
    if DIAL
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_none()
    {
        return;
    }
    std::thread::spawn(move || {
        let mut said: Option<String> = None;
        loop {
            match probe_pool(&agent) {
                None => return,
                Some(Ok(())) => {
                    eprintln!("conductor instance serve: the host's token pool answers");
                    return;
                }
                Some(Err(error)) => {
                    if said.as_deref() != Some(error.as_str()) {
                        eprintln!("conductor instance serve: {error}; probing again");
                        said = Some(error);
                    }
                }
            }
            std::thread::sleep(PROBE_EVERY);
        }
    });
}

/// How long a pool call through conductord may take.
const DIAL_TIMEOUT: Duration = Duration::from_secs(30);

/// The longest request or answer of a pool call read.
const DIAL_ANSWER_MAX: u64 = 64 * 1024;

/// Sends `request` to conductord's pool over a new dial-in connection; `None` when this process
/// holds no dial ([`hold_dial`]).
fn dial_call(request: &Value) -> Option<Result<Value, String>> {
    let (address, hello) = {
        let held = DIAL.lock().unwrap_or_else(PoisonError::into_inner);
        let dial = held.as_ref()?;
        (
            dial.address.clone(),
            format!(
                "{} {} {}\n",
                crate::instance::POOL_HELLO,
                dial.instance,
                dial.token
            ),
        )
    };
    Some(ask_pool(&address, &hello, request))
}

/// A pool call conductord closed before any answer: it refused the connection's greeting. One
/// text however the close arrives, a reset (the request unread on its side) or an end of stream,
/// so [`keep_probing`] says one refusal once.
const CLOSED_UNANSWERED: &str = "the host's token pool: conductord closed the connection \
                                 unanswered: this runtime's start is not the one it routes";

/// Whether `error` is the peer's close of the connection: a reset, an abort or a broken pipe.
fn closed_by_peer(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::BrokenPipe
    )
}

/// One pool call: `hello`, then `request`, on a new connection to `address`; the answer.
fn ask_pool(address: &str, hello: &str, request: &Value) -> Result<Value, String> {
    let mut stream = TcpStream::connect(address).map_err(|error| {
        format!("the host's token pool: reach conductord at {address}: {error}")
    })?;
    let _ = stream.set_read_timeout(Some(DIAL_TIMEOUT));
    let _ = stream.set_write_timeout(Some(DIAL_TIMEOUT));
    stream
        .write_all(hello.as_bytes())
        .and_then(|()| stream.write_all(format!("{request}\n").as_bytes()))
        .map_err(|error| {
            if closed_by_peer(&error) {
                CLOSED_UNANSWERED.to_owned()
            } else {
                format!("the host's token pool: ask conductord: {error}")
            }
        })?;
    let mut line = String::new();
    BufReader::new(stream.take(DIAL_ANSWER_MAX))
        .read_line(&mut line)
        .map_err(|error| {
            if line.is_empty() && closed_by_peer(&error) {
                CLOSED_UNANSWERED.to_owned()
            } else {
                format!("the host's token pool: read conductord's answer: {error}")
            }
        })?;
    if line.trim().is_empty() {
        return Err(CLOSED_UNANSWERED.to_owned());
    }
    serde_json::from_str(&line)
        .map_err(|_| "the host's token pool: conductord's answer is no JSON".to_owned())
}

/// A lease and the value of its token, which goes into the harness's environment at its start
/// and nowhere else.
struct Leased {
    lease: Lease,
    value: String,
}

/// The value of the secret `secret`, `what` naming it, read from `secrets` now, as one line of
/// text.
fn secret_value(
    what: &str,
    secret: &SecretName,
    secrets: &conductor_model::config::SecretsSource,
) -> Result<String, String> {
    let source = crate::secret::open(secrets).map_err(|unusable| unusable.to_string())?;
    let value = match source.reveal(secret) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return Err(format!(
                "SecretMissing: {what}: the secrets source holds no value for it"
            ));
        }
        Err(error) => {
            return Err(format!(
                "SourceUnreachable: {what}: the {:?} source did not answer",
                error.source
            ));
        }
    };
    let value = std::str::from_utf8(value.expose())
        .map_err(|_| format!("{what}: its value is no one line of text"))?
        .trim_end_matches(['\n', '\r'])
        .to_owned();
    Ok(value)
}

/// The lease of the agent `agent` of `instance` at its start (module documentation), with its
/// token's value: `None` when the host's pool holds no token, so the start is today's; the role's
/// `token_owner`'s own token when it names one. A runtime in a box asks conductord ([`hold_dial`]); any other process leases in
/// the daemon's store and reads the value from the daemon's secrets source.
fn lease_for_start(
    instance: &Instance,
    role: &Role,
    agent: &str,
) -> Result<Option<Leased>, String> {
    let owner = role.token_owner.as_ref();
    let refusal = |instance: &str| {
        format!(
            "NoTokenAvailable: instance {instance}: no token of the host's pool has headroom{}; \
             nothing was started",
            owner
                .map(|owner| format!(" for its token_owner {}", owner.0))
                .unwrap_or_default()
        )
    };
    if let Some(answer) = dial_call(&json!({
        "call": "lease",
        "agent": agent,
        "owner": owner.map(|owner| &owner.0),
    })) {
        let answer = answer?;
        if answer["empty"] == true {
            return Ok(None);
        }
        if answer["none_available"] == true {
            return Err(refusal(&instance.name.0));
        }
        let lease = &answer["lease"];
        let text = |key: &str| lease[key].as_str().map(str::to_owned);
        let (Some(lease_id), Some(token_id), Some(owner), Some(seat), Some(kind), Some(value)) = (
            text("lease_id"),
            text("token_id"),
            text("owner"),
            text("seat"),
            text("kind").and_then(|kind| crate::store::token_kind_of(&kind)),
            text("value"),
        ) else {
            return Err(answer["error"].as_str().map_or_else(
                || "the host's token pool: conductord answered no lease".to_owned(),
                |error| format!("the host's token pool: {error}"),
            ));
        };
        return Ok(Some(Leased {
            lease: Lease {
                lease_id: TokenLeaseId(Uuid(lease_id)),
                token_id: TokenId(Uuid(token_id)),
                owner: OperatorName(owner),
                seat: Seat(seat),
                secret: SecretName(String::new()),
                kind,
            },
            value,
        }));
    }
    let Some((url, caps)) = runtime_pool() else {
        return Ok(None);
    };
    let store = open_store(&url).map_err(|error| format!("{error:#}"))?;
    if pool(&store).is_empty() {
        return Ok(None);
    }
    drop(store);
    let start = Start {
        instance: instance.name.clone(),
        agent: AgentName(agent.to_owned()),
        owner: owner.cloned(),
        actor: crate::actor::agent(instance, agent),
        runtime: runtime(),
    };
    let lease = match lease(&url, caps, &start) {
        Ok(Ok(lease)) => lease,
        Ok(Err(_)) => return Err(refusal(&instance.name.0)),
        Err(error) => return Err(format!("{error:#}")),
    };
    let what = format!("the pool token {}", lease.token_id.0.0);
    let value = daemon_config()
        .ok_or_else(|| {
            format!("{what}: the config file that names the daemon's secrets source is not read")
        })
        .and_then(|daemon| secret_value(&what, &lease.secret, &daemon.secrets));
    match value {
        Ok(value) => Ok(Some(Leased { lease, value })),
        Err(error) => {
            let _ = release_for_stop(instance, agent);
            Err(error)
        }
    }
}

/// Releases the lease `agent` of `instance` holds: through conductord for a runtime in a box,
/// else in the runtime's store when it is there.
///
/// # Errors
///
/// The store cannot be written, or conductord does not answer.
pub fn release_for_stop(instance: &Instance, agent: &str) -> Result<()> {
    if let Some(answer) = dial_call(&json!({"call": "release", "agent": agent})) {
        let answer = answer.map_err(|error| anyhow!("{error}"))?;
        if let Some(error) = answer["error"].as_str() {
            bail!("the host's token pool: {error}");
        }
        return Ok(());
    }
    if let Some((url, _)) = runtime_pool() {
        release(
            &url,
            &instance.name,
            &AgentName(agent.to_owned()),
            crate::actor::agent(instance, agent),
        )?;
    }
    Ok(())
}

/// The owner of the token `agent` runs on, when it holds a lease in the runtime's store.
#[must_use]
pub fn owner_of(agent: &AgentName) -> Option<OperatorName> {
    let store = runtime_store()?;
    store
        .token_leases()
        .into_iter()
        .find(|lease| lease.state == TokenLeaseState::Held && lease.data.agent == *agent)
        .map(|lease| lease.data.owner)
}

/// `text` as one shell word.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The `apiKeyHelper` that hands the harness the `model_token` of `instance`, of the kind
/// `api_key` (`story:container-start-without-prompt`): this binary's `token reveal --model-token`,
/// with the config file `config` when there is one. Names the instance, never the value.
#[must_use]
pub fn model_helper(program: &Path, config: Option<&str>, instance: &InstanceName) -> String {
    let mut words = vec![quoted(&program.display().to_string())];
    if let Some(config) = config {
        words.extend(["--config".to_owned(), quoted(config)]);
    }
    words.extend([
        "token".to_owned(),
        "reveal".to_owned(),
        "--model-token".to_owned(),
        quoted(&instance.0),
    ]);
    words.join(" ")
}

/// [`copy_settings`] with [`HELPER_KEY`] naming the helper `helper` builds from this binary and
/// the config file.
fn apply_helper(
    launch: &mut Launch,
    helper: impl FnOnce(&Path, Option<&str>) -> String,
    remote_control: bool,
) -> Result<PathBuf> {
    let config = launch
        .env
        .iter()
        .find(|(key, _)| key == crate::config::CONFIG_VARIABLE)
        .map(|(_, value)| value.clone());
    let program = std::env::current_exe().context("the token's helper: read the binary")?;
    let helper = helper(&program, config.as_deref());
    copy_settings(
        launch,
        |settings| {
            settings.insert(HELPER_KEY.to_owned(), Value::String(helper));
        },
        remote_control,
    )
}

/// Writes the launch's `--settings` file, read as it is and changed by `edit`, again to
/// `<state>/sessions/<session name>.settings.json`, and starts the launch with that copy;
/// `remote_control` adds `--remote-control <session name>`. Answers the settings file written.
fn copy_settings(
    launch: &mut Launch,
    edit: impl FnOnce(&mut serde_json::Map<String, Value>),
    remote_control: bool,
) -> Result<PathBuf> {
    let name = launch.request.session_name.0.clone();
    let at = launch.args.iter().position(|arg| arg == "--settings");
    let mut settings: Value = match at.and_then(|at| launch.args.get(at + 1)) {
        Some(file) => match fs::read_to_string(file) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("the settings {file} are no JSON"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => json!({}),
            Err(error) => return Err(error).with_context(|| format!("read the settings {file}")),
        },
        None => json!({}),
    };
    let Some(object) = settings.as_object_mut() else {
        bail!("the settings of {name} are no JSON object");
    };
    edit(object);
    let dir = launch.state.join("sessions");
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let file = dir.join(format!("{name}.settings.json"));
    let text = serde_json::to_string_pretty(&settings).context("write the settings")?;
    fs::write(&file, format!("{text}\n")).with_context(|| format!("write {}", file.display()))?;
    let shown = file.display().to_string();
    match at {
        Some(at) if at + 1 < launch.args.len() => launch.args[at + 1] = shown,
        _ => launch.args.extend(["--settings".to_owned(), shown]),
    }
    if remote_control {
        launch
            .args
            .extend(["--remote-control".to_owned(), name.clone()]);
    }
    Ok(file)
}

/// The variable a token of `kind` reaches the harness in.
#[must_use]
pub const fn variable_of(kind: ModelTokenKind) -> &'static str {
    match kind {
        ModelTokenKind::SubscriptionToken => OAUTH_VARIABLE,
        ModelTokenKind::ApiKey => API_KEY_VARIABLE,
    }
}

/// Puts `value`, the token of `kind` `what` names, into `launch`: as [`variable_of`] the kind, of
/// the launch's environment, which the provider sets on the harness's process alone; and the
/// launch's settings copied without a [`HELPER_KEY`], which would outrank the variable. One the
/// settings named is left out, and one line of standard error says so, without its text. The
/// value is in no argument, no file and no line written. `remote_control` adds `--remote-control
/// <session name>`.
fn apply_value(
    launch: &mut Launch,
    what: &str,
    kind: ModelTokenKind,
    value: String,
    remote_control: bool,
) -> Result<(), String> {
    let variable = variable_of(kind);
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(format!(
            "{what}: its value is no one line of text, which {variable} holds"
        ));
    }
    let mut held = false;
    let file = copy_settings(
        launch,
        |settings| held = settings.remove(HELPER_KEY).is_some(),
        remote_control,
    )
    .map_err(|error| format!("{error:#}"))?;
    if held {
        eprintln!(
            "conductor: session {}: its settings named an {HELPER_KEY}, which outranks \
             {variable}; the session's settings {} leave it out, since {what} reaches it in \
             {variable}",
            launch.request.session_name.0,
            file.display()
        );
    }
    launch
        .env
        .retain(|(key, _)| key != OAUTH_VARIABLE && key != API_KEY_VARIABLE);
    launch.env.push((variable.to_owned(), value));
    Ok(())
}

/// The model token the runtime of a box read from its sealed slot at its start
/// (`story:box-subscription-token`): set once, by [`hold_sealed_model_token`], and only in that
/// process; never written anywhere.
static SEALED_MODEL_TOKEN: OnceLock<String> = OnceLock::new();

/// Keeps `value`, the model token the runtime of a box read from its sealed slot
/// (`crate::instance`), for the agents it starts: [`lease_into`] hands it to the harness of an
/// instance whose `model_token_kind` is `subscription_token`. A second call keeps the first value.
pub fn hold_sealed_model_token(value: String) {
    let _ = SEALED_MODEL_TOKEN.set(value);
}

/// The value [`lease_into`] hands the harness of an agent of `instance`, at a placement that
/// opens a box, as [`OAUTH_VARIABLE`], when its `model_token_kind` is `subscription_token`: the
/// model token the runtime of its box read from its sealed slot ([`hold_sealed_model_token`]),
/// else, in the `conductor session start` the runtime started and handed it to explicitly (the
/// maintainer, `run::handlers`; [`crate::instance::command`] passes it to no other process), the
/// variable that process was started with. `None` for another kind, an instance with no
/// `model_token`, or none of the two.
#[must_use]
pub fn boxed_subscription_token(instance: &Instance) -> Option<String> {
    if instance.model_token.is_none()
        || instance.model_token_kind != Some(ModelTokenKind::SubscriptionToken)
        || !crate::placement::boxed(instance)
    {
        return None;
    }
    SEALED_MODEL_TOKEN.get().cloned().or_else(|| {
        std::env::var(OAUTH_VARIABLE)
            .ok()
            .filter(|value| !value.trim().is_empty())
    })
}

/// Leases a token of the host's pool for the launch of `role`'s agent and puts its value into
/// `launch` ([`apply_value`]); a role that names `token_owner` starts for Remote Control. Answers
/// the lease, `None` with an empty pool. With an empty pool the launch is left as it is, but for
/// an instance whose placement opens no box and that names a `model_token`, where the config file
/// says `daemon.unattended` (`story:container-start-without-prompt`): the agent then runs on that
/// token, by its `model_token_kind`. An `api_key`, the kind of a token that names none: its
/// settings name [`model_helper`]. A `subscription_token`: its value, read from the daemon's
/// secrets source now, into the launch's environment. Without `daemon.unattended` an empty pool
/// leaves every launch as it is, so an agent on a workstation keeps the harness's own login. At a
/// placement that opens a box, a `subscription_token` the runtime holds from its box's sealed slot
/// ([`boxed_subscription_token`], `story:box-subscription-token`) goes into the launch's
/// environment the same way; an `api_key` there leaves the launch as it is.
///
/// # Errors
///
/// `NoTokenAvailable`, the settings cannot be written, or a token cannot be read; one line,
/// which holds no token's value.
pub fn lease_into(
    instance: &Instance,
    role: &Role,
    launch: &mut Launch,
) -> Result<Option<Lease>, String> {
    let name = launch.request.session_name.0.clone();
    let Some(Leased { lease, value }) = lease_for_start(instance, role, &name)? else {
        let what = format!("the model_token of instance {}", instance.name.0);
        if let Some(value) = boxed_subscription_token(instance) {
            apply_value(
                launch,
                &what,
                ModelTokenKind::SubscriptionToken,
                value,
                false,
            )?;
        } else if let Some(secret) = &instance.model_token
            && !crate::placement::boxed(instance)
            && let Some(daemon) = daemon_config().filter(|daemon| daemon.unattended)
        {
            match instance.model_token_kind {
                Some(ModelTokenKind::SubscriptionToken) => {
                    let value = secret_value(&what, secret, &daemon.secrets)?;
                    apply_value(
                        launch,
                        &what,
                        ModelTokenKind::SubscriptionToken,
                        value,
                        false,
                    )?;
                }
                Some(ModelTokenKind::ApiKey) | None => {
                    apply_helper(
                        launch,
                        |program, config| model_helper(program, config, &instance.name),
                        false,
                    )
                    .map_err(|error| format!("{error:#}"))?;
                }
            }
        }
        return Ok(None);
    };
    let what = format!("the pool token {}", lease.token_id.0.0);
    if let Err(error) = apply_value(launch, &what, lease.kind, value, role.token_owner.is_some()) {
        let _ = release_for_stop(instance, &name);
        return Err(error);
    }
    Ok(Some(lease))
}

/// What a usage-limit stop of an agent did to the host's pool ([`on_limit`],
/// `story:limit-aware-token-reassignment`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LimitAnswer {
    /// The token the agent held, now Limited, and until when; none when it held no lease.
    pub limited: Option<(String, OffsetDateTime)>,
    /// A token of the pool has headroom now: the agent is resumed at once.
    pub other: bool,
    /// Else the first instant a token of the pool resets ([`first_reset`]): the agent is resumed
    /// then.
    pub first_reset: Option<OffsetDateTime>,
}

impl LimitAnswer {
    /// The answer conductord sends a runtime in a box for it ([`pool_call`]).
    fn to_json(&self) -> Value {
        let stamp = |at: &OffsetDateTime| at.format(&Rfc3339).ok();
        json!({
            "limited": self.limited.is_some(),
            "token": self.limited.as_ref().map(|(token, _)| token),
            "until": self.limited.as_ref().and_then(|(_, until)| stamp(until)),
            "other": self.other,
            "first_reset": self.first_reset.as_ref().and_then(stamp),
        })
    }

    /// The answer `answer` of conductord reads as.
    fn of_json(answer: &Value) -> Self {
        let at = |key: &str| answer[key].as_str().and_then(instant);
        let limited = match (
            answer["limited"] == true,
            answer["token"].as_str(),
            at("until"),
        ) {
            (true, Some(token), Some(until)) => Some((token.to_owned(), until)),
            _ => None,
        };
        Self {
            other: answer["other"] == true,
            first_reset: at("first_reset"),
            limited,
        }
    }
}

/// What the loop does on a usage-limit stop of `session` (module documentation): marks its
/// token Limited until `until` ([`limit`]) and releases its lease; answers the token and its
/// reset, whether another token of the pool has headroom, so the session is resumed at once, and
/// else the first instant a token of the pool resets, so it is resumed then. No probe of the
/// account is asked: the tokens' state is the answer.
///
/// # Errors
///
/// The store cannot be written, or conductord does not answer.
pub fn on_limit(
    instance: &Instance,
    session: &str,
    until: Option<OffsetDateTime>,
) -> Result<LimitAnswer> {
    let until_text = until
        .map(|until| until.format(&Rfc3339))
        .transpose()
        .context("format the end of a usage limit")?;
    if let Some(answer) = dial_call(&json!({
        "call": "limit",
        "agent": session,
        "until": until_text,
    })) {
        let answer = answer.map_err(|error| anyhow!("{error}"))?;
        if let Some(error) = answer["error"].as_str() {
            bail!("the host's token pool: {error}");
        }
        return Ok(LimitAnswer::of_json(&answer));
    }
    let Some((url, caps)) = runtime_pool() else {
        return Ok(LimitAnswer::default());
    };
    limit_in(
        &url,
        caps,
        &instance.name,
        &AgentName(session.to_owned()),
        crate::actor::agent(instance, session),
        until,
    )
}

/// [`limit`] of `agent` in the pool at `url`, and what the pool holds for it then
/// ([`LimitAnswer`]).
///
/// # Errors
///
/// The store cannot be written.
pub fn limit_in(
    url: &str,
    caps: Caps,
    instance: &InstanceName,
    agent: &AgentName,
    actor: Actor,
    until: Option<OffsetDateTime>,
) -> Result<LimitAnswer> {
    let Some(token) = limit(url, instance, agent, actor, until)? else {
        return Ok(LimitAnswer::default());
    };
    let store = open_store(url)?;
    let tokens = pool(&store);
    let limited_until = tokens
        .iter()
        .find(|found| found.data.token_id == token.token_id)
        .and_then(|found| found.data.limited_until.as_ref())
        .and_then(|at| instant(&at.0))
        .unwrap_or_else(|| limit_end(until, OffsetDateTime::now_utc()));
    let live = store.live_leases();
    let now = OffsetDateTime::now_utc();
    let other = pick(
        &tokens,
        &|id| live.get(&id.0.0).copied().unwrap_or(0),
        caps,
        None,
        now,
    )
    .is_some();
    Ok(LimitAnswer {
        limited: Some((token.token_id.0.0, limited_until)),
        other,
        first_reset: if other {
            None
        } else {
            first_reset(&tokens, now)
        },
    })
}

/// The agents `<run dir>/tokens-withdrawn.json` names, and the file emptied.
#[must_use]
pub fn take_withdrawn(run: &Path) -> Vec<String> {
    let file = run.join(WITHDRAWN_FILE);
    let names: Vec<String> = fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    if !names.is_empty() {
        let _ = fs::remove_file(&file);
    }
    names
}

/// Adds `agents` to the instance's `<state>/run/tokens-withdrawn.json`.
fn note_withdrawn(state: &Path, agents: &[String]) -> Result<()> {
    if agents.is_empty() {
        return Ok(());
    }
    let run = state.join("run");
    fs::create_dir_all(&run).with_context(|| format!("create {}", run.display()))?;
    let file = run.join(WITHDRAWN_FILE);
    let mut names: Vec<String> = fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    for agent in agents {
        if !names.contains(agent) {
            names.push(agent.clone());
        }
    }
    fs::write(&file, serde_json::to_string(&names)?)
        .with_context(|| format!("write {}", file.display()))
}

// ---------------------------------------------------------------------------------------------
// The daemon's routes
// ---------------------------------------------------------------------------------------------

/// What the daemon's pool routes read: its config file, its store and its operators.
pub struct Daemon<'a> {
    /// The config file.
    pub file: &'a Path,
    /// `daemon.store`.
    pub store: &'a str,
    /// `daemon.operators`.
    pub operators: &'a [DaemonOperator],
}

/// The body of `POST /v1/join`.
#[derive(Deserialize)]
struct JoinBody {
    seat: Option<String>,
    token: String,
    /// `SubscriptionToken` or `ApiKey`; absent, `SubscriptionToken`.
    #[serde(default)]
    kind: Option<String>,
}

const fn grant_of(grant: OperatorGrant) -> Grant {
    match grant {
        OperatorGrant::AnswerDecisions => Grant::AnswerDecisions,
        OperatorGrant::ChangeSettings => Grant::ChangeSettings,
        OperatorGrant::StartInstances => Grant::StartInstances,
        OperatorGrant::StopInstances => Grant::StopInstances,
        OperatorGrant::UpgradeInstances => Grant::UpgradeInstances,
        OperatorGrant::Purge => Grant::Purge,
        OperatorGrant::Join => Grant::Join,
    }
}

/// An answer of a route: status, reason, body.
pub type Answer = (u16, &'static str, Value);

fn failed(error: &anyhow::Error) -> Answer {
    (
        500,
        "Internal Server Error",
        json!({"error": format!("{error:#}")}),
    )
}

/// The config file of `daemon`, loaded.
fn loaded(daemon: &Daemon<'_>) -> Result<conductor_model::config::Config> {
    let environment = crate::config::Environment::from_process()?;
    Ok(crate::config::load(Some(daemon.file), &environment)?.config)
}

fn open(daemon: &Daemon<'_>) -> Result<Store> {
    if let Ok(crate::store::Location::Sqlite(path)) = crate::store::Location::parse(daemon.store)
        && let Some(parent) = path.parent()
    {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    open_store(daemon.store)
}

/// `GET /v1/tokens` and `GET /v1/instances/{name}/tokens`: the rows of the host's pool
/// ([`rows`]).
#[must_use]
pub fn tokens_route(daemon: &Daemon<'_>) -> Answer {
    let Some(store) = existing_store(daemon.store) else {
        return (200, "OK", json!([]));
    };
    (200, "OK", json!(rows(&store)))
}

/// `GET /v1/tokens/leases`: the leases of the host's pool ([`lease_rows`]).
#[must_use]
pub fn leases_route(daemon: &Daemon<'_>) -> Answer {
    let Some(store) = existing_store(daemon.store) else {
        return (200, "OK", json!([]));
    };
    (200, "OK", json!(lease_rows(&store)))
}

/// The instance of `config` named `name`, or the route's `404`.
fn named_instance(
    config: &conductor_model::config::Config,
    name: Option<&str>,
) -> Result<Option<InstanceName>, Answer> {
    let Some(name) = name else {
        return Ok(None);
    };
    config
        .instances
        .iter()
        .find(|instance| instance.name.0 == name)
        .map(|instance| Some(instance.name.clone()))
        .ok_or_else(|| {
            (
                404,
                "Not Found",
                json!({"error": format!("the config file holds no instance {name}")}),
            )
        })
}

/// `POST /v1/join`, or with `name` `POST /v1/instances/{name}/join`, as `operator`, a person the
/// listener admitted with the `Join` grant (`implied`: the loopback operator while
/// `daemon.operators` is empty, who holds every grant): with `name`, `JoinInstance` with the
/// grants `daemon.operators` gives the operator; then `ContributeToken` with the body's token,
/// into the host's pool. The answer names the token and the instances it serves, never its value.
#[must_use]
pub fn join_route(
    daemon: &Daemon<'_>,
    name: Option<&str>,
    operator: &str,
    implied: bool,
    body: &[u8],
) -> Answer {
    let request: JoinBody = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(_) => {
            return (
                400,
                "Bad Request",
                json!({"error": "the body is not a join: write {\"seat\", \"token\"}"}),
            );
        }
    };
    let kind = match request.kind.as_deref() {
        None => ModelTokenKind::SubscriptionToken,
        Some(name) => match crate::store::token_kind_of(name) {
            Some(kind) => kind,
            None => {
                return (
                    400,
                    "Bad Request",
                    json!({"error": format!(
                        "the kind {name:?} is none of SubscriptionToken, ApiKey"
                    )}),
                );
            }
        },
    };
    let value = match SecretValue::new(request.token.into_bytes()) {
        Ok(value) if !value.is_empty() => value,
        _ => {
            return (
                400,
                "Bad Request",
                json!({"error": "the body carries no token"}),
            );
        }
    };
    let config = match loaded(daemon) {
        Ok(config) => config,
        Err(error) => return failed(&error),
    };
    let instance = match named_instance(&config, name) {
        Ok(instance) => instance,
        Err(answer) => return answer,
    };
    let serves: Vec<String> = config
        .instances
        .iter()
        .map(|instance| instance.name.0.clone())
        .collect();
    let grants: Vec<Grant> = if implied {
        vec![
            Grant::AnswerDecisions,
            Grant::ChangeSettings,
            Grant::StartInstances,
            Grant::StopInstances,
            Grant::UpgradeInstances,
            Grant::Purge,
            Grant::Join,
        ]
    } else {
        daemon
            .operators
            .iter()
            .find(|entry| entry.name.0 == operator)
            .map(|entry| entry.grants.iter().map(|grant| grant_of(*grant)).collect())
            .unwrap_or_default()
    };
    let secrets = match crate::secret::open(&config.daemon.secrets) {
        Ok(secrets) => secrets,
        Err(unusable) => {
            return (
                503,
                "Service Unavailable",
                json!({"error": unusable.to_string()}),
            );
        }
    };
    let actor = crate::actor::operator(operator);
    let at = match now() {
        Ok(at) => at,
        Err(error) => return failed(&error),
    };
    let mut store = match open(daemon) {
        Ok(store) => store,
        Err(error) => return failed(&error),
    };
    if let Some(instance) = &instance {
        let mut generated = Generated::new(store);
        let joined = match generated.join_instance(JoinInstance {
            actor: actor.clone(),
            key: OperatorKey {
                operator: OperatorName(operator.to_owned()),
                instance: instance.clone(),
            },
            grants: grants.clone(),
            joined_at: at,
        }) {
            Ok(joined) => joined,
            Err(unmet_obligation) => return failed(&unmet(unmet_obligation)),
        };
        if let JoinInstanceOutcome::ActorNotHuman { .. } = joined {
            return (
                403,
                "Forbidden",
                json!({"error": "conductor.operator.ActorNotHuman"}),
            );
        }
        if let Err(error) = generated.ports.check() {
            return failed(&anyhow!("record the operator: {error}"));
        }
        store = generated.ports;
    }
    let seat = Seat(request.seat.unwrap_or_else(|| operator.to_owned()));
    let owner = OperatorName(operator.to_owned());
    let contribution = Contribution {
        actor,
        owner: owner.clone(),
        seat: seat.clone(),
        kind,
    };
    match contribute(store, &secrets, &contribution, value) {
        Ok(Ok(ContributeTokenOutcome::Contributed { token_contributed })) => {
            println!(
                "conductord: {} contributed token {} to the host's pool, seat {}{}",
                owner.0,
                token_contributed.token_id.0.0,
                seat.0,
                instance
                    .as_ref()
                    .map(|instance| format!(", and joined {}", instance.0))
                    .unwrap_or_default()
            );
            let granted: Option<Vec<&str>> = instance.as_ref().map(|_| {
                grants
                    .iter()
                    .map(|grant| crate::store::grant_name(*grant))
                    .collect()
            });
            (
                200,
                "OK",
                json!({
                    "instance": instance.as_ref().map(|instance| &instance.0),
                    "operator": owner.0,
                    "grants": granted,
                    "token_id": token_contributed.token_id.0.0,
                    "seat": seat.0,
                    "kind": crate::store::token_kind_name(kind),
                    "state": "Available",
                    "serves": serves,
                }),
            )
        }
        Ok(Ok(ContributeTokenOutcome::ActorNotHuman { .. })) => (
            403,
            "Forbidden",
            json!({"error": "conductor.operator.ActorNotHuman"}),
        ),
        Ok(Err(refusal)) => (503, "Service Unavailable", json!({"error": refusal})),
        Err(error) => failed(&error),
    }
}

/// `POST /v1/leave`, or with `name` `POST /v1/instances/{name}/leave`, as `operator`: with
/// `name`, `LeaveInstance`; then `WithdrawToken` of each token of the host's pool the operator
/// owns, each lease on it released and its agent named for its instance's loop to resume on
/// another token ([`WITHDRAWN_FILE`]). Under the `files` source the file of each token withdrawn
/// is removed (`story:pool-token-kind-files-source`).
#[must_use]
pub fn leave_route(daemon: &Daemon<'_>, name: Option<&str>, operator: &str) -> Answer {
    let config = match loaded(daemon) {
        Ok(config) => config,
        Err(error) => return failed(&error),
    };
    let instance = match named_instance(&config, name) {
        Ok(instance) => instance,
        Err(answer) => return answer,
    };
    let source = config.daemon.secrets.clone();
    let actor = crate::actor::operator(operator);
    let owner = OperatorName(operator.to_owned());
    let at = match now() {
        Ok(at) => at,
        Err(error) => return failed(&error),
    };
    let store = match open(daemon) {
        Ok(store) => store,
        Err(error) => return failed(&error),
    };
    let mut was_operator = false;
    let store = if let Some(instance) = &instance {
        let mut generated = Generated::new(store);
        let left = match generated.leave_instance(LeaveInstance {
            actor: actor.clone(),
            key: OperatorKey {
                operator: owner.clone(),
                instance: instance.clone(),
            },
            left_at: at,
        }) {
            Ok(left) => left,
            Err(unmet_obligation) => return failed(&unmet(unmet_obligation)),
        };
        if let LeaveInstanceOutcome::ActorNotHuman { .. } = left {
            return (
                403,
                "Forbidden",
                json!({"error": "conductor.operator.ActorNotHuman"}),
            );
        }
        if let Err(error) = generated.ports.check() {
            return failed(&anyhow!("record the leave: {error}"));
        }
        was_operator = !matches!(
            left,
            LeaveInstanceOutcome::NotJoined { .. } | LeaveInstanceOutcome::UnknownOperator { .. }
        );
        generated.ports
    } else {
        store
    };
    let owned: Vec<(TokenId, SecretName)> = pool(&store)
        .into_iter()
        .filter(|token| token.data.owner == owner)
        .map(|token| (token.data.token_id, token.data.secret))
        .collect();
    drop(store);
    let files = if source.source == conductor_model::secret::Source::Files {
        match crate::secret::open(&source) {
            Ok(secrets) => Some(secrets),
            Err(unusable) => {
                return (
                    503,
                    "Service Unavailable",
                    json!({"error": unusable.to_string()}),
                );
            }
        }
    } else {
        None
    };
    let mut withdrawn = Vec::new();
    let mut resume: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (token_id, secret) in owned {
        match withdraw(daemon.store, &token_id, actor.clone()) {
            Ok((WithdrawTokenOutcome::Withdrawn { .. }, agents)) => {
                withdrawn.push(token_id.0.0.clone());
                for (instance, agent) in agents {
                    resume.entry(instance.0).or_default().push(agent.0);
                }
                if let Some(secrets) = &files
                    && let Err(error) = secrets.remove_pool_token(&secret)
                {
                    return failed(&error);
                }
            }
            Ok(_) => {}
            Err(error) => return failed(&error),
        }
    }
    for (name, agents) in &resume {
        if let Some(found) = config.instances.iter().find(|found| found.name.0 == *name)
            && let Err(error) = note_withdrawn(Path::new(&found.state), agents)
        {
            return failed(&error);
        }
    }
    let resumed: Vec<String> = resume.values().flatten().cloned().collect();
    if !was_operator && withdrawn.is_empty() {
        let why = match &instance {
            Some(instance) => format!(
                "conductor.operator.NotJoined: {operator} is no operator of {} and owns no token \
                 of the host's pool",
                instance.0
            ),
            None => format!("{operator} owns no token of the host's pool"),
        };
        return (409, "Conflict", json!({"error": why}));
    }
    println!(
        "conductord: {} left{}; {} token(s) withdrawn from the host's pool; {} agent(s) to resume \
         on another token",
        owner.0,
        instance
            .as_ref()
            .map(|instance| format!(" {}", instance.0))
            .unwrap_or_default(),
        withdrawn.len(),
        resumed.len()
    );
    (
        200,
        "OK",
        json!({
            "instance": instance.as_ref().map(|instance| &instance.0),
            "operator": owner.0,
            "withdrawn": withdrawn,
            "resume": resumed,
        }),
    )
}

/// The request of a runtime in a box to conductord's pool, one JSON line after its greeting
/// ([`crate::daemon::dial`]). It names no instance: conductord takes the one it admitted.
#[derive(Deserialize)]
struct PoolRequest {
    call: String,
    agent: String,
    #[serde(default)]
    owner: Option<String>,
    #[serde(default)]
    until: Option<String>,
}

/// Whether `name` may name an agent: a session name's characters, at most 200 of them.
fn agent_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 200
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// conductord's answer to one pool call of the runtime of `instance`, the instance conductord
/// admitted the connection as ([`crate::daemon::api::Api::admit_dial`]), whose start `runtime`
/// names ([`runtime_tag`]): a lease (with its token's value, the one place it is sent), a release
/// or a limit of that instance's agent. The request names no instance; one it names is not read.
/// The answer carries no value but a lease's.
#[must_use]
pub fn pool_call(daemon: &Daemon<'_>, instance: &str, runtime: &str, request: &[u8]) -> Value {
    let refused = |why: &str| json!({"error": why});
    let Ok(request) = serde_json::from_slice::<PoolRequest>(request) else {
        return refused("the request is no pool call");
    };
    if !agent_name(&request.agent) {
        return refused("the agent's name is no session name");
    }
    let config = match loaded(daemon) {
        Ok(config) => config,
        Err(error) => return refused(&format!("{error:#}")),
    };
    let Some(found) = config
        .instances
        .iter()
        .find(|found| found.name.0 == instance)
    else {
        return refused("the config file holds no such instance");
    };
    let agent = AgentName(request.agent.clone());
    let actor = crate::actor::agent(found, &request.agent);
    let caps = Caps::of(&config.daemon);
    match request.call.as_str() {
        "lease" => {
            if existing_store(daemon.store).is_none_or(|store| pool(&store).is_empty()) {
                return json!({"empty": true});
            }
            let start = Start {
                instance: found.name.clone(),
                agent: agent.clone(),
                owner: request.owner.map(OperatorName),
                actor,
                runtime: Some(runtime.to_owned()),
            };
            match lease(daemon.store, caps, &start) {
                Ok(Ok(lease)) => {
                    let what = format!("the pool token {}", lease.token_id.0.0);
                    match secret_value(&what, &lease.secret, &config.daemon.secrets) {
                        Ok(value) => {
                            println!(
                                "conductord: {instance}: {} leases token {}",
                                agent.0, lease.token_id.0.0
                            );
                            json!({"lease": {
                                "lease_id": lease.lease_id.0.0,
                                "token_id": lease.token_id.0.0,
                                "owner": lease.owner.0,
                                "seat": lease.seat.0,
                                "kind": crate::store::token_kind_name(lease.kind),
                                "value": value,
                            }})
                        }
                        Err(error) => {
                            let _ =
                                release(daemon.store, &found.name, &agent, crate::actor::daemon());
                            refused(&error)
                        }
                    }
                }
                Ok(Err(_)) => json!({"none_available": true}),
                Err(error) => refused(&format!("{error:#}")),
            }
        }
        "release" => match release(daemon.store, &found.name, &agent, actor) {
            Ok(token) => json!({"released": token.is_some()}),
            Err(error) => refused(&format!("{error:#}")),
        },
        "limit" => {
            let until = match request.until.as_deref().map(instant) {
                None => None,
                Some(Some(until)) => Some(until),
                Some(None) => return refused("the end of the limit is no RFC 3339 time"),
            };
            match limit_in(daemon.store, caps, &found.name, &agent, actor, until) {
                Ok(answer) => {
                    if let Some((token, until)) = &answer.limited {
                        println!(
                            "conductord: {instance}: {} limits token {token} until {}",
                            agent.0,
                            until.format(&Rfc3339).unwrap_or_default()
                        );
                    }
                    answer.to_json()
                }
                Err(error) => refused(&format!("{error:#}")),
            }
        }
        "probe" => {
            // story:upgrade-checks-daemon-protocols: the start reached the pool through its
            // route and admission; the pool's store, when there is one, opens.
            if let Some(store) = existing_store(daemon.store)
                && let Err(error) = store.check()
            {
                return refused(&format!("{error:#}"));
            }
            record_probe(instance, runtime);
            json!({"probed": true})
        }
        _ => refused("the call is none of lease, release, limit, probe"),
    }
}

/// Reads one pool call from `stream` after its greeting and writes conductord's answer
/// ([`pool_call`]); the request is bounded in size and time.
///
/// # Errors
///
/// The request cannot be read or the answer written.
pub fn serve_pool_call(
    stream: &mut TcpStream,
    daemon: &Daemon<'_>,
    instance: &str,
    runtime: &str,
) -> Result<()> {
    stream.set_read_timeout(Some(DIAL_TIMEOUT))?;
    let mut line = Vec::new();
    BufReader::new((&*stream).take(DIAL_ANSWER_MAX)).read_until(b'\n', &mut line)?;
    let answer = pool_call(daemon, instance, runtime, &line);
    stream.write_all(format!("{answer}\n").as_bytes())?;
    stream.flush()?;
    Ok(())
}

/// Reads one pool call from `stream` after its greeting and answers it `{"empty": true}`: the
/// answer of a conductord that serves no config file, and so holds no pool. A probe of the start
/// `runtime` of `instance` is kept as one ([`probed`]).
///
/// # Errors
///
/// The request cannot be read or the answer written.
pub fn serve_no_pool(stream: &mut TcpStream, instance: &str, runtime: &str) -> Result<()> {
    stream.set_read_timeout(Some(DIAL_TIMEOUT))?;
    let mut line = Vec::new();
    BufReader::new((&*stream).take(DIAL_ANSWER_MAX)).read_until(b'\n', &mut line)?;
    if serde_json::from_slice::<PoolRequest>(&line).is_ok_and(|request| request.call == "probe") {
        record_probe(instance, runtime);
    }
    stream.write_all(format!("{}\n", json!({"empty": true})).as_bytes())?;
    stream.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// The CLI
// ---------------------------------------------------------------------------------------------

/// The daemon the global `--daemon` or `CONDUCTOR_DAEMON` names, which `join` and `leave` need.
fn named_daemon<'a>(daemon: Option<&'a str>, command: &str) -> Result<&'a str> {
    daemon
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "{command}: name the daemon: --daemon <url> or {}",
                crate::remote::VARIABLE
            )
        })
}

/// The refusal a daemon answered, on one line.
fn refusal(answer: &Value, status: u16) -> String {
    answer["error"]
        .as_str()
        .map_or_else(|| format!("the daemon answered {status}"), str::to_owned)
}

/// The route of `verb` (`join` or `leave`): the instance's when `--instance` names one, else the
/// host's.
fn route_of(instance: Option<&String>, verb: &str) -> String {
    match instance
        .map(String::as_str)
        .filter(|name| !name.trim().is_empty())
    {
        Some(instance) => format!("/v1/instances/{instance}/{verb}"),
        None => format!("/v1/{verb}"),
    }
}

/// The strings of `answer[key]`, joined by `", "`.
fn listed(answer: &Value, key: &str) -> String {
    answer[key]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default()
}

/// `conductor join --daemon <url> [--instance <i>] [--seat <s>]`, the token on standard input
/// (module documentation); `daemon` is the global `--daemon` or `CONDUCTOR_DAEMON`.
///
/// # Errors
///
/// Standard input holds no value, or the daemon does not answer. A refusal is one line of
/// standard error and exit 1.
pub fn join(args: &JoinArgs, daemon: Option<&str>) -> Result<ExitCode> {
    const WHAT: &str = "join";
    let url = named_daemon(daemon, WHAT)?;
    let actor = crate::actor::chosen();
    if actor.kind != ActorKind::Human {
        eprintln!(
            "conductor: {WHAT}: ActorNotHuman: {}",
            crate::actor::not_human(&actor)
        );
        return Ok(ExitCode::FAILURE);
    }
    let value = crate::secret::value_from_stdin("your Claude token").context(WHAT)?;
    let token = String::from_utf8(value.expose().to_vec())
        .map_err(|_| anyhow!("{WHAT}: the token on standard input is no text"))?;
    let body = json!({"seat": args.seat, "token": token, "kind": args.kind});
    drop(token);
    let (status, answer) = crate::remote::call(
        url,
        "POST",
        &route_of(args.instance.as_ref(), WHAT),
        Some(&body),
    )
    .context(WHAT)?;
    drop(body);
    if status != 200 {
        eprintln!("conductor: {WHAT}: refused: {}", refusal(&answer, status));
        return Ok(ExitCode::FAILURE);
    }
    let mut out = std::io::stdout().lock();
    if let Some(instance) = answer["instance"].as_str() {
        writeln!(
            out,
            "joined {instance} as {} (grants {})",
            answer["operator"].as_str().unwrap_or_default(),
            listed(&answer, "grants").replace(", ", ","),
        )
        .context(WHAT)?;
    }
    let serves = listed(&answer, "serves");
    writeln!(
        out,
        "token {} in the host's pool, seat {}, kind {}, {}; it serves {}",
        answer["token_id"].as_str().unwrap_or_default(),
        answer["seat"].as_str().unwrap_or_default(),
        answer["kind"].as_str().unwrap_or_default(),
        answer["state"].as_str().unwrap_or_default(),
        if serves.is_empty() {
            "no instance yet: every instance on the host leases from the pool".to_owned()
        } else {
            format!("every instance on the host: {serves}")
        },
    )
    .context(WHAT)?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor leave --daemon <url> [--instance <i>]` (module documentation); `daemon` is the
/// global `--daemon` or `CONDUCTOR_DAEMON`.
///
/// # Errors
///
/// The daemon does not answer. A refusal is one line of standard error and exit 1.
pub fn leave(args: &LeaveArgs, daemon: Option<&str>) -> Result<ExitCode> {
    const WHAT: &str = "leave";
    let url = named_daemon(daemon, WHAT)?;
    let actor = crate::actor::chosen();
    if actor.kind != ActorKind::Human {
        eprintln!(
            "conductor: {WHAT}: ActorNotHuman: {}",
            crate::actor::not_human(&actor)
        );
        return Ok(ExitCode::FAILURE);
    }
    let (status, answer) = crate::remote::call(
        url,
        "POST",
        &route_of(args.instance.as_ref(), WHAT),
        Some(&json!({})),
    )
    .context(WHAT)?;
    if status != 200 {
        eprintln!("conductor: {WHAT}: refused: {}", refusal(&answer, status));
        return Ok(ExitCode::FAILURE);
    }
    println!(
        "left {}: withdrew {}; resumed on another token: {}",
        answer["instance"].as_str().unwrap_or("the host's pool"),
        Some(listed(&answer, "withdrawn"))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "no token".to_owned()),
        Some(listed(&answer, "resume"))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "no agent".to_owned()),
    );
    Ok(ExitCode::SUCCESS)
}

/// `conductor token reveal --model-token <instance>`: the value of that instance's `model_token`
/// (`story:container-start-without-prompt`), for an agent at a placement with no box that the
/// pool leased no token ([`model_helper`]); an instance whose placement opens a box is refused,
/// and nothing of its token is printed. A token of the host's pool is never printed: its value
/// reaches the harness once, in its environment at its start (`story:host-token-pool`).
///
/// # Errors
///
/// The config does not load, or standard output cannot be written. A refusal is one line of
/// standard error and exit 1, naming the specification's error where one applies; the refusal of
/// a boxed instance names its placement.
pub fn token_reveal(config: Option<&Path>, args: &TokenRevealArgs) -> Result<ExitCode> {
    const WHAT: &str = "token reveal";
    let name = args.model_token.as_deref().unwrap_or_default().trim();
    let refuse = |line: String| {
        eprintln!("conductor: {WHAT}: {line}");
        Ok(ExitCode::FAILURE)
    };
    if name.is_empty() {
        return refuse("--model-token names no instance; nothing is revealed".to_owned());
    }
    let environment = crate::config::Environment::from_process().context(WHAT)?;
    let loaded = crate::config::load(config, &environment).context(WHAT)?;
    let daemon = &loaded.config.daemon;
    let Some(instance) = loaded
        .config
        .instances
        .iter()
        .find(|instance| instance.name.0 == name)
    else {
        return refuse(format!(
            "UnknownInstance: {name}: the config file names no such instance; nothing is revealed"
        ));
    };
    // A box receives its model token sealed through a slot of the daemon's pool; nothing prints
    // it.
    if crate::placement::boxed(instance) {
        return refuse(format!(
            "instance {name}: its placement {} opens a box, which receives its model_token \
             sealed through a slot; --model-token serves an instance whose placement opens \
             none, and nothing is revealed",
            instance.placement.0
        ));
    }
    let Some(secret) = instance.model_token.clone() else {
        return refuse(format!(
            "SecretMissing: instance {name} names no model_token; nothing is revealed"
        ));
    };
    let what = format!("the model_token of instance {name}");
    let secrets = match crate::secret::open(&daemon.secrets) {
        Ok(secrets) => secrets,
        Err(unusable) => return refuse(unusable.to_string()),
    };
    let value = match secrets.reveal(&secret) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return refuse(format!(
                "SecretMissing: {what}: the secrets source holds no value for it"
            ));
        }
        Err(error) => {
            return refuse(format!(
                "SourceUnreachable: the {:?} source did not answer",
                error.source
            ));
        }
    };
    let mut out = std::io::stdout().lock();
    out.write_all(value.expose()).context(WHAT)?;
    out.write_all(b"\n").context(WHAT)?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use conductor_model::operator::ActorKind;

    fn token(
        id: &str,
        owner: &str,
        state: PoolTokenState,
        until: Option<&str>,
    ) -> PoolTokenSnapshot {
        PoolTokenSnapshot {
            state,
            data: PoolTokenData {
                token_id: TokenId(Uuid(id.to_owned())),
                owner: OperatorName(owner.to_owned()),
                seat: Seat(format!("{owner}@example-org.example")),
                secret: SecretName(format!("claude-token/{id}")),
                kind: None,
                token_state: conductor_model::token::TokenState::Ok,
                limited_until: until.map(|at| Timestamp(at.to_owned())),
                contributed_at: Timestamp("2026-11-08T09:00:00Z".to_owned()),
                actor: Actor {
                    kind: ActorKind::Human,
                    operator: Some(OperatorName(owner.to_owned())),
                    agent: None,
                },
            },
        }
    }

    fn at(text: &str) -> OffsetDateTime {
        OffsetDateTime::parse(text, &Rfc3339).expect("a time")
    }

    const NO_CAP: Caps = Caps {
        subscription_token: 0,
        api_key: 0,
    };

    #[test]
    fn the_pick_takes_the_token_with_the_fewest_live_leases_ties_by_id() {
        let tokens = [
            token("b", "alice", PoolTokenState::Available, None),
            token("a", "bob", PoolTokenState::Available, None),
            token("c", "carol", PoolTokenState::Withdrawn, None),
        ];
        let now = at("2026-11-08T10:00:00Z");
        let even = |_: &TokenId| 0;
        assert_eq!(
            pick(&tokens, &even, NO_CAP, None, now).map(|id| id.0.0),
            Some("a".to_owned())
        );
        let a_used = |id: &TokenId| u64::from(id.0.0 == "a") * 3;
        assert_eq!(
            pick(&tokens, &a_used, NO_CAP, None, now).map(|id| id.0.0),
            Some("b".to_owned())
        );
    }

    #[test]
    fn a_token_at_its_cap_is_not_picked() {
        let tokens = [token("a", "alice", PoolTokenState::Available, None)];
        let now = at("2026-11-08T10:00:00Z");
        let caps = Caps {
            subscription_token: 4,
            api_key: 2,
        };
        // A token of no kind is an API key.
        assert!(pick(&tokens, &|_| 1, caps, None, now).is_some());
        assert!(pick(&tokens, &|_| 2, caps, None, now).is_none());
    }

    #[test]
    fn a_named_owner_gets_only_their_own_token_and_no_other() {
        let tokens = [
            token(
                "a",
                "alice",
                PoolTokenState::Limited,
                Some("2026-11-08T12:00:00Z"),
            ),
            token("b", "bob", PoolTokenState::Available, None),
        ];
        let now = at("2026-11-08T10:00:00Z");
        let alice = OperatorName("alice".to_owned());
        let bob = OperatorName("bob".to_owned());
        assert_eq!(pick(&tokens, &|_| 0, NO_CAP, Some(&alice), now), None);
        assert_eq!(
            pick(&tokens, &|_| 0, NO_CAP, Some(&bob), now).map(|id| id.0.0),
            Some("b".to_owned())
        );
    }

    #[test]
    fn a_limited_token_is_not_picked_before_its_reset() {
        let tokens = [token(
            "a",
            "alice",
            PoolTokenState::Limited,
            Some("2026-11-08T12:00:00Z"),
        )];
        let before = at("2026-11-08T10:00:00Z");
        assert_eq!(pick(&tokens, &|_| 0, NO_CAP, None, before), None);
        let after = at("2026-11-08T12:00:01Z");
        assert!(pick(&tokens, &|_| 0, NO_CAP, None, after).is_some());
    }

    #[test]
    fn with_every_token_limited_the_first_reset_is_the_earliest_ahead() {
        let tokens = [
            token(
                "a",
                "alice",
                PoolTokenState::Limited,
                Some("2026-11-08T15:00:00Z"),
            ),
            token(
                "b",
                "bob",
                PoolTokenState::Limited,
                Some("2026-11-08T11:00:00Z"),
            ),
            token("c", "carol", PoolTokenState::Withdrawn, None),
        ];
        let now = at("2026-11-08T10:00:00Z");
        assert_eq!(pick(&tokens, &|_| 0, NO_CAP, None, now), None);
        assert_eq!(first_reset(&tokens, now), Some(at("2026-11-08T11:00:00Z")));
        // At the first reset the token that reset is picked, and the next reset is the other's.
        let then = at("2026-11-08T11:00:00Z");
        assert_eq!(
            pick(&tokens, &|_| 0, NO_CAP, None, then).map(|id| id.0.0),
            Some("b".to_owned())
        );
        assert_eq!(first_reset(&tokens, then), Some(at("2026-11-08T15:00:00Z")));
        assert_eq!(first_reset(&tokens[2..], now), None);
    }

    #[test]
    fn a_limit_answer_reads_as_conductord_sent_it_and_names_no_value() {
        let answer = LimitAnswer {
            limited: Some(("t-1".to_owned(), at("2026-11-08T15:00:00Z"))),
            other: false,
            first_reset: Some(at("2026-11-08T11:00:00Z")),
        };
        let sent = answer.to_json();
        assert_eq!(sent["token"], "t-1");
        assert_eq!(LimitAnswer::of_json(&sent), answer);
        // An older conductord answered `limited` and `other` only.
        let older = LimitAnswer::of_json(&json!({"limited": true, "other": true}));
        assert_eq!(
            older,
            LimitAnswer {
                limited: None,
                other: true,
                first_reset: None
            }
        );
    }

    #[test]
    fn a_limit_ends_at_most_a_week_and_an_hour_ahead() {
        let now = at("2026-11-08T10:00:00Z");
        assert_eq!(limit_end(None, now), now + LIMIT_WINDOW);
        let soon = at("2026-11-08T12:00:00Z");
        assert_eq!(limit_end(Some(soon), now), soon);
        assert_eq!(
            limit_end(Some(at("2027-11-08T10:00:00Z")), now),
            now + LIMIT_CEILING
        );
    }

    #[test]
    fn a_runtime_tag_tells_starts_apart_and_holds_no_token() {
        let tag = runtime_tag("s3cret-token");
        assert_eq!(tag.len(), 16);
        assert!(!tag.contains("s3cret"));
        assert_ne!(tag, runtime_tag("another-token"));
    }
}
