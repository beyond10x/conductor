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
//!
//! A runtime alone in a pod of its own, with no daemon process beside it and no directory it
//! shares, is told so by [`LISTEN_VARIABLE`], an address and port: it serves the instance API on
//! TCP there instead of the socket ([`crate::dashboard::serve_tcp`]), where every request carries
//! its token, reads too, but the liveness probe `GET /healthz`; it refuses to start without a
//! token handed to it. It then seeds its own instance as the unattended daemon seeds each one
//! ([`crate::daemon::unattended::seed_instance`]), each step only where missing, while the API
//! answers and before the loop starts; before the seeding it clones each repository its sources
//! include that has no checkout ([`crate::collect::checkouts::at_start`]), and after it it takes
//! its first snapshot, whatever `cycle.last` says ([`first_snapshot`]).
//! A runtime in a box on the local substrate daemon, where substrate refuses a Unix socket, is
//! told by [`DIAL_VARIABLE`] to bind nothing and dial its API out to the daemon instead
//! ([`crate::dashboard::serve_dial`], [`crate::daemon::dial`]).
//! [`CONFIG_SOURCE_VARIABLE`] names a file it copies over its
//! `--config` file at every start, before it reads it: how a config mounted read-only reaches a
//! place the runtime writes beside.

use std::fs;
use std::net::TcpListener;
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

/// `program` to start, told of no sealed slot ([`SLOTS_VARIABLE`] removed), of no daemon it is
/// to end with ([`PARENT_VARIABLE`] removed) and with no [`TOKEN_VARIABLE`] (removed): every
/// process the runtime starts itself is built here, which `tests/placement.rs` holds for the
/// crate's sources. A caller that hands a child the token, as the daemon hands a runtime its own,
/// sets it after. In a process that works for an instance with `forge_access: anonymous`
/// ([`fetch_nothing_lazily`]) it carries [`NO_LAZY_FETCH_VARIABLE`]`=1`, so that no git it starts,
/// or that a program it starts starts, fetches by itself what a partial clone lacks, and
/// [`crate::collect::SSH_COMMAND`] as plain `ssh` with [`crate::collect::SSH_OPTIONS`], so that no
/// `core.sshCommand` of a configuration and no caller's ssh command reaches it
/// (story:anonymous-pod-no-forge-secret).
#[must_use]
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
    let mut command = std::process::Command::new(program);
    command
        .env_remove(SLOTS_VARIABLE)
        .env_remove(PARENT_VARIABLE)
        .env_remove(TOKEN_VARIABLE);
    if works_for_anonymous() {
        command.env(NO_LAZY_FETCH_VARIABLE, "1").env(
            crate::collect::SSH_COMMAND,
            format!("ssh {}", crate::collect::SSH_OPTIONS),
        );
    }
    command
}

/// Whether this process was marked as one that works for an instance with `forge_access:
/// anonymous` ([`fetch_nothing_lazily`]).
#[must_use]
pub fn works_for_anonymous() -> bool {
    NO_LAZY_FETCH.load(std::sync::atomic::Ordering::Relaxed)
}

/// The variable that, set to `1`, keeps git from fetching what a partial clone lacks from its
/// promisor remote on demand: a local read (`git show`, `git archive`, `git log -p`) of a blob
/// the clone lacks fails instead of fetching it, outside the one route an anonymous fetch takes.
pub const NO_LAZY_FETCH_VARIABLE: &str = "GIT_NO_LAZY_FETCH";

