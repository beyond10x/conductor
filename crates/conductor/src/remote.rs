//! A command against a daemon on another box (`story:daemon-remote`, design architecture-next
//! § 4): the global `--daemon <url>`, or else `CONDUCTOR_DAEMON` ([`VARIABLE`]), names the
//! daemon a command talks to; without either a command works on this box, the default. A command
//! with a route on the daemon's global API goes through it with the bearer of this client's login
//! ([`crate::login::bearer`]): `config put` (`PUT /v1/config`) and `instance upgrade`
//! (`POST /v1/instances/{name}/upgrade`), and `login` itself. Any other command given `--daemon`
//! is refused by name ([`refuse_unrouted`]); `CONDUCTOR_DAEMON` is read by the routed commands
//! only, so a shell that sets it keeps every other command working on its own box.
//!
//! A write the daemon refuses is one line of standard error naming the specification's error:
//! `conductor.operator.GrantMissing` with the grant, `ActorNotHuman`, or the refusal of the
//! change itself. No bearer is printed.

use std::net::TcpStream;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use rustls::pki_types::ServerName;
use serde_json::{Value, json};

use crate::cli::{InstanceUpgradeArgs, PutConfigArgs};

/// The variable that names the daemon when no `--daemon` is given.
pub const VARIABLE: &str = "CONDUCTOR_DAEMON";

/// The leaves that take `--daemon`: each has a route on the global API, or is `login`; `tui`
/// reads the global API (`story:tui-readonly`).
pub const ROUTED: [&[&str]; 6] = [
    &["login"],
    &["tui"],
    &["join"],
    &["leave"],
    &["config", "put"],
    &["instance", "upgrade"],
];

/// How long a request to a daemon or an Identity service may take.
const TIMEOUT: Duration = Duration::from_secs(30);

/// How often `instance upgrade` looks at the instance it upgrades.
const POLL: Duration = Duration::from_secs(2);

/// The daemon a routed command talks to: the `--daemon` flag, else [`VARIABLE`], else none (this
/// box).
#[must_use]
pub fn chosen(flag: Option<&str>) -> Option<String> {
    flag.map(str::to_owned)
        .or_else(|| std::env::var(VARIABLE).ok())
        .filter(|url| !url.trim().is_empty())
}

/// Refuses `--daemon` on a leaf with no route on the global API, naming the leaf.
///
/// # Errors
///
/// `flag` is given and `leaf` is not one of [`ROUTED`].
pub fn refuse_unrouted(leaf: &[String], flag: Option<&str>) -> Result<()> {
    let Some(url) = flag else {
        return Ok(());
    };
    let words: Vec<&str> = leaf.iter().map(String::as_str).collect();
    if leaf.is_empty() || ROUTED.contains(&words.as_slice()) {
        return Ok(());
    }
    bail!(
        "{}: --daemon {url}: this command has no route on the daemon's global API; run it on \
         the daemon's box, or leave --daemon out (the commands that take it: login, join, leave, \
         config put, instance upgrade, tui)",
        words.join(" ")
    )
}

/// An origin a client reaches: `https://`, or `http://` on `127.0.0.1`, `localhost` or a cluster's
/// `*.svc.cluster.local` only, as the daemon reaches its Identity service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Whether it is reached over TLS.
    pub https: bool,
    /// Its host.
    pub host: String,
    /// Its port.
    pub port: u16,
}

impl Origin {
    /// The origin `text` writes: a scheme, a host and an optional port, no path.
    ///
    /// # Errors
    ///
    /// It is no such origin, or plain `http://` off this host.
    pub fn parse(text: &str) -> Result<Self> {
        let (https, rest) = if let Some(rest) = text.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = text.strip_prefix("http://") {
            (false, rest)
        } else {
            bail!("{text} is no https:// address");
        };
        let authority = rest.trim_end_matches('/');
        if authority.is_empty() || authority.contains(['/', '@', '?', '#', ' ']) {
            bail!("{text} is no origin: give the scheme, the host and the port only");
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (
                host.to_owned(),
                port.parse::<u16>()
                    .map_err(|_| anyhow!("{text}: {port} is no port"))?,
            ),
            None => (authority.to_owned(), if https { 443 } else { 80 }),
        };
        let internal =
            host == "127.0.0.1" || host == "localhost" || host.ends_with(".svc.cluster.local");
        if !https && !internal {
            bail!("{text}: a server off this host is reached over https://");
        }
        Ok(Self { https, host, port })
    }

