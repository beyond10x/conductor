//! `conductor daemon serve` (`story:daemon-reconcile`, design architecture-next § 4): conductord,
//! the one daemon of the control host. It reads the config file the global `--config` or
//! `CONDUCTOR_CONFIG` names (else `~/.b10x/conductor/conductor.yaml`), reconciles the instance
//! runtimes from it ([`reconcile`]), records each action in its store, and serves its API ([`api`]):
//! the dashboard app and `/v1/` on `127.0.0.1:<daemon.port>` and on the TLS listener `daemon.tls`
//! names, the instances proxied (`story:instance-api`), and upgrades an instance on
//! `POST /v1/instances/{name}/upgrade` ([`upgrade`]). Its directory is the global
//! `--state-dir`, else `~/.b10x/conductor/daemon` ([`DIR`]): what it started. Its store is the
//! deployment's one database, the URL `daemon.store` writes, opened under the tenant [`TENANT`];
//! it refuses to start on a file that writes none. `--install` and `--uninstall` make it the user
//! unit `conductord.service` ([`install`]). On a box with no local operator
//! (`daemon.loopback: false`, `story:daemon-remote`) the loopback listener is off, the TLS
//! listener is its only API, and `--install` writes the system unit instead.

pub mod api;
pub mod browser;
pub mod federate;
pub mod host_roles;
pub mod install;
pub mod reconcile;
pub mod units;
pub mod upgrade;

