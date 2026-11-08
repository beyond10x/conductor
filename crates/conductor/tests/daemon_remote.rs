//! conductord on a remote box (`story:daemon-remote`, design architecture-next § 4, § 5.3): the
//! loopback listener off (`daemon.loopback: false`), refused without a TLS listener; the install
//! as a system unit, refused outside a git checkout and on the OS keychain; `conductor login`
//! obtaining an Identity session through the authorization-code flow and keeping it in the
//! client's keychain; `--daemon <url>` naming the daemon a command talks to, a command without a
//! login refused by name, a command without a global-API route refused by name; a write naming
//! the operator the bearer resolves to, or refused with the grant's name. The Identity service is
//! a fake on 127.0.0.1 ([`FakeIdentity`]); the keychain is the fake `common::conductor` names; no
//! case reaches systemd.

mod active;
mod common;

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use conductor_cli::confine::systemd::Service;
use conductor_cli::daemon::install;
use conductor_cli::daemon::units::Units;
use serde_json::{Value, json};

struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("daemon-remote")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("box/home")).expect("create the box's home");
        fs::create_dir_all(dir.join("owner")).expect("create the owner's home");
        fs::create_dir_all(dir.join("reader")).expect("create the reader's home");
        Self { dir }
    }

    /// The config repository on the box.
    fn repository(&self) -> PathBuf {
        self.dir.join("box/config")
    }

    fn file(&self) -> PathBuf {
        self.repository().join("conductor.yaml")
    }

    /// Writes the config file: `daemon` with the case's store added, and one instance whose
    /// directories lie in the case.
    fn config(&self, daemon: &str) {
        let daemon = daemon.replacen(
            '{',
            &format!(
                "{{store: {:?}, ",
                format!("sqlite:{}", self.dir.join("box/shared.sqlite").display())
            ),
            1,
        );
        let root = self.dir.join("box/alpha");
        fs::create_dir_all(self.repository()).expect("create the config repository");
        fs::write(
            self.file(),
            format!(
                "version: conductor.config/2\ndaemon: {daemon}\ninstances:\n  - name: alpha\n    \
                 placement: local\n    session_prefix: alpha\n    \
                 sources: [{{github: example-org}}]\n    \
                 checkouts: {{root: {checkouts:?}, trees: {trees:?}}}\n    \
                 records: {records:?}\n    state: {state:?}\n    cache: {cache:?}\n",
                checkouts = root.join("checkouts"),
                trees = root.join("trees"),
                records = root.join("records"),
                state = root.join("state"),
                cache = root.join("cache"),
            ),
        )
        .expect("write the config file");
    }

    /// Makes the config repository a git checkout with the config file committed.
    fn commit_config(&self) {
        let dir = self.repository();
        for words in [
            &["init", "-q", "-b", "main"][..],
            &["config", "user.name", "example-org box"],
            &["config", "user.email", "box@example-org.test"],
            &["add", "conductor.yaml"],
            &["commit", "-q", "-m", "config"],
        ] {
            let status = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(words)
                .status()
                .expect("run git");
            assert!(status.success(), "git {words:?}");
        }
    }

    /// `conductor` on the box, with the case's config file.
    fn on_box(&self) -> Command {
        let mut command = common::conductor(self.dir.join("box/home"));
        command.env("CONDUCTOR_CONFIG", self.file());
        command
    }

    /// `conductor` on a workstation whose home is `who`, trusting the daemon's certificate.
    fn workstation(&self, who: &str) -> Command {
        let mut command = common::conductor(self.dir.join(who));
        command.env("SSL_CERT_FILE", self.dir.join("box/cert.pem"));
        command
    }

    /// The TLS listener's certificate and key, for `localhost`.
    fn certificate(&self) {
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("a certificate");
        fs::write(self.dir.join("box/cert.pem"), generated.cert.pem()).expect("write the cert");
        fs::write(
            self.dir.join("box/key.pem"),
            generated.key_pair.serialize_pem(),
        )
        .expect("write the key");
    }

    /// A directory holding `git` alone, the `PATH` of the daemon: it commits, and reaches no
    /// service manager of the host.
    fn git_only(&self) -> PathBuf {
        let bin = self.dir.join("box/bin");
        fs::create_dir_all(&bin).expect("create bin/");
        let git = ["/usr/bin/git", "/bin/git", "/usr/local/bin/git"]
            .into_iter()
            .find(|path| Path::new(path).exists())
            .expect("git is installed");
        let _ = std::os::unix::fs::symlink(git, bin.join("git"));
        bin
    }
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind a free port")
        .local_addr()
        .expect("address")
        .port()
}

