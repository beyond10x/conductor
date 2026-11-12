//! `conductor daemon serve` (`story:daemon-reconcile`, design architecture-next § 4): conductord,
//! the one daemon of the control host. It reads the config file the global `--config` or
//! `CONDUCTOR_CONFIG` names (else `~/.b10x/conductor/conductor.yaml`), reconciles the instance
//! runtimes from it ([`reconcile`]), records each action in its store, and serves its API ([`api`]):
//! the dashboard app and `/v1/` on `<daemon.address>:<daemon.port>` (`127.0.0.1` unless the file
//! names another address) and on the TLS listener `daemon.tls` names, the instances proxied (`story:instance-api`), and upgrades an instance on
//! `POST /v1/instances/{name}/upgrade` ([`upgrade`]). Its directory is the global
//! `--state-dir`, else `~/.b10x/conductor/daemon` ([`DIR`]): what it started. Its store is the
//! deployment's one database, the URL `daemon.store` writes, opened under the tenant [`TENANT`];
//! it refuses to start on a file that writes none. `--install` and `--uninstall` make it the user
//! unit `conductord.service` ([`install`]). On a box with no local operator
//! (`daemon.loopback: false`, `story:daemon-remote`) the loopback listener is off, the TLS
//! listener is its only API, and `--install` writes the system unit instead.
//!
//! In a container (`story:container-start-without-prompt`) it starts with no terminal and no
//! prompt: `daemon.unattended: true` seeds each instance's records, trust and `worktree`
//! workspace where they are missing ([`unattended`]) while its listeners answer, `/readyz` `503`
//! meanwhile and after a step that failed; and where no user service manager is, each runtime
//! is a child process of the daemon ([`units::Children`]).
//!
//! In a cluster (design: instance pods) `--daemon-config` names a file that holds only `version`
//! and `daemon`, such as a mounted ConfigMap: at every start, before the config file is read,
//! [`seed::merge`] writes the config file from it, keeping the file's instances.
//!
//! It serves before its store answers (`story:daemon-probes-and-metrics`): the listeners take
//! requests as soon as the config is read, `GET /healthz` answers `200` and `GET /readyz` `503`
//! with the reason ([`metrics`]). While the store does not answer the daemon opens no store,
//! starts no runtime and records nothing; when it answers, the same process opens it and
//! reconciles. A request that is no probe waits for the reconciler's first publication while the
//! store answers at once, and is answered without it once the store is found not to answer
//! ([`api::Api::starting`]): a read of what the reconciler publishes is then `503` with the
//! readiness reason. A record the store refuses later is held and written on a handle opened
//! anew at the next tick, and nothing is acted on until it is ([`Daemon::record_held`]): an
//! upgrade asked meanwhile is answered `503` with the reason ([`answer_upgrade`]), and
//! `GET /readyz` is `503`.