/// Whether this process works for an instance with `forge_access: anonymous`
/// ([`fetch_nothing_lazily`]).
static NO_LAZY_FETCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Marks this process as one that works for an instance with `forge_access: anonymous`
/// (story:anonymous-pod-no-forge-secret): from then on every process [`command`] builds carries
/// [`NO_LAZY_FETCH_VARIABLE`]`=1` and plain `ssh`, and the collectors' git environment is the
/// anonymous route's (`collect::git_environment`). Marked where a process learns it: when the
/// config fixes such an instance as the one it runs ([`config::set_active`]), and where a
/// collector's sources are made of one (`collect::blockers::Sources::of`,
/// `collect::repositories::Sources::of`). Nothing takes the mark back.
pub fn fetch_nothing_lazily() {
    NO_LAZY_FETCH.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// The most a sealed token is read to: a token is far shorter, and a slot is not read whole on
/// trust.
const SEALED_TOKEN_BOUND: u64 = 4096;

/// The daemon's token as `slots`, the value of [`SLOTS_VARIABLE`], seals it: read from the
/// descriptor of the slot that carries it, or `None` where no such slot is named
/// ([`sealed_slot`]).
///
/// # Errors
///
/// The slot is named and its descriptor is not a number, is a standard stream, cannot be read,
/// or holds nothing: a runtime that minted its own token then would refuse every write the daemon
/// proxies, silently.
fn sealed_token(slots: &str) -> Result<Option<String>> {
    sealed_slot(slots, crate::placement::is_api_token_slot, "token")
}

/// The instance's model token as `slots` seals it (`story:box-subscription-token`): read from
/// the descriptor of the model-token slot the daemon named for the box
/// ([`crate::placement::is_model_token_slot`]), or `None` where none is named ([`sealed_slot`]).
///
/// # Errors
///
/// The slot is named and its descriptor is not a number, is a standard stream, cannot be read,
/// or holds nothing.
fn sealed_model_token(slots: &str) -> Result<Option<String>> {
    sealed_slot(slots, crate::placement::is_model_token_slot, "model token")
}

/// The value of the first slot `slots`, the value of [`SLOTS_VARIABLE`], names that `wanted`
/// knows, read from its descriptor, trimmed; `None` where no such slot is named. The descriptor
/// is closed once it is read, whatever it held: substrate hands it over without close-on-exec,
/// so every process the runtime starts would inherit it and could read the value from it.
/// Nothing reads it again: the runtime reads it once, at its start, and a restart in the same box
/// is a new process substrate hands a slot of its own. `what` names the value in an error.
///
/// # Errors
///
/// The slot is named and its descriptor is not a number, is a standard stream, cannot be read,
/// or holds nothing.
fn sealed_slot(slots: &str, wanted: fn(&str) -> bool, what: &str) -> Result<Option<String>> {
    use std::io::Read as _;
    let Some((name, descriptor)) = slots
        .split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(name, descriptor)| (name.trim(), descriptor.trim()))
        .find(|(name, _)| wanted(name))
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
            let mut value = String::new();
            file.take(SEALED_TOKEN_BOUND)
                .read_to_string(&mut value)
                .with_context(|| format!("read the sealed slot {name} at descriptor {descriptor}"))
                .map(|_| value)
        });
    // One the process does not hold is closed already: nothing to answer for.
    let _ = nix::unistd::close(descriptor);
    let value = read?;
    let value = value.trim();
    if value.is_empty() {
        bail!("the sealed slot {name} at descriptor {descriptor} holds no {what}");
    }
    Ok(Some(value.to_owned()))
}

/// The socket's file name in the instance's state directory.
pub const SOCKET: &str = "api.sock";

/// The longest path a Unix socket address holds, less its terminating byte.
const SOCKET_PATH_MAX: usize = 107;

/// The variable the daemon hands the runtime its socket's path in, so both name the same one
/// whatever temporary directory each sees.
pub const SOCKET_VARIABLE: &str = "CONDUCTOR_API_SOCKET";

/// The variable that asks the runtime for its API on TCP at an address and port
/// (`0.0.0.0:<port>`), instead of the socket: a runtime alone in a pod of its own, which the
/// daemon reaches over the network ([`crate::daemon::api::Route::tcp`]). Its presence is also
/// what tells the runtime it runs alone and seeds its own instance.
pub const LISTEN_VARIABLE: &str = "CONDUCTOR_API_LISTEN";

/// The variable that asks the runtime to dial its API out to an address and port (`127.0.0.1:<daemon.dial_port>`),
/// the daemon's dial-in listener ([`crate::daemon::dial`]), instead of binding the socket: a
/// runtime in a substrate box, where a Unix socket is refused (substrate's ADR 0039) and nothing
/// dials in. It keeps [`DIAL_CONNECTIONS`] connections open there, each opened with
/// [`DIAL_HELLO`], the instance's name and its token, and answers one exchange on each
/// ([`crate::dashboard::serve_dial`]); it refuses to start without a token handed to it.
pub const DIAL_VARIABLE: &str = "CONDUCTOR_API_DIAL";

