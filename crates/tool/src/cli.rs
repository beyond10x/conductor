//! The `tool` command tree: the binding `crates/tool/cli.yaml`, in clap derive form. Its words are
//! the verbs of the read group and `mcp`; nothing else parses.

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
            Self::Mcp => return None,
        };
        Some(Request {
            verb: verb.to_owned(),
            args,
        })
    }
}