    /// Whether it is this host's loopback listener of a daemon: `http://` on `127.0.0.1` or
    /// `localhost`.
    #[must_use]
    pub fn loopback(&self) -> bool {
        !self.https && (self.host == "127.0.0.1" || self.host == "localhost")
    }

    /// The origin as a URL, `https://host:port`.
    #[must_use]
    pub fn url(&self) -> String {
        format!(
            "{}://{}:{}",
            if self.https { "https" } else { "http" },
            self.host,
            self.port
        )
    }

    /// The path of `endpoint`, an absolute URL on this origin.
    ///
    /// # Errors
    ///
    /// `endpoint` lies on another origin.
    pub fn path_of<'a>(&self, endpoint: &'a str) -> Result<&'a str> {
        let url = self.url();
        let default = format!(
            "{}://{}",
            if self.https { "https" } else { "http" },
            self.host
        );
        [url.as_str(), default.as_str()]
            .iter()
            .find_map(|prefix| {
                endpoint
                    .strip_prefix(prefix)
                    .filter(|path| path.starts_with('/'))
            })
            .ok_or_else(|| anyhow!("{endpoint} lies off the Identity service {url}"))
    }
}

/// One request to `origin`: `method` on `target` with `headers` and `body` (its content type and
/// bytes); answers the status and the body.
///
/// # Errors
///
/// The origin cannot be reached, its certificate is not trusted, or its answer does not parse.
pub fn send(
    origin: &Origin,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: Option<(&str, &[u8])>,
) -> Result<(u16, Vec<u8>)> {
    let mut request = format!(
        "{method} {target} HTTP/1.1\r\nHost: {}:{}\r\nAccept: application/json\r\nConnection: close\r\n",
        origin.host, origin.port
    );
    for (name, value) in headers {
        if value.contains(['\r', '\n']) {
            bail!("the header {name} holds a line break");
        }
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    let bytes = body.map_or(&[][..], |(_, bytes)| bytes);
    if let Some((kind, bytes)) = body {
        request.push_str(&format!(
            "Content-Type: {kind}\r\nContent-Length: {}\r\n",
            bytes.len()
        ));
    } else if method != "GET" {
        request.push_str("Content-Length: 0\r\n");
    }
    request.push_str("\r\n");
    let mut whole = request.into_bytes();
    whole.extend_from_slice(bytes);
    let tcp = TcpStream::connect((origin.host.as_str(), origin.port))
        .with_context(|| format!("connect to {}", origin.url()))?;
    tcp.set_read_timeout(Some(TIMEOUT))?;
    tcp.set_write_timeout(Some(TIMEOUT))?;
    let answer = if origin.https {
        let config = crate::daemon::api::client_config()?;
        let name = ServerName::try_from(origin.host.clone())
            .map_err(|_| anyhow!("{} is no server name", origin.host))?;
        let connection = rustls::ClientConnection::new(config, name)?;
        let mut tls = rustls::StreamOwned::new(connection, tcp);
        exchange(&mut tls, &whole)
    } else {
        let mut tcp = tcp;
        exchange(&mut tcp, &whole)
    }
    .with_context(|| format!("{method} {}{target}", origin.url()))?;
    crate::daemon::api::split(&answer)
}

fn exchange<S: std::io::Read + std::io::Write>(stream: &mut S, request: &[u8]) -> Result<Vec<u8>> {
    stream.write_all(request)?;
    stream.flush()?;
    let mut bytes = Vec::new();
    match stream.read_to_end(&mut bytes) {
        Ok(_) => Ok(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof && !bytes.is_empty() => {
            Ok(bytes)
        }
        Err(error) => Err(error.into()),
    }
}

/// One request to the daemon at `url` with this client's bearer; answers the status and the JSON
/// body. A daemon that refuses the bearer is named with the login that fixes it.
///
/// # Errors
///
/// `url` is no daemon address; there is no login for it; the daemon cannot be reached or refuses
/// the bearer.
pub fn call(url: &str, method: &str, target: &str, body: Option<&Value>) -> Result<(u16, Value)> {
    let daemon = Origin::parse(url).context("--daemon")?;
    let credential = crate::login::credential(&daemon)?;
    let headers: Vec<(&str, &str)> = credential
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let text = body.map(Value::to_string);
    let (status, bytes) = send(
        &daemon,
        method,
        target,
        &headers,
        text.as_deref()
            .map(|text| ("application/json", text.as_bytes())),
    )?;
    let answer: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if status == 401 {
        bail!(
            "the daemon {} refused this client's login: {}; log in again: conductor login \
             --daemon {}",
            daemon.url(),
            answer["error"].as_str().unwrap_or("no reason given"),
            daemon.url()
        );
    }
    Ok((status, answer))
}

/// `GET <path>` on the global API of the daemon at `url` with this client's bearer: the JSON it
/// answered `200` with. The read `conductor tui` makes against a remote daemon
/// (`story:tui-readonly`).
///
/// # Errors
///
/// Those of [`call`]; any other status, naming it and the daemon's reason.
pub fn get(url: &str, path: &str) -> Result<Value> {
    let (status, answer) = call(url, "GET", path, None)?;
    if status != 200 {
        bail!("GET {path} on {url}: {status}: {}", refusal(&answer));
    }
    Ok(answer)
}

/// The one line of a refusal the daemon answered: its specification error with what it names.
fn refusal(answer: &Value) -> String {
    match answer["error"].as_str() {
        Some("conductor.operator.GrantMissing") => format!(
            "conductor.operator.GrantMissing: {} holds no {} grant",
            answer["operator"].as_str().unwrap_or("the operator"),
            answer["grant"].as_str().unwrap_or("needed")
        ),
        Some("conductor.operator.ActorNotHuman") => format!(
            "conductor.operator.ActorNotHuman: {} is not a person",
            answer["principal"].as_str().unwrap_or("the principal")
        ),
        Some(error) => error.to_owned(),
        None => answer.to_string(),
    }
}

/// `conductor --daemon <url> config put`: the change as `PUT /v1/config`, done by the operator
/// the bearer resolves to.
///
/// # Errors
///
/// A required flag is missing, or the daemon cannot be reached. A refusal is one line of standard
/// error and exit 1.
pub fn config_put(url: &str, args: &PutConfigArgs) -> Result<ExitCode> {
    const WHAT: &str = "config put";
    let required = |flag: &str, value: &Option<String>| {
        value
            .clone()
            .ok_or_else(|| anyhow!("{WHAT}: --{flag} is required"))
    };
    let body = json!({
        "level": required("level", &args.level)?,
        "instance": args.instance,
        "role": args.role,
        "path": required("path", &args.path)?,
        "value": args.value,
        "reason": required("reason", &args.reason)?,
    });
    let (status, answer) =
        call(url, "PUT", crate::daemon::api::CONFIG, Some(&body)).context(WHAT)?;
    if status != 200 {
        eprintln!("conductor: {WHAT}: {}", refusal(&answer));
        return Ok(ExitCode::FAILURE);
    }
    let commit = match answer["commit"]
        .as_str()
        .filter(|commit| !commit.is_empty())
    {
        Some(commit) => format!("committed {commit}"),
        None => "not committed: the daemon's config file is in no git checkout".to_owned(),
    };
    println!(
        "{}: {} at {} by {}, {commit}",
        answer["revision_id"].as_str().unwrap_or_default(),
        answer["path"].as_str().unwrap_or_default(),
        answer["level"]
            .as_str()
            .unwrap_or_default()
            .to_ascii_lowercase(),
        answer["actor"].as_str().unwrap_or_default(),
    );
    Ok(ExitCode::SUCCESS)
}

/// `conductor --daemon <url> instance upgrade`: each `POST /v1/instances/{name}/upgrade`, then
/// `GET /v1/instances` until it no longer upgrades, as on this box; `--all` takes the instances
/// the daemon lists.
///
/// # Errors
///
/// No `--tag`; both or neither of `--instance` and `--all`; the daemon cannot be reached; an
/// upgrade is rolled back or refused.
pub fn instance_upgrade(url: &str, args: &InstanceUpgradeArgs) -> Result<ExitCode> {
    const WHAT: &str = "instance upgrade";
    let all = args.all == Some(true);
    let Some(tag) = args.tag.as_deref().filter(|tag| !tag.trim().is_empty()) else {
        bail!("{WHAT}: give --tag v<x.y.z>, the release to install");
    };
    let rows = || -> Result<Vec<Value>> {
        let (status, rows) = call(url, "GET", crate::daemon::api::INSTANCES, None)?;
        if status != 200 {
            bail!("GET {}: {}", crate::daemon::api::INSTANCES, refusal(&rows));
        }
        Ok(rows.as_array().cloned().unwrap_or_default())
    };
    let names: Vec<String> = match (&args.instance, all) {
        (Some(_), true) => bail!("{WHAT}: --instance and --all are given; give one"),
        (None, false) => bail!("{WHAT}: give --instance <name> or --all"),
        (Some(name), false) => vec![name.clone()],
        (None, true) => rows()
            .context(WHAT)?
            .iter()
            .filter_map(|row| row["name"].as_str().map(str::to_owned))
            .collect(),
    };
    let version = tag.strip_prefix('v').unwrap_or(tag);
    let bound = Duration::from_secs(crate::config::UPGRADE_TIMEOUT + 60);
    for name in names {
        let (status, body) = call(
            url,
            "POST",
            &format!("/v1/instances/{name}/upgrade"),
            Some(&json!({"tag": tag})),
        )
        .with_context(|| format!("{WHAT}: {name}"))?;
        match status {
            202 => {}
            200 => bail!(
                "{WHAT}: {name}: rolled back: {}",
                body["detail"].as_str().unwrap_or_default()
            ),
            _ => {
                eprintln!("conductor: {WHAT}: {name}: refused: {}", refusal(&body));
                return Ok(ExitCode::FAILURE);
            }
        }
        let started = Instant::now();
        let row = loop {
            let row = rows()
                .with_context(|| format!("{WHAT}: {name}"))?
                .into_iter()
                .find(|row| row["name"] == name.as_str())
                .ok_or_else(|| anyhow!("{WHAT}: {name}: the daemon lists no {name}"))?;
            if row["state"] != "Upgrading" {
                break row;
            }
            if started.elapsed() > bound {
                bail!(
                    "{WHAT}: {name}: still upgrading after {} s",
                    bound.as_secs()
                );
            }
            thread::sleep(POLL);
        };
        let now_on = row["binary_version"].as_str().unwrap_or("unknown");
        if now_on != version {
            bail!("{WHAT}: {name}: rolled back to {now_on}: {version} did not come up");
        }
        println!("{name}: upgraded to {version}");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_origin_is_https_or_loopback_http_with_no_path() {
        let origin = Origin::parse("https://daemon.example-org.test:7443/").expect("an origin");
        assert_eq!(origin.url(), "https://daemon.example-org.test:7443");
        assert_eq!(
            Origin::parse("https://daemon.example-org.test")
                .expect("an origin")
                .port,
            443
        );
        assert!(Origin::parse("http://127.0.0.1:9").is_ok());
        assert!(Origin::parse("http://daemon.example-org.test").is_err());
        assert!(Origin::parse("https://daemon.example-org.test/v1").is_err());
        assert!(Origin::parse("https://user@daemon.example-org.test").is_err());
        let identity = Origin::parse("https://identity.example-org.test").expect("an origin");
        assert_eq!(
            identity
                .path_of("https://identity.example-org.test/oauth/token")
                .expect("on the origin"),
            "/oauth/token"
        );
        assert!(
            identity
                .path_of("https://elsewhere.example-org.test/oauth/token")
                .is_err()
        );
    }

    #[test]
    fn daemon_is_refused_by_name_on_a_command_without_a_route() {
        let leaf = |words: &[&str]| {
            words
                .iter()
                .map(|word| (*word).to_owned())
                .collect::<Vec<_>>()
        };
        assert!(refuse_unrouted(&leaf(&["config", "put"]), Some("https://d")).is_ok());
        assert!(refuse_unrouted(&leaf(&["doctor"]), None).is_ok());
        let refused = refuse_unrouted(&leaf(&["snapshot", "start-snapshot"]), Some("https://d"))
            .expect_err("refused");
        assert!(
            refused
                .to_string()
                .starts_with("snapshot start-snapshot: --daemon")
        );
    }
}
