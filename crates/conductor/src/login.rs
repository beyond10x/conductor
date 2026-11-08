//! `conductor login` (`story:daemon-remote`, design architecture-next § 4): this client logs in
//! to the daemon the global `--daemon <url>` or `CONDUCTOR_DAEMON` names, through the
//! organization's Identity service, and keeps what it obtained in its own keychain through the
//! `secrets` crates ([`crate::secret::client`]); never in a file, a variable, an argument or a
//! log line.
//!
//! 1. The daemon names its Identity service in the answer to a request without a bearer (`401`,
//!    `identity`).
//! 2. The service's CLI login metadata (`GET /.well-known/identity-cli-login`) names its
//!    authorization and token endpoints and the public client id.
//! 3. The authorization-code flow with PKCE (S256): the login prints the authorization address
//!    to open in a browser and waits on a loopback callback, `http://127.0.0.1:<port>/callback`,
//!    for the code; the state it sent must come back.
//! 4. The code and the verifier are exchanged at the token endpoint for an opaque Identity
//!    session, kept in the keychain with the service's origin under the daemon's name
//!    ([`entry`]).
//!
//! Each command against the daemon then mints a short-lived access token from that session for
//! the daemon's audience ([`bearer`], `POST /v1/access-token`), which the daemon resolves at
//! `GET /v1/access-authority`. A session Identity no longer takes is named with this command.
//!
//! The API other commands use: [`login`], [`credential`], [`bearer`], [`access_token`] and
//! [`entry`]; `conductor join` and `leave` (`story:token-pool`) send through [`credential`].

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::secret::SecretName;
use secrets_core::storage::SecretValue;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::remote::{self, Origin};

/// The audience a bearer is minted for: the daemon's ([`crate::daemon::api::AUDIENCE`]).
pub const AUDIENCE: &str = crate::daemon::api::AUDIENCE;

/// The scope a bearer is minted with: the coarse gate Identity registers for the audience; the
/// daemon's own grants decide every write.
pub const SCOPE: &str = "conductor.operate";

/// How long the login waits for the browser.
const WAIT: Duration = Duration::from_secs(300);

/// What the keychain keeps for one daemon: the Identity service it was logged in through and the
/// session. No `Debug`: it holds the session.
#[derive(Serialize, Deserialize)]
struct Kept {
    identity: String,
    session: String,
}

/// The keychain name of the login to `daemon`: `daemon/<host>.<port>`, lower case.
#[must_use]
pub fn entry(daemon: &Origin) -> SecretName {
    SecretName(format!(
        "daemon/{}.{}",
        daemon.host.to_ascii_lowercase(),
        daemon.port
    ))
}

/// The headers a request to `daemon` carries to be taken as this client's write: on this host's
/// loopback listener the daemon's own token from its directory, with the listener's origin, as
/// `conductor instance upgrade` presents it (no login); on any other daemon `Authorization:
/// Bearer` with [`bearer`]. No value is printed.
///
/// # Errors
///
/// The loopback daemon keeps no token here, or [`bearer`]'s.
pub fn credential(daemon: &Origin) -> Result<Vec<(String, String)>> {
    if daemon.loopback() {
        let environment = crate::config::Environment::from_process()?;
        let home = environment.home()?;
        let token =
            crate::daemon::api::kept_token(&home.join(crate::daemon::DIR)).ok_or_else(|| {
                anyhow!(
                    "the daemon keeps no token in ~/{}: is conductor daemon serve running here?",
                    crate::daemon::DIR
                )
            })?;
        return Ok(vec![
            ("Origin".to_owned(), daemon.url()),
            (crate::dashboard::TOKEN_HEADER.to_owned(), token),
        ]);
    }
    Ok(vec![(
        "Authorization".to_owned(),
        format!("Bearer {}", bearer(daemon)?),
    )])
}

/// The bearer this client presents to `daemon`: an access token minted from the session its
/// login kept.
///
/// # Errors
///
/// No login to `daemon` is kept, the keychain does not answer, or Identity refuses the session;
/// each names `conductor login`.
pub fn bearer(daemon: &Origin) -> Result<String> {
    let again = format!("conductor login --daemon {}", daemon.url());
    let keychain = crate::secret::client().map_err(|unusable| anyhow!("{unusable}"))?;
    let value = keychain
        .reveal(&entry(daemon))
        .map_err(|_| anyhow!("this client's keychain does not answer"))?
        .ok_or_else(|| {
            anyhow!(
                "not logged in to {}: no Identity session for it in this client's keychain; run \
                 {again}",
                daemon.url()
            )
        })?;
    let kept: Kept = serde_json::from_slice(value.expose()).map_err(|_| {
        anyhow!(
            "the login to {} in the keychain does not read; run {again}",
            daemon.url()
        )
    })?;
    let identity = Origin::parse(&kept.identity).context("the kept Identity service")?;
    access_token(&identity, &kept.session).with_context(|| format!("run {again}"))
}

