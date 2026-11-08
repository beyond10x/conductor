//! `SettingsProposalStorage` and `FrictionFindingStorage` on [`super::Store`] (`story:tuner`):
//! each `conductor.improvement.SettingsProposal` the tuner writes and each
//! `conductor.improvement.FrictionFinding` it raises for a key outside the tuning grant is one
//! stream, keyed by its identity, in the instance's store.

use conductor_model::behaviour::{FrictionFindingStorage, SettingsProposalStorage};
use conductor_model::improvement::{
    FindingId, FrictionFindingData, FrictionFindingSnapshot, FrictionFindingState, MetricId,
    MetricReadingId, ProposalId, SettingsProposalData, SettingsProposalSnapshot,
    SettingsProposalState,
};
use conductor_model::primitives::{Decimal, Timestamp};
use serde_json::{Value, json};

use super::{DELETED, STORED, Store, StoreError, text, undecodable};

/// `conductor.improvement.SettingsProposal`.
const PROPOSAL: &str = "conductor.improvement.SettingsProposal";

/// `conductor.improvement.FrictionFinding`.
const FINDING: &str = "conductor.improvement.FrictionFinding";

/// Each state of a settings proposal with its name as the specification spells it.
const PROPOSAL_STATES: [(SettingsProposalState, &str); 5] = [
    (SettingsProposalState::Proposed, "Proposed"),
    (SettingsProposalState::Applied, "Applied"),
    (SettingsProposalState::Kept, "Kept"),
    (SettingsProposalState::Reverted, "Reverted"),
    (SettingsProposalState::Refused, "Refused"),
];

/// Each state of a friction finding with its name as the specification spells it.
const FINDING_STATES: [(FrictionFindingState, &str); 4] = [
    (FrictionFindingState::Open, "Open"),
    (FrictionFindingState::Filed, "Filed"),
    (FrictionFindingState::Verified, "Verified"),
    (FrictionFindingState::Closed, "Closed"),
];

/// The name of a settings proposal's state.
#[must_use]
pub fn proposal_state_name(state: SettingsProposalState) -> &'static str {
    PROPOSAL_STATES
        .iter()
        .find(|(one, _)| *one == state)
        .map_or("Proposed", |(_, name)| name)
}

fn proposal_state_named(name: &str) -> Option<SettingsProposalState> {
    PROPOSAL_STATES
        .iter()
        .find(|(_, one)| *one == name)
        .map(|(state, _)| *state)
}

/// The name of a friction finding's state.
#[must_use]
pub fn finding_state_name(state: FrictionFindingState) -> &'static str {
    FINDING_STATES
        .iter()
        .find(|(one, _)| *one == state)
        .map_or("Open", |(_, name)| name)
}

fn finding_state_named(name: &str) -> Option<FrictionFindingState> {
    FINDING_STATES
        .iter()
        .find(|(_, one)| *one == name)
        .map(|(state, _)| *state)
}