fn said(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The fake Identity service: the CLI login metadata, the code exchange (checked against the
/// PKCE challenge the browser saw), the access token minted from a session, and the authority of
/// an access token. A code `code-<who>` is `<who>@example-org.test`'s.
struct FakeIdentity {
    origin: String,
    /// The `code_challenge` of the authorization request, as the browser saw it.
    challenge: Arc<Mutex<Option<String>>>,
    /// Every request line it answered, in order.
    seen: Arc<Mutex<Vec<String>>>,
}

fn read_request(stream: &mut TcpStream) -> Option<(String, BTreeMap<String, String>, Vec<u8>)> {
    let mut head = Vec::new();
    let mut byte = [0_u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return None,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let mut lines = head.split("\r\n");
    let line = lines.next()?.to_owned();
    let headers: BTreeMap<String, String> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_lowercase(), value.trim().to_owned()))
        .collect();
    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0_u8; length];
    stream.read_exact(&mut body).ok()?;
    Some((line, headers, body))
}

fn form(body: &[u8]) -> BTreeMap<String, String> {
    String::from_utf8_lossy(body)
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.to_owned(), unescape(value)))
        .collect()
}

fn unescape(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' if at + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[at + 1..at + 3]).unwrap_or("00");
                out.push(u8::from_str_radix(hex, 16).unwrap_or(b'?'));
                at += 3;
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

fn query(url: &str) -> BTreeMap<String, String> {
    url.split_once('?')
        .map(|(_, query)| form(query.as_bytes()))
        .unwrap_or_default()
}

/// base64url without padding of SHA-256 of `verifier`, the PKCE S256 challenge.
fn s256(verifier: &str) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, verifier.as_bytes());
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
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

impl FakeIdentity {
    fn start() -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind the fake Identity");
        let origin = format!(
            "http://127.0.0.1:{}",
            listener.local_addr().expect("address").port()
        );
        let challenge: Arc<Mutex<Option<String>>> = Arc::default();
        let seen: Arc<Mutex<Vec<String>>> = Arc::default();
        let (shared, log, at) = (Arc::clone(&challenge), Arc::clone(&seen), origin.clone());
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut stream = stream;
                let Some((line, headers, body)) = read_request(&mut stream) else {
                    continue;
                };
                log.lock().expect("log").push(line.clone());
                let bearer = headers
                    .get("authorization")
                    .and_then(|value| value.strip_prefix("Bearer "))
                    .unwrap_or_default()
                    .to_owned();
                let who = |prefix: &str| {
                    bearer
                        .strip_prefix(prefix)
                        .filter(|who| ["owner", "reader"].contains(who))
                        .map(str::to_owned)
                };
                let answer: Option<Value> = if line
                    .starts_with("GET /.well-known/identity-cli-login ")
                {
                    Some(json!({
                        "issuer": at,
                        "authorization_endpoint": format!("{at}/oauth/authorize"),
                        "token_endpoint": format!("{at}/oauth/token"),
                        "access_token_endpoint": format!("{at}/v1/access-token"),
                        "cli_client_id": "conductor-cli",
                        "response_types_supported": ["code"],
                        "grant_types_supported": ["authorization_code"],
                        "code_challenge_methods_supported": ["S256"],
                    }))
                } else if line.starts_with("POST /oauth/token ") {
                    let fields = form(&body);
                    let expected = shared.lock().expect("challenge").clone();
                    let verified = fields
                        .get("code_verifier")
                        .is_some_and(|verifier| Some(s256(verifier)) == expected);
                    fields
                        .get("code")
                        .and_then(|code| code.strip_prefix("code-"))
                        .filter(|who| verified && ["owner", "reader"].contains(who))
                        .filter(|_| {
                            fields.get("grant_type").map(String::as_str)
                                == Some("authorization_code")
                        })
                        .map(|who| {
                            json!({
                                "session": format!("session-{who}"),
                                "session_type": "opaque_server_session",
                                "expires_in": 86400,
                                "tenant_id": "t",
                                "subject": format!("sub-{who}"),
                                "email": format!("{who}@example-org.test"),
                            })
                        })
                } else if line.starts_with("POST /v1/access-token ") {
                    let request: Value = serde_json::from_slice(&body).unwrap_or_default();
                    who("session-")
                        .filter(|_| request["audience"] == "conductor")
                        .map(|who| {
                            json!({
                                "access_token": format!("access-{who}"),
                                "token_type": "Bearer",
                                "expires_in": 300,
                                "audience": "conductor",
                                "scope": request["scope"],
                            })
                        })
                } else if line.starts_with("GET /v1/access-authority ") {
                    who("access-")
                        .filter(|_| {
                            headers.get("x-b10x-audience").map(String::as_str) == Some("conductor")
                        })
                        .map(|who| {
                            json!({
                                "iss": at, "sub": format!("sub-{who}"), "aud": "conductor",
                                "iat": 0, "nbf": 0, "exp": i64::MAX, "jti": "j",
                                "act": {"sub": format!("sub-{who}")}, "scope": "",
                                "principal_kind": "human", "tenant_id": "t",
                                "email": format!("{who}@example-org.test"), "groups": [],
                            })
                        })
                } else {
                    None
                };
                let reply = match answer {
                    Some(body) => {
                        let body = body.to_string();
                        format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nCache-Control: \
                             no-store\r\nPragma: no-cache\r\nContent-Length: {}\r\nConnection: \
                             close\r\n\r\n{body}",
                            body.len()
                        )
                    }
                    None => "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: \
                             close\r\n\r\n"
                        .to_owned(),
                };
                let _ = stream.write_all(reply.as_bytes());
            }
        });
        Self {
            origin,
            challenge,
            seen,
        }
    }
}

