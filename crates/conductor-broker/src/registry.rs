//! The sessions a broker serves: one registration file each, `sessions/<session>.json` under the
//! broker's directory, and the socket it serves the session on, `sockets/<session>.sock`.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use crate::grant::SessionGrant;

/// One session the broker serves (`conductor.tool.ToolSession`): its name, its instance, the
/// directory its verbs read in, its grant, and when it started (RFC 3339).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub session: String,
    pub instance: String,
    pub workdir: PathBuf,
    pub grant: SessionGrant,
    pub started_at: String,
}

/// The broker's directory under an instance's state directory: `<state>/broker`.
#[must_use]
pub fn broker_dir(state: &Path) -> PathBuf {
    state.join("broker")
}

/// The socket the broker serves `session` on.
#[must_use]
pub fn socket_path(dir: &Path, session: &str) -> PathBuf {
    dir.join("sockets").join(format!("{session}.sock"))
}

pub(crate) fn sessions_dir(dir: &Path) -> PathBuf {
    dir.join("sessions")
}

pub(crate) fn control_path(dir: &Path) -> PathBuf {
    dir.join("control.sock")
}

fn registration_path(dir: &Path, session: &str) -> PathBuf {
    sessions_dir(dir).join(format!("{session}.json"))
}

/// A session name is one path word: no separator, no leading dot, not empty.
pub(crate) fn well_formed(session: &str) -> bool {
    !session.is_empty()
        && !session.starts_with('.')
        && session
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

impl Registration {
    fn to_json(&self) -> Value {
        json!({
            "session": self.session,
            "instance": self.instance,
            "workdir": self.workdir.to_string_lossy(),
            "grant": self.grant.to_json(),
            "started_at": self.started_at,
        })
    }

    fn from_json(value: &Value) -> Result<Self, String> {
        let text = |key: &str| {
            value[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("registration has no text {key}"))
        };
        Ok(Self {
            session: text("session")?,
            instance: text("instance")?,
            workdir: PathBuf::from(text("workdir")?),
            grant: SessionGrant::from_json(&value["grant"])?,
            started_at: text("started_at")?,
        })
    }
}

/// Registers a session with the broker of the directory `dir`, and answers the socket it is
/// served on: writes its registration, then asks a broker serving `dir`, if one does, to open
/// the socket now. A broker that starts later opens it at its start.
///
/// # Errors
///
/// When the session's name is not one path word, or the registration cannot be written.
pub fn register(dir: &Path, registration: &Registration) -> io::Result<PathBuf> {
    if !well_formed(&registration.session) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{:?} is not a session name", registration.session),
        ));
    }
    fs::create_dir_all(sessions_dir(dir))?;
    fs::create_dir_all(dir.join("sockets"))?;
    let path = registration_path(dir, &registration.session);
    let partial = path.with_extension("json.partial");
    fs::write(&partial, registration.to_json().to_string())?;
    fs::rename(&partial, &path)?;
    let _ = ask_control(dir, &json!({"open": registration.session}));
    Ok(socket_path(dir, &registration.session))
}

/// Removes a session's registration: its next call is refused `UnknownSession`. Removing one that
/// is not there is not an error.
///
/// # Errors
///
/// When the registration exists and cannot be removed.
pub fn unregister(dir: &Path, session: &str) -> io::Result<()> {
    if !well_formed(session) {
        return Ok(());
    }
    match fs::remove_file(registration_path(dir, session)) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

/// The registration of `session`, if there is one.
pub(crate) fn read(dir: &Path, session: &str) -> Option<Result<Registration, String>> {
    let text = fs::read_to_string(registration_path(dir, session)).ok()?;
    Some(
        serde_json::from_str::<Value>(&text)
            .map_err(|error| error.to_string())
            .and_then(|value| Registration::from_json(&value)),
    )
}

/// The names of every registered session.
pub(crate) fn sessions(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(sessions_dir(dir)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_name()
                .to_str()
                .and_then(|name| name.strip_suffix(".json"))
                .filter(|name| well_formed(name))
                .map(str::to_owned)
        })
        .collect();
    names.sort();
    names
}

/// Sends one request to the control socket of a broker serving `dir`, and answers its reply.
fn ask_control(dir: &Path, request: &Value) -> io::Result<Value> {
    let mut stream = UnixStream::connect(control_path(dir))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    writeln!(stream, "{request}")?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line)?;
    serde_json::from_str(&line).map_err(io::Error::other)
}