/// An access token for the daemon's audience, minted by the Identity service `identity` from the
/// Identity session `session` (`POST /v1/access-token`). The daemon's browser login uses it too
/// ([`crate::daemon::browser`]).
///
/// # Errors
///
/// The service cannot be reached, or refuses the session (expired or revoked).
pub fn access_token(identity: &Origin, session: &str) -> Result<String> {
    let authorization = format!("Bearer {session}");
    let request = json!({"audience": AUDIENCE, "scope": SCOPE}).to_string();
    let (status, body) = remote::send(
        identity,
        "POST",
        "/v1/access-token",
        &[("Authorization", authorization.as_str())],
        Some(("application/json", request.as_bytes())),
    )?;
    if status != 200 {
        bail!(
            "the Identity service {} answered {status} for the session: it is expired or revoked",
            identity.url()
        );
    }
    let answer: Value =
        serde_json::from_slice(&body).context("the Identity service's access token")?;
    answer["access_token"]
        .as_str()
        .filter(|token| !token.is_empty() && answer["token_type"] == "Bearer")
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("the Identity service answered no access token"))
}

/// `conductor login`, as the module documentation says, for the daemon at `daemon`.
///
/// # Errors
///
/// No daemon is named or it is no address; the daemon names no Identity service; the service or
/// the browser does not complete the login; the keychain does not answer.
pub fn login(daemon: Option<&str>) -> Result<ExitCode> {
    const WHAT: &str = "login";
    let url = daemon.ok_or_else(|| {
        anyhow!(
            "{WHAT}: name the daemon to log in to: --daemon <url>, or {}",
            remote::VARIABLE
        )
    })?;
    let daemon = Origin::parse(url).with_context(|| format!("{WHAT}: --daemon"))?;
    let (status, body) =
        remote::send(&daemon, "GET", crate::daemon::api::INSTANCES, &[], None).context(WHAT)?;
    let answer: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let Some(identity) = answer["identity"].as_str().filter(|_| status == 401) else {
        bail!(
            "{WHAT}: {} answered {status} without a bearer and names no Identity service: it is \
             no daemon's TLS listener with bearers (daemon.tls.identity)",
            daemon.url()
        );
    };
    let identity = Origin::parse(identity)
        .with_context(|| format!("{WHAT}: the daemon's Identity service"))?;
    let (status, body) = remote::send(
        &identity,
        "GET",
        "/.well-known/identity-cli-login",
        &[],
        None,
    )
    .context(WHAT)?;
    if status != 200 {
        bail!(
            "{WHAT}: the Identity service {} answered {status} for its login metadata",
            identity.url()
        );
    }
    let metadata: Value = serde_json::from_slice(&body)
        .with_context(|| format!("{WHAT}: the Identity service's login metadata"))?;
    let field = |key: &str| {
        metadata[key]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("{WHAT}: the Identity service's login metadata names no {key}"))
    };
    let (authorize, token, client) = (
        field("authorization_endpoint")?,
        field("token_endpoint")?,
        field("cli_client_id")?,
    );
    let token_path = identity.path_of(&token).context(WHAT)?.to_owned();
    identity.path_of(&authorize).context(WHAT)?;

    let verifier = crate::dashboard::mint().context("login: a PKCE verifier")?;
    let state = crate::dashboard::mint().context("login: a state")?;
    let callback = TcpListener::bind(("127.0.0.1", 0)).context("login: listen for the browser")?;
    let redirect = format!(
        "http://127.0.0.1:{}/callback",
        callback.local_addr().context(WHAT)?.port()
    );
    let address = format!(
        "{authorize}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}\
         &code_challenge_method=S256&state={}",
        escape(&client),
        escape(&redirect),
        s256(&verifier),
        escape(&state),
    );
    eprintln!("conductor login: open this address in a browser and sign in: {address}");
    let code = wait_for_code(&callback, &state).context(WHAT)?;

    let form = format!(
        "grant_type=authorization_code&client_id={}&code={}&redirect_uri={}&code_verifier={}",
        escape(&client),
        escape(&code),
        escape(&redirect),
        escape(&verifier),
    );
    let (status, body) = remote::send(
        &identity,
        "POST",
        &token_path,
        &[],
        Some(("application/x-www-form-urlencoded", form.as_bytes())),
    )
    .context(WHAT)?;
    if status != 200 {
        bail!(
            "{WHAT}: the Identity service {} refused the code ({status})",
            identity.url()
        );
    }
    let session: Value = serde_json::from_slice(&body)
        .with_context(|| format!("{WHAT}: the Identity service's session"))?;
    let Some(value) = session["session"]
        .as_str()
        .filter(|value| !value.is_empty() && session["session_type"] == "opaque_server_session")
    else {
        bail!("{WHAT}: the Identity service answered no session");
    };
    let who = session["email"]
        .as_str()
        .or_else(|| session["subject"].as_str())
        .unwrap_or("an unnamed principal")
        .to_owned();
    let kept = serde_json::to_vec(&Kept {
        identity: identity.url(),
        session: value.to_owned(),
    })
    .context(WHAT)?;
    let kept = SecretValue::new(kept).map_err(|_| anyhow!("{WHAT}: the session is too large"))?;
    let keychain = crate::secret::client().map_err(|unusable| anyhow!("{WHAT}: {unusable}"))?;
    keychain
        .keep(&entry(&daemon), kept)
        .map_err(|_| anyhow!("{WHAT}: this client's keychain does not answer"))?;
    // The login works: the daemon takes a bearer minted from it.
    let (status, answer) =
        remote::call(url, "GET", crate::daemon::api::INSTANCES, None).context(WHAT)?;
    if status != 200 {
        bail!(
            "{WHAT}: the daemon {} answered {status} with the new login: {}",
            daemon.url(),
            answer["error"].as_str().unwrap_or("no reason given")
        );
    }
    println!(
        "logged in to {} as {who}; the session is kept in this client's keychain",
        daemon.url()
    );
    Ok(ExitCode::SUCCESS)
}

