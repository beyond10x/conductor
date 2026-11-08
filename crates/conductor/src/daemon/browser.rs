//! The dashboard's login on the daemon's TLS listener (`story:daemon-remote`, design
//! architecture-next § 5.3): a browser on a workstation has no keychain the daemon can read, so
//! the daemon runs the Identity authorization-code flow with PKCE for it and hands it a session
//! cookie instead of a bearer.
//!
//! - `GET /login` ([`LOGIN`]) answers `302` to the Identity service's authorization endpoint, with
//!   a fresh verifier and state kept here ([`PENDING_FOR`] at most) and the callback
//!   `https://<host>/login/callback` on the listener's own host.
//! - `GET /login/callback` ([`CALLBACK`]) takes the code back for the state it was sent with,
//!   exchanges it at the token endpoint for an Identity session, mints an access token from it
//!   and resolves that as any bearer is resolved; the session and the token are then dropped. It
//!   answers `302` to `/` with the cookie [`COOKIE`]: a random value, `HttpOnly`, `Secure`,
//!   `SameSite=Strict`, `Path=/`, for [`LIFETIME`]. The daemon keeps only the value's operator and
//!   expiry, in memory; a restart logs every browser out.
//!
//! The listener takes the cookie where a request carries no bearer ([`Sessions::who`]); a write
//! with it must also name the listener's own origin in `Origin`, and the operator's grant decides
//! it as for a bearer.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use serde_json::{Value, json};

use super::api::Identity;
use crate::remote::{self, Origin};

/// The route that starts a browser's login.
pub const LOGIN: &str = "/login";

/// The route Identity sends the browser back to.
pub const CALLBACK: &str = "/login/callback";

/// The session cookie's name.
pub const COOKIE: &str = "conductor_session";

/// How long a browser's session lasts: short, a working hour.
pub const LIFETIME: Duration = Duration::from_secs(3600);

/// How long a started login waits for its callback.
pub const PENDING_FOR: Duration = Duration::from_secs(600);

/// The most logins kept waiting, and the most sessions kept: a flood costs the oldest.
const MOST: usize = 4096;

/// A login started and not yet called back: its PKCE verifier and its callback address.
struct Pending {
    verifier: String,
    redirect: String,
    at: Instant,
}

/// A browser's session: the operator it is bound to and until when.
#[derive(Clone)]
struct Session {
    name: String,
    human: bool,
    until: Instant,
}

/// The logins waiting and the sessions handed out, shared by every connection.
#[derive(Default)]
pub struct Sessions {
    pending: Mutex<HashMap<String, Pending>>,
    sessions: Mutex<HashMap<String, Session>>,
}

impl Sessions {
    /// The operator name and whether it is a person, of the live session the `Cookie` header
    /// `cookie` carries.
    #[must_use]
    pub fn who(&self, cookie: Option<&str>) -> Option<(String, bool)> {
        let value = cookie?
            .split(';')
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(name, _)| *name == COOKIE)
            .map(|(_, value)| value.trim())?;
        let mut sessions = self.sessions.lock().unwrap_or_else(PoisonError::into_inner);
        let now = Instant::now();
        sessions.retain(|_, session| session.until > now);
        sessions
            .get(value)
            .map(|session| (session.name.clone(), session.human))
    }
}

/// The listener's own origin, `https://<host>`, from the `Host` header `host`: a host and an
/// optional port, nothing else.
#[must_use]
pub fn own_origin(host: Option<&str>) -> Option<String> {
    let host = host?.trim();
    let fine = !host.is_empty()
        && host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-:[]".contains(&byte));
    fine.then(|| format!("https://{host}"))
}

/// `GET /login`: the authorization address to send the browser to.
///
/// # Errors
///
/// The `Host` header names no host, or the Identity service does not answer its metadata.
pub fn begin(identity: &Identity, sessions: &Sessions, host: Option<&str>) -> Result<String> {
    let own = own_origin(host).ok_or_else(|| anyhow!("the request names no host"))?;
    let service = Origin::parse(&identity.origin()).context("daemon.tls.identity")?;
    let metadata = metadata(&service)?;
    let authorize = text(&metadata, "authorization_endpoint")?;
    service.path_of(&authorize)?;
    let client = text(&metadata, "cli_client_id")?;
    let verifier = crate::dashboard::mint().context("a PKCE verifier")?;
    let state = crate::dashboard::mint().context("a state")?;
    let redirect = format!("{own}{CALLBACK}");
    let address = format!(
        "{authorize}?response_type=code&client_id={}&redirect_uri={}&code_challenge={}\
         &code_challenge_method=S256&state={}",
        crate::login::escape(&client),
        crate::login::escape(&redirect),
        crate::login::s256(&verifier),
        crate::login::escape(&state),
    );
    let mut pending = sessions
        .pending
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    pending.retain(|_, waiting| waiting.at.elapsed() < PENDING_FOR);
    if pending.len() >= MOST
        && let Some(oldest) = pending
            .iter()
            .min_by_key(|(_, waiting)| waiting.at)
            .map(|(key, _)| key.clone())
    {
        pending.remove(&oldest);
    }
    pending.insert(
        state,
        Pending {
            verifier,
            redirect,
            at: Instant::now(),
        },
    );
    Ok(address)
}

