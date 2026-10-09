//! The one way a test builds a command for the built `conductor` binary: [`conductor`], isolated
//! from the real home. `tests/isolated_home.rs` fails on a test file that names the binary any
//! other way.

use std::path::Path;
use std::process::Command;

use conductor_cli::config::{CONFIG_VARIABLE, INSTANCE_VARIABLE};

/// The built binary, with `HOME` naming `home` and neither `CONDUCTOR_CONFIG` nor
/// `CONDUCTOR_INSTANCE` set, so no config file of the real home, and no variable of the shell
/// the tests run in, decides what it reads. A case that wants a config file sets
/// `CONDUCTOR_CONFIG` on the command afterwards; one that clears the environment sets `HOME`
/// again after `env_clear`. A case that needs only the binary's path, to put it on a `PATH`,
/// reads it as the command's program.
pub fn conductor(home: impl AsRef<Path>) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_conductor"));
    command
        .env("HOME", home.as_ref())
        .env_remove(CONFIG_VARIABLE)
        .env_remove(INSTANCE_VARIABLE);
    command
}
