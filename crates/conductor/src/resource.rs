//! `conductor resource`: build slots, disk and usage asked for before they are spent, and the
//! ledger of every request (design § 6).
//!
//! A command reads its input from its flags, opens the store under the state directory
//! (`--state-dir`, else `state/`; [`crate::state::open`]), runs the generated behaviour, and checks
//! that the store kept what the behaviour reported ([`Store::check`]) before it answers. An
//! accepted command exits 0; `request-resource` prints the id of the request on one line. A
//! refused command exits 1 with one line on standard error: the command, the error the
//! specification declares, and what it means. A flag left out is named on standard error (exit 1)
//! before any store is opened. `--input-json` is not read yet, and answers
//! [`NotImplemented`](crate::NotImplemented).
//!
//! The view opens only an existing store ([`crate::state::open_existing`]) and creates nothing.

use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{Generated, ResourceRequestStorage, TryContext, unmet_context};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::obligations::{
    GrantResourceBehavior, RefuseResourceBehavior, ReleaseResourceBehavior,
    RequestResourceBehavior, ResourceRequestsQuery,
};
use conductor_model::dispatch::{
    GrantResource, GrantResourceOutcome, GuardDecisionId, MessageId, RefuseResource,
    RefuseResourceOutcome, ReleaseResource, ReleaseResourceOutcome, RequestResource,
    RequestResourceOutcome, ResourceKind, ResourceRequestId, ResourceRequestSnapshot,
    ResourceRequestState,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::Value;

use crate::cli::{
    GrantResourceArgs, JsonInput, RefuseResourceArgs, ReleaseResourceArgs, RequestResourceArgs,
    ViewArgs,
};
use crate::not_implemented;
use crate::store::Store;

/// Every resource kind, for reading one by its name.
const KINDS: [ResourceKind; 3] = [
    ResourceKind::BuildSlot,
    ResourceKind::Disk,
    ResourceKind::Usage,
];

/// `conductor resource request-resource`: the command `conductor.dispatch.RequestResource`.
/// Prints the id of the request.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the request, the id cannot be printed, or the specification
/// refuses it: `InvalidAmount`.
pub fn request_resource(state: Option<&Path>, args: RequestResourceArgs) -> Result<ExitCode> {
    const COMMAND: &str = "resource request-resource";
    flags_only(args.input_json, "resource request-resource --input-json")?;
    let resource = flag(COMMAND, "--resource", args.resource)?;
    let input = RequestResource {
        repository: RepositoryName(flag(COMMAND, "--repository", args.repository)?),
        resource: KINDS
            .into_iter()
            .find(|kind| kind_name(*kind) == resource)
            .ok_or_else(|| anyhow!("{COMMAND}: --resource {resource:?} is no ResourceKind"))?,
        amount: flag(COMMAND, "--amount", args.amount)?,
        requested_at: Timestamp(flag(COMMAND, "--requested-at", args.requested_at)?),
    };
    let mut generated = Generated::new(Ports {
        store: crate::state::open(state)?,
    });
    let outcome = generated
        .request_resource(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports.store)?;
    match outcome {
        RequestResourceOutcome::Requested { resource_requested } => {
            print_line(COMMAND, &resource_requested.resource_request_id.0.0)
        }
        RequestResourceOutcome::InvalidAmount { error } => refused(
            COMMAND,
            "InvalidAmount",
            &format!(
                "the amount {} is not positive, so nothing was requested",
                error.amount
            ),
        ),
    }
}

/// `conductor resource grant-resource`: the command `conductor.dispatch.GrantResource`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DiskBelowFloor`,
/// `ResourceRequestNotFound`, `ResourceRequestStateConflict`.
pub fn grant_resource(state: Option<&Path>, args: GrantResourceArgs) -> Result<ExitCode> {
    const COMMAND: &str = "resource grant-resource";
    flags_only(args.input_json, "resource grant-resource --input-json")?;
    let input = GrantResource {
        resource_request_id: request_id(COMMAND, args.resource_request_id)?,
        disk_free_bytes: flag(COMMAND, "--disk-free-bytes", args.disk_free_bytes)?,
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .grant_resource(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        GrantResourceOutcome::Granted { .. } => Ok(ExitCode::SUCCESS),
        GrantResourceOutcome::BelowDiskFloor { error } => refused(
            COMMAND,
            "DiskBelowFloor",
            &format!(
                "{} bytes free is under the 30G floor, so build slot {} was not granted; the \
                 request stays open",
                error.disk_free_bytes, error.resource_request_id.0.0
            ),
        ),
        GrantResourceOutcome::NoSuchRequest { error } => refused(
            COMMAND,
            "ResourceRequestNotFound",
            &no_request(&error.resource_request_id),
        ),
        GrantResourceOutcome::WrongState { error } => refused(
            COMMAND,
            "ResourceRequestStateConflict",
            &answered(&error.resource_request_id),
        ),
    }
}

/// `conductor resource refuse-resource`: the command `conductor.dispatch.RefuseResource`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it:
/// `ResourceRequestNotFound`, `ResourceRequestStateConflict`.
pub fn refuse_resource(state: Option<&Path>, args: RefuseResourceArgs) -> Result<ExitCode> {
    const COMMAND: &str = "resource refuse-resource";
    flags_only(args.input_json, "resource refuse-resource --input-json")?;
    let input = RefuseResource {
        resource_request_id: request_id(COMMAND, args.resource_request_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .refuse_resource(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        RefuseResourceOutcome::Refused { .. } => Ok(ExitCode::SUCCESS),
        RefuseResourceOutcome::NoSuchRequest { error } => refused(
            COMMAND,
            "ResourceRequestNotFound",
            &no_request(&error.resource_request_id),
        ),
        RefuseResourceOutcome::WrongState { error } => refused(
            COMMAND,
            "ResourceRequestStateConflict",
            &answered(&error.resource_request_id),
        ),
    }
}

/// `conductor resource release-resource`: the command `conductor.dispatch.ReleaseResource`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it:
/// `ResourceRequestNotFound`, `ResourceRequestStateConflict`.
pub fn release_resource(state: Option<&Path>, args: ReleaseResourceArgs) -> Result<ExitCode> {
    const COMMAND: &str = "resource release-resource";
    flags_only(args.input_json, "resource release-resource --input-json")?;
    let input = ReleaseResource {
        resource_request_id: request_id(COMMAND, args.resource_request_id)?,
    };
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = generated
        .release_resource(input)
        .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
    kept(COMMAND, &generated.ports)?;
    match outcome {
        ReleaseResourceOutcome::Released { .. } => Ok(ExitCode::SUCCESS),
        ReleaseResourceOutcome::NoSuchRequest { error } => refused(
            COMMAND,
            "ResourceRequestNotFound",
            &no_request(&error.resource_request_id),
        ),
        ReleaseResourceOutcome::WrongState { error } => refused(
            COMMAND,
            "ResourceRequestStateConflict",
            &format!(
                "resource request {} is not granted, and only a granted request is released",
                error.resource_request_id.0.0
            ),
        ),
    }
}

/// `conductor resource resource-requests`: the view `conductor.dispatch.ResourceRequests`, the
/// ledger, served by the generated query over the existing store under the state directory
/// (`--state-dir`, else `state/`; [`crate::state::dir`]) and rendered as `--format` asks. A view
/// creates no store.
///
/// Lists every request it can read, and names each record it cannot on standard error, after
/// them, exiting 1 ([`crate::store::show`]).
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn resource_requests(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/dispatch.yaml`).
    const COLUMNS: [&str; 6] = [
        "resource_request_id",
        "state",
        "repository",
        "resource",
        "amount",
        "requested_at",
    ];

    /// The state's name as the specification spells it. Exhaustive, so a state the specification
    /// adds fails to compile here rather than rendering under a guessed name.
    fn state_name(state: ResourceRequestState) -> &'static str {
        match state {
            ResourceRequestState::Granted => "Granted",
            ResourceRequestState::Refused => "Refused",
            ResourceRequestState::Released => "Released",
            ResourceRequestState::Requested => "Requested",
        }
    }

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .resource_requests()
        .map_err(|unmet| anyhow!("resource resource-requests: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.resource_request_id.0.0),
                Value::from(state_name(row.state)),
                Value::from(row.repository.0),
                Value::from(kind_name(row.resource)),
                Value::from(row.amount),
                Value::from(row.requested_at.0),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "resource requests",
        &view.render(&COLUMNS, &rows),
    )
}

/// The kind's name as the specification spells it. Exhaustive, so a kind the specification adds
/// fails to compile here rather than being read or rendered under a guessed name.
fn kind_name(kind: ResourceKind) -> &'static str {
    match kind {
        ResourceKind::BuildSlot => "BuildSlot",
        ResourceKind::Disk => "Disk",
        ResourceKind::Usage => "Usage",
    }
}

/// The ports `RequestResource` runs over: the store, and the request id it assigns.
struct Ports {
    store: Store,
}

impl ResourceRequestStorage for Ports {
    fn get(&self, identity: &ResourceRequestId) -> Option<ResourceRequestSnapshot> {
        ResourceRequestStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: ResourceRequestSnapshot) {
        ResourceRequestStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &ResourceRequestId) {
        ResourceRequestStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<ResourceRequestSnapshot> {
        ResourceRequestStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Ok(ResourceRequestId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }
}

/// Refuses `--input-json`, which no handler reads yet, as `what` not implemented.
fn flags_only(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(_) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of the flag `name`, or an error naming it.
fn flag<T>(command: &str, name: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| anyhow!("{command}: {name} is missing"))
}

/// `--resource-request-id`, which the command line has already read as a UUID.
fn request_id(command: &str, value: Option<String>) -> Result<ResourceRequestId> {
    Ok(ResourceRequestId(Uuid(flag(
        command,
        "--resource-request-id",
        value,
    )?)))
}

/// Whether the store kept every read and write the behaviour made; an outcome is answered only
/// then.
fn kept(command: &str, store: &Store) -> Result<()> {
    store
        .check()
        .with_context(|| format!("{command}: the store did not keep it"))
}

/// The refusal `error` of `command`, as the one line the binary writes on standard error.
fn refused(command: &str, error: &str, detail: &str) -> Result<ExitCode> {
    bail!("{command}: {error}: {detail}")
}

/// Writes `line` to standard output.
fn print_line(command: &str, line: &str) -> Result<ExitCode> {
    writeln!(std::io::stdout().lock(), "{line}")
        .with_context(|| format!("{command}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}

fn no_request(request: &ResourceRequestId) -> String {
    format!("no resource request has the id {}", request.0.0)
}

fn answered(request: &ResourceRequestId) -> String {
    format!("resource request {} was already answered", request.0.0)
}
