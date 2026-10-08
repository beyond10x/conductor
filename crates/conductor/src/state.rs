//! Where the binary keeps its records: the state directory, whose `tree/` and `observations/` are
//! conductor's store.
//!
//! The directory is the `--state-dir` flag when it is given; else, when a config file names the
//! instance this process runs ([`config::active`]), that instance's `state`; else `state/` under
//! the working directory, as before there was a config file. The flag's name is `globals.state`
//! of the `ess-cli/1` binding `crates/conductor/cli.yaml`. It exists for `conductor guard`, which
//! runs as a controller's hook with the controller's checkout as its working directory, while its
//! verdicts belong in conductor's own store (design § 7, § 10).
//!
//! [`dir`] is the only place the directory is decided, and every handler that opens the store does
//! so through one of two functions: a command through [`open`], which creates the store where none
//! is, and a view through [`open_existing`], which never creates anything.

use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

use crate::config;
use crate::store::{self, Store};

/// The state directory's name under the working directory, when neither a flag nor a config file
/// names one.
const DEFAULT: &str = "state";

/// The state directory: `flag` when `--state-dir` was given; else the `state` of the instance a
/// config file names; else `state/` under the working directory.
///
/// # Errors
///
/// Neither a flag nor a config file names the directory, and the working directory cannot be
/// read.
pub fn dir(flag: Option<&Path>) -> Result<PathBuf> {
    if let Some(dir) = flag {
        return Ok(dir.to_owned());
    }
    let active = config::active();
    if active.from_file {
        return Ok(PathBuf::from(&active.instance.state));
    }
    Ok(env::current_dir()
        .context("read the working directory to find state/ under it")?
        .join(DEFAULT))
}

/// For a command: opens the store under the state directory [`dir`] decides, creating it when it
/// is not there.
///
/// # Errors
///
/// [`dir`] fails, or [`store::open`] does.
pub fn open(flag: Option<&Path>) -> Result<Store> {
    store::open(&dir(flag)?)
}

/// For a view: opens the store under the state directory [`dir`] decides only when it is there,
/// and creates nothing, so "no records" and "no store here" do not look alike.
///
/// # Errors
///
/// [`dir`] fails, or [`store::open_existing`] does: there is no store, or it does not open.
pub fn open_existing(flag: Option<&Path>) -> Result<Store> {
    store::open_existing(&dir(flag)?)
}
