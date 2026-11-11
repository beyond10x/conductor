//! `conductor instance serve` (`story:instance-api`, design architecture-next § 2, § 5.1): one
//! instance's runtime. It serves the instance API, the routes of [`crate::dashboard`] under `/v1/`,
//! on the Unix socket [`socket`] of the instance's state directory (or the one the daemon names in
//! [`SOCKET_VARIABLE`]), until it is stopped; conductord
//! starts it ([`crate::daemon::reconcile`]) and proxies `/v1/instances/{name}/<route>` to it.
//!
//! The instance is the one the global `--config` and `CONDUCTOR_INSTANCE` name, as for every
//! instance command. A write is taken with the token the daemon minted for the instance at its
//! start: in [`TOKEN_VARIABLE`] for a runtime the daemon starts as a unit, in the sealed slot
//! [`SLOTS_VARIABLE`] names for one it starts in a box on the local substrate daemon, whose
//! environment carries no credential. Started without either, the runtime mints its own, and
//! no client can write. The socket is created mode 0600, after a stale
//! one is removed; a socket another runtime answers on is refused.
//!
//! The runtime also runs the instance's loop (design architecture-next § 2: "the loop
//! (`conductor-run.md`), the instance API, the starter of the instance's agents"; § 11: the loop
//! is the runtime's core), in this process beside the API, under the lock `conductor run` takes
//! ([`crate::run::in_runtime`]): its watch ticks, its cycle ticks, whose dispatch v1 starts the
//! controllers, and its daily tick. A `conductor run` started by hand beside it is refused; one
//! that held the lock first keeps the loop, and the runtime takes it once that one ends. The API
//! serves in its own thread; should it end, the process exits 1, so the service manager restarts
//! the runtime as a whole. [`LOOP_VARIABLE`] set to `off` serves the API alone.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};

use crate::{config, dashboard};

/// The variable the daemon hands the runtime the instance's token in: a write on the instance API
/// carries it in `X-Conductor-Token`.
pub const TOKEN_VARIABLE: &str = "CONDUCTOR_API_TOKEN";

/// The variable substrate tells a boxed process its sealed secret slots in, `name=fd` pairs
/// separated by commas. A runtime in a box has no [`TOKEN_VARIABLE`]: the daemon's token is in
/// the slot [`crate::placement::is_api_token_slot`] knows (`story:box-reaches-forge-and-daemon`).
pub const SLOTS_VARIABLE: &str = "SUBSTRATE_SECRET_SLOTS";

/// The most a sealed token is read to: a token is far shorter, and a slot is not read whole on
/// trust.
const SEALED_TOKEN_BOUND: u64 = 4096;

/// The daemon's token as `slots`, the value of [`SLOTS_VARIABLE`], seals it: read from the
/// descriptor of the slot that carries it, or `None` where no such slot is named. The
/// descriptor is closed once it is read, whatever it held: substrate hands it over without
/// close-on-exec, so every process the runtime starts would inherit it and could read the token
/// from it. Nothing reads it again: the runtime reads it once, at its start, and a restart in the
/// same box is a new process substrate hands a slot of its own.
///
/// # Errors
///
/// The slot is named and its descriptor is not a number, is a standard stream, cannot be read,
/// or holds nothing: a runtime that minted its own token then would refuse every write the daemon
/// proxies, silently.
fn sealed_token(slots: &str) -> Result<Option<String>> {
    use std::io::Read as _;
    let Some((name, descriptor)) = slots
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, descriptor)| (name.trim(), descriptor.trim()))
        .find(|(name, _)| crate::placement::is_api_token_slot(name))
    else {
        return Ok(None);
    };
    let descriptor: std::os::fd::RawFd = descriptor
        .parse()
        .ok()
        .filter(|descriptor| *descriptor > 2)
        .with_context(|| {
            format!(
                "{SLOTS_VARIABLE} names the slot {name} at {descriptor:?}, which is no descriptor \
                 of a slot"
            )
        })?;
    // The descriptor is opened by its path: the crate forbids unsafe code, and a sealed slot
    // opened this way is read from its start whatever was read of it before.
    let read = fs::File::open(format!("/proc/self/fd/{descriptor}"))
        .or_else(|_| fs::File::open(format!("/dev/fd/{descriptor}")))
        .with_context(|| format!("open the sealed slot {name} at descriptor {descriptor}"))
        .and_then(|file| {
            let mut token = String::new();
            file.take(SEALED_TOKEN_BOUND)
                .read_to_string(&mut token)
                .with_context(|| format!("read the sealed slot {name} at descriptor {descriptor}"))
                .map(|_| token)
        });
    // One the process does not hold is closed already: nothing to answer for.
    let _ = nix::unistd::close(descriptor);
    let token = read?;
    let token = token.trim();
    if token.is_empty() {
        bail!("the sealed slot {name} at descriptor {descriptor} holds no token");
    }
    Ok(Some(token.to_owned()))
}