/// The first word of the line a dialed connection opens with.
pub const DIAL_HELLO: &str = "CONDUCTOR-DIAL/1";

/// The line the daemon sends on a dialed connection it admitted and closes because the
/// instance's pool is full ([`crate::daemon::dial::MAX_IDLE`]): a surplus connection, which the
/// runtime does not take for a refusal (`story:dial-refusal-diagnosable`).
pub const DIAL_FULL: &str = "CONDUCTOR-DIAL/1 FULL";

/// How many connections a runtime that dials its API keeps open at once: as many exchanges as it
/// answers at once.
pub const DIAL_CONNECTIONS: usize = 4;

/// The variable that names a config file the runtime copies over its `--config` file at every
/// start, before it reads it, creating the file's directory: a config mounted read-only reaches a
/// place the runtime may write, and what it writes beside the file has somewhere to go.
pub const CONFIG_SOURCE_VARIABLE: &str = "CONDUCTOR_CONFIG_SOURCE";

/// The variable the daemon names the runtime's log file in: a runtime in a substrate box writes
/// what it says there, not to its session, which substrate ends at its output bound. The
/// runtime turns to it before anything else, so an error that ends it at once is in the file.
/// Each line there starts with the time it was written, RFC 3339 ([`log_to`]).
pub const LOG_VARIABLE: &str = "CONDUCTOR_API_LOG";

/// The variable that, set to `off`, has the runtime serve the instance API without the loop: for
/// a case that tests the API alone, or an operator who runs `conductor run` by hand. The daemon
/// never sets it.
pub const LOOP_VARIABLE: &str = "CONDUCTOR_RUNTIME_LOOP";

/// The variable the daemon names its own process in when it starts the runtime as a child
/// process of its own, where no user service manager is
/// (`crate::daemon::units::Children`, `story:container-start-without-prompt`): the runtime ends
/// once its parent is no longer that process, so a daemon that was killed leaves no runtime.
pub const PARENT_VARIABLE: &str = "CONDUCTOR_DAEMON_PID";

/// How often a runtime started as a child looks whether its daemon still runs.
const PARENT_TICK: std::time::Duration = std::time::Duration::from_secs(1);

/// Ends this process once its parent is no longer `daemon` ([`PARENT_VARIABLE`]).
fn end_with(daemon: u32) {
    let _ = std::thread::Builder::new()
        .name("instance-parent".to_owned())
        .spawn(move || {
            while std::os::unix::process::parent_id() == daemon {
                std::thread::sleep(PARENT_TICK);
            }
            crate::run::say(
                true,
                &format!("conductor: instance serve: the daemon, process {daemon}, ended"),
            );
            end_log();
            std::process::exit(0);
        });
}

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
/// created mode 0600 with its directory when missing, each line led by the time it was written
/// (RFC 3339) and a space: both go into a pipe, whose lines a thread writes to the file stamped
/// ([`stamp`]), so what any part of the runtime or a program it starts says carries its time,
/// and can be matched with the daemon's journal (`story:dial-refusal-diagnosable`). Before the
/// process exits both go back to the file and the thread writes what the pipe still holds
/// ([`end_log`]).
///
/// # Errors
///
/// The directory or the file cannot be created, the pipe not opened, or a descriptor not
/// replaced.
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
    let (reader, writer) = std::io::pipe().context("open the log's pipe")?;
    let stamped = file
        .try_clone()
        .with_context(|| format!("open the log {}", log.display()))?;
    nix::unistd::dup2_stdout(&writer)
        .and_then(|()| nix::unistd::dup2_stderr(&writer))
        .with_context(|| format!("write the output to {}", log.display()))?;
    drop(writer);
    let _ = STAMPED.set(Stamped::new(file));
    std::thread::Builder::new()
        .name("instance-log".to_owned())
        .spawn(move || stamp(reader, stamped))
        .context("start the log's writer")?;
    Ok(())
}

/// How long the exit waits for the log's writer to write what the pipe still holds.
const DRAIN: std::time::Duration = std::time::Duration::from_millis(500);

/// The log file [`log_to`] stamps lines into, and whether its writer has written all.
struct Stamped {
    file: fs::File,
    written: std::sync::Mutex<bool>,
    ended: std::sync::Condvar,
}

