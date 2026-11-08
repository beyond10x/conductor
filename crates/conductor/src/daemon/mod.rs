//! `conductor daemon serve` (`story:daemon-reconcile`, design architecture-next § 4): conductord,
//! the one daemon of the control host. It reads the config file the global `--config` or
//! `CONDUCTOR_CONFIG` names (else `~/.b10x/conductor/conductor.yaml`), reconciles the instance
//! runtimes from it ([`reconcile`]), records each action in its store, and serves
//! `GET 127.0.0.1:<daemon.port>/v1/instances` ([`api`]). Its directory is the global
//! `--state-dir`, else `~/.b10x/conductor/daemon` ([`DIR`]): what it started. Its store is the
//! deployment's one database, the URL `daemon.store` writes, opened under the tenant [`TENANT`];
//! it refuses to start on a file that writes none. `--install` and `--uninstall` make it the user
//! unit `conductord.service` ([`install`]).

pub mod api;
pub mod install;
pub mod reconcile;
pub mod units;

use std::net::TcpListener;
use std::path::Path;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};

use crate::cli::DaemonServeArgs;
use crate::config::{self, Environment};
use reconcile::Daemon;
use units::Systemd;

/// The tenant of the daemon's own records in the deployment's database (`daemon.store`): its
/// `Reconciliation`s, beside each instance's tenant.
pub const TENANT: &str = "daemon";

/// The daemon's directory under the home directory, when no `--state-dir` names one: what it
/// started (`runtimes.json`).
pub const DIR: &str = ".b10x/conductor/daemon";

/// How often the daemon looks at the config file and the runtimes it started.
const TICK: Duration = Duration::from_secs(2);

/// `conductor daemon serve`, as the module documentation says.
///
/// # Errors
///
/// Both `--install` and `--uninstall` are given; the home directory is not known; the config
/// file does not read or is not valid at start; the store does not open; the port cannot be
/// bound; or the service manager refuses the install.
pub fn daemon_serve(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &DaemonServeArgs,
) -> Result<ExitCode> {
    const WHAT: &str = "daemon serve";
    if args.install == Some(true) && args.uninstall == Some(true) {
        bail!("{WHAT}: --install and --uninstall are given; give at most one");
    }
    let environment = Environment::from_process().context(WHAT)?;
    let home = environment.home().context(WHAT)?.to_owned();
    let (file, _) = config::locate(config_flag, &environment).context(WHAT)?;
    let file = environment.cwd.join(file);
    let state = state.map(|state| environment.cwd.join(state));
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from);
    let unit_file = install::unit_file(&home, xdg.as_deref());
    if args.install == Some(true) {
        let program = std::env::current_exe().context("daemon serve --install: read the binary")?;
        let path = std::env::var("PATH").ok();
        let text = install::unit_text(&program, Some(&file), state.as_deref(), path.as_deref());
        println!(
            "{}",
            install::install(&mut Systemd, &unit_file, &text).context(WHAT)?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if args.uninstall == Some(true) {
        println!(
            "{}",
            install::uninstall(&mut Systemd, &unit_file).context(WHAT)?
        );
        return Ok(ExitCode::SUCCESS);
    }
    let dir = state.unwrap_or_else(|| home.join(DIR));
    let program = std::env::current_exe().context("daemon serve: read the binary")?;
    let mut daemon =
        Daemon::open(file.clone(), home, dir, reconcile::probe(&program), Systemd).context(WHAT)?;
    let port = u16::try_from(daemon.config().daemon.port).context("daemon serve: daemon.port")?;
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("daemon serve: listen on 127.0.0.1:{port}"))?;
    let rows: api::Rows = Arc::new(Mutex::new(daemon.rows()));
    let served = Arc::clone(&rows);
    thread::spawn(move || api::serve(&listener, &served));
    println!(
        "conductord: config {}, serving http://127.0.0.1:{port}{}",
        file.display(),
        api::INSTANCES
    );
    loop {
        let now = Instant::now();
        let passed = daemon.due(now).and_then(|due| match due {
            Some(reason) => {
                daemon.binary = reconcile::probe(&program);
                daemon.pass(reason, now)
            }
            None => Ok(Vec::new()),
        });
        match passed {
            Ok(done) => {
                for done in done {
                    println!(
                        "{}: {:?} ({:?}): {:?}{}",
                        done.instance,
                        done.action,
                        done.reason,
                        done.result,
                        done.detail
                            .map(|detail| format!(": {detail}"))
                            .unwrap_or_default()
                    );
                }
            }
            Err(error) => eprintln!("conductor: {WHAT}: {error:#}"),
        }
        if let Ok(mut published) = rows.lock() {
            *published = daemon.rows();
        }
        thread::sleep(TICK);
    }
}