/// The socket's file name in the instance's state directory.
pub const SOCKET: &str = "api.sock";

/// The longest path a Unix socket address holds, less its terminating byte.
const SOCKET_PATH_MAX: usize = 107;

/// The variable the daemon hands the runtime its socket's path in, so both name the same one
/// whatever temporary directory each sees.
pub const SOCKET_VARIABLE: &str = "CONDUCTOR_API_SOCKET";

/// The variable the daemon names the runtime's log file in: a runtime in a substrate box writes
/// what it says there, not to its session, which substrate ends at its output bound. The
/// runtime turns to it before anything else, so an error that ends it at once is in the file.
pub const LOG_VARIABLE: &str = "CONDUCTOR_API_LOG";

/// The variable that, set to `off`, has the runtime serve the instance API without the loop: for
/// a case that tests the API alone, or an operator who runs `conductor run` by hand. The daemon
/// never sets it.
pub const LOOP_VARIABLE: &str = "CONDUCTOR_RUNTIME_LOOP";

/// The instance API's socket of the instance whose state directory is `state`: `<state>/api.sock`,
/// or, where that is longer than a socket address holds, `conductor-api-<hash>.sock` in the
/// temporary directory, the hash (FNV-1a, 64 bits) of the state directory's path.
#[must_use]
pub fn socket(state: &Path) -> PathBuf {
    use std::os::unix::ffi::OsStrExt as _;
    let path = state.join(SOCKET);
    if path.as_os_str().len() <= SOCKET_PATH_MAX {
        return path;
    }
    let hash = state
        .as_os_str()
        .as_bytes()
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        });
    std::env::temp_dir().join(format!("conductor-api-{hash:016x}.sock"))
}

/// Binds `path` for the instance API: refuses a path too long for a socket address and a socket
/// another process answers on; removes a stale one; creates the socket mode 0600.
///
/// # Errors
///
/// As above, or the directory cannot be created or the socket bound.
pub fn bind(path: &Path) -> Result<UnixListener> {
    if path.as_os_str().len() > SOCKET_PATH_MAX {
        bail!(
            "the socket path {} is longer than a socket address holds ({SOCKET_PATH_MAX} bytes)",
            path.display()
        );
    }
    if UnixStream::connect(path).is_ok() {
        bail!(
            "{} answers already: another runtime serves it",
            path.display()
        );
    }
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| format!("remove the stale {}", path.display()));
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let listener = UnixListener::bind(path).with_context(|| format!("bind {}", path.display()))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("make {} the owner's only", path.display()))?;
    Ok(listener)
}

/// `conductor instance relay` (`story:placement-substrate-remote`): one instance API exchange,
/// the request read from standard input to its end, sent to the instance's socket
/// ([`SOCKET_VARIABLE`], else the active instance's `<state>/api.sock`), and its answer written
/// to standard output. conductord runs it as a substrate pipe session in a box on a remote
/// substrate daemon, where the runtime's socket is ([`crate::placement::Exchange`]).
///
/// # Errors
///
/// The config file does not load (without the variable), or the socket does not answer: one
/// line naming it.
pub fn instance_relay(config_flag: Option<&Path>) -> Result<ExitCode> {
    use std::io::{Read as _, Write as _};
    const WHAT: &str = "instance relay";
    let path = match std::env::var_os(SOCKET_VARIABLE).filter(|path| !path.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => {
            config::set_active(config_flag).context(WHAT)?;
            socket(Path::new(&config::active().instance.state))
        }
    };
    let mut request = Vec::new();
    std::io::stdin()
        .take(RELAY_MAX)
        .read_to_end(&mut request)
        .context("instance relay: read the request")?;
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("{WHAT}: connect to {}", path.display()))?;
    stream
        .write_all(&request)
        .and_then(|()| stream.shutdown(std::net::Shutdown::Write))
        .with_context(|| format!("{WHAT}: send to {}", path.display()))?;
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .with_context(|| format!("{WHAT}: read from {}", path.display()))?;
    let mut out = std::io::stdout().lock();
    out.write_all(&answer)
        .and_then(|()| out.flush())
        .context("instance relay: write the answer")?;
    Ok(ExitCode::SUCCESS)
}

