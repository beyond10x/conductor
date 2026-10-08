#![forbid(unsafe_code)]
//! `conductor-docs`: generates the derived pages of the conductor documentation site, checks them
//! for drift, and binds a built site to its source commit. See the library for what it generates.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about = "Generate and check the derived pages of the conductor documentation site")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Regenerate website/docs/reference/, website/docs/status.md and website/data/status.json, or
    /// check them for drift.
    Generate {
        /// Write nothing; exit 1 naming each generated file that is missing, stale or edited, each
        /// file under website/docs/reference/ the generator does not write, and each place a page
        /// under website/docs/ is not passive Markdown.
        #[arg(long)]
        check: bool,
        /// The repository root the pages are generated from.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// The directory that holds website/ to write or check. Default: the repository root.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Bind a built site to its source commit: write `.well-known/b10x-site.json` and the route
    /// inventory `.well-known/b10x-routes.json` into it.
    Provenance {
        /// The built site, `website/build`.
        #[arg(long)]
        site: PathBuf,
        /// The full Git revision the site was built from.
        #[arg(long)]
        commit: String,
    },
}

fn run(cli: Cli) -> anyhow::Result<bool> {
    match cli.command {
        Action::Generate { check, root, out } => {
            let outputs = conductor_docs::outputs(&root)?;
            let out = out.unwrap_or_else(|| root.clone());
            if check {
                let problems = conductor_docs::check(&outputs, &out)?;
                if !problems.is_empty() {
                    for problem in &problems {
                        eprintln!("conductor-docs: {problem}");
                    }
                    eprintln!(
                        "conductor-docs: {} problems; run `task docs-generate` for generated files",
                        problems.len()
                    );
                    return Ok(false);
                }
                println!("{} generated files current", outputs.len());
            } else {
                conductor_docs::write(&outputs, &out)?;
                for path in outputs.keys() {
                    println!("{path}: written");
                }
            }
        }
        Action::Provenance { site, commit } => {
            conductor_docs::write_provenance(&site, &commit)?;
            println!("{}: bound to {commit}", site.display());
        }
    }
    Ok(true)
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(error) => {
            eprintln!("conductor-docs: {error:#}");
            ExitCode::from(1)
        }
    }
}
