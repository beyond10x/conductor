//! `conductor snapshot publish-board`: `STATUS.md` from the newest complete snapshot, and the
//! `snapshot boards` view.
//!
//! Filled by `story:status-board`. Every handler answers
//! [`NotImplemented`](crate::NotImplemented) until then.

use std::process::ExitCode;

use anyhow::Result;

use crate::cli::{PublishBoardArgs, ViewArgs};
use crate::not_implemented;

/// `conductor snapshot publish-board`: the command `conductor.observation.PublishBoard`.
///
/// # Errors
///
/// [`NotImplemented`](crate::NotImplemented) until `story:status-board` lands.
pub fn publish_board(_args: PublishBoardArgs) -> Result<ExitCode> {
    not_implemented("snapshot publish-board")
}

/// `conductor snapshot boards`: the view `conductor.observation.Boards`.
///
/// # Errors
///
/// [`NotImplemented`](crate::NotImplemented) until `story:status-board` lands.
pub fn boards(_view: ViewArgs) -> Result<ExitCode> {
    not_implemented("snapshot boards")
}
