//! The dial-in listener (`daemon.dial_port`): how conductord reaches the instance API of a runtime
//! in a box on the local substrate daemon. Substrate refuses a Unix socket inside a box (its ADR
//! 0039), and a local box serves no ingress, so such a runtime binds nothing; it dials out to this
//! listener on `127.0.0.1:<daemon.dial_port>` through the egress aperture [`APERTURE`] instead
//! ([`crate::instance::DIAL_VARIABLE`]).
//!
//! Each connection opens with one line, [`crate::instance::DIAL_HELLO`], the instance's name and
//! the token the daemon minted for its runtime, separated by single spaces. A line that does not
//! read within [`HELLO_TIMEOUT`], names an instance the daemon does not route by dial-in, or
//! carries another token closes the connection unanswered ([`super::api::Api::admit_dial`]): a
//! process on the host that dials in as an instance would otherwise receive the writes proxied to
//! it, the token among them. An admitted connection waits idle in the instance's pool ([`POOL`])
//! until a proxied request takes it ([`Pool::take`]) for one exchange: the request, the end of
//! the daemon's writes, the answer, the close. The runtime dials a new one for each it answered.
//!
//! The daemon says in its journal why it closed a dialed connection ([`Closed`]), a repeat once
//! until the cause changes ([`Said`]); it tells the connection nothing but that the pool is full
//! ([`crate::instance::DIAL_FULL`]), which only an admitted one learns
//! (`story:dial-refusal-diagnosable`).

use std::collections::BTreeMap;
use std::io::{self, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use super::api::{Admission, Api};

/// The egress aperture a runtime in a local box dials conductord through: the operator declares
/// it on the substrate daemon as `--egress-aperture conductord=127.0.0.1:<daemon.dial_port>/tcp`.
pub const APERTURE: &str = "conductord";

/// How long a dialed connection may take to send its first line.
pub const HELLO_TIMEOUT: Duration = Duration::from_secs(5);

/// The longest first line read.
const HELLO_MAX: usize = 1024;

/// The most idle connections one instance keeps in the pool; one past it is closed.
pub const MAX_IDLE: usize = 16;

/// The idle connections of every instance by its name, which the dial-in listener fills and
/// the proxy takes from.
#[derive(Default)]
pub struct Pool {
    idle: Mutex<BTreeMap<String, Vec<TcpStream>>>,
    arrived: Condvar,
}

/// The daemon's one pool.
pub static POOL: LazyLock<Pool> = LazyLock::new(Pool::default);

impl Pool {
    /// Keeps `stream`, dialed in by the runtime of `instance`, until a request takes it; hands it
    /// back when the instance holds [`MAX_IDLE`] already.
    #[must_use = "a connection handed back is surplus, to be told so and closed"]
    pub fn keep(&self, instance: &str, stream: TcpStream) -> Option<TcpStream> {
        let mut idle = self.idle.lock().unwrap_or_else(PoisonError::into_inner);
        let held = idle.entry(instance.to_owned()).or_default();
        held.retain(alive);
        if held.len() >= MAX_IDLE {
            return Some(stream);
        }
        held.push(stream);
        drop(idle);
        self.arrived.notify_all();
        None
    }

    /// An idle connection of `instance` that is still open, waiting at most `within` for one to
    /// arrive.
    ///
    /// # Errors
    ///
    /// None arrived within `within`.
    pub fn take(&self, instance: &str, within: Duration) -> Result<TcpStream> {
        let deadline = Instant::now() + within;
        let mut idle = self.idle.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if let Some(held) = idle.get_mut(instance) {
                // The newest first: an older one is the likelier to have been closed.
                while let Some(stream) = held.pop() {
                    if alive(&stream) {
                        return Ok(stream);
                    }
                }
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                bail!(
                    "the runtime of {instance} holds no connection dialed in to the daemon \
                     (daemon.dial_port): it does not run, or cannot reach the aperture {APERTURE}"
                );
            }
            idle = self
                .arrived
                .wait_timeout(idle, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    /// How many idle connections `instance` holds.
    #[must_use]
    pub fn idle(&self, instance: &str) -> usize {
        self.idle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(instance)
            .map_or(0, Vec::len)
    }
}

/// Whether `stream`, an idle connection, is still open: the runtime sends nothing on it until it
/// is asked, so anything to read is its end.
fn alive(stream: &TcpStream) -> bool {
    if stream.set_nonblocking(true).is_err() {
        return false;
    }
    let mut byte = [0_u8; 1];
    let open =
        matches!(stream.peek(&mut byte), Err(error) if error.kind() == io::ErrorKind::WouldBlock);
    open && stream.set_nonblocking(false).is_ok()
}

/// Why the dial-in listener closed a dialed connection, as conductord's journal says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Closed {
    /// Its first line was no greeting, or did not arrive within [`HELLO_TIMEOUT`]: what [`hello`]
    /// said.
    Greeting(String),
    /// It names an instance the daemon does not route by dial-in.
    NotRouted(String),
    /// It names an instance routed by dial-in with another token than the one the daemon minted
    /// for its runtime's start.
    WrongToken(String),
    /// It was admitted, and the instance holds [`MAX_IDLE`] idle connections already.
    Full(String),
}

impl Closed {
    /// What a repeat is told apart by ([`Said`]): the instance a token was checked for, else
    /// none, since any process may send any name.
    fn key(&self) -> &str {
        match self {
            Self::Greeting(_) | Self::NotRouted(_) => "",
            Self::WrongToken(instance) | Self::Full(instance) => instance,
        }
    }
}

impl std::fmt::Display for Closed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Greeting(why) => write!(formatter, "no dial-in greeting: {why}"),
            Self::NotRouted(instance) => write!(
                formatter,
                "{instance:?}: the daemon does not route it by dial-in: no such instance, or its \
                 runtime is not started in a local box or not seen running"
            ),
            Self::WrongToken(instance) => write!(
                formatter,
                "{instance}: not the token the daemon minted for its runtime's start: a runtime \
                 of an earlier start may still be dialing"
            ),
            Self::Full(instance) => write!(
                formatter,
                "{instance}: the pool is full, {MAX_IDLE} idle connections: a surplus one, told \
                 so"
            ),
        }
    }
}