/// `GET /login/callback?<query>`: the session cookie's value, bound to the operator `named`
/// answers for the authority Identity resolves; the `Set-Cookie` header's value is
/// [`set_cookie`].
///
/// # Errors
///
/// The state is not one this daemon sent, Identity refused the sign-in or the code, or the
/// resolved principal cannot be named.
pub fn finish(
    identity: &Identity,
    sessions: &Sessions,
    query: &str,
    named: impl FnOnce(&str) -> Result<(String, bool)>,
) -> Result<String> {
    let fields: Vec<(String, String)> = query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_owned(), crate::login::unescape(value)))
        .collect();
    let get = |wanted: &str| {
        fields
            .iter()
            .find(|(key, _)| key == wanted)
            .map(|(_, value)| value.clone())
    };
    let waiting = get("state")
        .and_then(|state| {
            sessions
                .pending
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&state)
        })
        .filter(|waiting| waiting.at.elapsed() < PENDING_FOR)
        .ok_or_else(|| anyhow!("this sign-in was not started here, or it waited too long"))?;
    if let Some(error) = get("error") {
        bail!("the Identity service refused the sign-in: {error}");
    }
    let code = get("code")
        .filter(|code| !code.is_empty())
        .ok_or_else(|| anyhow!("the sign-in brought no code"))?;
    let service = Origin::parse(&identity.origin()).context("daemon.tls.identity")?;
    let metadata = metadata(&service)?;
    let token = text(&metadata, "token_endpoint")?;
    let token = service.path_of(&token)?.to_owned();
    let client = text(&metadata, "cli_client_id")?;
    let form = format!(
        "grant_type=authorization_code&client_id={}&code={}&redirect_uri={}&code_verifier={}",
        crate::login::escape(&client),
        crate::login::escape(&code),
        crate::login::escape(&waiting.redirect),
        crate::login::escape(&waiting.verifier),
    );
    let (status, body) = remote::send(
        &service,
        "POST",
        &token,
        &[],
        Some(("application/x-www-form-urlencoded", form.as_bytes())),
    )?;
    if status != 200 {
        bail!("the Identity service refused the code ({status})");
    }
    let answer: Value = serde_json::from_slice(&body).context("the Identity service's session")?;
    let session = answer["session"]
        .as_str()
        .filter(|value| !value.is_empty() && answer["session_type"] == "opaque_server_session")
        .ok_or_else(|| anyhow!("the Identity service answered no session"))?;
    let access = crate::login::access_token(&service, session)?;
    let (name, human) = named(&access)?;
    let value = crate::dashboard::mint().context("a session cookie")?;
    let mut kept = sessions
        .sessions
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    kept.retain(|_, live| live.until > now);
    if kept.len() >= MOST
        && let Some(oldest) = kept
            .iter()
            .min_by_key(|(_, live)| live.until)
            .map(|(key, _)| key.clone())
    {
        kept.remove(&oldest);
    }
    kept.insert(
        value.clone(),
        Session {
            name,
            human,
            until: now + LIFETIME,
        },
    );
    Ok(value)
}

/// The `Set-Cookie` header's value for the session `value`.
#[must_use]
pub fn set_cookie(value: &str) -> String {
    format!(
        "{COOKIE}={value}; Path=/; Max-Age={}; HttpOnly; Secure; SameSite=Strict",
        LIFETIME.as_secs()
    )
}

fn metadata(service: &Origin) -> Result<Value> {
    let (status, body) =
        remote::send(service, "GET", "/.well-known/identity-cli-login", &[], None)?;
    if status != 200 {
        bail!(
            "the Identity service {} answered {status} for its login metadata",
            service.url()
        );
    }
    serde_json::from_slice(&body).context("the Identity service's login metadata")
}

fn text(metadata: &Value, key: &str) -> Result<String> {
    metadata[key]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("the Identity service's login metadata names no {key}"))
}

/// The JSON of a refused browser login.
#[must_use]
pub fn refusal(error: &anyhow::Error) -> Value {
    json!({"error": format!("the login did not complete: {error:#}")})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_plain_host_is_an_own_origin() {
        assert_eq!(
            own_origin(Some("daemon.example-org.test:7443")).as_deref(),
            Some("https://daemon.example-org.test:7443")
        );
        assert_eq!(own_origin(Some("evil/x")), None);
        assert_eq!(own_origin(Some("a b")), None);
        assert_eq!(own_origin(None), None);
    }

    #[test]
    fn a_cookie_is_found_among_others_and_unknown_values_are_nobody() {
        let sessions = Sessions::default();
        sessions.sessions.lock().expect("lock").insert(
            "abc".to_owned(),
            Session {
                name: "owner@example-org.test".to_owned(),
                human: true,
                until: Instant::now() + LIFETIME,
            },
        );
        assert_eq!(
            sessions.who(Some("theme=dark; conductor_session=abc")),
            Some(("owner@example-org.test".to_owned(), true))
        );
        assert_eq!(sessions.who(Some("conductor_session=abd")), None);
        assert_eq!(sessions.who(None), None);
        let cookie = set_cookie("abc");
        for flag in ["HttpOnly", "Secure", "SameSite=Strict"] {
            assert!(cookie.contains(flag), "{cookie}");
        }
    }
}
