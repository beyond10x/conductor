//! `daemon.unattended: true` (`story:container-start-without-prompt`): a daemon started with no
//! terminal and no person, as in a container, does at its start what `conductor setup --apply`
//! does for each instance of its config file, each step only where what it makes is missing:
//!
//! - the records skeleton, written and committed as `conductor init` does, where the records
//!   directory holds no `NORTHSTAR.md`, and committed as it lies where it is there in a
//!   repository with no commit yet;
//! - the checkouts root, created where it is not there;
//! - the trust of each checkout and of the records directory in `~/.claude.json`, for the paths
//!   not trusted yet, as `conductor trust` marks them;
//! - the `worktree` workspace of the checkouts root, activated where `worktree` finds it not
//!   activated, as `setup --apply` does.
//!
//! What a person or a session changed since is kept: no step writes what is there. A step that
//! fails is said in one line naming the instance, and the daemon starts anyway, serving and
//! reconciling as it would; its `/readyz` stays `503` ([`super::metrics::SEEDING_FAILED`]) for as
//! long as the process runs, and doctor names what is left. The next start tries each step again.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use conductor_model::config::Instance;

use crate::cli::InitArgs;
use crate::config::{self, Environment};
use crate::trust::{self, CLAUDE_JSON};

/// What a refusal of a step starts with.
const WHAT: &str = "conductor: daemon serve: unattended";

/// Seeds each instance of the config file `file`, as the module documentation says. Answers
/// whether every step of every instance succeeded; each one that failed is one line of standard
/// error.
pub fn seed(file: &Path, home: &Path) -> bool {
    let loaded = match Environment::from_process()
        .and_then(|environment| config::load(Some(file), &environment))
    {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("{WHAT}: {}", one_line(&error));
            return false;
        }
    };
    let mut seeded = true;
    for (index, instance) in loaded.config.instances.iter().enumerate() {
        let name = &instance.name.0;
        let steps = [
            records(file, instance),
            root(instance),
            trusted(home, instance, index),
            crate::setup::activate_root(&loaded, instance),
        ];
        for error in steps.into_iter().filter_map(Result::err) {
            eprintln!("{WHAT}: {name}: {}", one_line(&error));
            seeded = false;
        }
    }
    seeded
}

/// `error` and its causes on one line: a cause that spans lines, as a program's standard error
/// does, is joined with spaces.
fn one_line(error: &anyhow::Error) -> String {
    format!("{error:#}")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// The records skeleton of `instance`, where its records directory holds no `NORTHSTAR.md`; and
/// its commit, where the skeleton is there and its repository has no commit yet, as when the
/// commit of an earlier start failed ([`crate::init::commit_lying`]). A failed commit is said in
/// one line with git's reason, at every start that finds it so.
fn records(file: &Path, instance: &Instance) -> Result<()> {
    let records = Path::new(&instance.records);
    let northstar = records.join("NORTHSTAR.md");
    if northstar.symlink_metadata().is_err() {
        let args = InitArgs {
            instance: Some(instance.name.0.clone()),
        };
        let made = crate::init::init(Some(file), &args).map(|_| ());
        // `init` wrote the skeleton and could not commit it: the commit below says why.
        if made.is_ok() || northstar.symlink_metadata().is_err() {
            return made;
        }
    }
    crate::init::commit_lying(records).map(|_| ())
}

/// The checkouts root of `instance`, created where it is not there.
fn root(instance: &Instance) -> Result<()> {
    let root = Path::new(&instance.checkouts.root);
    if root.exists() {
        return Ok(());
    }
    fs::create_dir_all(root).with_context(|| format!("create {}", root.display()))?;
    println!("created: {}", root.display());
    Ok(())
}

/// The trust of `instance`'s checkouts and records directory, for each path not trusted yet.
fn trusted(home: &Path, instance: &Instance, index: usize) -> Result<()> {
    let paths = trust::paths(instance, index)?;
    let file = home.join(CLAUDE_JSON);
    let mut edit = trust::Edit::read(&file)?;
    let missing: Vec<String> = paths
        .into_iter()
        .filter(|path| !edit.trusts(path))
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    for path in &missing {
        edit.trust(path);
    }
    edit.write()?;
    for path in &missing {
        println!("trusted: {path}");
    }
    Ok(())
}