impl Stamped {
    const fn new(file: fs::File) -> Self {
        Self {
            file,
            written: std::sync::Mutex::new(false),
            ended: std::sync::Condvar::new(),
        }
    }
}

static STAMPED: std::sync::OnceLock<Stamped> = std::sync::OnceLock::new();

/// Writes each line `reader` gives to `file`, led by the time and a space, until the pipe ends.
fn stamp(reader: std::io::PipeReader, mut file: fs::File) {
    use std::io::{BufRead as _, Write as _};
    let mut lines = std::io::BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        match lines.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let _ = file.write_all(&stamped(&line));
    }
    if let Some(stamped) = STAMPED.get() {
        *stamped
            .written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = true;
        stamped.ended.notify_all();
    }
}

/// `line` led by the time now, RFC 3339, and a space, ending in a newline.
#[must_use]
pub fn stamped(line: &[u8]) -> Vec<u8> {
    let at = crate::records::now().map(|at| at.0).unwrap_or_default();
    let mut out = Vec::with_capacity(at.len() + 1 + line.len() + 1);
    out.extend_from_slice(at.as_bytes());
    out.push(b' ');
    out.extend_from_slice(line);
    if !line.ends_with(b"\n") {
        out.push(b'\n');
    }
    out
}

/// Before the process exits, where [`log_to`] stamps its lines: standard output and error back to
/// the log file, which ends the pipe unless a program the runtime started holds it, then up to
/// [`DRAIN`] for the writer to write the rest. Elsewhere it does nothing.
pub fn end_log() {
    let Some(stamped) = STAMPED.get() else {
        return;
    };
    let _ = nix::unistd::dup2_stdout(&stamped.file);
    let _ = nix::unistd::dup2_stderr(&stamped.file);
    let written = stamped
        .written
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _ = stamped
        .ended
        .wait_timeout_while(written, DRAIN, |written| !*written);
}

/// Copies the file [`CONFIG_SOURCE_VARIABLE`] names, where it names one, over the config file
/// the runtime reads (`config_flag`, else `CONDUCTOR_CONFIG`, else the default file), creating
/// its directory. The copy is written mode 0600 beside the file and renamed over it, so a file
/// the runtime or a person changed since is replaced whole, whatever its mode.
///
/// # Errors
///
/// The source does not read, or the file or its directory cannot be written.
fn copy_config_source(config_flag: Option<&Path>) -> Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let Some(source) = std::env::var_os(CONFIG_SOURCE_VARIABLE).filter(|source| !source.is_empty())
    else {
        return Ok(());
    };
    let source = PathBuf::from(source);
    let environment = config::Environment::from_process()?;
    let file = environment
        .cwd
        .join(config::locate(config_flag, &environment)?.0);
    let text = fs::read(&source).with_context(|| {
        format!(
            "read the config source {} ({CONFIG_SOURCE_VARIABLE})",
            source.display()
        )
    })?;
    let dir = file.parent().unwrap_or_else(|| Path::new("/"));
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let name = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    let partial = dir.join(format!(".{name}.source"));
    let _ = fs::remove_file(&partial);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&partial)
        .and_then(|mut out| out.write_all(&text).and_then(|()| out.sync_all()))
        .and_then(|()| fs::rename(&partial, &file))
        .with_context(|| {
            format!(
                "copy the config source {} over {}",
                source.display(),
                file.display()
            )
        })
}

/// How long the snapshot a runtime alone in a pod takes at its start may run, as long as a step
/// of the cycle.
const START_SNAPSHOT_BOUND: std::time::Duration = std::time::Duration::from_secs(3600);