impl SettingsProposalStorage for Store {
    fn get(&self, identity: &ProposalId) -> Option<SettingsProposalSnapshot> {
        let body = self.read(PROPOSAL, &identity.0)?;
        match decode_proposal(&body) {
            Ok(found) if found.data.proposal_id == *identity => Some(found),
            Ok(found) => {
                self.fail(undecodable(
                    PROPOSAL,
                    &identity.0,
                    format!("holds the proposal {:?}", found.data.proposal_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(PROPOSAL, &identity.0, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: SettingsProposalSnapshot) {
        let body = encode_proposal(&snapshot);
        self.write(PROPOSAL, &snapshot.data.proposal_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &ProposalId) {
        self.write(PROPOSAL, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<SettingsProposalSnapshot> {
        self.rows(PROPOSAL)
            .into_iter()
            .filter_map(|(stream, body)| match decode_proposal(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: PROPOSAL,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

impl FrictionFindingStorage for Store {
    fn get(&self, identity: &FindingId) -> Option<FrictionFindingSnapshot> {
        let body = self.read(FINDING, &identity.0)?;
        match decode_finding(&body) {
            Ok(found) if found.data.finding_id == *identity => Some(found),
            Ok(found) => {
                self.fail(undecodable(
                    FINDING,
                    &identity.0,
                    format!("holds the finding {:?}", found.data.finding_id.0),
                ));
                None
            }
            Err(detail) => {
                self.fail(undecodable(FINDING, &identity.0, detail));
                None
            }
        }
    }

    fn put(&mut self, snapshot: FrictionFindingSnapshot) {
        let body = encode_finding(&snapshot);
        self.write(FINDING, &snapshot.data.finding_id.0, STORED, body);
    }

    fn delete(&mut self, identity: &FindingId) {
        self.write(FINDING, &identity.0, DELETED, json!({}));
    }

    fn list(&self) -> Vec<FrictionFindingSnapshot> {
        self.rows(FINDING)
            .into_iter()
            .filter_map(|(stream, body)| match decode_finding(&body) {
                Ok(found) => Some(found),
                Err(detail) => {
                    self.fail(StoreError::Undecodable {
                        entity: FINDING,
                        stream,
                        detail,
                    });
                    None
                }
            })
            .collect()
    }
}

fn encode_proposal(found: &SettingsProposalSnapshot) -> Value {
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": proposal_state_name(found.state),
        "proposal_id": data.proposal_id.0,
        "instance": data.instance,
        "key": data.key,
        "old": data.old,
        "new": data.new,
        "metric_id": data.metric_id.0,
        "reading": data.reading.0,
        "baseline": data.baseline.0,
        "window_days": data.window_days,
        "proposed_at": data.proposed_at.0,
        "revision": data.revision,
        "reason": data.reason,
        "finding": data.finding.as_ref().map(|id| &id.0),
        "verdict": data.verdict,
    })
}

fn decode_proposal(body: &Value) -> Result<SettingsProposalSnapshot, String> {
    let state = text(body, "state")?;
    Ok(SettingsProposalSnapshot {
        state: proposal_state_named(&state)
            .ok_or_else(|| format!("unknown SettingsProposal state {state:?}"))?,
        data: SettingsProposalData {
            actor: crate::actor::stored(body)?,
            proposal_id: ProposalId(text(body, "proposal_id")?),
            instance: text(body, "instance")?,
            key: text(body, "key")?,
            old: text(body, "old")?,
            new: text(body, "new")?,
            metric_id: MetricId(text(body, "metric_id")?),
            reading: MetricReadingId(text(body, "reading")?),
            baseline: Decimal(text(body, "baseline")?),
            window_days: body
                .get("window_days")
                .and_then(Value::as_i64)
                .ok_or_else(|| "no integer field \"window_days\"".to_owned())?,
            proposed_at: Timestamp(text(body, "proposed_at")?),
            revision: optional_text(body, "revision")?,
            reason: optional_text(body, "reason")?,
            finding: optional_text(body, "finding")?.map(FindingId),
            verdict: optional_text(body, "verdict")?,
        },
    })
}

fn encode_finding(found: &FrictionFindingSnapshot) -> Value {
    let data = &found.data;
    json!({
        "actor": crate::actor::to_json(&data.actor),
        "state": finding_state_name(found.state),
        "finding_id": data.finding_id.0,
        "signature": data.signature,
        "evidence": data.evidence,
        "metric_id": data.metric_id.0,
        "raised_at": data.raised_at.0,
        "story": data.story,
        "verdict": data.verdict,
    })
}

fn decode_finding(body: &Value) -> Result<FrictionFindingSnapshot, String> {
    let state = text(body, "state")?;
    let evidence = match body.get("evidence") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "an evidence id that is not text".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err("no list field \"evidence\"".to_owned()),
    };
    Ok(FrictionFindingSnapshot {
        state: finding_state_named(&state)
            .ok_or_else(|| format!("unknown FrictionFinding state {state:?}"))?,
        data: FrictionFindingData {
            actor: crate::actor::stored(body)?,
            finding_id: FindingId(text(body, "finding_id")?),
            signature: text(body, "signature")?,
            evidence,
            metric_id: MetricId(text(body, "metric_id")?),
            raised_at: Timestamp(text(body, "raised_at")?),
            story: optional_text(body, "story")?,
            verdict: optional_text(body, "verdict")?,
        },
    })
}

/// A field that is text or null; an absent field reads as null.
fn optional_text(body: &Value, field: &str) -> Result<Option<String>, String> {
    match body.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!("no text or null field {field:?}")),
    }
}
