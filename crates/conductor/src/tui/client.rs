//! The terminal UI's client of the daemon's global API (`story:tui-readonly`): one `GET` per
//! request over `http://` or `https://`, the answer's status and its JSON. To a daemon that is
//! not this host's loopback listener it presents this client's login
//! ([`crate::login::credential`], `conductor login`); a read on the loopback listener needs none.
//! Without a login the request goes bare, and the daemon's `401` is answered with the reason the
//! login is missing.

use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs as _};
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use rustls::pki_types::ServerName;
use serde_json::Value;

/// The daemon a terminal UI reads: its scheme, host and port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    https: bool,
    host: String,
    port: u16,
    timeout: Duration,
}

impl Client {
    /// The daemon at `url`, an `http://` or `https://` origin (a trailing `/` is taken), each
    /// request bounded by `timeout`.
    ///
    /// # Errors
    ///
    /// `url` is no such origin, or carries a path.
    pub fn parse(url: &str, timeout: Duration) -> Result<Self> {
        let (https, rest) = if let Some(rest) = url.strip_prefix("https://") {
            (true, rest)
        } else if let Some(rest) = url.strip_prefix("http://") {
            (false, rest)
        } else {
            bail!("--daemon {url}: give an http:// or https:// URL");
        };
        let authority = rest.trim_end_matches('/');
        if authority.is_empty() || authority.contains(['/', '@', '?', '#']) {
            bail!("--daemon {url}: give the scheme, the host and the port only");
        }
        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) if !authority.ends_with(']') => (
                host.trim_start_matches('[')
                    .trim_end_matches(']')
                    .to_owned(),
                port.parse::<u16>()
                    .map_err(|_| anyhow!("--daemon {url}: the port {port} is no port"))?,
            ),
            _ => (
                authority
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .to_owned(),
                if https { 443 } else { 80 },
            ),
        };
        Ok(Self {
            https,
            host,
            port,
            timeout,
        })
    }

    /// The daemon's origin, as the UI names it.
    #[must_use]
    pub fn origin(&self) -> String {
        let scheme = if self.https { "https" } else { "http" };
        if self.host.contains(':') {
            format!("{scheme}://[{}]:{}", self.host, self.port)
        } else {
            format!("{scheme}://{}:{}", self.host, self.port)
        }
    }

    /// Whether the daemon is on this host: its sessions' terminals are reached here, by
    /// `conductor session attach`.
    #[must_use]
    pub fn local(&self) -> bool {
        matches!(self.host.as_str(), "127.0.0.1" | "localhost" | "::1")
    }

    /// `GET <path>`: the status and the JSON of the answer (`null` when its body is none).
    ///
    /// # Errors
    ///
    /// The daemon does not connect or answer within the timeout, or its answer is no HTTP.
    pub fn get(&self, path: &str) -> Result<(u16, Value)> {
        let host = if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        };
        let origin = crate::remote::Origin {
            https: self.https,
            host: self.host.clone(),
            port: self.port,
        };
        let (credential, missing) = if origin.loopback() {
            (Vec::new(), None)
        } else {
            match crate::login::credential(&origin) {
                Ok(credential) => (credential, None),
                Err(error) => (Vec::new(), Some(error)),
            }
        };
        let mut request =
            format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nAccept: application/json\r\n");
        for (name, value) in &credential {
            if value.contains(['\r', '\n']) {
                bail!("the header {name} holds a line break");
            }
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        request.push_str("Connection: close\r\n\r\n");
        let address = (self.host.as_str(), self.port)
            .to_socket_addrs()
            .with_context(|| format!("resolve {}", self.origin()))?
            .next()
            .ok_or_else(|| anyhow!("{} resolves to no address", self.origin()))?;
        let tcp = TcpStream::connect_timeout(&address, self.timeout)
            .with_context(|| format!("connect to {}", self.origin()))?;
        tcp.set_read_timeout(Some(self.timeout))?;
        tcp.set_write_timeout(Some(self.timeout))?;
        let bytes = if self.https {
            let name = ServerName::try_from(self.host.clone())
                .map_err(|_| anyhow!("{} is no server name", self.host))?;
            let connection =
                rustls::ClientConnection::new(crate::daemon::api::client_config()?, name)?;
            exchange(&mut rustls::StreamOwned::new(connection, tcp), &request)?
        } else {
            exchange(&mut { tcp }, &request)?
        };
        let (status, body) =
            crate::daemon::api::split(&bytes).with_context(|| format!("GET {path}"))?;
        if status == 401
            && let Some(missing) = missing
        {
            return Err(missing.context(format!("GET {path}: 401")));
        }
        let value = if body.iter().all(u8::is_ascii_whitespace) {
            Value::Null
        } else {
            serde_json::from_slice(&body)
                .with_context(|| format!("GET {path}: the answer is no JSON"))?
        };
        Ok((status, value))
    }

    /// `GET <path>` answered `200`: its JSON.
    ///
    /// # Errors
    ///
    /// What [`Client::get`] answers, or another status, named with the answer's `error`.
    pub fn json(&self, path: &str) -> Result<Value> {
        let (status, value) = self.get(path)?;
        if status != 200 {
            bail!("GET {path}: {status}{}", reason(&value));
        }
        Ok(value)
    }
}

/// `: <error>` of an answer that names one, else nothing.
#[must_use]
pub fn reason(value: &Value) -> String {
    value["error"]
        .as_str()
        .map(|error| format!(": {error}"))
        .unwrap_or_default()
}

fn exchange<S: Read + Write>(stream: &mut S, request: &str) -> Result<Vec<u8>> {
    stream.write_all(request.as_bytes())?;
    stream.flush()?;
    let mut bytes = Vec::new();
    match stream.read_to_end(&mut bytes) {
        Ok(_) => Ok(bytes),
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof && !bytes.is_empty() => {
            Ok(bytes)
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_origin_and_refuses_a_path() {
        let client = Client::parse("http://127.0.0.1:7300/", Duration::from_secs(1)).unwrap();
        assert_eq!(client.origin(), "http://127.0.0.1:7300");
        assert!(client.local());
        let remote = Client::parse("https://daemon.example.org", Duration::from_secs(1)).unwrap();
        assert_eq!(remote.origin(), "https://daemon.example.org:443");
        assert!(!remote.local());
        assert!(Client::parse("daemon.example.org:7300", Duration::from_secs(1)).is_err());
        assert!(Client::parse("http://127.0.0.1:7300/v1", Duration::from_secs(1)).is_err());
        assert!(Client::parse("http://127.0.0.1:x", Duration::from_secs(1)).is_err());
    }
}
