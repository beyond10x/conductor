// generated from conductor v1
// model digest 90be4e2c6c23f286962f8f65af3ffd942d3f17e3bd9e0cba69fb2f3c58630472
// contract digest d909f3155a8d82e6f61f9b63b21eb3a57fb7230ca37cf7a9932e5762eeb17f5a
// do not edit: regenerate with `ess synthesize --layout crate`

//! conductor-broker — the `conductor-broker` component of `conductor` v1.
//!
//! The broker of one instance. It serves each session of the instance on that session's own socket under the instance's state directory, checks every call against the session's grant, runs the verb (a read; a build through the host ledger; a commit through the bot route with the instance's Gates policy; a team message or a report one level up, recorded as a message), and records every call, run or refused, as a tool call in the instance's store.
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
    /// `conductor.tool.ToolCallRecorded`.
    ToolCallRecorded(crate::tool::ToolCallRecorded),
}

/// conductor-broker — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct ConductorBroker<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> ConductorBroker<B> {
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

impl<B> ConductorBroker<B>
where
    B: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
{
    /// Accepts `conductor.tool.RecordToolCall`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_tool_call(&mut self, input: crate::tool::RecordToolCall) -> Result<crate::tool::RecordToolCallOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_tool_call(input)?;
        match &outcome {
            crate::tool::RecordToolCallOutcome::Recorded { tool_call_recorded, .. } => {
                self.outbox.push(PublishedEvent::ToolCallRecorded(tool_call_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `conductor.tool.ToolCalls` at `read_your_writes` consistency, from the owed projection.
    pub fn tool_calls(&self) -> Result<Vec<crate::tool::ToolCalls>, crate::obligation::UnmetObligation> {
        self.behaviors.tool_calls()
    }
}