use std::net::TcpListener;
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

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
    // A box with no local operator (`daemon.loopback: false`) installs the system unit
    // (`story:daemon-remote`).
    if args.install == Some(true) || args.uninstall == Some(true) {
        let loaded = config::load(config_flag, &environment).context(WHAT)?;
        if !loaded.config.daemon.loopback {
            let system = install::system_unit_file();
            if args.uninstall == Some(true) {
                println!(
                    "{}",
                    install::uninstall_system(&mut Systemd, &system).context(WHAT)?
                );
                return Ok(ExitCode::SUCCESS);
            }
            install::remote_ready(&file, &loaded.config.daemon).context(WHAT)?;
            let program =
                std::env::current_exe().context("daemon serve --install: read the binary")?;
            let user = std::env::var("SUDO_USER")
                .or_else(|_| std::env::var("USER"))
                .ok()
                .filter(|user| !user.is_empty());
            let path = std::env::var("PATH").ok();
            let text =
                install::system_unit_text(&program, &file, user.as_deref(), &home, path.as_deref());
            println!(
                "{}",
                install::install_system(&mut Systemd, &system, &text).context(WHAT)?
            );
            return Ok(ExitCode::SUCCESS);
        }
    }
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
    let mut daemon = Daemon::open(
        file.clone(),
        home.clone(),
        dir.clone(),
        reconcile::probe(&program),
        Systemd,
    )
    .context(WHAT)?;
    // The secrets source (`story:secrets-source`, decision S1): one the daemon cannot use refuses
    // the start; otherwise its line, and every instance whose config names a secret it does not
    // hold. No value is printed.
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("daemon serve: read the config file {}", file.display()))?;
    match crate::secret::at_start(&daemon.config().daemon.secrets, &text) {
        Ok(lines) => lines.iter().for_each(|line| println!("{line}")),
        Err(unusable) => bail!("{WHAT}: {unusable}; the daemon does not start"),
    }
    let port = u16::try_from(daemon.config().daemon.port).context("daemon serve: daemon.port")?;
    // The loopback listener is off on a box with no local operator (`story:daemon-remote`); the
    // TLS listener is then the daemon's only API.
    let listener = if daemon.config().daemon.loopback {
        Some(
            TcpListener::bind(("127.0.0.1", port))
                .with_context(|| format!("daemon serve: listen on 127.0.0.1:{port}"))?,
        )
    } else {
        None
    };
    let secure = match &daemon.config().daemon.tls {
        Some(tls) => Some(tls_listener(tls)?),
        None if listener.is_none() => bail!(
            "{WHAT}: daemon.loopback: is false and daemon.tls names no TLS listener: the daemon \
             would serve no API"
        ),
        None => None,
    };
    let local = crate::actor::human()
        .operator
        .map_or_else(|| "operator".to_owned(), |name| name.0);
    let token = crate::dashboard::mint().context("daemon serve: mint the daemon's token")?;
    api::keep_token(&dir, &token).context(WHAT)?;
    let api = Arc::new(
        api::Api::new(
            token,
            local,
            secure.as_ref().map(|(_, _, identity)| identity.clone()),
        )
        .with_config(file.clone(), home, daemon.config().daemon.store.0.clone())
        .with_timeout(Duration::from_secs(
            config::seconds(&daemon.config().daemon.timeout).unwrap_or(config::FEDERATED_TIMEOUT),
        ))
        .with_secrets(daemon.config().daemon.secrets.source),
    );
    publish(&api, &daemon);
    let served = Arc::clone(&api);
    let upgrades = upgrade::listen();
    if let Some(listener) = listener {
        thread::spawn(move || api::serve(&listener, &served));
        println!(
            "conductord: config {}, serving the dashboard and /v1/ on http://127.0.0.1:{port}/",
            file.display(),
        );
    } else {
        println!(
            "conductord: config {}, the loopback listener off (daemon.loopback: false)",
            file.display(),
        );
    }
    if let Some((secure, config, _)) = secure {
        let address = secure
            .local_addr()
            .map_or_else(|_| "?".to_owned(), |address| address.to_string());
        let served = Arc::clone(&api);
        thread::spawn(move || api::serve_tls(&secure, &served, &config));
        println!("conductord: serving the dashboard and /v1/ on https://{address}/ with bearers");
    }
    loop {
        while let Ok(request) = upgrades.try_recv() {
            daemon.answer(request);
        }
        let now = Instant::now();
        // An upgrade that answered or is to roll back is settled first, so a runtime lost while
        // upgrading rolls back rather than starting again on the new binary. A host role asked
        // for through `POST /v1/roles/{role}/start` starts after the pass.
        let passed = daemon.settle(SystemTime::now()).and_then(|mut done| {
            if let Some(reason) = daemon.due(now)? {
                daemon.binary = reconcile::probe(&program);
                done.extend(daemon.pass(reason, now)?);
            }
            let asked = host_roles::take_asked();
            if !asked.is_empty() {
                done.extend(daemon.start_asked(&asked)?);
            }
            Ok(done)
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
        publish(&api, &daemon);
        thread::sleep(TICK);
    }
}

/// `conductor daemon start-role --role <role>` (`story:host-roles-by-daemon`): asks the daemon of
/// the config file for the host role `role` now, `POST /v1/roles/{role}/start` on its loopback
/// listener with the token it keeps in its directory (the global `--state-dir`, else [`DIR`]).
/// Prints one line naming the role asked for.
///
/// # Errors
///
/// `--role` is missing, the config does not load, the daemon is not reached, or it refuses the
/// role (no host role, or one `globals.roles` does not list).
pub fn start_role(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &crate::cli::DaemonStartRoleArgs,
) -> Result<ExitCode> {
    const WHAT: &str = "daemon start-role";
    let Some(role) = args.role.as_deref().filter(|role| !role.trim().is_empty()) else {
        bail!("{WHAT}: --role is required: supervisor or teacher");
    };
    let environment = Environment::from_process().context(WHAT)?;
    let loaded = config::load(config_flag, &environment).context(WHAT)?;
    let port =
        u16::try_from(loaded.config.daemon.port).context("daemon start-role: daemon.port")?;
    let dir = match state {
        Some(state) => environment.cwd.join(state),
        None => environment.home().context(WHAT)?.join(DIR),
    };
    let token = api::kept_token(&dir);
    let (status, body) = upgrade::request(
        token.as_deref(),
        port,
        "POST",
        &format!("{}{role}/start", api::ROLES),
        Some("{}"),
    )
    .context(WHAT)?;
    if status != 202 {
        bail!(
            "{WHAT}: refused: {}",
            body["error"]
                .as_str()
                .unwrap_or("the daemon answered no reason")
        );
    }
    println!(
        "asked the daemon for the {role} at {}",
        body["placement"].as_str().unwrap_or(host_roles::PLACEMENT)
    );
    Ok(ExitCode::SUCCESS)
}

/// Publishes what `daemon` last saw to `api`: the instances' rows, the routes to their runtimes and
/// the deployment's operators.
fn publish(api: &api::Api, daemon: &Daemon<Systemd>) {
    api.publish(
        daemon.rows(),
        daemon.routes(),
        daemon.config().daemon.operators.clone(),
    );
    api.publish_host(host_roles::listed(daemon.config()), daemon.supervisor());
}

/// The TLS listener `daemon.tls` names, bound, with its rustls config and the Identity service
/// that resolves its bearers; refused while `cert`, `key` or `identity` is absent, naming its YAML
/// path (`story:instance-api`).
fn tls_listener(
    tls: &conductor_model::config::TlsListener,
) -> Result<(TcpListener, Arc<rustls::ServerConfig>, api::Identity)> {
    const WHAT: &str = "daemon serve";
    let named = |value: &Option<String>, key: &str| {
        value
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_owned)
            .with_context(|| format!("{WHAT}: daemon.tls.{key}: the TLS listener needs it"))
    };
    let (cert, key, identity) = (
        named(&tls.cert, "cert")?,
        named(&tls.key, "key")?,
        named(&tls.identity, "identity")?,
    );
    let config = api::tls_config(Path::new(&cert), Path::new(&key))
        .with_context(|| format!("{WHAT}: daemon.tls.cert, daemon.tls.key"))?;
    let identity = api::Identity::new(&identity, api::AUDIENCE)
        .with_context(|| format!("{WHAT}: daemon.tls.identity"))?;
    let listener = TcpListener::bind(tls.listen.as_str())
        .with_context(|| format!("{WHAT}: daemon.tls.listen: listen on {}", tls.listen))?;
    Ok((listener, config, identity))
}