/// The last cause said per [`Closed::key`]: each said once until it changes.
#[derive(Debug, Default)]
pub struct Said(Mutex<BTreeMap<String, String>>);

impl Said {
    /// The journal line of `closed`, unless it is the line said last for its key.
    pub fn first(&self, closed: &Closed) -> Option<String> {
        let line = format!("conductord: dial-in closed: {closed}");
        let mut said = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        if said.get(closed.key()) == Some(&line) {
            return None;
        }
        said.insert(closed.key().to_owned(), line.clone());
        Some(line)
    }
}

/// What the dial-in listener says, a line at a time.
pub type Say = Arc<dyn Fn(&str) + Send + Sync>;

/// Takes every connection `listener` accepts, each on a thread of its own: admitted by its first
/// line ([`hello`], [`Api::admit_dial`]), it waits in [`POOL`]; any other is closed, and why goes
/// to standard error, the daemon's journal ([`serve_saying`]).
pub fn serve(listener: &TcpListener, api: &Arc<Api>) {
    serve_saying(listener, api, Arc::new(|line: &str| eprintln!("{line}")));
}

/// [`serve`], saying each closed connection's cause through `say`, once until it changes.
pub fn serve_saying(listener: &TcpListener, api: &Arc<Api>, say: Say) {
    let said = Arc::new(Said::default());
    for stream in listener.incoming().flatten() {
        let (api, said, say) = (Arc::clone(api), Arc::clone(&said), Arc::clone(&say));
        std::thread::spawn(move || {
            if let Err(closed) = admit(stream, &api)
                && let Some(line) = said.first(&closed)
            {
                say(&line);
            }
        });
    }
}

