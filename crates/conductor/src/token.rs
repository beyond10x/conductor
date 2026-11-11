//! The token pool of an instance (`story:token-pool`, design architecture-next § 5.3, § 17,
//! decisions 10 and 11): an engineer who joins an instance contributes their Claude token, and
//! the agents of the `claude-code` kind lease from the pool.
//!
//! - The pool is `conductor.token.PoolToken` records in the daemon's tenant of the deployment's
//!   store, each with its owner, seat, state and the name of the secret that holds its value; the
//!   value is in the daemon's secrets source only ([`contribute`]).
//! - [`pick`] is the lease rule: a named owner's own tokens only, else every token of the
//!   instance (decision K4: no fallback); among them the Available token leased the fewest times,
//!   ties by `token_id` (decision K3). A Limited token whose `limited_until` is past counts as
//!   Available; with `headroom` (the account has headroom again, today's `auto-resume`) a
//!   Limited one is taken last, the earliest `limited_until` first.
//! - The `claude-code` kind leases one token per agent start ([`lease_for_start`]) and runs the
//!   agent on it: its settings name `conductor token reveal --token <id>` as their
//!   `apiKeyHelper` ([`apply`]), so the value reaches the harness from the secrets source when it
//!   asks, and never a config, an environment, an argument, a log or a record. A role that names
//!   `token_owner` (the controller only) leases that owner's token and starts with
//!   `--remote-control`, so it appears in that owner's Claude app (§ 17).
//! - A usage-limit stop marks the agent's token Limited for [`LIMIT_WINDOW`] ([`limit`]); the
//!   loop resumes the agent at once when another token is Available, else, as today, once the
//!   account has headroom. A stop releases the token ([`release`]).
//! - `conductor join` and `conductor leave` ([`join`], [`leave`]) are clients of the daemon's
//!   `POST /v1/instances/{name}/join` and `leave` ([`join_route`], [`leave_route`]); `GET
//!   /v1/instances/{name}/tokens` ([`rows`]) lists owners, seats and states, never a value nor
//!   the secret's name. Only a token's owner withdraws it (decision K5); each agent on a
//!   withdrawn token is named in the instance's `<state>/run/tokens-withdrawn.json`, and the
//!   loop ends it and resumes it on another token.
//!
//! With no token in the pool (no daemon store, or nothing contributed) every start is today's,
//! byte for byte.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::agent::AgentName;
use conductor_model::behaviour::{
    ExternalCommand, Generated, PoolTokenStorage, TryContext, unmet_context,
};
use conductor_model::config::{DaemonOperator, Instance, InstanceName, OperatorGrant, Role};
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
    ReleaseTokenBehavior as _, WithdrawTokenBehavior as _,
};
use conductor_model::token::{
    ContributeToken, ContributeTokenOutcome, LeaseToken, LeaseTokenOutcome, MarkTokenLimited,
    MarkTokenLimitedOutcome, NoTokenAvailable, PoolTokenData, PoolTokenSnapshot, PoolTokenState,
    ReleaseToken, ReleaseTokenOutcome, Seat, TokenId, WithdrawToken, WithdrawTokenOutcome,
};
use secrets_core::storage::SecretValue;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{JoinArgs, LeaveArgs, TokenRevealArgs};
use crate::confine::Launch;
use crate::store::Store;

/// How long a token stays Limited after a usage-limit stop: Claude's usage window.
pub const LIMIT_WINDOW: time::Duration = time::Duration::hours(5);

/// The file under the instance's `<state>/run/` naming the agents on a withdrawn token, which
/// the loop resumes on another.
pub const WITHDRAWN_FILE: &str = "tokens-withdrawn.json";

/// The settings key the harness asks for its credential with.
pub const HELPER_KEY: &str = "apiKeyHelper";

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

// ---------------------------------------------------------------------------------------------
// The pool
// ---------------------------------------------------------------------------------------------

/// The token an agent runs on: its id, owner and seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lease {
    /// The token.
    pub token_id: TokenId,
    /// Its owner, whom the agent's record names.
    pub owner: OperatorName,
    /// Its seat.
    pub seat: Seat,
}

fn instant(at: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(at, &Rfc3339).ok()
}

fn now() -> Result<Timestamp> {
    crate::records::now()
}

