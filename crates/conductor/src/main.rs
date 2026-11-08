//! The `conductor` binary: parse the command tree, run the handler, and turn its answer into an
//! exit status. A handler that is not written yet exits 2, any other failure 1.

use std::io::IsTerminal as _;
use std::process::ExitCode;

use clap::{CommandFactory as _, FromArgMatches as _};
use conductor_cli::cli::{Cli, actor_of, leaf_of};
use conductor_cli::{NOT_IMPLEMENTED_EXIT, NotImplemented};

fn main() -> ExitCode {
    // `conductor` with no arguments at a terminal opens the terminal UI (`story:tui-readonly`);
    // anywhere else it answers as clap does, with the commands it takes.
    let bare = std::env::args_os().len() == 1
        && std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal();
    let matches = if bare {
        Cli::command().get_matches_from(["conductor", "tui"])
    } else {
        Cli::command().get_matches()
    };
    let mut cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    cli.leaf = leaf_of(&matches);
    cli.actor = actor_of(&matches);
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
