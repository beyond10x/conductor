// generated from conductor v1
// contract digest 87114b906e7caa0ed7e4b7971f214d3afd0390d8d014449d58b0c6bb611e9266
// do not edit: regenerate with `ess synthesize --layout crate`

//! Ephemeral generated server. Durable storage and production authentication remain ports.
use clap::Parser;

#[derive(Parser)]
struct Options {
#[arg(long, default_value = "127.0.0.1:8080")]
listen: String,
#[arg(long, default_value = "none", value_parser = ["none", "actor-header"])]
callers: String,
#[arg(long = "static")]
static_root: Option<std::path::PathBuf>,
}

fn main() -> std::process::ExitCode {
let options = Options::parse();
let _ = options;
eprintln!("unmet startup obligations:\ncommand behaviour: conductor.agent.AttachAgent\ncommand behaviour: conductor.agent.DeliverToAgent\ncommand behaviour: conductor.agent.StartAgent\nexternal branch answer: conductor.agent.RestartAgent/launch-failed");
std::process::ExitCode::FAILURE
}