/// Waits on `callback` for the browser's `GET /callback?code=…&state=…` and answers the code.
fn wait_for_code(callback: &TcpListener, state: &str) -> Result<String> {
    callback.set_nonblocking(true)?;
    let deadline = Instant::now() + WAIT;
    loop {
        let stream = match callback.accept() {
            Ok((stream, _)) => stream,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() > deadline {
                    bail!("the browser did not come back within {} s", WAIT.as_secs());
                }
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(error) => return Err(error.into()),
        };
        let mut stream = stream;
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        let mut head = Vec::new();
        let mut byte = [0_u8; 1];
        while !head.ends_with(b"\r\n\r\n") && head.len() < 16 * 1024 {
            match stream.read(&mut byte) {
                Ok(1) => head.push(byte[0]),
                _ => break,
            }
        }
        let head = String::from_utf8_lossy(&head);
        let target = head
            .lines()
            .next()
            .and_then(|line| line.strip_prefix("GET "))
            .and_then(|rest| rest.split(' ').next())
            .unwrap_or_default();
        let Some(query) = target.strip_prefix("/callback?") else {
            let _ = stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            continue;
        };
        let fields: Vec<(String, String)> = query
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .map(|(key, value)| (key.to_owned(), unescape(value)))
            .collect();
        let get = |wanted: &str| {
            fields
                .iter()
                .find(|(key, _)| key == wanted)
                .map(|(_, value)| value.clone())
        };
        let (status, page, outcome) = if get("state").as_deref() != Some(state) {
            (
                "400 Bad Request",
                "This sign-in did not come from this login; run conductor login again.",
                Err(anyhow!("the browser came back with another state")),
            )
        } else if let Some(error) = get("error") {
            (
                "400 Bad Request",
                "The sign-in was refused.",
                Err(anyhow!("the Identity service refused the sign-in: {error}")),
            )
        } else if let Some(code) = get("code").filter(|code| !code.is_empty()) {
            ("200 OK", "Signed in. You may close this page.", Ok(code))
        } else {
            (
                "400 Bad Request",
                "The sign-in brought no code.",
                Err(anyhow!("the browser came back with no code")),
            )
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\
             Cache-Control: no-store\r\nConnection: close\r\n\r\n{page}",
            page.len()
        );
        return outcome;
    }
}

/// `text` percent-encoded for a query: every byte but the unreserved ones.
pub(crate) fn escape(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(byte).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// A query value decoded: `%XX` and `+`.
pub(crate) fn unescape(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' if at + 2 < bytes.len() => {
                match std::str::from_utf8(&bytes[at + 1..at + 3])
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
                {
                    Some(byte) => {
                        out.push(byte);
                        at += 3;
                    }
                    None => {
                        out.push(b'%');
                        at += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                at += 1;
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The PKCE S256 challenge of `verifier`: SHA-256, base64url without padding.
pub(crate) fn s256(verifier: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let digest = ring::digest::digest(&ring::digest::SHA256, verifier.as_bytes());
    let mut out = String::new();
    for chunk in digest.as_ref().chunks(3) {
        let bits = chunk.iter().enumerate().fold(0_u32, |bits, (index, byte)| {
            bits | (u32::from(*byte) << (16 - 8 * index))
        });
        for index in 0..=chunk.len() {
            out.push(char::from(
                ALPHABET[((bits >> (18 - 6 * index)) & 63) as usize],
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_is_rfc_7636s_s256() {
        // RFC 7636, appendix B.
        assert_eq!(
            s256("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn a_query_value_round_trips() {
        let text = "http://127.0.0.1:9/callback?a=b c&d";
        assert_eq!(unescape(&escape(text)), text);
        assert_eq!(unescape("a%2"), "a%2");
    }

    #[test]
    fn the_keychain_entry_names_the_daemon() {
        let daemon = Origin::parse("https://Daemon.example-org.test:7443").expect("an origin");
        assert_eq!(entry(&daemon).0, "daemon/daemon.example-org.test.7443");
    }
}
