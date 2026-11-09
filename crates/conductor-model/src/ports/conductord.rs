// generated from conductor v1
// contract digest 3ad5226d4517a7ae569bdc15103168ac09f97332327ca0e0972b62aad9feb62a
// do not edit: regenerate with `ess synthesize --layout crate`

//! conductord — the `conductord` component of `conductor` v1.
//!
//! The daemon of the control host. It reconciles every instance of the config file against what runs at its placement, opens and closes each instance's box, resolves the secrets the config names, records who operates each instance and the tokens they contribute, writes the config on an accepted change, and serves the global API.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {
    /// `conductor.config.ConfigChanged`.
    ConfigChanged(crate::config::ConfigChanged),
    /// `conductor.instance.InstanceAnswered`.
    InstanceAnswered(crate::instance::InstanceAnswered),
    /// `conductor.instance.InstancePurged`.
    InstancePurged(crate::instance::InstancePurged),
    /// `conductor.instance.InstanceReconciled`.
    InstanceReconciled(crate::instance::InstanceReconciled),
    /// `conductor.instance.InstanceRestarted`.
    InstanceRestarted(crate::instance::InstanceRestarted),
    /// `conductor.instance.InstanceRolledBack`.
    InstanceRolledBack(crate::instance::InstanceRolledBack),
    /// `conductor.instance.InstanceStarted`.
    InstanceStarted(crate::instance::InstanceStarted),
    /// `conductor.instance.InstanceStopped`.
    InstanceStopped(crate::instance::InstanceStopped),
    /// `conductor.instance.InstanceUnreachable`.
    InstanceUnreachable(crate::instance::InstanceUnreachable),
    /// `conductor.instance.InstanceUpgrading`.
    InstanceUpgrading(crate::instance::InstanceUpgrading),
    /// `conductor.operator.OperatorJoined`.
    OperatorJoined(crate::operator::OperatorJoined),
    /// `conductor.operator.OperatorLeft`.
    OperatorLeft(crate::operator::OperatorLeft),
    /// `conductor.placement.BoxClosed`.
    BoxClosed(crate::placement::BoxClosed),
    /// `conductor.placement.BoxOpened`.
    BoxOpened(crate::placement::BoxOpened),
    /// `conductor.placement.BoxRefused`.
    BoxRefused(crate::placement::BoxRefused),
    /// `conductor.secret.SecretPut`.
    SecretPut(crate::secret::SecretPut),
    /// `conductor.secret.SecretsResolved`.
    SecretsResolved(crate::secret::SecretsResolved),
    /// `conductor.store.StoreMigrated`.
    StoreMigrated(crate::store::StoreMigrated),
    /// `conductor.token.TokenContributed`.
    TokenContributed(crate::token::TokenContributed),
    /// `conductor.token.TokenLeased`.
    TokenLeased(crate::token::TokenLeased),
    /// `conductor.token.TokenLimited`.
    TokenLimited(crate::token::TokenLimited),
    /// `conductor.token.TokenReleased`.
    TokenReleased(crate::token::TokenReleased),
    /// `conductor.token.TokenWithdrawn`.
    TokenWithdrawn(crate::token::TokenWithdrawn),
}

/// conductord — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct Conductord<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> Conductord<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}

