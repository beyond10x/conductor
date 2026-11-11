// generated from conductor v1
// contract digest 2b03183365aedd50e23f3659c527d6ee72cb8b1ac517c9e046bb013284b8a454
// do not edit: regenerate with `ess synthesize --layout crate`

//! conductor-instance — the `conductor-instance` component of `conductor` v1.
//!
//! One instance's runtime. It runs the instance's loop, starts each role slot's agent of a declared kind at its placement, delivers to it, attaches to it, records its reports and stops it, and serves the instance API.
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
    /// `conductor.agent.AgentKindDeclared`.
    AgentKindDeclared(crate::agent::AgentKindDeclared),
    /// `conductor.agent.AgentReported`.
    AgentReported(crate::agent::AgentReported),
    /// `conductor.agent.AgentStarted`.
    AgentStarted(crate::agent::AgentStarted),
    /// `conductor.agent.AgentStopped`.
    AgentStopped(crate::agent::AgentStopped),
}

/// conductor-instance — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct ConductorInstance<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> ConductorInstance<B> {
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

impl<B> ConductorInstance<B>
where
    B: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
{
    /// Accepts `conductor.agent.AttachAgent`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn attach_agent(&mut self, input: crate::agent::AttachAgent) -> Result<crate::agent::AttachAgentOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.attach_agent(input)?;
        match &outcome {
            crate::agent::AttachAgentOutcome::NotInteractive { .. } => {}
            crate::agent::AttachAgentOutcome::NotRunning { .. } => {}
            crate::agent::AttachAgentOutcome::Unreachable { .. } => {}
            crate::agent::AttachAgentOutcome::Attached { .. } => {}
            crate::agent::AttachAgentOutcome::NoSuchAgent { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.agent.DeclareAgentKind`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn declare_agent_kind(&mut self, input: crate::agent::DeclareAgentKind) -> Result<crate::agent::DeclareAgentKindOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.declare_agent_kind(input)?;
        match &outcome {
            crate::agent::DeclareAgentKindOutcome::Declared { agent_kind_declared, .. } => {
                self.outbox.push(PublishedEvent::AgentKindDeclared(agent_kind_declared.clone()));
            }
            crate::agent::DeclareAgentKindOutcome::Redeclared { agent_kind_declared, .. } => {
                self.outbox.push(PublishedEvent::AgentKindDeclared(agent_kind_declared.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.agent.DeliverToAgent`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn deliver_to_agent(&mut self, input: crate::agent::DeliverToAgent) -> Result<crate::agent::DeliverToAgentOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.deliver_to_agent(input)?;
        match &outcome {
            crate::agent::DeliverToAgentOutcome::NotInteractive { .. } => {}
            crate::agent::DeliverToAgentOutcome::NotRunning { .. } => {}
            crate::agent::DeliverToAgentOutcome::Unreachable { .. } => {}
            crate::agent::DeliverToAgentOutcome::Delivered { .. } => {}
            crate::agent::DeliverToAgentOutcome::NoSuchAgent { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.agent.RecordAgentReport`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_agent_report(&mut self, input: crate::agent::RecordAgentReport) -> Result<crate::agent::RecordAgentReportOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_agent_report(input)?;
        match &outcome {
            crate::agent::RecordAgentReportOutcome::Reported { agent_reported, .. } => {
                self.outbox.push(PublishedEvent::AgentReported(agent_reported.clone()));
            }
            crate::agent::RecordAgentReportOutcome::NoSuchAgent { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.agent.RestartAgent`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn restart_agent(&mut self, input: crate::agent::RestartAgent) -> Result<crate::agent::RestartAgentOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.restart_agent(input)?;
        match &outcome {
            crate::agent::RestartAgentOutcome::LaunchFailed { .. } => {}
            crate::agent::RestartAgentOutcome::Restarted { agent_started, .. } => {
                self.outbox.push(PublishedEvent::AgentStarted(agent_started.clone()));
            }
            crate::agent::RestartAgentOutcome::NoSuchAgent { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.agent.StartAgent`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn start_agent(&mut self, input: crate::agent::StartAgent) -> Result<crate::agent::StartAgentOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.start_agent(input)?;
        match &outcome {
            crate::agent::StartAgentOutcome::UnknownKind { .. } => {}
            crate::agent::StartAgentOutcome::LaunchFailed { .. } => {}
            crate::agent::StartAgentOutcome::Started { agent_started, .. } => {
                self.outbox.push(PublishedEvent::AgentStarted(agent_started.clone()));
            }
            crate::agent::StartAgentOutcome::Exists { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.agent.StopAgent`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn stop_agent(&mut self, input: crate::agent::StopAgent) -> Result<crate::agent::StopAgentOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.stop_agent(input)?;
        match &outcome {
            crate::agent::StopAgentOutcome::Stopped { agent_stopped, .. } => {
                self.outbox.push(PublishedEvent::AgentStopped(agent_stopped.clone()));
            }
            crate::agent::StopAgentOutcome::NoSuchAgent { .. } => {}
            crate::agent::StopAgentOutcome::NotRunning { .. } => {}
        }
        Ok(outcome)
    }

    /// Serves `conductor.agent.AgentKinds` at `read_your_writes` consistency, from the owed projection.
    pub fn agent_kinds(&self) -> Result<Vec<crate::agent::AgentKinds>, crate::obligation::UnmetObligation> {
        self.behaviors.agent_kinds()
    }

    /// Serves `conductor.agent.Agents` at `read_your_writes` consistency, from the owed projection.
    pub fn agents(&self) -> Result<Vec<crate::agent::Agents>, crate::obligation::UnmetObligation> {
        self.behaviors.agents()
    }
}
