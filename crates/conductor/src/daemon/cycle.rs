//! `conductor instance cycle` (`story:cycle-now`): the client of `POST
//! /v1/instances/{name}/cycle` on the daemon of the config file, which the daemon proxies to the
//! runtime's `POST /v1/cycle`. The runtime's loop then runs its next cycle at its next tick,
//! within seconds and under its own lock, rather than when `cadence.cycle` has passed; `conductor
//! run --once` cannot, since the loop holds `<state>/run/lock`.

use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};

use super::{DIR, api, upgrade};
use crate::config::{self, Environment};

/// Asks the daemon of the config file for a cycle of the active instance now: the instance
/// `CONDUCTOR_INSTANCE` or the config file selects ([`config::active`]). Prints one line naming
/// the instance, and the start of a cycle that runs now, which the ask then follows.
///
/// # Errors
///
/// The config does not load, the daemon is not reached, or it or the runtime refuses the ask.
pub fn instance_cycle(config_flag: Option<&Path>) -> Result<ExitCode> {
    const WHAT: &str = "instance cycle";
    let environment = Environment::from_process().context(WHAT)?;
    let loaded = config::load(config_flag, &environment).context(WHAT)?;
    let port = u16::try_from(loaded.config.daemon.port).context("instance cycle: daemon.port")?;
    let name = config::active().instance.name.0.clone();
    // The daemon takes a write from a local client with the token it keeps in its directory.
    let token = environment
        .home()
        .ok()
        .and_then(|home| api::kept_token(&home.join(DIR)));
    let (status, body) = upgrade::request(
        token.as_deref(),
        port,
        "POST",
        &format!("{}/{name}/cycle", api::INSTANCES),
        Some("{}"),
    )
    .with_context(|| format!("{WHAT}: {name}"))?;
    if status != 200 {
        bail!(
            "{WHAT}: {name}: refused ({status}): {}",
            body["error"]
                .as_str()
                .unwrap_or("the daemon answered no reason")
        );
    }
    match body["running_since"].as_str() {
        Some(since) => println!(
            "{name}: a cycle is asked; the one running since {since} ends first, then the loop \
             runs it"
        ),
        None => println!("{name}: a cycle is asked; the loop runs it at its next tick"),
    }
    Ok(ExitCode::SUCCESS)
}
