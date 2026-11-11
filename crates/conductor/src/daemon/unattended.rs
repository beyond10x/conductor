//! `daemon.unattended: true` (`story:container-start-without-prompt`): a daemon started with no
//! terminal and no person, as in a container, does at its start what `conductor setup --apply`
//! does for each instance of its config file, each step only where what it makes is missing:
//!
//! - the records skeleton, written and committed as `conductor init` does, where the records
//!   directory holds no `NORTHSTAR.md`;
//! - the checkouts root, created where it is not there;
//! - the trust of each checkout and of the records directory in `~/.claude.json`, for the paths
//!   not trusted yet, as `conductor trust` marks them;
//! - the `worktree` workspace of the checkouts root, activated where `worktree` finds it not
//!   activated, as `setup --apply` does.
//!
//! What a person or a session changed since is kept: no step writes what is there. A step that
//! fails is said in one line naming the instance, and the daemon starts anyway: doctor names what
//! is left.

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use conductor_model::config::Instance;

use crate::cli::InitArgs;
use crate::config::{self, Environment};
use crate::trust::{self, CLAUDE_JSON};

/// What a refusal of a step starts with.
const WHAT: &str = "conductor: daemon serve: unattended";

/// Seeds each instance of the config file `file`, as the module documentation says.
pub fn seed(file: &Path, home: &Path) {
    let loaded = match Environment::from_process()
        .and_then(|environment| config::load(Some(file), &environment))
    {
        Ok(loaded) => loaded,
        Err(error) => {
            eprintln!("{WHAT}: {error:#}");
            return;
        }
    };
    for (index, instance) in loaded.config.instances.iter().enumerate() {
        let name = &instance.name.0;
        let steps = [
            records(file, instance),
            root(instance),
            trusted(home, instance, index),
            crate::setup::activate_root(&loaded, instance),
        ];
        for error in steps.into_iter().filter_map(Result::err) {
            eprintln!("{WHAT}: {name}: {error:#}");
        }
    }
}

/// The records skeleton of `instance`, where its records directory holds no `NORTHSTAR.md`.
fn records(file: &Path, instance: &Instance) -> Result<()> {
    let records = Path::new(&instance.records);
    if records.join("NORTHSTAR.md").symlink_metadata().is_ok() {
        return Ok(());
    }
    let args = InitArgs {
        instance: Some(instance.name.0.clone()),
    };
    crate::init::init(Some(file), &args).map(|_| ())
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