/// The daemon process, stopped when dropped.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Starts `daemon serve` on the box until it prints its TLS serving line; its process and what
/// it printed.
fn serve(case: &Case) -> (Running, String) {
    let mut child = case
        .on_box()
        .env("PATH", case.git_only())
        .args(["daemon", "serve", "--state-dir"])
        .arg(case.dir.join("box/daemon"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run daemon serve");
    let stdout = child.stdout.take().expect("stdout");
    let (sender, receiver) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            text.push_str(&line);
            text.push('\n');
            if line.contains("with bearers") {
                let _ = sender.send(text.clone());
            }
        }
        let _ = sender.send(text);
    });
    let text = receiver
        .recv_timeout(Duration::from_secs(60))
        .unwrap_or_default();
    let mut running = Running(child);
    if !text.contains("with bearers") {
        let _ = running.0.kill();
        let mut stderr = String::new();
        let _ = running
            .0
            .stderr
            .take()
            .expect("stderr")
            .read_to_string(&mut stderr);
        panic!("the daemon did not serve its TLS listener: {text}{stderr}");
    }
    (running, text)
}

/// `conductor --daemon <daemon> login` as `who`, the browser played by the case: it reads the
/// authorization address the login prints, hands its challenge to the fake Identity, and calls
/// the login's loopback callback with the code Identity would have issued.
fn login(case: &Case, identity: &FakeIdentity, daemon: &str, who: &str) -> Output {
    let mut child = case
        .workstation(who)
        .args(["--daemon", daemon, "login"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run login");
    let stderr = child.stderr.take().expect("stderr");
    let (sender, receiver) = std::sync::mpsc::channel();
    let collected = Arc::new(Mutex::new(String::new()));
    let all = Arc::clone(&collected);
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            all.lock().expect("stderr").push_str(&format!("{line}\n"));
            if let Some(at) = line.find("http://127.0.0.1").or_else(|| line.find("http")) {
                let url = line[at..].split_whitespace().next().unwrap_or_default();
                if url.contains("/oauth/authorize") {
                    let _ = sender.send(url.to_owned());
                }
            }
        }
    });
    let browser = receiver.recv_timeout(Duration::from_secs(30));
    if let Ok(url) = &browser {
        assert!(
            url.starts_with(&format!("{}/oauth/authorize?", identity.origin)),
            "{url}"
        );
        let fields = query(url);
        assert_eq!(fields["response_type"], "code", "{url}");
        assert_eq!(fields["client_id"], "conductor-cli", "{url}");
        assert_eq!(fields["code_challenge_method"], "S256", "{url}");
        *identity.challenge.lock().expect("challenge") = Some(fields["code_challenge"].clone());
        let redirect = &fields["redirect_uri"];
        let rest = redirect
            .strip_prefix("http://127.0.0.1:")
            .unwrap_or_else(|| panic!("the callback is no loopback address: {redirect}"));
        let (port, path) = rest.split_once('/').unwrap_or((rest, ""));
        let port: u16 = port.parse().expect("the callback's port");
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("reach the callback");
        write!(
            stream,
            "GET /{path}?code=code-{who}&state={} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\
             Connection: close\r\n\r\n",
            fields["state"]
        )
        .expect("call back");
        let mut page = String::new();
        let _ = stream.read_to_string(&mut page);
        assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    }
    let deadline = Instant::now() + Duration::from_secs(60);
    while child.try_wait().expect("wait").is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let mut output = child.wait_with_output().expect("login's output");
    thread::sleep(Duration::from_millis(100));
    output.stderr = collected.lock().expect("stderr").clone().into_bytes();
    assert!(
        browser.is_ok(),
        "login printed no authorization address: {}",
        said(&output)
    );
    output
}

