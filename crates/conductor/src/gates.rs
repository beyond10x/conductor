//! The Gates hooks of a checkout (`story:gates-hooks-fresh`): which `b10x-gates` a checkout's
//! hooks run, and which one is on `PATH`.
//!
//! `b10x-gates install` puts its hooks in `<checkout>/.git/b10x-gates-hooks/`, each a copy of the
//! `b10x-gates` binary that installed them, beside one named `b10x-gates`; `--version` on it
//! prints `b10x-gates <version>`. [`installed`] reads that version, [`version`] the one a command
//! (`b10x-gates --version` on `PATH`) prints. A hook whose version differs from the one on `PATH`
//! is stale ([`stale`]): the repositories collector records both versions, the board names such a
//! repository `stale gates hooks`, the watch counts them, and `conductor repository refresh-hooks`
//! installs the hooks afresh.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use anyhow::Result;

use crate::collect;

/// The program, on `PATH`, and the name of the hook in a checkout's hooks directory.
pub const PROGRAM: &str = "b10x-gates";

/// A checkout's hooks directory, relative to the checkout.
pub const HOOKS_DIR: &str = ".git/b10x-gates-hooks";

/// The version recorded for a hook that is there but prints no version.
pub const UNREAD: &str = "unread";

/// How long `--version` may run.
pub const BOUND: Duration = Duration::from_secs(10);

/// The hook `b10x-gates` in `checkout`'s hooks directory.
#[must_use]
pub fn hook(checkout: &Path) -> PathBuf {
    checkout.join(HOOKS_DIR).join(PROGRAM)
}

/// The version the Gates hook of `checkout` prints: none when the checkout has no hook, [`UNREAD`]
/// when it has one that prints no version within `bound`.
///
/// # Errors
///
/// The collector's budget is spent ([`collect::is_over_budget`]).
pub fn installed(checkout: &Path, bound: Duration) -> Result<Option<String>> {
    let hook = hook(checkout);
    if !hook.is_file() {
        return Ok(None);
    }
    let command = [hook.to_string_lossy().into_owned(), "--version".to_owned()];
    Ok(Some(
        version(&command, bound)?.unwrap_or_else(|| UNREAD.to_owned()),
    ))
}

/// The version `command` prints, as `b10x-gates --version` does (`b10x-gates <version>`): the last
/// word of its first line. None when it does not start, fails, or prints nothing.
///
/// # Errors
///
/// The collector's budget is spent ([`collect::is_over_budget`]).
pub fn version(command: &[String], bound: Duration) -> Result<Option<String>> {
    let Some((program, arguments)) = command.split_first() else {
        return Ok(None);
    };
    let output = match collect::run(Command::new(program).args(arguments), bound) {
        Ok(output) => output,
        Err(error) if collect::is_over_budget(&error) => return Err(error),
        Err(_) => return Ok(None),
    };
    if !output.status.success() {
        return Ok(None);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().last())
        .map(str::to_owned))
}

/// Whether a hook of version `installed` is stale against the `b10x-gates` of version `current`
/// on `PATH`: both are known and differ. A checkout without a hook, or a host without
/// `b10x-gates`, has none that is stale.
#[must_use]
pub fn stale(installed: Option<&str>, current: Option<&str>) -> bool {
    matches!((installed, current), (Some(installed), Some(current)) if installed != current)
}