/// A runtime alone in a pod seeds its own instance, the active one of the config file, as the
/// unattended daemon seeds each one, then takes its first snapshot ([`first_snapshot`]); what
/// failed is said, and the runtime goes on.
fn seed_alone(config_flag: Option<&Path>, state_flag: Option<&Path>, what: &str) {
    const SEEDING: &str = "conductor: instance serve: seeding";
    let loaded = config::Environment::from_process().and_then(|environment| {
        let loaded = config::load(config_flag, &environment)?;
        Ok((loaded, environment.home()?.to_owned()))
    });
    let (loaded, home) = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            crate::run::say(true, &format!("{SEEDING}: {error:#}"));
            return;
        }
    };
    let name = &config::active().instance.name;
    let Some(index) = loaded
        .config
        .instances
        .iter()
        .position(|instance| &instance.name == name)
    else {
        return;
    };
    // The repositories its sources include are cloned before the seeding, so the workspace
    // file, the trust and the activation it derives see them (`story:pod-source-checkouts`).
    let instance = &loaded.config.instances[index];
    let cloned = crate::collect::checkouts::at_start(instance, what);
    let seeded = crate::daemon::unattended::seed_instance(&loaded, index, &home, SEEDING);
    // A seeding that found its records written writes no workspace file of its own.
    if cloned > 0
        && let Err(error) = crate::collect::checkouts::refresh(instance)
    {
        crate::run::say(
            true,
            &format!(
                "{SEEDING}: the workspace file and the trust of the new checkouts: {}",
                crate::collect::checkouts::redacted(&format!("{error:#}"))
            ),
        );
    }
    first_snapshot(config_flag, state_flag, what);
    if seeded {
        crate::run::say(false, &format!("{what}: seeded its instance"));
    } else {
        crate::run::say(
            true,
            &format!(
                "{what}: the seeding failed in the steps above; the runtime serves and runs its \
                 loop, and its next start tries each step again"
            ),
        );
    }
}

/// The first collect pass of a runtime alone in a pod, at its start, after its clones and its
/// seeding (`story:pod-source-checkouts`): `snapshot start-snapshot`, a run of this binary with
/// the runtime's `--config` and `--state-dir`, as the cycle's first step runs it, whatever
/// `cycle.last` says, so health answers a snapshot newer than the start and lists each clone
/// that failed at it, which this pass tries again. It is no cycle: `cycle.last` keeps its stamp,
/// and nothing else of a cycle runs (the board, `records check`, dispatch v1). How it ended is
/// said, and the start goes on.
fn first_snapshot(config_flag: Option<&Path>, state_flag: Option<&Path>, what: &str) {
    let program = match std::env::current_exe() {
        Ok(program) => program,
        Err(error) => {
            crate::run::say(
                true,
                &format!("{what}: no snapshot at its start: read this binary's path: {error}"),
            );
            return;
        }
    };
    let mut command = command(&program);
    if let Some(config) = config_flag {
        command.arg("--config").arg(config);
    }
    if let Some(state) = state_flag {
        command.arg("--state-dir").arg(state);
    }
    command
        .args(["snapshot", "start-snapshot"])
        .env(crate::actor::DAEMON_VARIABLE, "1");
    let said = match crate::collect::run(&mut command, START_SNAPSHOT_BOUND) {
        Ok(output) if output.status.success() => {
            crate::run::say(false, &format!("{what}: took a snapshot at its start"));
            return;
        }
        Ok(output) => crate::confine::exited(&output),
        Err(error) => crate::collect::checkouts::redacted(&format!("{error:#}")),
    };
    crate::run::say(
        true,
        &format!(
            "{what}: the snapshot at its start {said}; health answers the newest snapshot until \
             the next cycle"
        ),
    );
}

/// Where the instance API is served: the socket, TCP for a runtime alone in a pod, or the
/// connections a runtime in a box dials out to the daemon.
enum Listener {
    Unix(UnixListener),
    Tcp(TcpListener),
    Dial(String),
}