fn config_put(case: &Case, who: &str, daemon: &str, value: &str) -> Output {
    case.workstation(who)
        .args(["--daemon", daemon, "config", "put", "--level", "global"])
        .args([
            "--path",
            "cadence.watch",
            "--value",
            value,
            "--reason",
            "faster watch",
        ])
        .stdin(Stdio::null())
        .output()
        .expect("run config put")
}

#[test]
fn loopback_off_without_a_tls_listener_is_refused_by_its_yaml_path() {
    let case = Case::new("loopback-off");
    case.config("{loopback: false}");
    let output = case
        .on_box()
        .args(["config", "validate"])
        .output()
        .expect("run config validate");
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("daemon.loopback"),
        "{}",
        said(&output)
    );

    case.config("{loopback: false, tls: {listen: \"127.0.0.1:0\"}}");
    let output = case
        .on_box()
        .args(["config", "show", "--format", "json"])
        .output()
        .expect("run config show");
    assert_eq!(output.status.code(), Some(0), "{}", said(&output));
    let shown: Value = serde_json::from_slice(&output.stdout).expect("config show's JSON");
    let loopback = shown
        .pointer("/daemon/loopback")
        .or_else(|| shown.pointer("/config/daemon/loopback"));
    assert_eq!(loopback, Some(&Value::Bool(false)), "{shown}");
}

#[test]
fn a_remote_daemon_takes_a_logged_in_workstations_writes_as_its_operator() {
    let case = Case::new("remote");
    let identity = FakeIdentity::start();
    case.certificate();
    let (loopback, tls) = (free_port(), free_port());
    case.config(&format!(
        "{{port: {loopback}, loopback: false, \
         tls: {{listen: \"127.0.0.1:{tls}\", cert: {cert:?}, key: {key:?}, identity: {identity:?}}}, \
         secrets: {{source: remote, url: \"https://secrets.example-org.test\"}}, \
         operators: [{{name: owner@example-org.test, role: owner, grants: [change-settings]}}, \
                     {{name: reader@example-org.test, role: reader, grants: []}}]}}",
        cert = case.dir.join("box/cert.pem"),
        key = case.dir.join("box/key.pem"),
        identity = identity.origin,
    ));
    case.commit_config();
    let (_daemon, started) = serve(&case);
    assert!(started.contains("loopback listener off"), "{started}");
    assert!(
        TcpStream::connect(("127.0.0.1", loopback)).is_err(),
        "the loopback listener answers on {loopback}: {started}"
    );
    let daemon = format!("https://localhost:{tls}");

    // No login: refused by name, and nothing written.
    let unsent = config_put(&case, "owner", &daemon, "5m");
    assert_eq!(unsent.status.code(), Some(1), "{}", said(&unsent));
    assert!(
        String::from_utf8_lossy(&unsent.stderr).contains("conductor login"),
        "{}",
        said(&unsent)
    );

    // A command with no global-API route is refused by name.
    let doctor = case
        .workstation("owner")
        .args(["--daemon", &daemon, "doctor"])
        .output()
        .expect("run doctor");
    assert_eq!(doctor.status.code(), Some(1), "{}", said(&doctor));
    let refusal = String::from_utf8_lossy(&doctor.stderr);
    assert!(refusal.contains("--daemon"), "{refusal}");
    assert!(refusal.contains("doctor"), "{refusal}");

    // Login: the session in the keychain, never on the screen.
    let logged = login(&case, &identity, &daemon, "owner");
    assert_eq!(logged.status.code(), Some(0), "{}", said(&logged));
    assert!(
        String::from_utf8_lossy(&logged.stdout).contains("owner@example-org.test"),
        "{}",
        said(&logged)
    );
    assert!(
        !said(&logged).contains("session-owner"),
        "{}",
        said(&logged)
    );
    let keychain =
        fs::read_to_string(case.dir.join("owner/.test-keychain.ron")).expect("the keychain");
    // The keychain holds the login under the daemon's name in the login namespace, sealed by
    // the `secrets` crates: the session is not written in the clear even there. That it is the
    // session the next command presents, the write below shows.
    assert!(
        keychain.contains(&format!("conductor-login/default/daemon/localhost.{tls}")),
        "the login is not kept under the daemon's name: {keychain}"
    );
    assert!(
        !keychain.contains("session-owner"),
        "the session is in the clear"
    );

    // A write through the daemon: committed in the config repository, by the operator.
    let changed = config_put(&case, "owner", &daemon, "5m");
    assert_eq!(changed.status.code(), Some(0), "{}", said(&changed));
    assert!(
        !said(&changed).contains("access-owner"),
        "{}",
        said(&changed)
    );
    let text = fs::read_to_string(case.file()).expect("read the config file");
    assert!(text.contains("watch: 5m"), "{text}");
    let log = Command::new("git")
        .arg("-C")
        .arg(case.repository())
        .args(["log", "-1", "--format=%s"])
        .output()
        .expect("git log");
    let subject = String::from_utf8_lossy(&log.stdout);
    assert!(
        subject.contains("by human:owner@example-org.test"),
        "{subject}"
    );

    // A bearer without the grant: refused with the grant's name, nothing written.
    let reader = login(&case, &identity, &daemon, "reader");
    assert_eq!(reader.status.code(), Some(0), "{}", said(&reader));
    let refused = config_put(&case, "reader", &daemon, "7m");
    assert_eq!(refused.status.code(), Some(1), "{}", said(&refused));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("ChangeSettings"),
        "{}",
        said(&refused)
    );
    let text = fs::read_to_string(case.file()).expect("read the config file");
    assert!(!text.contains("watch: 7m"), "{text}");
    let seen = identity.seen.lock().expect("seen").clone();
    assert!(
        seen.iter()
            .any(|line| line.starts_with("POST /v1/access-token ")),
        "{seen:?}"
    );
}

