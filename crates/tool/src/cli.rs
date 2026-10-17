//! The `tool` command tree: the binding `crates/tool/cli.yaml`, in clap derive form. Its words are
//! the verbs of the read, build, version-control and team groups and `mcp`; nothing else parses.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use conductor_broker::{Request, SOCKET_VARIABLE};
use serde_json::{Map, Value, json};

#[derive(Debug, Parser)]
#[command(
    name = "tool",
    version,
    about = "The verbs of a session's grant, each answered by the instance's broker over the session's socket"
)]
pub struct Cli {
    /// The session's socket to the instance's broker; the broker names it in the session's
    /// environment when the session starts
    #[arg(long, global = true, env = SOCKET_VARIABLE, value_name = "SOCKET")]
    pub socket: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Verb,
}

#[derive(Debug, Subcommand)]
pub enum Verb {
    /// Print a bounded range of one file of the grant, or that it is unchanged since this session last read it
    Show {
        /// The file, relative to the session's working directory
        path: String,
        /// The lines a:b, 1-based, both ends included; either end may be left out
        #[arg(long, conflicts_with = "symbol")]
        lines: Option<String>,
        /// The item that names this symbol, from its line to the end of its block
        #[arg(long = "fn", value_name = "SYMBOL")]
        symbol: Option<String>,
        /// The most bytes of the answer, at most the grant's limit
        #[arg(long)]
        max: Option<u64>,
    },
    /// Print the lines under the grant's paths that hold a text
    Search {
        /// The text, matched as written
        pattern: String,
        /// The file or directory to search under, relative to the session's working directory
        #[arg(long)]
        path: Option<String>,
    },
    /// Print the uncommitted change of the session's checkout, as a stat (the default, or --stat), or with --patch the change itself
    Diff {
        /// A stat of the change (the default)
        #[arg(long, conflicts_with = "patch")]
        stat: bool,
        /// The change itself, bounded as every answer is
        #[arg(long)]
        patch: bool,
    },
    /// Run the tests of the session's checkout through the host ledger at nice 19; print `ok test <n> passed`, or the failing tests and the log
    Test {
        /// The Cargo package to test (cargo's -p); none, the working directory's own
        package: Option<String>,
        /// Only the tests whose name holds this text
        #[arg(long)]
        filter: Option<String>,
    },
    /// Lint the session's checkout (clippy over every target, warnings denied) through the host ledger at nice 19; print `ok check`, or the errors and the log
    Check {
        /// The Cargo package to lint (cargo's -p); none, the working directory's own
        package: Option<String>,
    },
    /// Format the session's checkout through the host ledger at nice 19; print `ok fmt <n> files changed`
    Fmt {
        /// The Cargo package to format (cargo's -p); none, the working directory's own
        package: Option<String>,
    },
    /// Stage the named paths, new ones included, and commit them through the bot route; print `<sha> <n> files`
    Commit {
        /// The commit message
        #[arg(short = 'm', long = "message")]
        message: String,
        /// The paths to commit, each relative to the session's working directory and under the grant's paths
        #[arg(last = true, required = true, value_name = "PATHS")]
        paths: Vec<String>,
    },
    /// Send a message to a session of the caller's team, by its name or its role; the broker records it and answers whom to deliver it to
    Send {
        /// The recipient: a session of the caller's team, by its session name or its role
        to: String,
        /// The message
        text: String,
    },
    /// Report the caller's status one level up, moving its open dispatch when it is a lead; the broker records it and answers whom to deliver it to
    Report {
        /// What the report says of the caller's work
        #[arg(value_parser = ["started", "progress", "blocked", "unblocked", "done", "failed"])]
        status: String,
        /// One line: the reason, the evidence or the progress
        line: String,
        /// The session the report goes to; it must be the one one level above the caller
        #[arg(long)]
        to: Option<String>,
    },
    /// Serve the granted verbs as MCP tools over stdio, each answered through the same socket
    Mcp,
}

impl Verb {
    /// The call this verb sends, or `None` for `mcp`.
    #[must_use]
    pub fn request(&self) -> Option<Request> {
        let (verb, args) = match self {
            Self::Show {
                path,
                lines,
                symbol,
                max,
            } => {
                let mut args = Map::new();
                args.insert("path".to_owned(), json!(path));
                if let Some(lines) = lines {
                    args.insert("lines".to_owned(), json!(lines));
                }
                if let Some(symbol) = symbol {
                    args.insert("symbol".to_owned(), json!(symbol));
                }
                if let Some(max) = max {
                    args.insert("max".to_owned(), json!(max));
                }
                ("show", Value::Object(args))
            }
            Self::Search { pattern, path } => {
                let mut args = Map::new();
                args.insert("pattern".to_owned(), json!(pattern));
                if let Some(path) = path {
                    args.insert("path".to_owned(), json!(path));
                }
                ("search", Value::Object(args))
            }
            Self::Diff { stat, patch } => ("diff", json!({"stat": stat, "patch": patch})),
            Self::Test { package, filter } => {
                let mut args = Map::new();
                if let Some(package) = package {
                    args.insert("package".to_owned(), json!(package));
                }
                if let Some(filter) = filter {
                    args.insert("filter".to_owned(), json!(filter));
                }
                ("test", Value::Object(args))
            }
            Self::Check { package } => ("check", package_args(package.as_deref())),
            Self::Fmt { package } => ("fmt", package_args(package.as_deref())),
            Self::Commit { message, paths } => {
                ("commit", json!({"message": message, "paths": paths}))
            }
            Self::Send { to, text } => ("send", json!({"to": to, "text": text})),
            Self::Report { status, line, to } => {
                let mut args = Map::new();
                args.insert("status".to_owned(), json!(status));
                args.insert("line".to_owned(), json!(line));
                if let Some(to) = to {
                    args.insert("to".to_owned(), json!(to));
                }
                ("report", Value::Object(args))
            }
            Self::Mcp => return None,
        };
        Some(Request {
            verb: verb.to_owned(),
            args,
        })
    }
}

/// The arguments of a build verb that names at most a package.
fn package_args(package: Option<&str>) -> Value {
    match package {
        Some(package) => json!({"package": package}),
        None => json!({}),
    }
}
