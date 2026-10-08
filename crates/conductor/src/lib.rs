//! The `conductor` command line: the records conductor and the repository controllers keep, as
//! the specification under `spec/` declares them.
//!
//! [`cli`] is the command tree. Each group's handlers live in the module of the story that fills
//! them, and answer [`NotImplemented`] until it lands. A command opens the
//! [`store`] through [`state::open`]; a view opens it through [`state::open_existing`], which
//! creates nothing. The domain types are the generated `conductor_model` crate.

#![forbid(unsafe_code)]

use std::fmt;

pub mod cli;
pub mod collect;
pub mod config;
pub mod controller;
pub mod dashboard;
pub mod decide;
pub mod dispatch;
pub mod goal;
pub mod guard;
pub mod message;
pub mod repository;
pub mod resource;
pub mod shipped;
pub mod snapshot;
pub mod spawn;
pub mod state;
pub mod status;
pub mod store;
pub mod usage;
pub mod watch;

/// The exit status of a command whose handler has not been written yet.
pub const NOT_IMPLEMENTED_EXIT: u8 = 2;

/// The answer of a handler whose story has not landed yet; it names the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotImplemented(pub &'static str);

impl fmt::Display for NotImplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: not implemented", self.0)
    }
}

impl std::error::Error for NotImplemented {}

/// Answers [`NotImplemented`] for `what`.
///
/// # Errors
///
/// Always.
pub fn not_implemented<T>(what: &'static str) -> anyhow::Result<T> {
    Err(NotImplemented(what).into())
}