/// Admits `stream` to [`POOL`] by its first line, or answers why it is closed; a surplus one is
/// told [`crate::instance::DIAL_FULL`] first.
fn admit(mut stream: TcpStream, api: &Api) -> Result<(), Closed> {
    let (greeting, instance, token) =
        greeting(&mut stream).map_err(|error| Closed::Greeting(format!("{error:#}")))?;
    match api.admit_dial(&instance, &token) {
        Admission::Admitted => {}
        Admission::NotRouted => return Err(Closed::NotRouted(instance)),
        Admission::WrongToken => return Err(Closed::WrongToken(instance)),
    }
    if greeting == Greeting::Pool {
        // One call to the host's token pool (story:host-token-pool), for the instance this
        // connection was admitted as, never one its request names.
        if let Some(target) = api.pool_target() {
            let operators = api.operators();
            let daemon = crate::token::Daemon {
                file: &target.0,
                store: &target.1,
                operators: &operators,
            };
            let runtime = crate::token::runtime_tag(&token);
            if let Err(error) =
                crate::token::serve_pool_call(&mut stream, &daemon, &instance, &runtime)
            {
                eprintln!("conductord: {instance}: a token pool call failed: {error}");
            }
        } else if let Err(error) = crate::token::serve_no_pool(&mut stream) {
            // A conductord that serves no config file holds no pool: the call is answered
            // empty, so a start leases nothing and a stop releases nothing, rather than closed
            // unanswered, which a runtime takes for a refusal.
            eprintln!("conductord: {instance}: a token pool call failed: {error}");
        }
        return Ok(());
    }
    stream
        .set_read_timeout(None)
        .map_err(|error| Closed::Greeting(format!("{error}")))?;
    match POOL.keep(&instance, stream) {
        None => Ok(()),
        Some(mut surplus) => {
            let _ = writeln!(surplus, "{}", crate::instance::DIAL_FULL);
            Err(Closed::Full(instance))
        }
    }
}

/// What a dialed connection's first line opens: an idle connection of the instance API, or one
/// call to the host's token pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Greeting {
    /// [`crate::instance::DIAL_HELLO`].
    Dial,
    /// [`crate::instance::POOL_HELLO`].
    Pool,
}

/// The greeting, the instance and the token of a dialed connection's first line.
///
/// # Errors
///
/// As [`hello`], or with [`crate::instance::POOL_HELLO`] in place of
/// [`crate::instance::DIAL_HELLO`].
pub fn greeting(stream: &mut TcpStream) -> Result<(Greeting, String, String)> {
    let line = first_line(stream)?;
    if let Some(rest) = line
        .strip_prefix(crate::instance::POOL_HELLO)
        .and_then(|rest| rest.strip_prefix(' '))
    {
        let (instance, token) = parse_hello(&format!("{} {rest}", crate::instance::DIAL_HELLO))?;
        return Ok((Greeting::Pool, instance, token));
    }
    let (instance, token) = parse_hello(&line)?;
    Ok((Greeting::Dial, instance, token))
}

/// The instance and the token of a dialed connection's first line.
///
/// # Errors
///
/// The line does not arrive within [`HELLO_TIMEOUT`], is longer than [`HELLO_MAX`], or is not
/// [`crate::instance::DIAL_HELLO`] followed by a name and a token.
pub fn hello(stream: &mut TcpStream) -> Result<(String, String)> {
    parse_hello(&first_line(stream)?)
}

/// A dialed connection's first line, without its newline.
fn first_line(stream: &mut TcpStream) -> Result<String> {
    stream.set_read_timeout(Some(HELLO_TIMEOUT))?;
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    // Byte by byte: what follows the line is the runtime's to answer, never read here.
    loop {
        let read = match stream.read(&mut byte) {
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                bail!(
                    "its first line did not arrive within {} s",
                    HELLO_TIMEOUT.as_secs()
                )
            }
            read => read?,
        };
        if read == 0 {
            bail!("the connection closed before its first line");
        }
        if byte[0] == b'\n' {
            break;
        }
        line.push(byte[0]);
        if line.len() > HELLO_MAX {
            bail!("the first line is longer than {HELLO_MAX} bytes");
        }
    }
    Ok(String::from_utf8_lossy(&line).into_owned())
}