/// The token the lease rule picks for an agent of `instance` (module documentation), given how
/// many times each was leased; `None` when no token of the pool is admitted.
#[must_use]
pub fn pick(
    tokens: &[PoolTokenSnapshot],
    leases: &dyn Fn(&TokenId) -> u64,
    instance: &InstanceName,
    owner: Option<&OperatorName>,
    now: OffsetDateTime,
    headroom: bool,
) -> Option<TokenId> {
    let past = |token: &PoolTokenSnapshot| {
        token
            .data
            .limited_until
            .as_ref()
            .and_then(|at| instant(&at.0))
            .is_none_or(|until| until <= now)
    };
    let mut admitted: Vec<(u8, String, u64, String, TokenId)> = tokens
        .iter()
        .filter(|token| token.data.instance == *instance)
        .filter(|token| owner.is_none_or(|owner| token.data.owner == *owner))
        .filter_map(|token| {
            let class = match token.state {
                PoolTokenState::Available => 0,
                PoolTokenState::Limited if past(token) => 0,
                PoolTokenState::Limited if headroom => 1,
                _ => return None,
            };
            let until = if class == 1 {
                token
                    .data
                    .limited_until
                    .as_ref()
                    .map(|at| at.0.clone())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            Some((
                class,
                until,
                leases(&token.data.token_id),
                token.data.token_id.0.0.clone(),
                token.data.token_id.clone(),
            ))
        })
        .collect();
    admitted.sort_by(|a, b| (a.0, &a.1, a.2, &a.3).cmp(&(b.0, &b.1, b.2, &b.3)));
    admitted.into_iter().next().map(|(.., id)| id)
}

/// The tokens of `instance`'s pool that are not withdrawn.
#[must_use]
pub fn pool(store: &Store, instance: &InstanceName) -> Vec<PoolTokenSnapshot> {
    store
        .pool_tokens()
        .into_iter()
        .filter(|token| {
            token.data.instance == *instance && token.state != PoolTokenState::Withdrawn
        })
        .collect()
}

/// The token `agent` holds a lease on, if any.
#[must_use]
pub fn held_by(store: &Store, agent: &AgentName) -> Option<PoolTokenSnapshot> {
    store.pool_tokens().into_iter().find(|token| {
        token.state == PoolTokenState::Leased && token.data.agent.as_ref() == Some(agent)
    })
}

/// `LeaseToken` for `agent` of `instance` on the token [`pick`] picks: an agent that holds a
/// lease already keeps it.
///
/// # Errors
///
/// The store cannot be written, or the generated behaviour cannot run.
pub fn lease(
    store: Store,
    instance: &InstanceName,
    agent: &AgentName,
    owner: Option<&OperatorName>,
    actor: Actor,
    headroom: bool,
) -> Result<Result<Lease, NoTokenAvailable>> {
    if let Some(held) = held_by(&store, agent) {
        return Ok(Ok(Lease {
            token_id: held.data.token_id,
            owner: held.data.owner,
            seat: held.data.seat,
        }));
    }
    let tokens = store.pool_tokens();
    let Some(token_id) = pick(
        &tokens,
        &|id| store.token_leases(id),
        instance,
        owner,
        OffsetDateTime::now_utc(),
        headroom,
    ) else {
        return Ok(Err(NoTokenAvailable {
            instance: instance.clone(),
        }));
    };
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .lease_token(LeaseToken {
            actor,
            instance: instance.clone(),
            token_id,
            agent: agent.clone(),
            preferred_owner: owner.cloned(),
            leased_at: now()?,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the lease: {error}"))?;
    match outcome {
        LeaseTokenOutcome::Leased { token_leased } => Ok(Ok(Lease {
            token_id: token_leased.token_id,
            owner: token_leased.owner,
            seat: token_leased.seat,
        })),
        LeaseTokenOutcome::NoneAvailable { error } => Ok(Err(error)),
        LeaseTokenOutcome::WrongState { error } => {
            bail!("TokenStateConflict: token {}", error.token_id.0.0)
        }
        LeaseTokenOutcome::WrongStateUnknownInstance => bail!("the picked token is gone"),
    }
}

/// `ReleaseToken` of the token `agent` holds, if any; answers it.
///
/// # Errors
///
/// The store cannot be written.
pub fn release(store: Store, agent: &AgentName, actor: Actor) -> Result<Option<TokenId>> {
    let Some(held) = held_by(&store, agent) else {
        return Ok(None);
    };
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .release_token(ReleaseToken {
            actor,
            token_id: held.data.token_id.clone(),
            released_at: now()?,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the release: {error}"))?;
    Ok(matches!(outcome, ReleaseTokenOutcome::Released { .. }).then_some(held.data.token_id))
}

/// `MarkTokenLimited` of the token `agent` holds, until [`LIMIT_WINDOW`] from now; answers the
/// token as it was.
///
/// # Errors
///
/// The store cannot be written.
pub fn limit(store: Store, agent: &AgentName, actor: Actor) -> Result<Option<PoolTokenData>> {
    let Some(held) = held_by(&store, agent) else {
        return Ok(None);
    };
    let now_at = OffsetDateTime::now_utc();
    let until = (now_at + LIMIT_WINDOW)
        .format(&Rfc3339)
        .context("format the end of a usage limit")?;
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .mark_token_limited(MarkTokenLimited {
            actor,
            token_id: held.data.token_id.clone(),
            limited_until: Timestamp(until),
            limited_at: now()?,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the limit: {error}"))?;
    Ok(matches!(outcome, MarkTokenLimitedOutcome::Limited { .. }).then_some(held.data))
}

/// `WithdrawToken` of `token_id` by `actor`; answers the outcome and the agent the token held.
///
/// # Errors
///
/// The store cannot be written.
pub fn withdraw(
    store: Store,
    token_id: &TokenId,
    actor: Actor,
) -> Result<(WithdrawTokenOutcome, Option<AgentName>)> {
    let agent = PoolTokenStorage::get(&store, token_id).and_then(|token| token.data.agent);
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .withdraw_token(WithdrawToken {
            actor,
            token_id: token_id.clone(),
            withdrawn_at: now()?,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the withdrawal: {error}"))?;
    let agent = agent.filter(|_| matches!(outcome, WithdrawTokenOutcome::Withdrawn { .. }));
    Ok((outcome, agent))
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

/// `ContributeToken`: `value` into the secrets source under a name of its own, then the token in
/// the pool, Available and Ok, with its owner and seat. Answers the outcome, or why the source
/// refused the value.
///
/// # Errors
///
/// The store cannot be written.
pub fn contribute(
    mut store: Store,
    secrets: &crate::secret::Secrets,
    actor: Actor,
    instance: &InstanceName,
    owner: &OperatorName,
    seat: &Seat,
    value: SecretValue,
) -> Result<Result<ContributeTokenOutcome, String>> {
    let at = now()?;
    if actor.kind != ActorKind::Human {
        return Ok(Err(format!(
            "ActorNotHuman: {}",
            crate::actor::not_human(&actor)
        )));
    }
    let name = SecretName(format!(
        "claude-token/{}/{}",
        segment(&instance.0),
        segment(&eventlog_core::new_event_id())
    ));
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
    let mut generated = Generated::new(Ports { store });
    let outcome = generated
        .contribute_token(ContributeToken {
            actor,
            instance: instance.clone(),
            owner: owner.clone(),
            seat: seat.clone(),
            secret: name,
            contributed_at: at,
        })
        .map_err(unmet)?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the token: {error}"))?;
    Ok(Ok(outcome))
}

/// One row of `GET /v1/instances/{name}/tokens` (`conductor.token.Tokens`): never a value, nor
/// the name of the secret that holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenRow {
    /// The token's id.
    pub token_id: String,
    /// `Available`, `Leased`, `Limited` or `Withdrawn`.
    pub state: &'static str,
    /// The instance.
    pub instance: String,
    /// Its owner.
    pub owner: String,
    /// Its seat.
    pub seat: String,
    /// `Ok` or `Limited`.
    pub token_state: &'static str,
    /// Until when it is limited.
    pub limited_until: Option<String>,
    /// The agent on it.
    pub agent: Option<String>,
    /// When it was contributed.
    pub contributed_at: String,
    /// Who last moved it.
    pub actor: String,
}

/// The rows of `instance`'s pool, withdrawn tokens included, by contribution time.
#[must_use]
pub fn rows(store: &Store, instance: &InstanceName) -> Vec<TokenRow> {
    let mut rows: Vec<TokenRow> = store
        .pool_tokens()
        .into_iter()
        .filter(|token| token.data.instance == *instance)
        .map(|token| {
            let data = token.data;
            TokenRow {
                token_id: data.token_id.0.0,
                state: crate::store::token_state_name(token.state),
                instance: data.instance.0,
                owner: data.owner.0,
                seat: data.seat.0,
                token_state: crate::store::usage_name(data.token_state),
                limited_until: data.limited_until.map(|at| at.0),
                agent: data.agent.map(|agent| agent.0),
                contributed_at: data.contributed_at.0,
                actor: crate::actor::written(&data.actor),
            }
        })
        .collect();
    rows.sort_by(|a, b| (&a.contributed_at, &a.token_id).cmp(&(&b.contributed_at, &b.token_id)));
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

/// The daemon's store as the runtime of this process sees it: none without a config file or a
/// database.
#[must_use]
pub fn runtime_store() -> Option<Store> {
    existing_store(&daemon_config()?.store.0)
}

/// The lease of the agent `agent` of `instance` at its start (module documentation): `None`
/// when the instance's pool holds no token, so the start is today's; the role's `token_owner`'s
/// own token when it names one. `headroom`: the account has headroom again after a usage limit.
///
/// # Errors
///
/// `NoTokenAvailable`, or the store cannot be written; one line.
pub fn lease_for_start(
    instance: &Instance,
    role: &Role,
    agent: &str,
    headroom: bool,
) -> Result<Option<Lease>, String> {
    let Some(store) = runtime_store() else {
        return Ok(None);
    };
    if pool(&store, &instance.name).is_empty() {
        return Ok(None);
    }
    let owner = role.token_owner.as_ref();
    let actor = crate::actor::agent(instance, agent);
    match lease(
        store,
        &instance.name,
        &AgentName(agent.to_owned()),
        owner,
        actor,
        headroom,
    ) {
        Ok(Ok(lease)) => Ok(Some(lease)),
        Ok(Err(error)) => Err(format!(
            "NoTokenAvailable: instance {}: no token of its pool is Available{}; nothing was \
             started",
            error.instance.0,
            owner
                .map(|owner| format!(" for its token_owner {}", owner.0))
                .unwrap_or_default()
        )),
        Err(error) => Err(format!("{error:#}")),
    }
}

/// Releases the token `agent` holds, when the runtime's store is there.
///
/// # Errors
///
/// The store cannot be written.
pub fn release_for_stop(instance: &Instance, agent: &str) -> Result<()> {
    if let Some(store) = runtime_store() {
        release(
            store,
            &AgentName(agent.to_owned()),
            crate::actor::agent(instance, agent),
        )?;
    }
    Ok(())
}

/// The owner of the token `agent` runs on, when it runs on one.
#[must_use]
pub fn owner_of(agent: &AgentName) -> Option<OperatorName> {
    held_by(&runtime_store()?, agent).map(|token| token.data.owner)
}

/// `text` as one shell word.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The `apiKeyHelper` that hands the harness the value of `token_id`: this binary's `token
/// reveal`, with the config file `config` when there is one. Names the token, never its value.
#[must_use]
pub fn helper(program: &Path, config: Option<&str>, token_id: &TokenId) -> String {
    reveal_words(program, config, "--token", &token_id.0.0)
}

/// The `apiKeyHelper` that hands the harness the `model_token` of `instance`
/// (`story:container-start-without-prompt`): this binary's `token reveal --model-token`, with
/// the config file `config` when there is one. Names the instance, never the value.
#[must_use]
pub fn model_helper(program: &Path, config: Option<&str>, instance: &InstanceName) -> String {
    reveal_words(program, config, "--model-token", &instance.0)
}

fn reveal_words(program: &Path, config: Option<&str>, flag: &str, value: &str) -> String {
    let mut words = vec![quoted(&program.display().to_string())];
    if let Some(config) = config {
        words.extend(["--config".to_owned(), quoted(config)]);
    }
    words.extend([
        "token".to_owned(),
        "reveal".to_owned(),
        flag.to_owned(),
        quoted(value),
    ]);
    words.join(" ")
}

/// Puts the leased token into `launch` (module documentation): its `--settings` file, read as
/// it is, is written again to `<state>/sessions/<session name>.settings.json` with
/// [`HELPER_KEY`] naming [`helper`], and the launch starts with that copy; `remote_control` adds
/// `--remote-control <session name>`. Answers the settings file written.
///
/// # Errors
///
/// The settings are no JSON object, or the copy cannot be written.
pub fn apply(launch: &mut Launch, lease: &Lease, remote_control: bool) -> Result<PathBuf> {
    apply_helper(
        launch,
        |program, config| helper(program, config, &lease.token_id),
        remote_control,
    )
}

/// [`apply`] with the helper `helper` builds from this binary and the config file.
fn apply_helper(
    launch: &mut Launch,
    helper: impl FnOnce(&Path, Option<&str>) -> String,
    remote_control: bool,
) -> Result<PathBuf> {
    let name = launch.request.session_name.0.clone();
    let config = launch
        .env
        .iter()
        .find(|(key, _)| key == crate::config::CONFIG_VARIABLE)
        .map(|(_, value)| value.clone());
    let program = std::env::current_exe().context("the token's helper: read the binary")?;
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
    object.insert(
        HELPER_KEY.to_owned(),
        Value::String(helper(&program, config.as_deref())),
    );
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

/// Leases a token for the launch of `role`'s agent and puts it into `launch` ([`apply`]); a role
/// that names `token_owner` starts for Remote Control. Answers the lease, `None` with an empty
/// pool. With an empty pool the launch is left as it is, but for an instance whose placement
/// opens no box and that names a `model_token`, where the config file says `daemon.unattended`
/// (`story:container-start-without-prompt`): its settings then name [`model_helper`], so the
/// agent runs on that token. Without that key an empty pool leaves every launch as it is, so an
/// agent on a workstation keeps the harness's own login.
///
/// # Errors
///
/// `NoTokenAvailable`, or the settings cannot be written; one line.
pub fn lease_into(
    instance: &Instance,
    role: &Role,
    launch: &mut Launch,
    headroom: bool,
) -> Result<Option<Lease>, String> {
    let name = launch.request.session_name.0.clone();
    let Some(lease) = lease_for_start(instance, role, &name, headroom)? else {
        if instance.model_token.is_some()
            && !crate::placement::boxed(instance)
            && daemon_config().is_some_and(|daemon| daemon.unattended)
        {
            apply_helper(
                launch,
                |program, config| model_helper(program, config, &instance.name),
                false,
            )
            .map_err(|error| format!("{error:#}"))?;
        }
        return Ok(None);
    };
    if let Err(error) = apply(launch, &lease, role.token_owner.is_some()) {
        let _ = release_for_stop(instance, &name);
        return Err(format!("{error:#}"));
    }
    Ok(Some(lease))
}

/// What the loop does on a usage-limit stop of `session` (module documentation): marks its
/// token Limited; answers whether another token of the pool is Available, so the session is
/// resumed at once rather than once the account has headroom.
///
/// # Errors
///
/// The store cannot be written.
pub fn on_limit(instance: &Instance, session: &str) -> Result<bool> {
    let Some(store) = runtime_store() else {
        return Ok(false);
    };
    let agent = AgentName(session.to_owned());
    if held_by(&store, &agent).is_none() {
        return Ok(false);
    }
    limit(store, &agent, crate::actor::agent(instance, session))?;
    let Some(store) = runtime_store() else {
        return Ok(false);
    };
    let tokens = store.pool_tokens();
    Ok(pick(
        &tokens,
        &|id| store.token_leases(id),
        &instance.name,
        None,
        OffsetDateTime::now_utc(),
        false,
    )
    .is_some())
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

/// The body of `POST /v1/instances/{name}/join`.
#[derive(Deserialize)]
struct JoinBody {
    seat: Option<String>,
    token: String,
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

/// The config file's instance `name`, and its secrets source.
fn instance_of(
    daemon: &Daemon<'_>,
    name: &str,
) -> Result<Option<(Instance, conductor_model::config::SecretsSource)>> {
    let environment = crate::config::Environment::from_process()?;
    let loaded = crate::config::load(Some(daemon.file), &environment)?;
    let secrets = loaded.config.daemon.secrets.clone();
    Ok(loaded
        .config
        .instances
        .into_iter()
        .find(|instance| instance.name.0 == name)
        .map(|instance| (instance, secrets)))
}

fn open(daemon: &Daemon<'_>) -> Result<Store> {
    if let Ok(crate::store::Location::Sqlite(path)) = crate::store::Location::parse(daemon.store)
        && let Some(parent) = path.parent()
    {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    crate::store::open_url(daemon.store, crate::daemon::TENANT)
        .map_err(|error| anyhow!("daemon.store: {error:#}"))
}

/// `GET /v1/instances/{name}/tokens`: the rows of its pool ([`rows`]).
#[must_use]
pub fn tokens_route(daemon: &Daemon<'_>, name: &str) -> Answer {
    let Some(store) = existing_store(daemon.store) else {
        return (200, "OK", json!([]));
    };
    (
        200,
        "OK",
        json!(rows(&store, &InstanceName(name.to_owned()))),
    )
}

/// `POST /v1/instances/{name}/join` as `operator`, a person the listener admitted with the
/// `Join` grant (`implied`: the loopback operator while `daemon.operators` is empty, who holds
/// every grant): `JoinInstance` with the grants `daemon.operators` gives the operator, then
/// `ContributeToken` with the body's token. The answer names the token, never its value.
#[must_use]
pub fn join_route(
    daemon: &Daemon<'_>,
    name: &str,
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
    let (instance, source) = match instance_of(daemon, name) {
        Ok(Some(found)) => found,
        Ok(None) => {
            return (
                404,
                "Not Found",
                json!({"error": format!("the config file holds no instance {name}")}),
            );
        }
        Err(error) => return failed(&error),
    };
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
    let secrets = match crate::secret::open(&source) {
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
    let store = match open(daemon) {
        Ok(store) => store,
        Err(error) => return failed(&error),
    };
    let key = OperatorKey {
        operator: OperatorName(operator.to_owned()),
        instance: instance.name.clone(),
    };
    let mut generated = Generated::new(store);
    let joined = match generated.join_instance(JoinInstance {
        actor: actor.clone(),
        key,
        grants: grants.clone(),
        joined_at: at,
    }) {
        Ok(joined) => joined,
        Err(unmet_obligation) => return failed(&unmet(unmet_obligation)),
    };
    match joined {
        JoinInstanceOutcome::Joined { .. }
        | JoinInstanceOutcome::Rejoined { .. }
        | JoinInstanceOutcome::AlreadyJoined { .. } => {}
        JoinInstanceOutcome::ActorNotHuman { .. } => {
            return (
                403,
                "Forbidden",
                json!({"error": "conductor.operator.ActorNotHuman"}),
            );
        }
    }
    if let Err(error) = generated.ports.check() {
        return failed(&anyhow!("record the operator: {error}"));
    }
    let seat = Seat(request.seat.unwrap_or_else(|| operator.to_owned()));
    let owner = OperatorName(operator.to_owned());
    match contribute(
        generated.ports,
        &secrets,
        actor,
        &instance.name,
        &owner,
        &seat,
        value,
    ) {
        Ok(Ok(ContributeTokenOutcome::Contributed { token_contributed })) => {
            println!(
                "conductord: {} joined {} with grants {}; token {} contributed, seat {}",
                owner.0,
                instance.name.0,
                grants
                    .iter()
                    .map(|grant| crate::store::grant_name(*grant))
                    .collect::<Vec<_>>()
                    .join(","),
                token_contributed.token_id.0.0,
                seat.0
            );
            (
                200,
                "OK",
                json!({
                    "instance": instance.name.0,
                    "operator": owner.0,
                    "grants": grants.iter().map(|grant| crate::store::grant_name(*grant)).collect::<Vec<_>>(),
                    "token_id": token_contributed.token_id.0.0,
                    "seat": seat.0,
                    "state": "Available",
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

/// `POST /v1/instances/{name}/leave` as `operator`: `LeaveInstance`, then `WithdrawToken` of each
/// token of the instance's pool the operator owns; each agent on one is named for the loop to
/// resume on another token ([`WITHDRAWN_FILE`]).
#[must_use]
pub fn leave_route(daemon: &Daemon<'_>, name: &str, operator: &str) -> Answer {
    let instance = match instance_of(daemon, name) {
        Ok(Some((instance, _))) => instance,
        Ok(None) => {
            return (
                404,
                "Not Found",
                json!({"error": format!("the config file holds no instance {name}")}),
            );
        }
        Err(error) => return failed(&error),
    };
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
    let mut generated = Generated::new(store);
    let left = match generated.leave_instance(LeaveInstance {
        actor: actor.clone(),
        key: OperatorKey {
            operator: owner.clone(),
            instance: instance.name.clone(),
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
    let mut store = generated.ports;
    if let Err(error) = store.check() {
        return failed(&anyhow!("record the leave: {error}"));
    }
    let owned: Vec<TokenId> = pool(&store, &instance.name)
        .into_iter()
        .filter(|token| token.data.owner == owner)
        .map(|token| token.data.token_id)
        .collect();
    let mut withdrawn = Vec::new();
    let mut resume = Vec::new();
    for token_id in owned {
        match withdraw(store, &token_id, actor.clone()) {
            Ok((WithdrawTokenOutcome::Withdrawn { .. }, agent)) => {
                withdrawn.push(token_id.0.0.clone());
                resume.extend(agent.map(|agent| agent.0));
            }
            Ok(_) => {}
            Err(error) => return failed(&error),
        }
        store = match open(daemon) {
            Ok(store) => store,
            Err(error) => return failed(&error),
        };
    }
    if let Err(error) = note_withdrawn(Path::new(&instance.state), &resume) {
        return failed(&error);
    }
    let was_operator = !matches!(
        left,
        LeaveInstanceOutcome::NotJoined { .. } | LeaveInstanceOutcome::UnknownOperator { .. }
    );
    if !was_operator && withdrawn.is_empty() {
        return (
            409,
            "Conflict",
            json!({"error": format!(
                "conductor.operator.NotJoined: {operator} is no operator of {name} and owns no token of its pool"
            )}),
        );
    }
    println!(
        "conductord: {} left {}; {} token(s) withdrawn; {} agent(s) to resume on another token",
        owner.0,
        instance.name.0,
        withdrawn.len(),
        resume.len()
    );
    (
        200,
        "OK",
        json!({
            "instance": instance.name.0,
            "operator": owner.0,
            "withdrawn": withdrawn,
            "resume": resume,
        }),
    )
}

// ---------------------------------------------------------------------------------------------
// The CLI
// ---------------------------------------------------------------------------------------------

fn required<'a>(value: Option<&'a String>, command: &str, flag: &str) -> Result<&'a str> {
    value
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("{command}: --{flag} is required"))
}

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

/// `conductor join --daemon <url> --instance <i> [--seat <s>]`, the token on standard input
/// (module documentation); `daemon` is the global `--daemon` or `CONDUCTOR_DAEMON`.
///
/// # Errors
///
/// A flag is missing, standard input holds no value, or the daemon does not answer. A refusal is
/// one line of standard error and exit 1.
pub fn join(args: &JoinArgs, daemon: Option<&str>) -> Result<ExitCode> {
    const WHAT: &str = "join";
    let url = named_daemon(daemon, WHAT)?;
    let instance = required(args.instance.as_ref(), WHAT, "instance")?;
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
    let body = json!({"seat": args.seat, "token": token});
    drop(token);
    let (status, answer) = crate::remote::call(
        url,
        "POST",
        &format!("/v1/instances/{instance}/join"),
        Some(&body),
    )
    .context(WHAT)?;
    drop(body);
    if status != 200 {
        eprintln!("conductor: {WHAT}: refused: {}", refusal(&answer, status));
        return Ok(ExitCode::FAILURE);
    }
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "joined {} as {} (grants {}): token {} in its pool, seat {}, {}",
        answer["instance"].as_str().unwrap_or(instance),
        answer["operator"].as_str().unwrap_or_default(),
        answer["grants"]
            .as_array()
            .map(|grants| {
                grants
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default(),
        answer["token_id"].as_str().unwrap_or_default(),
        answer["seat"].as_str().unwrap_or_default(),
        answer["state"].as_str().unwrap_or_default(),
    )
    .context(WHAT)?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor leave --daemon <url> --instance <i>` (module documentation); `daemon` is the
/// global `--daemon` or `CONDUCTOR_DAEMON`.
///
/// # Errors
///
/// A flag is missing, or the daemon does not answer. A refusal is one line of standard error and
/// exit 1.
pub fn leave(args: &LeaveArgs, daemon: Option<&str>) -> Result<ExitCode> {
    const WHAT: &str = "leave";
    let url = named_daemon(daemon, WHAT)?;
    let instance = required(args.instance.as_ref(), WHAT, "instance")?;
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
        &format!("/v1/instances/{instance}/leave"),
        Some(&json!({})),
    )
    .context(WHAT)?;
    if status != 200 {
        eprintln!("conductor: {WHAT}: refused: {}", refusal(&answer, status));
        return Ok(ExitCode::FAILURE);
    }
    let list = |key: &str| {
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
    };
    println!(
        "left {}: withdrew {}; resumed on another token: {}",
        answer["instance"].as_str().unwrap_or(instance),
        Some(list("withdrawn"))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "no token".to_owned()),
        Some(list("resume"))
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| "no agent".to_owned()),
    );
    Ok(ExitCode::SUCCESS)
}

/// `conductor token reveal --token <id>`: the value of a leased token on standard output, for
/// the harness whose settings name it as their `apiKeyHelper`. With `--model-token <instance>`
/// instead, the value of that instance's `model_token` (`story:container-start-without-prompt`),
/// for an agent at a placement with no box that the pool leased no token ([`model_helper`]); an
/// instance whose placement opens a box is refused, and nothing of its token is printed.
///
/// # Errors
///
/// Neither or both of `--token` and `--model-token`, the config does not load, or standard
/// output cannot be written. A refusal is one line of standard error and exit 1, naming the
/// specification's error where one applies; the refusal of a boxed instance names its placement.
pub fn token_reveal(config: Option<&Path>, args: &TokenRevealArgs) -> Result<ExitCode> {
    const WHAT: &str = "token reveal";
    let model = args
        .model_token
        .as_deref()
        .filter(|name| !name.trim().is_empty());
    if model.is_some() && args.token.is_some() {
        bail!("{WHAT}: give --token or --model-token, not both");
    }
    let id = match model {
        Some(_) => "",
        None => required(args.token.as_ref(), WHAT, "token")
            .map_err(|_| anyhow!("{WHAT}: --token or --model-token is required"))?,
    };
    let environment = crate::config::Environment::from_process().context(WHAT)?;
    let loaded = crate::config::load(config, &environment).context(WHAT)?;
    let daemon = &loaded.config.daemon;
    let refuse = |line: String| {
        eprintln!("conductor: {WHAT}: {line}");
        Ok(ExitCode::FAILURE)
    };
    let (what, secret) = if let Some(name) = model {
        let Some(instance) = loaded
            .config
            .instances
            .iter()
            .find(|instance| instance.name.0 == name)
        else {
            return refuse(format!(
                "UnknownInstance: {name}: the config file names no such instance; nothing is \
                 revealed"
            ));
        };
        // A box receives its model token sealed through a slot of the daemon's pool; nothing
        // prints it.
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
        (format!("the model_token of instance {name}"), secret)
    } else {
        let Some(store) = existing_store(&daemon.store.0) else {
            return refuse(format!(
                "TokenStateConflict: token {id}: no pool is on record"
            ));
        };
        let token_id = TokenId(Uuid(id.to_owned()));
        let Some(token) = PoolTokenStorage::get(&store, &token_id) else {
            return refuse(format!(
                "TokenStateConflict: token {id}: it is not in the pool"
            ));
        };
        if token.state != PoolTokenState::Leased {
            return refuse(format!(
                "TokenStateConflict: token {id}: it is {}, not Leased; nothing is revealed",
                crate::store::token_state_name(token.state)
            ));
        }
        (format!("token {id}"), token.data.secret)
    };
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
                instance: InstanceName("example-org".to_owned()),
                owner: OperatorName(owner.to_owned()),
                seat: Seat(format!("{owner}@example-org.example")),
                secret: SecretName(format!("claude-token/example-org/{id}")),
                token_state: conductor_model::token::TokenState::Ok,
                limited_until: until.map(|at| Timestamp(at.to_owned())),
                agent: None,
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

    #[test]
    fn the_pick_takes_the_least_leased_available_token_ties_by_id() {
        let tokens = [
            token("b", "alice", PoolTokenState::Available, None),
            token("a", "bob", PoolTokenState::Available, None),
            token("c", "carol", PoolTokenState::Leased, None),
        ];
        let instance = InstanceName("example-org".to_owned());
        let now = at("2026-11-08T10:00:00Z");
        let even = |_: &TokenId| 0;
        assert_eq!(
            pick(&tokens, &even, &instance, None, now, false).map(|id| id.0.0),
            Some("a".to_owned())
        );
        let a_used = |id: &TokenId| u64::from(id.0.0 == "a") * 3;
        assert_eq!(
            pick(&tokens, &a_used, &instance, None, now, false).map(|id| id.0.0),
            Some("b".to_owned())
        );
    }

    #[test]
    fn a_named_owner_gets_only_their_own_token_and_no_other() {
        let tokens = [
            token("a", "alice", PoolTokenState::Leased, None),
            token("b", "bob", PoolTokenState::Available, None),
        ];
        let instance = InstanceName("example-org".to_owned());
        let now = at("2026-11-08T10:00:00Z");
        let alice = OperatorName("alice".to_owned());
        let bob = OperatorName("bob".to_owned());
        assert_eq!(
            pick(&tokens, &|_| 0, &instance, Some(&alice), now, false),
            None
        );
        assert_eq!(
            pick(&tokens, &|_| 0, &instance, Some(&bob), now, false).map(|id| id.0.0),
            Some("b".to_owned())
        );
    }

    #[test]
    fn a_limited_token_is_taken_once_past_or_on_headroom_last() {
        let tokens = [token(
            "a",
            "alice",
            PoolTokenState::Limited,
            Some("2026-11-08T12:00:00Z"),
        )];
        let instance = InstanceName("example-org".to_owned());
        assert_eq!(
            pick(
                &tokens,
                &|_| 0,
                &instance,
                None,
                at("2026-11-08T10:00:00Z"),
                false
            ),
            None
        );
        assert!(
            pick(
                &tokens,
                &|_| 0,
                &instance,
                None,
                at("2026-11-08T10:00:00Z"),
                true
            )
            .is_some()
        );
        assert!(
            pick(
                &tokens,
                &|_| 0,
                &instance,
                None,
                at("2026-11-08T12:00:01Z"),
                false
            )
            .is_some()
        );
    }

    #[test]
    fn the_helper_names_the_token_and_never_a_value() {
        let id = TokenId(Uuid("0190-a".to_owned()));
        let line = helper(Path::new("/bin/conductor"), Some("/etc/c.yaml"), &id);
        assert_eq!(
            line,
            "'/bin/conductor' --config '/etc/c.yaml' token reveal --token '0190-a'"
        );
    }
}