/// `conductor instance serve`, as the module documentation says.
///
/// # Errors
///
/// The config source does not copy, the config file does not load, the home directory is not
/// absolute, the token cannot be minted or is not handed to a runtime on TCP, the socket or the
/// TCP address cannot be bound, or the loop cannot start ([`crate::run::in_runtime`]).
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
    if let Some(daemon) = std::env::var(PARENT_VARIABLE)
        .ok()
        .and_then(|pid| pid.trim().parse::<u32>().ok())
    {
        end_with(daemon);
    }
    copy_config_source(config_flag).context(WHAT)?;
    config::set_active(config_flag).context(WHAT)?;
    let active = config::active();
    let state = PathBuf::from(&active.instance.state);
    let sources = dashboard::Sources::active(None).context(WHAT)?;
    let sealed = match std::env::var(SLOTS_VARIABLE) {
        Ok(slots) => {
            // The model token's slot is read and its descriptor closed whatever the instance's
            // `model_token_kind`, so no process the runtime starts inherits it; the runtime
            // hands its value to an agent's harness for a `subscription_token` only
            // (`story:box-subscription-token`, [`crate::token::hold_sealed_model_token`]).
            if let Some(value) =
                sealed_model_token(&slots).context("instance serve: the model token")?
            {
                crate::token::hold_sealed_model_token(value);
            }
            sealed_token(&slots).context("instance serve: the daemon's token")?
        }
        Err(_) => None,
    };
    let given = match (std::env::var(TOKEN_VARIABLE), sealed) {
        (Ok(token), _) if !token.trim().is_empty() => Some(token.trim().to_owned()),
        (_, sealed) => sealed,
    };
    let listen = std::env::var(LISTEN_VARIABLE)
        .ok()
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty());
    let dial = std::env::var(DIAL_VARIABLE)
        .ok()
        .map(|address| address.trim().to_owned())
        .filter(|address| !address.is_empty());
    let token = match (given, &listen, &dial) {
        (Some(token), _, _) => token,
        (None, Some(_), _) => bail!(
            "{WHAT}: {LISTEN_VARIABLE} asks for the API on TCP, and {TOKEN_VARIABLE} hands it no \
             token: every request on TCP carries one"
        ),
        (None, None, Some(_)) => bail!(
            "{WHAT}: {DIAL_VARIABLE} asks for the API dialed out to the daemon, and neither \
             {TOKEN_VARIABLE} nor a sealed slot hands it the daemon's token, which every dialed \
             connection opens with: declare the slot {} on the substrate daemon",
            crate::placement::api_token_slot(&active.instance.name.0)
        ),
        (None, None, None) => {
            dashboard::mint().context("instance serve: mint the instance's token")?
        }
    };
    let what = format!("conductor instance {}", active.instance.name.0);
    let listener = if let Some(address) = &dial {
        println!("{what}: serving /v1/ over connections dialed to {address}");
        Listener::Dial(address.clone())
    } else if let Some(address) = &listen {
        let listener = TcpListener::bind(address.as_str())
            .with_context(|| format!("{WHAT}: bind {address} ({LISTEN_VARIABLE})"))?;
        let at = listener
            .local_addr()
            .map_or_else(|_| address.clone(), |at| at.to_string());
        println!("{what}: serving /v1/ on {at}");
        Listener::Tcp(listener)
    } else {
        let path = std::env::var_os(SOCKET_VARIABLE)
            .filter(|path| !path.is_empty())
            .map_or_else(|| socket(&state), PathBuf::from);
        let listener = bind(&path).context(WHAT)?;
        println!("{what}: serving /v1/ on {}", path.display());
        Listener::Unix(listener)
    };
    let instance = active.instance.name.0.clone();
    let api = std::thread::Builder::new()
        .name("instance-api".to_owned())
        .spawn(move || {
            // The runtime is the API and the loop together: an API that ended, or panicked, ends
            // the process, so the service manager restarts both.
            struct Ended;
            impl Drop for Ended {
                fn drop(&mut self) {
                    crate::run::say(true, "conductor: instance serve: the instance API ended");
                    end_log();
                    std::process::exit(1);
                }
            }
            let _ended = Ended;
            match listener {
                Listener::Unix(listener) => {
                    dashboard::serve_unix(listener, sources, Some(state), token);
                }
                Listener::Tcp(listener) => {
                    dashboard::serve_tcp(listener, sources, Some(state), token);
                }
                Listener::Dial(address) => {
                    dashboard::serve_dial(
                        &address,
                        &instance,
                        sources,
                        Some(state),
                        token,
                        DIAL_CONNECTIONS,
                    );
                }
            }
        })
        .context("instance serve: start the instance API")?;
    if listen.is_some() {
        seed_alone(config_flag, state_flag, &what);
    }
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

/// `conductor instance cycle` (`story:cycle-now`): asks the daemon of the config file for a cycle
/// of this process's instance now, the one the global `--config` and `CONDUCTOR_INSTANCE` name
/// ([`crate::daemon::cycle::ask`]).
///
/// # Errors
///
/// The config does not load, the daemon is not reached, or it or the runtime refuses the ask.
pub fn instance_cycle(config_flag: Option<&Path>) -> Result<ExitCode> {
    crate::daemon::cycle::ask(config_flag, &config::active().instance.name.0)
}
