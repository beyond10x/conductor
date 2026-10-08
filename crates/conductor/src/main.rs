//! The `conductor` binary: parse the command tree, run the handler, and turn its answer into an
//! exit status. A handler that is not written yet exits 2, any other failure 1.

use std::process::ExitCode;

use clap::Parser;
use conductor_cli::cli::Cli;
use conductor_cli::{NOT_IMPLEMENTED_EXIT, NotImplemented};

fn main() -> ExitCode {
    match Cli::parse().run() {
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