pub mod api;
pub mod browser;
pub mod federate;
pub mod host_roles;
pub mod install;
pub mod metrics;
pub mod reconcile;
pub mod seed;
pub mod unattended;
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
/// file does not read, is not valid or writes no `daemon.store` at start; the port cannot be
/// bound; the store answers and then does not open, or what the daemon kept in its directory
/// does not read; or the service manager refuses the install.
pub fn daemon_serve(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &DaemonServeArgs,
) -> Result<ExitCode> {
    const WHAT: &str = "daemon serve";
    if args.install == Some(true) && args.uninstall == Some(true) {
        bail!("{WHAT}: --install and --uninstall are given; give at most one");
    }
    // Design: instance pods. The daemon-config file seeds the config file at every start of the
    // daemon, which a unit's start does not do.
    if args.daemon_config.is_some() && (args.install == Some(true) || args.uninstall == Some(true))
    {
        bail!("{WHAT}: --daemon-config is not taken with --install or --uninstall");
    }
    let environment = Environment::from_process().context(WHAT)?;
    let home = environment.home().context(WHAT)?.to_owned();
    let (file, _) = config::locate(config_flag, &environment).context(WHAT)?;
    let file = environment.cwd.join(file);
    let state = state.map(|state| environment.cwd.join(state));
    // Before anything reads the config file, it is written anew from the daemon-config file.
    if let Some(daemon_config) = &args.daemon_config {
        let daemon_config = environment.cwd.join(daemon_config);
        if seed::merge(&daemon_config, &file, &home).context(WHAT)? {
            println!(
                "conductord: wrote the config file {} from {}",
                file.display(),
                daemon_config.display()
            );
        }
    }
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
    // What the start reads before its store (`story:daemon-probes-and-metrics`): a file that does
    // not read, is not valid or writes no `daemon.store` refuses the start; a store that does not
    // answer does not.
    let (written, url) = reconcile::written(&file, &home).context(WHAT)?;
    // The secrets source (`story:secrets-source`, decision S1): one the daemon cannot use refuses
    // the start; otherwise its line, and every instance whose config names a secret it does not
    // hold. No value is printed.
    let text = std::fs::read_to_string(&file)
        .with_context(|| format!("daemon serve: read the config file {}", file.display()))?;
    match crate::secret::at_start(&written.daemon.secrets, &text) {
        Ok(lines) => lines.iter().for_each(|line| println!("{line}")),
        Err(unusable) => bail!("{WHAT}: {unusable}; the daemon does not start"),
    }
    let port = u16::try_from(written.daemon.port).context("daemon serve: daemon.port")?;
    // The loopback listener is off on a box with no local operator (`story:daemon-remote`); the
    // TLS listener is then the daemon's only API. It binds `daemon.address`, `127.0.0.1` unless
    // the file names another; on one that is not loopback, as in a container whose port is
    // published, it takes bearers as the TLS listener does ([`api::serve`]).
    let address: std::net::IpAddr = written
        .daemon
        .address
        .trim()
        .parse()
        .with_context(|| format!("{WHAT}: daemon.address: {}", written.daemon.address))?;
    let listener = if written.daemon.loopback {
        Some(
            TcpListener::bind((address, port))
                .with_context(|| format!("daemon serve: listen on {address}:{port}"))?,
        )
    } else {
        None
    };
    let secure = match &written.daemon.tls {
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
        .with_config(file.clone(), home.clone(), written.daemon.store.0.clone())
        .with_timeout(Duration::from_secs(
            config::seconds(&written.daemon.timeout).unwrap_or(config::FEDERATED_TIMEOUT),
        ))
        .with_secrets(written.daemon.secrets.source)
        .starting(written.daemon.operators.clone()),
    );
    // The listeners take requests before the store answers: the probes answer whatever it does,
    // and any other request waits for the reconciler's first publication ([`api::Api::starting`]).
    let mut serving = Vec::new();
    if let Some(listener) = listener {
        let served = Arc::clone(&api);
        thread::spawn(move || api::serve(&listener, &served));
        serving.push(format!(
            "conductord: config {}, serving the dashboard and /v1/ on http://{}/{}",
            file.display(),
            std::net::SocketAddr::new(address, port),
            if address.is_loopback() {
                ""
            } else {
                " with bearers"
            },
        ));
    } else {
        serving.push(format!(
            "conductord: config {}, the loopback listener off (daemon.loopback: false)",
            file.display(),
        ));
    }
    if let Some((secure, config, _)) = secure {
        let address = secure
            .local_addr()
            .map_or_else(|_| "?".to_owned(), |address| address.to_string());
        let served = Arc::clone(&api);
        thread::spawn(move || api::serve_tls(&secure, &served, &config));
        serving.push(format!(
            "conductord: serving the dashboard and /v1/ on https://{address}/ with bearers"
        ));
    }
    let watched = Arc::clone(&api);
    thread::spawn(move || metrics::watch(&url, &watched, TICK));
    // `daemon.unattended` (`story:container-start-without-prompt`): what `conductor setup
    // --apply` does for each instance, each only where it is missing, before the first pass and
    // while the listeners answer. `/readyz` is `503` meanwhile, and stays so for as long as the
    // process runs once a step failed; the daemon then goes on as it would.
    if written.daemon.unattended {
        api.probes().seeding();
        let seeded = unattended::seed(&file, &home);
        api.probes().seeded(seeded);
        if !seeded {
            eprintln!(
                "conductor: {WHAT}: the unattended start's seeding failed in the steps above; \
                 the daemon serves and reconciles, and {} answers 503 until a start seeds them",
                metrics::READINESS
            );
        }
    }
    // The serving lines follow what the reconciler first publishes when the store answers at
    // once, as they always did. Where it does not answer, or not within the readiness bound,
    // they are printed now and the daemon waits for the store: it opens none, starts no runtime
    // and records nothing until then.
    let asked = Instant::now();
    while !api.store_asked() && asked.elapsed() < metrics::STORE_BOUND {
        thread::sleep(Duration::from_millis(20));
    }
    let at_once = api.store_answers();
    if !at_once {
        api.started();
        serving.iter().for_each(|line| println!("{line}"));
        while !api.store_answers() {
            thread::sleep(TICK);
        }
    }
    // Where no user service manager is, as in a container, each runtime is a child process of
    // the daemon, and a signal that ends the daemon stops them first.
    let manager = units::Manager::chosen(
        std::env::var_os("XDG_RUNTIME_DIR").as_deref(),
        std::env::var_os("PATH").as_deref(),
    );
    if let units::Manager::Children(children) = &manager {
        println!(
            "conductord: no user service manager here: each runtime runs as a child process of \
             the daemon"
        );
        end_children_on_signal(children.clone()).context(WHAT)?;
    }
    let mut daemon =
        Daemon::open(file, home, dir, reconcile::probe(&program), manager).context(WHAT)?;
    let upgrades = upgrade::listen();
    publish(&api, &daemon);
    if at_once {
        serving.iter().for_each(|line| println!("{line}"));
    }
    loop {
        // A record the store refused is written first, on a handle opened anew; while it is not,
        // nothing is acted on, since nothing would be recorded, and an upgrade asked meanwhile is
        // answered `503` ([`answer_upgrade`]).
        let held = daemon
            .record_held()
            .context("record the reconciliations the store refused");
        while let Ok(request) = upgrades.try_recv() {
            answer_upgrade(&mut daemon, request);
        }
        let now = Instant::now();
        // An upgrade that answered or is to roll back is settled first, so a runtime lost while
        // upgrading rolls back rather than starting again on the new binary. A host role asked
        // for through `POST /v1/roles/{role}/start` starts after the pass.
        let passed = held
            .and_then(|()| daemon.settle(SystemTime::now()))
            .and_then(|mut done| {
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

/// Answers `request`, an upgrade asked through `POST /v1/instances/{name}/upgrade`: while
/// `daemon` holds a record the store refused ([`Daemon::record_held`]), `503` with the readiness
/// reason [`metrics::UNRECORDED`] and nothing done, since nothing would be recorded; otherwise
/// the upgrade ([`Daemon::answer`]).
pub fn answer_upgrade<U: units::Units>(daemon: &mut Daemon<U>, request: upgrade::Request) {
    if daemon.holds_records() {
        let _ = request.reply.send((
            "503 Service Unavailable",
            serde_json::json!({"error": metrics::UNRECORDED}),
        ));
        return;
    }
    daemon.answer(request);
}

/// Publishes what `daemon` last saw to `api`: whether it holds a record the store refused, the
/// instances' rows, the routes to their runtimes and the deployment's operators.
fn publish<U: units::Units>(api: &api::Api, daemon: &Daemon<U>) {
    api.publish_unrecorded(daemon.holds_records());
    api.publish(
        daemon.rows(),
        daemon.routes(),
        daemon.config().daemon.operators.clone(),
    );
    api.publish_host(host_roles::listed(daemon.config()), daemon.supervisor());
}

/// On `SIGTERM`, `SIGINT` or `SIGHUP`, stops `children`, then exits 0: a daemon whose runtimes
/// are its children ([`units::Children`]) ends with none left.
///
/// # Errors
///
/// A handler cannot be registered.
fn end_children_on_signal(children: units::Children) -> Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    let ended = Arc::new(AtomicBool::new(false));
    for signal in [
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGHUP,
    ] {
        signal_hook::flag::register(signal, Arc::clone(&ended))
            .with_context(|| format!("handle the signal {signal}"))?;
    }
    thread::Builder::new()
        .name("daemon-signals".to_owned())
        .spawn(move || {
            while !ended.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }
            println!("conductord: ending: stopping the runtimes it started");
            children.stop_all();
            std::process::exit(0);
        })
        .context("start the signal watch")?;
    Ok(())
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