/// The instance and the token of `line`, a dialed connection's first line without its newline.
///
/// # Errors
///
/// It is not [`crate::instance::DIAL_HELLO`], a name and a token, separated by single spaces.
pub fn parse_hello(line: &str) -> Result<(String, String)> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let mut words = line.split(' ');
    match (words.next(), words.next(), words.next(), words.next()) {
        (Some(crate::instance::DIAL_HELLO), Some(instance), Some(token), None)
            if !instance.is_empty() && !token.is_empty() =>
        {
            Ok((instance.to_owned(), token.to_owned()))
        }
        _ => bail!("the first line is no dial-in greeting"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_greeting_names_the_instance_and_the_token() {
        let line = format!("{} alpha s3cret", crate::instance::DIAL_HELLO);
        assert_eq!(
            parse_hello(&line).unwrap(),
            ("alpha".to_owned(), "s3cret".to_owned())
        );
        assert_eq!(
            parse_hello(&format!("{line}\r")).unwrap(),
            ("alpha".to_owned(), "s3cret".to_owned())
        );
        for refused in [
            String::new(),
            "GET / HTTP/1.1".to_owned(),
            format!("{} alpha", crate::instance::DIAL_HELLO),
            format!("{} alpha s3cret extra", crate::instance::DIAL_HELLO),
            format!("{}  s3cret", crate::instance::DIAL_HELLO),
        ] {
            assert!(parse_hello(&refused).is_err(), "{refused:?} was taken");
        }
    }

    #[test]
    fn the_pool_hands_out_an_open_connection_and_drops_a_closed_one() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let pool = Pool::default();
        let closed = TcpStream::connect(address).unwrap();
        let (accepted_closed, _) = listener.accept().unwrap();
        drop(closed);
        let mut open = TcpStream::connect(address).unwrap();
        let (accepted_open, _) = listener.accept().unwrap();
        // The closed one's end has to arrive before the pool looks.
        std::thread::sleep(Duration::from_millis(50));
        assert!(pool.keep("pool-case", accepted_open).is_none());
        assert!(pool.keep("pool-case", accepted_closed).is_none());
        let mut taken = pool.take("pool-case", Duration::from_secs(1)).unwrap();
        taken.write_all(b"x").unwrap();
        let mut byte = [0_u8; 1];
        open.read_exact(&mut byte).unwrap();
        assert_eq!(&byte, b"x", "the pool handed out the closed connection");
        assert_eq!(pool.idle("pool-case"), 0);
        let error = pool
            .take("pool-case", Duration::from_millis(50))
            .unwrap_err();
        assert!(error.to_string().contains(APERTURE), "{error}");
    }

    #[test]
    fn the_pool_keeps_at_most_its_bound() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let pool = Pool::default();
        let mut dialed = Vec::new();
        let mut surplus = 0;
        for _ in 0..=MAX_IDLE {
            dialed.push(TcpStream::connect(address).unwrap());
            surplus += usize::from(
                pool.keep("bound-case", listener.accept().unwrap().0)
                    .is_some(),
            );
        }
        assert_eq!(pool.idle("bound-case"), MAX_IDLE);
        assert_eq!(surplus, 1, "the one past the bound is handed back");
    }

    #[test]
    fn a_cause_is_said_once_until_it_changes() {
        let said = Said::default();
        let wrong = Closed::WrongToken("alpha".to_owned());
        let full = Closed::Full("alpha".to_owned());
        let line = said.first(&wrong).expect("said the first time");
        assert!(line.contains("alpha") && line.contains("token"), "{line}");
        assert_eq!(said.first(&wrong), None);
        assert!(said.first(&full).expect("a new cause").contains("full"));
        assert!(said.first(&wrong).is_some(), "changed back: said again");
        // Another instance's cause is its own.
        assert!(said.first(&Closed::WrongToken("beta".to_owned())).is_some());
        assert!(said.first(&wrong).is_none());
        // A name no token was checked for shares one key: any process may send any name.
        assert!(said.first(&Closed::NotRouted("x1".to_owned())).is_some());
        assert!(said.first(&Closed::NotRouted("x2".to_owned())).is_some());
        assert!(said.first(&Closed::NotRouted("x2".to_owned())).is_none());
    }
}
