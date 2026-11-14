//! The dial-in listener (`daemon.dial_port`): how conductord reaches the instance API of a runtime
//! in a box on the local substrate daemon. Substrate refuses a Unix socket inside a box (its ADR
//! 0039), and a local box serves no ingress, so such a runtime binds nothing; it dials out to this
//! listener on `127.0.0.1:<daemon.dial_port>` through the egress aperture [`APERTURE`] instead
//! ([`crate::instance::DIAL_VARIABLE`]).
//!
//! Each connection opens with one line, [`crate::instance::DIAL_HELLO`], the instance's name and
//! the token the daemon minted for its runtime, separated by single spaces. A line that does not
//! read within [`HELLO_TIMEOUT`], names an instance the daemon does not route by dial-in, or
//! carries another token closes the connection unanswered ([`super::api::Api::admits_dial`]): a
//! process on the host that dials in as an instance would otherwise receive the writes proxied to
//! it, the token among them. An admitted connection waits idle in the instance's pool ([`POOL`])
//! until a proxied request takes it ([`Pool::take`]) for one exchange: the request, the end of
//! the daemon's writes, the answer, the close. The runtime dials a new one for each it answered.

use std::collections::BTreeMap;
use std::io::{self, Read as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use super::api::Api;

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
    /// Keeps `stream`, dialed in by the runtime of `instance`, until a request takes it; closes
    /// it when the instance holds [`MAX_IDLE`] already.
    pub fn keep(&self, instance: &str, stream: TcpStream) {
        let mut idle = self.idle.lock().unwrap_or_else(PoisonError::into_inner);
        let held = idle.entry(instance.to_owned()).or_default();
        held.retain(alive);
        if held.len() >= MAX_IDLE {
            return;
        }
        held.push(stream);
        drop(idle);
        self.arrived.notify_all();
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

/// Takes every connection `listener` accepts, each on a thread of its own: admitted by its first
/// line ([`hello`], [`Api::admits_dial`]), it waits in [`POOL`]; any other is closed.
pub fn serve(listener: &TcpListener, api: &Arc<Api>) {
    for stream in listener.incoming().flatten() {
        let api = Arc::clone(api);
        std::thread::spawn(move || {
            let mut stream = stream;
            let Ok((instance, token)) = hello(&mut stream) else {
                return;
            };
            if !api.admits_dial(&instance, &token) {
                return;
            }
            if stream.set_read_timeout(None).is_ok() {
                POOL.keep(&instance, stream);
            }
        });
    }
}

/// The instance and the token of a dialed connection's first line.
///
/// # Errors
///
/// The line does not arrive within [`HELLO_TIMEOUT`], is longer than [`HELLO_MAX`], or is not
/// [`crate::instance::DIAL_HELLO`] followed by a name and a token.
pub fn hello(stream: &mut TcpStream) -> Result<(String, String)> {
    stream.set_read_timeout(Some(HELLO_TIMEOUT))?;
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    // Byte by byte: what follows the line is the runtime's to answer, never read here.
    loop {
        if stream.read(&mut byte)? == 0 {
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
    parse_hello(&String::from_utf8_lossy(&line))
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
    use std::io::Write as _;

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
        pool.keep("pool-case", accepted_open);
        pool.keep("pool-case", accepted_closed);
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
        for _ in 0..=MAX_IDLE {
            dialed.push(TcpStream::connect(address).unwrap());
            pool.keep("bound-case", listener.accept().unwrap().0);
        }
        assert_eq!(pool.idle("bound-case"), MAX_IDLE);
    }
}
