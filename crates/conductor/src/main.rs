//! The `conductor` binary: parse the command tree, run the handler, and turn its answer into an
//! exit status. A handler that is not written yet exits 2, any other failure 1.

use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _};
use conductor_cli::cli::{Cli, leaf_of};
use conductor_cli::{NOT_IMPLEMENTED_EXIT, NotImplemented};

fn main() -> ExitCode {
    let matches = Cli::command().get_matches();
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    cli.leaf = leaf_of(&matches);
    match cli.run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("conductor: {error:#}");
            if error.is::<NotImplemented>() {
                ExitCode::from(NOT_IMPLEMENTED_EXIT)
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