/// The service manager of an install case: the calls made.
#[derive(Debug, Default)]
struct Fake {
    calls: Vec<String>,
}

impl Units for Fake {
    fn start(&mut self, service: &Service) -> Result<()> {
        bail!("no runtime starts here: {}", service.unit)
    }
    fn stop(&mut self, unit: &str) -> Result<()> {
        bail!("no runtime stops here: {unit}")
    }
    fn active(&mut self, _: &str) -> Result<bool> {
        Ok(false)
    }
    fn enable(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("enable --user {unit}"));
        Ok(())
    }
    fn disable(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("disable --user {unit}"));
        Ok(())
    }
    fn enable_system(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("enable --system {unit}"));
        Ok(())
    }
    fn disable_system(&mut self, unit: &str) -> Result<()> {
        self.calls.push(format!("disable --system {unit}"));
        Ok(())
    }
}

#[test]
fn a_box_with_no_local_operator_installs_a_system_unit_from_a_git_checkout_on_the_secrets_service()
{
    let case = Case::new("install");
    let tls = "tls: {listen: \"0.0.0.0:7443\", cert: /etc/c.pem, key: /etc/k.pem, \
               identity: \"https://identity.example-org.test\"}";
    // Not in a git checkout: refused before the service manager is reached.
    case.config(&format!(
        "{{loopback: false, {tls}, secrets: {{source: remote, url: \"https://secrets.example-org.test\"}}}}"
    ));
    let output = case
        .on_box()
        .env("PATH", case.dir.join("no-programs"))
        .args(["daemon", "serve", "--install"])
        .output()
        .expect("run daemon serve --install");
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("git checkout"),
        "{}",
        said(&output)
    );

    // In a checkout, on the OS keychain: refused, naming daemon.secrets.source.
    case.config(&format!("{{loopback: false, {tls}}}"));
    case.commit_config();
    let output = case
        .on_box()
        .env("PATH", case.dir.join("no-programs"))
        .args(["daemon", "serve", "--install"])
        .output()
        .expect("run daemon serve --install");
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("daemon.secrets.source"),
        "{}",
        said(&output)
    );

    // The system unit: the daemon's own user and home, started at boot, enabled system-wide.
    active::isolate();
    let text = install::system_unit_text(
        Path::new("/usr/local/bin/conductor"),
        Path::new("/srv/config/conductor.yaml"),
        Some("conductor"),
        Path::new("/srv/conductor"),
        Some("/usr/bin"),
    );
    assert!(text.contains("WantedBy=multi-user.target"), "{text}");
    assert!(text.contains("User=conductor"), "{text}");
    assert!(
        text.contains("Environment=\"HOME=/srv/conductor\""),
        "{text}"
    );
    assert!(
        text.contains("\"--config\" \"/srv/config/conductor.yaml\" \"daemon\" \"serve\""),
        "{text}"
    );
    let file = case.dir.join("box/system/conductord.service");
    let mut units = Fake::default();
    let line = install::install_system(&mut units, &file, &text).expect("install");
    assert!(line.contains("system unit"), "{line}");
    assert_eq!(fs::read_to_string(&file).expect("the unit file"), text);
    assert_eq!(units.calls, ["enable --system conductord.service"]);
    let line = install::uninstall_system(&mut units, &file).expect("uninstall");
    assert!(!file.exists(), "{line}");
    assert_eq!(
        units.calls,
        [
            "enable --system conductord.service",
            "disable --system conductord.service"
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// The workstation's browser and the Decisions page against the remote daemon
// ---------------------------------------------------------------------------------------------

/// The token the daemon minted for the instance of an [`Instance`] case.
const INSTANCE_TOKEN: &str = "3333333333333333333333333333333333333333333333333333333333333333";

/// The escalated yes-no request every [`Instance`] case holds: a story approval the controller
/// raised.
const APPROVAL: &str = "REQ-approve-story";

/// An instance with one escalated request, served on its socket in this process, behind the
/// daemon's TLS listener (in this process too) with the fake Identity service.
struct Instance {
    dir: PathBuf,
    /// The TLS listener's port.
    port: u16,
    /// The listener's certificate, DER.
    certificate: Vec<u8>,
}

impl Drop for Instance {
    fn drop(&mut self) {
        if !thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Instance {
    fn start(name: &str, identity: &FakeIdentity) -> Self {
        use conductor_cli::daemon::api::{self, Api, Identity, Route};
        use conductor_model::config::{DaemonOperator, OperatorGrant, OperatorRole};
        use conductor_model::operator::OperatorName;

        active::isolate();
        // Short: the instance's socket path must fit a Unix socket address.
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("dr")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        let home = dir.join("h");
        let records = home.join("r");
        for sub in ["dispatches", "decisions"] {
            fs::create_dir_all(records.join(sub)).expect("create the records");
        }
        fs::create_dir_all(home.join("watch")).expect("create watch/");
        fs::write(home.join("agents.json"), "[]").expect("write the session list");
        let mut case = Self {
            dir,
            port: 0,
            certificate: Vec::new(),
        };
        case.decide(&[
            "decision",
            "raise-decision-request",
            "--request-id",
            APPROVAL,
            "--repository",
            "alpha",
            "--question",
            "Approve story:example for the next wave?",
            "--options",
            "Y: approve; N: not now",
            "--recommendation",
            "Y",
            "--raised-at",
            "2026-11-08T09:00:00Z",
        ]);
        case.decide(&[
            "decision",
            "escalate-request",
            "--request-id",
            APPROVAL,
            "--escalation-class",
            "O",
            "--reason",
            "a story approval is the operator's",
            "--kind",
            "yes-no",
            "--lettered-options",
            r#"[{"letter":"Y","label":"approve","cost":"a wave"},{"letter":"N","label":"not now","cost":"none"}]"#,
            "--recommended-letters",
            "Y",
            "--default-letters",
            "N",
            "--deadline",
            "2026-11-09T09:00:00Z",
        ]);

        let socket = case.dir.join("a.sock");
        let listener =
            std::os::unix::net::UnixListener::bind(&socket).expect("bind the instance's socket");
        let mut sources = conductor_cli::dashboard::Sources::new(home.clone(), records);
        let cat = |file: &Path| vec![std::ffi::OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = case.state();
        sources.instance_name = Some("example-org".to_owned());
        let state = case.state();
        thread::spawn(move || {
            conductor_cli::dashboard::serve_unix(
                listener,
                sources,
                Some(state),
                INSTANCE_TOKEN.to_owned(),
            );
        });

        let operator = |name: &str, grants: &[OperatorGrant]| DaemonOperator {
            name: OperatorName(name.to_owned()),
            role: OperatorRole("member".to_owned()),
            grants: grants.to_vec(),
        };
        let identity = Identity::new(&identity.origin, api::AUDIENCE).expect("an Identity origin");
        let daemon = Arc::new(Api::new(
            "4444444444444444444444444444444444444444444444444444444444444444".to_owned(),
            "operator".to_owned(),
            Some(identity),
        ));
        daemon.publish(
            vec![json!({"name": "example-org", "state": "Running"})],
            [(
                "example-org".to_owned(),
                Route {
                    socket,
                    token: INSTANCE_TOKEN.to_owned(),
                    remote: None,
                },
            )]
            .into_iter()
            .collect(),
            vec![
                operator("owner@example-org.test", &[OperatorGrant::AnswerDecisions]),
                operator("reader@example-org.test", &[]),
            ],
        );
        let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])
            .expect("a certificate");
        fs::write(case.dir.join("cert.pem"), generated.cert.pem()).expect("write the cert");
        fs::write(case.dir.join("key.pem"), generated.key_pair.serialize_pem())
            .expect("write the key");
        let tls = api::tls_config(&case.dir.join("cert.pem"), &case.dir.join("key.pem"))
            .expect("the TLS listener's config");
        let secure = TcpListener::bind(("127.0.0.1", 0)).expect("bind the TLS listener");
        let port = secure.local_addr().expect("address").port();
        thread::spawn(move || api::serve_tls(&secure, &daemon, &tls));
        case.port = port;
        case.certificate = generated.cert.der().to_vec();
        case
    }

    fn state(&self) -> PathBuf {
        self.dir.join("s")
    }

    /// Runs the built binary with `args` over the instance's store; its standard output.
    fn decide(&self, args: &[&str]) -> String {
        let output = common::conductor(self.dir.join("h"))
            .current_dir(&self.dir)
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("run conductor");
        assert!(output.status.success(), "{args:?}: {}", said(&output));
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// The rows of the view `view` of the instance's store.
    fn view(&self, view: &str) -> Vec<Value> {
        serde_json::from_str::<Value>(&self.decide(&["decision", view, "--format", "json"]))
            .expect("a view's JSON")
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    /// One request to the TLS listener with `headers`; its status, head and body.
    fn send(
        &self,
        method: &str,
        target: &str,
        headers: &[(&str, &str)],
        body: Option<&Value>,
    ) -> (u16, String, String) {
        let body = body.map(Value::to_string).unwrap_or_default();
        let mut request = format!(
            "{method} {target} HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n",
            self.port
        );
        for (name, value) in headers {
            request.push_str(&format!("{name}: {value}\r\n"));
        }
        if method != "GET" {
            request.push_str(&format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            ));
        }
        request.push_str("\r\n");
        request.push_str(&body);
        let mut roots = rustls::RootCertStore::empty();
        roots
            .add(rustls::pki_types::CertificateDer::from(
                self.certificate.clone(),
            ))
            .expect("trust the certificate");
        let config = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from("localhost").expect("a server name");
        let connection =
            rustls::ClientConnection::new(Arc::new(config), name).expect("a TLS connection");
        let tcp = TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
        tcp.set_read_timeout(Some(Duration::from_secs(60)))
            .expect("a read timeout");
        let mut tls = rustls::StreamOwned::new(connection, tcp);
        tls.write_all(request.as_bytes()).expect("send");
        let mut bytes = Vec::new();
        match tls.read_to_end(&mut bytes) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {}
            Err(error) => panic!("read over TLS: {error}"),
        }
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0);
        (status, head.to_owned(), body.to_owned())
    }

    /// The Decisions page's answer to [`APPROVAL`], approving it.
    fn approve(&self, headers: &[(&str, &str)]) -> (u16, String, String) {
        self.send(
            "POST",
            "/v1/instances/example-org/answers",
            headers,
            Some(&json!({"answers": [{"request_id": APPROVAL, "choice": ["Y"]}]})),
        )
    }
}

fn header<'a>(head: &'a str, name: &str) -> Option<&'a str> {
    head.lines().find_map(|line| {
        line.split_once(':')
            .filter(|(key, _)| key.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
    })
}

/// A story approval is a request the controller raised and an operator answered: through the
/// remote daemon, an operator holding AnswerDecisions answers it on the Decisions page's route,
/// recorded as theirs; one without the grant is refused naming it, and nothing is recorded.
#[test]
fn an_operator_with_answer_decisions_approves_a_story_through_the_remote_daemon() {
    let identity = FakeIdentity::start();
    let case = Instance::start("approve", &identity);

    let refused = case.approve(&[("Authorization", "Bearer access-reader")]);
    assert_eq!(refused.0, 403, "{refused:?}");
    let body: Value = serde_json::from_str(&refused.2).expect("a JSON refusal");
    assert_eq!(body["error"], "conductor.operator.GrantMissing", "{body}");
    assert_eq!(body["grant"], "AnswerDecisions", "{body}");
    assert!(case.view("decisions").is_empty());

    let answered = case.approve(&[("Authorization", "Bearer access-owner")]);
    assert_eq!(answered.0, 200, "{answered:?}");
    let decisions = case.view("decisions");
    assert_eq!(decisions.len(), 1, "{decisions:?}");
    assert_eq!(decisions[0]["choice"], "Y", "{}", decisions[0]);
    assert_eq!(decisions[0]["decided_by"], "Operator", "{}", decisions[0]);
    assert_eq!(
        decisions[0]["actor"], "human:owner@example-org.test",
        "{}",
        decisions[0]
    );
    let request = case
        .view("requests")
        .into_iter()
        .find(|row| row["request_id"] == APPROVAL)
        .expect("the approval request");
    assert_eq!(request["state"], "Answered", "{request}");
}

/// The dashboard from a workstation: `GET /login` on the TLS listener runs the Identity
/// authorization-code flow with PKCE and sets a short-lived HttpOnly, Secure, SameSite=Strict
/// session cookie bound to the operator; the listener takes that cookie as it takes a bearer, and
/// a write with it still needs the listener's own origin and the operator's grant.
#[test]
fn the_browser_logs_in_through_identity_and_its_cookie_is_taken_like_a_bearer() {
    let identity = FakeIdentity::start();
    let case = Instance::start("browser", &identity);
    let own = format!("https://localhost:{}", case.port);

    let login = |who: &str| -> String {
        let (status, head, body) = case.send("GET", "/login", &[], None);
        assert_eq!(status, 302, "{head}\n{body}");
        let location = header(&head, "location").expect("a Location").to_owned();
        assert!(
            location.starts_with(&format!("{}/oauth/authorize?", identity.origin)),
            "{location}"
        );
        let fields = query(&location);
        assert_eq!(fields["response_type"], "code", "{location}");
        assert_eq!(fields["code_challenge_method"], "S256", "{location}");
        assert_eq!(
            fields["redirect_uri"],
            format!("{own}/login/callback"),
            "{location}"
        );
        *identity.challenge.lock().expect("challenge") = Some(fields["code_challenge"].clone());

        let forged = case.send(
            "GET",
            &format!("/login/callback?code=code-{who}&state=forged"),
            &[],
            None,
        );
        assert_eq!(forged.0, 400, "{forged:?}");
        assert!(header(&forged.1, "set-cookie").is_none(), "{}", forged.1);

        let (status, head, body) = case.send(
            "GET",
            &format!("/login/callback?code=code-{who}&state={}", fields["state"]),
            &[],
            None,
        );
        assert_eq!(status, 302, "{head}\n{body}");
        assert_eq!(header(&head, "location"), Some("/"), "{head}");
        let cookie = header(&head, "set-cookie").expect("a session cookie");
        for flag in [
            "HttpOnly",
            "Secure",
            "SameSite=Strict",
            "Path=/",
            "Max-Age=",
        ] {
            assert!(cookie.contains(flag), "{flag} missing: {cookie}");
        }
        assert!(
            !cookie.contains("session-"),
            "the Identity session leaks: {cookie}"
        );
        cookie.split(';').next().expect("name=value").to_owned()
    };

    let (status, _, body) = case.send("GET", "/v1/instances", &[], None);
    assert_eq!(status, 401, "{body}");

    let owner = login("owner");
    let (status, _, body) = case.send("GET", "/v1/instances", &[("Cookie", &owner)], None);
    assert_eq!(status, 200, "{body}");
    let forged = case.send(
        "GET",
        "/v1/instances",
        &[("Cookie", "conductor_session=0123456789abcdef")],
        None,
    );
    assert_eq!(forged.0, 401, "{forged:?}");

    // A write with the cookie: only from the listener's own origin.
    let elsewhere = case.approve(&[
        ("Cookie", &owner),
        ("Origin", "https://elsewhere.example-org.test"),
    ]);
    assert_eq!(elsewhere.0, 403, "{elsewhere:?}");
    let unnamed = case.approve(&[("Cookie", &owner)]);
    assert_eq!(unnamed.0, 403, "{unnamed:?}");
    assert!(case.view("decisions").is_empty());

    // The operator's grant still decides.
    let reader = login("reader");
    let refused = case.approve(&[("Cookie", &reader), ("Origin", &own)]);
    assert_eq!(refused.0, 403, "{refused:?}");
    assert!(refused.2.contains("AnswerDecisions"), "{refused:?}");
    assert!(case.view("decisions").is_empty());

    let answered = case.approve(&[("Cookie", &owner), ("Origin", &own)]);
    assert_eq!(answered.0, 200, "{answered:?}");
    let decisions = case.view("decisions");
    assert_eq!(decisions.len(), 1, "{decisions:?}");
    assert_eq!(
        decisions[0]["actor"], "human:owner@example-org.test",
        "{}",
        decisions[0]
    );
}
