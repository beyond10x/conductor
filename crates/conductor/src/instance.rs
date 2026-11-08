//! `conductor instance serve` (`story:instance-api`, design architecture-next § 2, § 5.1): one
//! instance's runtime. It serves the instance API, the routes of [`crate::dashboard`] under `/v1/`,
//! on the Unix socket [`socket`] of the instance's state directory (or the one the daemon names in
//! [`SOCKET_VARIABLE`]), until it is stopped; conductord
//! starts it ([`crate::daemon::reconcile`]) and proxies `/v1/instances/{name}/<route>` to it.
//!
//! The instance is the one the global `--config` and `CONDUCTOR_INSTANCE` name, as for every
//! instance command. A write is taken with the token the daemon minted for the instance at its
//! start, [`TOKEN_VARIABLE`]; started by hand without it, the runtime mints its own, and no client
//! but one that reads its environment can write. The socket is created mode 0600, after a stale
//! one is removed; a socket another runtime answers on is refused.
//!
//! The loop (`conductor run`) and the agents are not started here yet: the runtime is the API.

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

/// The socket's file name in the instance's state directory.
pub const SOCKET: &str = "api.sock";

/// The longest path a Unix socket address holds, less its terminating byte.
const SOCKET_PATH_MAX: usize = 107;

/// The variable the daemon hands the runtime its socket's path in, so both name the same one
/// whatever temporary directory each sees.
pub const SOCKET_VARIABLE: &str = "CONDUCTOR_API_SOCKET";

/// The variable the daemon names the runtime's log file in: a runtime in a substrate box writes
/// what it says there, not to its session, which substrate ends at its output bound.
pub const LOG_VARIABLE: &str = "CONDUCTOR_API_LOG";

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
/// minted, or the socket cannot be bound.
pub fn instance_serve(config_flag: Option<&Path>) -> Result<ExitCode> {
    const WHAT: &str = "instance serve";
    config::set_active(config_flag).context(WHAT)?;
    let active = config::active();
    let state = PathBuf::from(&active.instance.state);
    let sources = dashboard::Sources::active(None).context(WHAT)?;
    let token = match std::env::var(TOKEN_VARIABLE) {
        Ok(token) if !token.trim().is_empty() => token.trim().to_owned(),
        _ => dashboard::mint().context("instance serve: mint the instance's token")?,
    };
    let path = std::env::var_os(SOCKET_VARIABLE)
        .filter(|path| !path.is_empty())
        .map_or_else(|| socket(&state), PathBuf::from);
    if let Some(log) = std::env::var_os(LOG_VARIABLE).filter(|log| !log.is_empty()) {
        log_to(Path::new(&log)).context(WHAT)?;
    }
    let listener = bind(&path).context(WHAT)?;
    println!(
        "conductor instance {}: serving /v1/ on {}",
        active.instance.name.0,
        path.display()
    );
    dashboard::serve_unix(listener, sources, Some(state), token);
    Ok(ExitCode::SUCCESS)
}