impl<B> Conductord<B>
where
    B: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    /// Accepts `conductor.config.PutConfig`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn put_config(&mut self, input: crate::config::PutConfig) -> Result<crate::config::PutConfigOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.put_config(input)?;
        match &outcome {
            crate::config::PutConfigOutcome::LevelScopeMismatch { .. } => {}
            crate::config::PutConfigOutcome::ActorNotHuman { .. } => {}
            crate::config::PutConfigOutcome::Invalid { .. } => {}
            crate::config::PutConfigOutcome::Changed { config_changed, .. } => {
                self.outbox.push(PublishedEvent::ConfigChanged(config_changed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.instance.ObserveInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn observe_instance(&mut self, input: crate::instance::ObserveInstance) -> Result<crate::instance::ObserveInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.observe_instance(input)?;
        match &outcome {
            crate::instance::ObserveInstanceOutcome::Unknown { .. } => {}
            crate::instance::ObserveInstanceOutcome::Answering { instance_answered, .. } => {
                self.outbox.push(PublishedEvent::InstanceAnswered(instance_answered.clone()));
            }
            crate::instance::ObserveInstanceOutcome::NotAnswering { instance_unreachable, .. } => {
                self.outbox.push(PublishedEvent::InstanceUnreachable(instance_unreachable.clone()));
            }
            crate::instance::ObserveInstanceOutcome::NotRunning { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.instance.PurgeInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn purge_instance(&mut self, input: crate::instance::PurgeInstance) -> Result<crate::instance::PurgeInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.purge_instance(input)?;
        match &outcome {
            crate::instance::PurgeInstanceOutcome::ActorNotHuman { .. } => {}
            crate::instance::PurgeInstanceOutcome::Unknown { .. } => {}
            crate::instance::PurgeInstanceOutcome::Purged { instance_purged, .. } => {
                self.outbox.push(PublishedEvent::InstancePurged(instance_purged.clone()));
            }
            crate::instance::PurgeInstanceOutcome::NotStopped { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.instance.Reconcile`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn reconcile(&mut self, input: crate::instance::Reconcile) -> Result<crate::instance::ReconcileOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.reconcile(input)?;
        match &outcome {
            crate::instance::ReconcileOutcome::Recorded { instance_reconciled, .. } => {
                self.outbox.push(PublishedEvent::InstanceReconciled(instance_reconciled.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.instance.RestartInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn restart_instance(&mut self, input: crate::instance::RestartInstance) -> Result<crate::instance::RestartInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.restart_instance(input)?;
        match &outcome {
            crate::instance::RestartInstanceOutcome::Unknown { .. } => {}
            crate::instance::RestartInstanceOutcome::PlacementRefused { .. } => {}
            crate::instance::RestartInstanceOutcome::Restarted { instance_restarted, .. } => {
                self.outbox.push(PublishedEvent::InstanceRestarted(instance_restarted.clone()));
            }
            crate::instance::RestartInstanceOutcome::NotRunning { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.instance.StartInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn start_instance(&mut self, input: crate::instance::StartInstance) -> Result<crate::instance::StartInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.start_instance(input)?;
        match &outcome {
            crate::instance::StartInstanceOutcome::Created { instance_started, .. } => {
                self.outbox.push(PublishedEvent::InstanceStarted(instance_started.clone()));
            }
            crate::instance::StartInstanceOutcome::Started { instance_started, .. } => {
                self.outbox.push(PublishedEvent::InstanceStarted(instance_started.clone()));
            }
            crate::instance::StartInstanceOutcome::AlreadyStarted { .. } => {}
            crate::instance::StartInstanceOutcome::NotInConfig { .. } => {}
            crate::instance::StartInstanceOutcome::PlacementRefused { .. } => {}
            crate::instance::StartInstanceOutcome::NotStartable { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.instance.StopInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn stop_instance(&mut self, input: crate::instance::StopInstance) -> Result<crate::instance::StopInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.stop_instance(input)?;
        match &outcome {
            crate::instance::StopInstanceOutcome::Unknown { .. } => {}
            crate::instance::StopInstanceOutcome::Stopped { instance_stopped, .. } => {
                self.outbox.push(PublishedEvent::InstanceStopped(instance_stopped.clone()));
            }
            crate::instance::StopInstanceOutcome::AlreadyStopped { .. } => {}
            crate::instance::StopInstanceOutcome::Purged { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.instance.UpgradeInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn upgrade_instance(&mut self, input: crate::instance::UpgradeInstance) -> Result<crate::instance::UpgradeInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.upgrade_instance(input)?;
        match &outcome {
            crate::instance::UpgradeInstanceOutcome::Unknown { .. } => {}
            crate::instance::UpgradeInstanceOutcome::Upgrading { instance_upgrading, .. } => {
                self.outbox.push(PublishedEvent::InstanceUpgrading(instance_upgrading.clone()));
            }
            crate::instance::UpgradeInstanceOutcome::RolledBack { instance_rolled_back, .. } => {
                self.outbox.push(PublishedEvent::InstanceRolledBack(instance_rolled_back.clone()));
            }
            crate::instance::UpgradeInstanceOutcome::NotRunning { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.operator.JoinInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn join_instance(&mut self, input: crate::operator::JoinInstance) -> Result<crate::operator::JoinInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.join_instance(input)?;
        match &outcome {
            crate::operator::JoinInstanceOutcome::ActorNotHuman { .. } => {}
            crate::operator::JoinInstanceOutcome::Joined { operator_joined, .. } => {
                self.outbox.push(PublishedEvent::OperatorJoined(operator_joined.clone()));
            }
            crate::operator::JoinInstanceOutcome::Rejoined { operator_joined, .. } => {
                self.outbox.push(PublishedEvent::OperatorJoined(operator_joined.clone()));
            }
            crate::operator::JoinInstanceOutcome::AlreadyJoined { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.operator.LeaveInstance`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn leave_instance(&mut self, input: crate::operator::LeaveInstance) -> Result<crate::operator::LeaveInstanceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.leave_instance(input)?;
        match &outcome {
            crate::operator::LeaveInstanceOutcome::ActorNotHuman { .. } => {}
            crate::operator::LeaveInstanceOutcome::Left { operator_left, .. } => {
                self.outbox.push(PublishedEvent::OperatorLeft(operator_left.clone()));
            }
            crate::operator::LeaveInstanceOutcome::NotJoined { .. } => {}
            crate::operator::LeaveInstanceOutcome::UnknownOperator { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.placement.CloseBox`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn close_box(&mut self, input: crate::placement::CloseBox) -> Result<crate::placement::CloseBoxOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.close_box(input)?;
        match &outcome {
            crate::placement::CloseBoxOutcome::Closed { box_closed, .. } => {
                self.outbox.push(PublishedEvent::BoxClosed(box_closed.clone()));
            }
            crate::placement::CloseBoxOutcome::NoSuchBox { .. } => {}
            crate::placement::CloseBoxOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.placement.OpenBox`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn open_box(&mut self, input: crate::placement::OpenBox) -> Result<crate::placement::OpenBoxOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.open_box(input)?;
        match &outcome {
            crate::placement::OpenBoxOutcome::Refused { box_refused, .. } => {
                self.outbox.push(PublishedEvent::BoxRefused(box_refused.clone()));
            }
            crate::placement::OpenBoxOutcome::AdapterRefused { box_refused, .. } => {
                self.outbox.push(PublishedEvent::BoxRefused(box_refused.clone()));
            }
            crate::placement::OpenBoxOutcome::Opened { box_opened, .. } => {
                self.outbox.push(PublishedEvent::BoxOpened(box_opened.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.secret.PutSecret`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn put_secret(&mut self, input: crate::secret::PutSecret) -> Result<crate::secret::PutSecretOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.put_secret(input)?;
        match &outcome {
            crate::secret::PutSecretOutcome::NotHuman { .. } => {}
            crate::secret::PutSecretOutcome::Unreachable { .. } => {}
            crate::secret::PutSecretOutcome::Stored { secret_put, .. } => {
                self.outbox.push(PublishedEvent::SecretPut(secret_put.clone()));
            }
            crate::secret::PutSecretOutcome::Exists { .. } => {}
            crate::secret::PutSecretOutcome::Replaced { secret_put, .. } => {
                self.outbox.push(PublishedEvent::SecretPut(secret_put.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.secret.ResolveSecrets`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn resolve_secrets(&mut self, input: crate::secret::ResolveSecrets) -> Result<crate::secret::ResolveSecretsOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.resolve_secrets(input)?;
        match &outcome {
            crate::secret::ResolveSecretsOutcome::Unreachable { .. } => {}
            crate::secret::ResolveSecretsOutcome::Missing { .. } => {}
            crate::secret::ResolveSecretsOutcome::Resolved { secrets_resolved, .. } => {
                self.outbox.push(PublishedEvent::SecretsResolved(secrets_resolved.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.store.MigrateStore`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn migrate_store(&mut self, input: crate::store::MigrateStore) -> Result<crate::store::MigrateStoreOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.migrate_store(input)?;
        match &outcome {
            crate::store::MigrateStoreOutcome::NotHuman { .. } => {}
            crate::store::MigrateStoreOutcome::NegativeCount { .. } => {}
            crate::store::MigrateStoreOutcome::AlreadyMigrated { .. } => {}
            crate::store::MigrateStoreOutcome::SourceMissing { .. } => {}
            crate::store::MigrateStoreOutcome::Migrated { store_migrated, .. } => {
                self.outbox.push(PublishedEvent::StoreMigrated(store_migrated.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.token.ContributeToken`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn contribute_token(&mut self, input: crate::token::ContributeToken) -> Result<crate::token::ContributeTokenOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.contribute_token(input)?;
        match &outcome {
            crate::token::ContributeTokenOutcome::ActorNotHuman { .. } => {}
            crate::token::ContributeTokenOutcome::Contributed { token_contributed, .. } => {
                self.outbox.push(PublishedEvent::TokenContributed(token_contributed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.token.LeaseToken`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn lease_token(&mut self, input: crate::token::LeaseToken) -> Result<crate::token::LeaseTokenOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.lease_token(input)?;
        match &outcome {
            crate::token::LeaseTokenOutcome::NoneAvailable { .. } => {}
            crate::token::LeaseTokenOutcome::Leased { token_leased, .. } => {
                self.outbox.push(PublishedEvent::TokenLeased(token_leased.clone()));
            }
            crate::token::LeaseTokenOutcome::WrongState { .. } => {}
            crate::token::LeaseTokenOutcome::WrongStateUnknownInstance => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.token.MarkTokenLimited`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn mark_token_limited(&mut self, input: crate::token::MarkTokenLimited) -> Result<crate::token::MarkTokenLimitedOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.mark_token_limited(input)?;
        match &outcome {
            crate::token::MarkTokenLimitedOutcome::Limited { token_limited, .. } => {
                self.outbox.push(PublishedEvent::TokenLimited(token_limited.clone()));
            }
            crate::token::MarkTokenLimitedOutcome::WrongState { .. } => {}
            crate::token::MarkTokenLimitedOutcome::WrongStateUnknownInstance => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.token.ReleaseToken`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn release_token(&mut self, input: crate::token::ReleaseToken) -> Result<crate::token::ReleaseTokenOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.release_token(input)?;
        match &outcome {
            crate::token::ReleaseTokenOutcome::Released { token_released, .. } => {
                self.outbox.push(PublishedEvent::TokenReleased(token_released.clone()));
            }
            crate::token::ReleaseTokenOutcome::WrongState { .. } => {}
            crate::token::ReleaseTokenOutcome::WrongStateUnknownInstance => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.token.WithdrawToken`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn withdraw_token(&mut self, input: crate::token::WithdrawToken) -> Result<crate::token::WithdrawTokenOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.withdraw_token(input)?;
        match &outcome {
            crate::token::WithdrawTokenOutcome::ActorNotHuman { .. } => {}
            crate::token::WithdrawTokenOutcome::NotOwner { .. } => {}
            crate::token::WithdrawTokenOutcome::Withdrawn { token_withdrawn, .. } => {
                self.outbox.push(PublishedEvent::TokenWithdrawn(token_withdrawn.clone()));
            }
            crate::token::WithdrawTokenOutcome::WrongState { .. } => {}
            crate::token::WithdrawTokenOutcome::WrongStateUnknownInstance => {}
        }
        Ok(outcome)
    }

    /// Serves `conductor.config.ConfigRevisions` at `read_your_writes` consistency, from the owed projection.
    pub fn config_revisions(&self) -> Result<Vec<crate::config::ConfigRevisions>, crate::obligation::UnmetObligation> {
        self.behaviors.config_revisions()
    }

    /// Serves `conductor.config.EffectiveSettings` at `eventual` consistency, from the owed projection.
    pub fn effective_settings(&self, instance: crate::config::InstanceName) -> Result<Vec<crate::config::EffectiveSettings>, crate::obligation::UnmetObligation> {
        self.behaviors.effective_settings(instance)
    }

    /// Serves `conductor.instance.Instances` at `read_your_writes` consistency, from the owed projection.
    pub fn instances(&self) -> Result<Vec<crate::instance::Instances>, crate::obligation::UnmetObligation> {
        self.behaviors.instances()
    }

    /// Serves `conductor.instance.Reconciliations` at `read_your_writes` consistency, from the owed projection.
    pub fn reconciliations(&self) -> Result<Vec<crate::instance::Reconciliations>, crate::obligation::UnmetObligation> {
        self.behaviors.reconciliations()
    }

    /// Serves `conductor.operator.Operators` at `read_your_writes` consistency, from the owed projection.
    pub fn operators(&self) -> Result<Vec<crate::operator::Operators>, crate::obligation::UnmetObligation> {
        self.behaviors.operators()
    }

    /// Serves `conductor.placement.BoxRefusals` at `read_your_writes` consistency, from the owed projection.
    pub fn box_refusals(&self) -> Result<Vec<crate::placement::BoxRefusals>, crate::obligation::UnmetObligation> {
        self.behaviors.box_refusals()
    }

    /// Serves `conductor.placement.Boxes` at `read_your_writes` consistency, from the owed projection.
    pub fn boxes(&self) -> Result<Vec<crate::placement::Boxes>, crate::obligation::UnmetObligation> {
        self.behaviors.boxes()
    }

    /// Serves `conductor.placement.Placements` at `read_your_writes` consistency, from the owed projection.
    pub fn placements(&self) -> Result<Vec<crate::placement::Placements>, crate::obligation::UnmetObligation> {
        self.behaviors.placements()
    }

    /// Serves `conductor.secret.Secrets` at `read_your_writes` consistency, from the owed projection.
    pub fn secrets(&self) -> Result<Vec<crate::secret::Secrets>, crate::obligation::UnmetObligation> {
        self.behaviors.secrets()
    }

    /// Serves `conductor.store.Migrations` at `read_your_writes` consistency, from the owed projection.
    pub fn migrations(&self) -> Result<Vec<crate::store::Migrations>, crate::obligation::UnmetObligation> {
        self.behaviors.migrations()
    }

    /// Serves `conductor.store.StoreStatus` at `read_your_writes` consistency, from the owed projection.
    pub fn store_status(&self) -> Result<Vec<crate::store::StoreStatus>, crate::obligation::UnmetObligation> {
        self.behaviors.store_status()
    }

    /// Serves `conductor.token.Tokens` at `read_your_writes` consistency, from the owed projection.
    pub fn tokens(&self, instance: crate::config::InstanceName) -> Result<Vec<crate::token::Tokens>, crate::obligation::UnmetObligation> {
        self.behaviors.tokens(instance)
    }
}
