//! `tool`: see the library's documentation.

#![forbid(unsafe_code)]

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use conductor_broker::{SOCKET_VARIABLE, call};
use tool::cli::Cli;
use tool::mcp;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Some(socket) = cli.socket else {
        println!(
            "UnknownSession: no socket: {SOCKET_VARIABLE} is not set and no --socket names one"
        );
        return ExitCode::from(1);
    };
    match cli.command.request() {
        Some(request) => {
            let answer = call(&socket, &request);
            let mut out = io::stdout().lock();
            let _ = out.write_all(answer.text.as_bytes());
            let _ = out.flush();
            ExitCode::from(u8::try_from(answer.exit).unwrap_or(1))
        }
        None => serve(&socket),
    }
}

fn serve(socket: &Path) -> ExitCode {
    match mcp::serve(socket, io::stdin().lock(), io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("tool mcp: {error}");
            ExitCode::from(1)
        }
    }
}
