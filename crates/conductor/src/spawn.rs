//! Launching and ending a controller's session: the obligation behind
//! `controller start-controller`, and the session end behind `controller stop-controller`.
//!
//! Filled by `story:spawn`; the [`crate::controller`] handlers call these once it lands. Both
//! answer [`NotImplemented`](crate::NotImplemented) until then.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{StartControllerArgs, StopControllerArgs};
use crate::not_implemented;

/// Starts the controller session for `args.repository` and prints its session id.
///
/// # Errors
///
/// [`NotImplemented`](crate::NotImplemented) until `story:spawn` lands.
pub fn launch(_args: &StartControllerArgs) -> Result<ExitCode> {
    not_implemented("spawn launch")
}

/// Ends the controller session `args.controller_id` names.
///
/// # Errors
///
/// [`NotImplemented`](crate::NotImplemented) until `story:spawn` lands.
pub fn end(_args: &StopControllerArgs) -> Result<ExitCode> {
    not_implemented("spawn end")
}
