// generated from conductor v1
// contract digest 3ad5226d4517a7ae569bdc15103168ac09f97332327ca0e0972b62aad9feb62a
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
eprintln!("unmet startup obligations:\nassigned value: String\nassigned value: conductor.placement.RefusalReason\ncommand behaviour: conductor.instance.StartInstance\ncommand behaviour: conductor.secret.PutSecret\ncommand behaviour: conductor.secret.ResolveSecrets\ncommand behaviour: conductor.store.MigrateStore\nexternal branch answer: conductor.config.PutConfig/actor-not-human\nexternal branch answer: conductor.config.PutConfig/invalid\nexternal branch answer: conductor.instance.RestartInstance/placement-refused\nexternal branch answer: conductor.placement.OpenBox/adapter-refused\nexternal branch answer: conductor.placement.OpenBox/refused\nexternal branch answer: conductor.token.LeaseToken/none-available\nexternal branch answer: conductor.token.WithdrawToken/not-owner\nview query: conductor.config.EffectiveSettings\nview query: conductor.token.Tokens");
std::process::ExitCode::FAILURE
}