/// The longest request [`instance_relay`] reads: the instance API's body bound and a head.
const RELAY_MAX: u64 = dashboard::MAX_BODY as u64 + 64 * 1024;

/// Points this process's standard output and error at the end of `log` ([`LOG_VARIABLE`]),
/// created mode 0600 with its directory when missing.
///
/// # Errors
///
/// The directory or the file cannot be created, or a descriptor not replaced.
fn log_to(log: &Path) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt as _;
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(log)
        .with_context(|| format!("open the log {}", log.display()))?;
    nix::unistd::dup2_stdout(&file)
        .and_then(|()| nix::unistd::dup2_stderr(&file))
        .with_context(|| format!("write the output to {}", log.display()))
}

/// `conductor instance serve`, as the module documentation says.
///
/// # Errors
///
/// The config file does not load, the home directory is not absolute, the token cannot be
/// minted, the socket cannot be bound, or the loop cannot start ([`crate::run::in_runtime`]).
pub fn instance_serve(
    config_flag: Option<&Path>,
    state_flag: Option<&Path>,
    providers: &dyn crate::confine::Providers,
) -> Result<ExitCode> {
    const WHAT: &str = "instance serve";
    // The log first: what fails from here on is said there too, where conductord reads why a
    // runtime in a box did not start (`story:box-crash-loop-reads-failed`).
    if let Some(log) = std::env::var_os(LOG_VARIABLE).filter(|log| !log.is_empty()) {
        log_to(Path::new(&log)).context(WHAT)?;
    }
    config::set_active(config_flag).context(WHAT)?;
    let active = config::active();
    let state = PathBuf::from(&active.instance.state);
    let sources = dashboard::Sources::active(None).context(WHAT)?;
    let sealed = match std::env::var(SLOTS_VARIABLE) {
        Ok(slots) => sealed_token(&slots).context("instance serve: the daemon's token")?,
        Err(_) => None,
    };
    let token = match (std::env::var(TOKEN_VARIABLE), sealed) {
        (Ok(token), _) if !token.trim().is_empty() => token.trim().to_owned(),
        (_, Some(token)) => token,
        _ => dashboard::mint().context("instance serve: mint the instance's token")?,
    };
    let path = std::env::var_os(SOCKET_VARIABLE)
        .filter(|path| !path.is_empty())
        .map_or_else(|| socket(&state), PathBuf::from);
    let listener = bind(&path).context(WHAT)?;
    let what = format!("conductor instance {}", active.instance.name.0);
    println!("{what}: serving /v1/ on {}", path.display());
    let api = std::thread::Builder::new()
        .name("instance-api".to_owned())
        .spawn(move || {
            // The runtime is the API and the loop together: an API that ended, or panicked, ends
            // the process, so the service manager restarts both.
            struct Ended;
            impl Drop for Ended {
                fn drop(&mut self) {
                    crate::run::say(true, "conductor: instance serve: the instance API ended");
                    std::process::exit(1);
                }
            }
            let _ended = Ended;
            dashboard::serve_unix(listener, sources, Some(state), token);
        })
        .context("instance serve: start the instance API")?;
    if std::env::var(LOOP_VARIABLE).is_ok_and(|value| value.trim() == "off") {
        crate::run::say(
            false,
            &format!("{what}: the loop is off ({LOOP_VARIABLE}=off)"),
        );
        let _ = api.join();
        return Ok(ExitCode::FAILURE);
    }
    let Err(error) = crate::run::in_runtime(&what, config_flag, state_flag, providers);
    Err(error.context(WHAT))
}
