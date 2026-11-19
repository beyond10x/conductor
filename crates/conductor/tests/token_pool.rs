//! `story:token-pool`, `story:host-token-pool` (design architecture-next § 5.3, § 17): an engineer
//! joins and contributes their Claude token to the host's pool, which conductord holds; the
//! `claude-code` kind of every instance on the host leases from it.
//!
//! - `conductor join --daemon <url> [--instance <i>]` against a running `conductor daemon serve`
//!   puts the caller's token, read from standard input, into the host's pool, and with
//!   `--instance` records the caller as an operator of that instance: `GET /v1/tokens` lists its
//!   owner, seat and state, never the value nor the secret's name; `conductor leave` withdraws it.
//! - `conductor session start` of any instance on the host leases a token, one lease per session:
//!   the value reaches the harness in its process's environment (`CLAUDE_CODE_OAUTH_TOKEN` or
//!   `ANTHROPIC_API_KEY`), its settings name no `apiKeyHelper`, and nothing else carries it; after
//!   a usage limit marks the token Limited, the next start runs on another token. No line of the
//!   daemon's output, of the harness's arguments or of any file under the instance's state and
//!   records carries the value.
//! - A runtime in a box, with no daemon database, leases, releases and marks limited through
//!   conductord over its dial-in connection, as the instance conductord admitted it as.
//! - The pool's rules on the library: the token with the fewest live leases, a cap per kind, a
//!   named owner's own token only (`NoTokenAvailable` otherwise), a Limited token taken again
//!   only once its reset passed, never on a probe's headroom, only the owner withdraws, a lease never outlives a withdrawal,
//!   the leases of a runtime start that ended are reaped, a limit is clamped to a week.
//! - `roles[].token_owner` is a controller's key only.
//!
//! Every case runs on the fake keychain `common::conductor` names under the case's home.

mod active;
mod common;
mod fake_provider;

use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::config;
use conductor_cli::confine::Launch;
use conductor_cli::daemon::api::{Api, Route};
use conductor_cli::token;
use conductor_model::agent::AgentName;
use conductor_model::behaviour::{OperatorStorage, PoolTokenStorage, TokenLeaseStorage};
use conductor_model::config::{InstanceName, Limits, ModelTokenKind, Network, SessionName};
use conductor_model::operator::{Actor, ActorKind, OperatorName, OperatorState};
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::secret::SecretName;
use conductor_model::session::SessionRequest;
use conductor_model::token::{
    PoolTokenData, PoolTokenSnapshot, PoolTokenState, Seat, TokenId, TokenLeaseState, TokenState,
    WithdrawTokenOutcome,
};
use fake_provider::{Child, calls};
use serde_json::Value;

/// A token value no output may carry.
const VALUE: &str = "sk-example-org-token-7d3a91c4";

/// A second token value, for the resume.
const SECOND: &str = "sk-example-org-token-2b8e55f0";

/// The child run of this test binary ([`fake_provider::command`]); a plain run of the suite
/// passes it at once.
#[test]
fn child() {
    if let Some(run) = Child::from_env() {
        if let Some(file) = run.config_file() {
            config::set_active(Some(&file)).expect("the case's config loads");
        }
        run.exit();
    }
}

struct Case {
    dir: PathBuf,
    port: u16,
    daemon: Option<std::process::Child>,
}

impl Drop for Case {
    fn drop(&mut self) {
        if let Some(mut daemon) = self.daemon.take() {
            let _ = daemon.kill();
            let _ = daemon.wait();
        }
    }
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .expect("a free port")
}

fn said(output: &Output) -> String {
    format!(
        "exit {:?}\n{}{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

impl Case {
    /// The case `name`: a config file of one instance `alpha` with a director and a controller
    /// role, a SQLite store, the keychain source, and a `claude` on the case's `PATH` that lists no
    /// live session and logs every other call.
    fn new(name: &str, roles: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("token-pool")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        for sub in ["home", "bin", "records", "state", "work", "fake"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
            port: free_port(),
            daemon: None,
        };
        let text = format!(
            "version: conductor.config/2\n\
             daemon: {{port: {port}, store: \"sqlite:{store}\", secrets: {{source: keychain}}}}\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: box-a\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
             \x20   records: {records}\n\
             \x20   state: {state}\n\
             \x20   cache: {cache}\n\
             \x20   roles:\n{roles}",
            port = case.port,
            store = case.store_file().display(),
            root = case.dir.join("home/example-org").display(),
            trees = case.dir.join("trees").display(),
            records = case.dir.join("records").display(),
            state = case.dir.join("state").display(),
            cache = case.dir.join("cache").display(),
        );
        fs::write(case.file(), text).expect("write the config file");
        let claude = case.dir.join("bin/claude");
        fs::write(
            &claude,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = agents ]; then echo '[]'; exit 0; fi\n\
                 if [ \"$1\" = --version ]; then echo '9.9.9 (Claude Code)'; exit 0; fi\n\
                 printf '%s\\n' \"$*\" >> '{log}'\n\
                 echo 'session 0190a1b2-0000-7000-8000-000000000001'\n",
                log = case.dir.join("claude.log").display()
            ),
        )
        .expect("write the fake claude");
        fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("runnable");
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn store_file(&self) -> PathBuf {
        self.dir.join("shared.sqlite")
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn conductor(&self) -> Command {
        let mut command = common::conductor(self.home());
        command
            .arg("--config")
            .arg(self.file())
            .current_dir(self.dir.join("work"))
            // The case's `claude` only: no `systemd-run` or `systemctl`, so every start goes
            // through the `none` provider and reaches no service manager of the host.
            .env("PATH", self.dir.join("bin"))
            .env("TMPDIR", self.dir.join("tmp"));
        command
    }

    /// Starts `daemon serve`, its output in `daemon.log`, and waits for its listener.
    fn serve(&mut self) {
        let empty = self.dir.join("no-programs");
        fs::create_dir_all(&empty).expect("an empty PATH directory");
        let log = fs::File::create(self.dir.join("daemon.log")).expect("the daemon's log");
        let child = self
            .conductor()
            .env("PATH", &empty)
            // Its directory is `~/.b10x/conductor/daemon` of the case's home, where a local client
            // finds the token it keeps.
            .args(["daemon", "serve"])
            .stdout(Stdio::from(log.try_clone().expect("the log")))
            .stderr(Stdio::from(log))
            .spawn()
            .expect("run daemon serve");
        self.daemon = Some(child);
        let deadline = Instant::now() + Duration::from_secs(60);
        while TcpStream::connect(("127.0.0.1", self.port)).is_err() {
            assert!(
                Instant::now() < deadline,
                "the daemon did not listen: {}",
                self.daemon_log()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }

    fn daemon_log(&self) -> String {
        fs::read_to_string(self.dir.join("daemon.log")).unwrap_or_default()
    }

    /// `conductor join` with `value` on standard input, of the default kind.
    fn join(&self, seat: &str, value: &str) -> Output {
        self.join_with(seat, value, &[])
    }

    /// `conductor join` with `value` on standard input and the arguments `more`.
    fn join_with(&self, seat: &str, value: &str, more: &[&str]) -> Output {
        let mut child = self
            .conductor()
            .args([
                "join",
                "--daemon",
                &self.url(),
                "--instance",
                "alpha",
                "--seat",
                seat,
            ])
            .args(more)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("run conductor join");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(format!("{value}\n").as_bytes())
            .expect("write the token");
        child.wait_with_output().expect("wait for join")
    }

    fn get(&self, path: &str) -> (u16, Value) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
            self.port
        )
        .expect("send");
        let mut answer = String::new();
        stream.read_to_string(&mut answer).expect("read");
        let status = answer
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or(0);
        let body = answer
            .split_once("\r\n\r\n")
            .map_or("", |(_, body)| body)
            .trim();
        (status, serde_json::from_str(body).unwrap_or(Value::Null))
    }

    fn tokens(&self) -> Vec<Value> {
        let (status, rows) = self.get("/v1/instances/alpha/tokens");
        assert_eq!(status, 200, "GET tokens: {rows}; {}", self.daemon_log());
        rows.as_array().cloned().unwrap_or_default()
    }

    fn store(&self) -> conductor_cli::store::Store {
        conductor_cli::store::open_url(&self.store_url(), conductor_cli::daemon::TENANT)
            .expect("open the shared store")
    }

    fn store_url(&self) -> String {
        format!("sqlite:{}", self.store_file().display())
    }

    /// Every file under `dir`, read as text where it is one.
    fn texts(dir: &Path, into: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                Self::texts(&path, into);
            } else if let Ok(bytes) = fs::read(&path) {
                into.push((path, String::from_utf8_lossy(&bytes).into_owned()));
            }
        }
    }

    /// Fails naming each place that carries `value`: the daemon's output, the shared store, the
    /// harness's launches (the fake provider's calls), and every file under the instance's state
    /// and records and the daemon's directory. The fake keychain holds it, and only it.
    fn assert_nowhere(&self, value: &str) {
        let mut texts = vec![(self.dir.join("daemon.log"), self.daemon_log())];
        for file in ["shared.sqlite", "shared.sqlite-wal"] {
            if let Ok(bytes) = fs::read(self.dir.join(file)) {
                texts.push((
                    self.dir.join(file),
                    String::from_utf8_lossy(&bytes).into_owned(),
                ));
            }
        }
        let mut dirs: Vec<_> = ["state", "records", "fake", "tmp"]
            .iter()
            .map(|dir| self.dir.join(dir))
            .collect();
        dirs.push(self.dir.join("home").join(".b10x"));
        for dir in dirs {
            Self::texts(&dir, &mut texts);
        }
        let carrying: Vec<String> = texts
            .iter()
            .filter(|(_, text)| text.contains(value))
            .map(|(path, _)| path.display().to_string())
            .collect();
        assert!(carrying.is_empty(), "a token value is in {carrying:?}");
    }
}

const ROLES: &str = "\x20     - {role: director, harness: claude, model: opus}\n\
                     \x20     - {role: controller, harness: claude, model: opus}\n";

#[test]
fn join_contributes_a_token_and_leave_withdraws_it_never_printing_its_value() {
    let mut case = Case::new("join-leave", ROLES);
    case.serve();

    let joined = case.join("operator@example-org.example", VALUE);
    assert_eq!(
        joined.status.code(),
        Some(0),
        "{}\n{}",
        said(&joined),
        case.daemon_log()
    );
    assert!(!said(&joined).contains(VALUE), "{}", said(&joined));

    let rows = case.tokens();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row["owner"], "operator", "{row}");
    assert_eq!(row["seat"], "operator@example-org.example", "{row}");
    assert_eq!(row["state"], "Available", "{row}");
    assert_eq!(row["token_state"], "Ok", "{row}");
    let shown = Value::Array(rows.clone()).to_string();
    assert!(!shown.contains(VALUE), "{shown}");
    assert!(
        !shown.contains("claude-token/"),
        "the secret's name is listed: {shown}"
    );

    let store = case.store();
    let operators = OperatorStorage::list(&store);
    let operator = operators
        .iter()
        .find(|found| found.data.key.operator.0 == "operator")
        .unwrap_or_else(|| panic!("no operator is recorded"));
    assert_eq!(operator.state, OperatorState::Active);
    assert_eq!(operator.data.key.instance.0, "alpha");
    assert!(
        !operator.data.grants.is_empty(),
        "the operator holds no grant"
    );

    let left = case
        .conductor()
        .args(["leave", "--daemon", &case.url(), "--instance", "alpha"])
        .output()
        .expect("run conductor leave");
    assert_eq!(
        left.status.code(),
        Some(0),
        "{}\n{}",
        said(&left),
        case.daemon_log()
    );
    let rows = case.tokens();
    assert_eq!(rows[0]["state"], "Withdrawn", "{rows:?}");
    case.assert_nowhere(VALUE);
}

#[test]
fn an_agent_session_joins_nothing() {
    let mut case = Case::new("join-agent", ROLES);
    case.serve();
    let mut child = case
        .conductor()
        .env(conductor_cli::actor::SESSION_VARIABLE, "alpha-controller")
        .args(["join", "--daemon", &case.url(), "--instance", "alpha"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run conductor join");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(VALUE.as_bytes())
        .expect("write");
    let output = child.wait_with_output().expect("wait");
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("ActorNotHuman"),
        "{}",
        said(&output)
    );
    assert!(case.tokens().is_empty());
}

/// The `--settings` file the last start the fake provider was given names.
fn started_settings(case: &Case) -> PathBuf {
    let started = calls(&case.dir.join("fake"))
        .into_iter()
        .rfind(|call| call["call"] == "start")
        .expect("a start");
    let args: Vec<String> = started["args"]
        .as_array()
        .expect("its arguments")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let at = args
        .iter()
        .position(|arg| arg == "--settings")
        .unwrap_or_else(|| panic!("no --settings in {args:?}"));
    PathBuf::from(&args[at + 1])
}

/// `conductor session start --role <role>` over the fake providers ([`fake_provider`]): nothing
/// reaches a real `systemd-run`, `systemctl` or harness.
fn start(case: &Case, role: &str) -> Output {
    let file = case.file();
    let output = fake_provider::command(
        &case.home(),
        &case.dir.join("fake"),
        &[
            "--config",
            file.to_str().expect("a UTF-8 path"),
            "session",
            "start",
            "--role",
            role,
        ],
    )
    .current_dir(case.dir.join("work"))
    .env("PATH", case.dir.join("bin"))
    .env("USER", "operator")
    .env(
        conductor_cli::secret::TEST_KEYCHAIN_VARIABLE,
        case.home().join(".test-keychain.ron"),
    )
    .output()
    .expect("the child runs");
    fake_provider::without_harness(output)
}

fn helper_of(settings: &Path) -> String {
    let text = fs::read_to_string(settings).expect("the settings");
    let json: Value = serde_json::from_str(&text).expect("the settings are JSON");
    json[token::HELPER_KEY]
        .as_str()
        .unwrap_or_else(|| panic!("no {} in {text}", token::HELPER_KEY))
        .to_owned()
}

/// The value the harness double's last launch had in [`API_KEY`], `None` where it was not set.
fn api_key_of(case: &Case) -> Option<String> {
    let set = fs::read_to_string(case.dir.join("harness/apikey-set")).expect("the double ran");
    (set == "set").then(|| {
        fs::read_to_string(case.dir.join("harness/apikey")).expect("the double kept the value")
    })
}

/// The leases of the host's pool, as `GET /v1/tokens/leases` lists them.
fn leases(case: &Case) -> Vec<Value> {
    let (status, rows) = case.get("/v1/tokens/leases");
    assert_eq!(status, 200, "GET leases: {rows}; {}", case.daemon_log());
    rows.as_array().cloned().unwrap_or_default()
}

/// Two API keys joined; a start leases one and its harness gets the value as `ANTHROPIC_API_KEY`,
/// its settings naming no `apiKeyHelper`; a usage limit marks that token Limited and releases the
/// lease, and the agent's next start runs on the other token; leave withdraws both and names the
/// agent on one for its loop.
#[test]
fn a_start_leases_a_token_and_a_limit_resumes_the_next_start_on_another() {
    let mut case = Case::new("lease-resume", ROLES);
    harness_double(&case);
    case.serve();
    for (seat, value) in [
        ("first@example-org.example", VALUE),
        ("second@example-org.example", SECOND),
    ] {
        let joined = case.join_with(seat, value, &["--kind", "ApiKey"]);
        assert_eq!(
            joined.status.code(),
            Some(0),
            "{}\n{}",
            said(&joined),
            case.daemon_log()
        );
    }

    let started = start_built(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    let held: Vec<Value> = leases(&case)
        .into_iter()
        .filter(|row| row["state"] == "Held")
        .collect();
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0]["agent"], "alpha-director", "{held:?}");
    assert_eq!(held[0]["instance"], "alpha", "{held:?}");
    let first = held[0]["token_id"].as_str().expect("an id").to_owned();
    let given = api_key_of(&case).expect("the harness got an API key");
    assert!(given == VALUE || given == SECOND, "another value: {given}");
    assert_eq!(oauth_of(&case), None);
    let text = fs::read_to_string(settings_of(&case)).expect("the settings copy");
    assert!(!text.contains(token::HELPER_KEY), "{text}");

    // A usage limit marks the token Limited and releases the lease; the agent's next start runs
    // on the other token.
    let marked = token::limit(
        &case.store_url(),
        &InstanceName("alpha".to_owned()),
        &AgentName("alpha-director".to_owned()),
        conductor_cli::actor::daemon(),
        None,
    )
    .expect("mark the token limited");
    assert!(marked.is_some(), "the agent held no lease");
    let resumed = start_built(&case, "director");
    assert_eq!(resumed.status.code(), Some(0), "{}", said(&resumed));
    let rows = case.tokens();
    let limited = rows
        .iter()
        .find(|row| row["token_id"] == first.as_str())
        .expect("the first token");
    assert_eq!(limited["state"], "Limited", "{rows:?}");
    assert_eq!(limited["token_state"], "Limited", "{rows:?}");
    let now_on: Vec<Value> = leases(&case)
        .into_iter()
        .filter(|row| row["state"] == "Held")
        .collect();
    assert_eq!(now_on.len(), 1, "{now_on:?}");
    assert_ne!(now_on[0]["token_id"], first.as_str(), "{now_on:?}");
    let again = api_key_of(&case).expect("the harness got an API key");
    assert_ne!(again, given, "the resumed start runs on the other token");

    // The owner leaves: both tokens are withdrawn, the lease on one released, and the agent on
    // it named for the loop to resume on another token.
    let left = case
        .conductor()
        .args(["leave", "--daemon", &case.url()])
        .output()
        .expect("run conductor leave");
    assert_eq!(
        left.status.code(),
        Some(0),
        "{}\n{}",
        said(&left),
        case.daemon_log()
    );
    assert!(
        String::from_utf8_lossy(&left.stdout).contains("alpha-director"),
        "{}",
        said(&left)
    );
    assert!(case.tokens().iter().all(|row| row["state"] == "Withdrawn"));
    assert!(leases(&case).iter().all(|row| row["state"] == "Released"));
    let named = fs::read_to_string(case.dir.join("state/run").join(token::WITHDRAWN_FILE))
        .expect("the agents to resume are named");
    assert!(named.contains("alpha-director"), "{named}");

    for (what, text) in [
        ("the first start's output", said(&started)),
        ("the second start's output", said(&resumed)),
        ("the harness's arguments", argv_of(&case)),
    ] {
        assert!(
            !text.contains(VALUE) && !text.contains(SECOND),
            "{what}: {text}"
        );
    }
    case.assert_nowhere(VALUE);
    case.assert_nowhere(SECOND);
}

#[test]
fn with_no_token_in_the_pool_a_start_is_today_s() {
    let case = Case::new("no-pool", ROLES);
    let started = start(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    let settings = started_settings(&case);
    let text = fs::read_to_string(&settings).expect("the settings");
    assert!(!text.contains(token::HELPER_KEY), "{text}");
    assert!(
        !case.store_file().exists(),
        "a start planted the daemon's store"
    );
}

/// The config file of `case` on the `files` source over `secrets`, `daemon` added to its daemon
/// section, its instance at `placement` naming the model token `secret:alpha-model`.
fn with_model_token(case: &Case, secrets: &Path, daemon: &str, placement: &str) {
    let text = fs::read_to_string(case.file())
        .expect("the config file")
        .replace(
            "secrets: {source: keychain}",
            &format!("secrets: {{source: files, dir: {secrets:?}}}{daemon}"),
        )
        .replace(
            "\x20   placement: box-a\n",
            &format!("\x20   placement: {placement}\n"),
        )
        .replace(
            "\x20   session_prefix: alpha\n",
            "\x20   session_prefix: alpha\n\x20   model_token: secret:alpha-model\n",
        );
    fs::write(case.file(), text).expect("write the config file");
}

/// `story:container-start-without-prompt`: where the config file says `daemon.unattended`, an
/// instance whose placement opens no box and whose pool leases no token runs its agents on its
/// `model_token`: the settings name `conductor token reveal --model-token <instance>` as their
/// `apiKeyHelper`, which prints the value from the daemon's secrets source, here a directory of
/// mounted files; no argument, setting or output of the start carries the value.
#[test]
fn with_no_token_in_the_pool_an_agent_runs_on_its_instance_s_model_token() {
    let case = Case::new("model-token", ROLES);
    let secrets = case.dir.join("secrets");
    fs::create_dir_all(&secrets).expect("the secrets directory");
    fs::write(secrets.join("alpha-model"), format!("{VALUE}\n")).expect("mount the token");
    with_model_token(&case, &secrets, ", unattended: true", "box-a");

    let started = start(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    assert!(!said(&started).contains(VALUE), "{}", said(&started));
    let settings = started_settings(&case);
    let helper = helper_of(&settings);
    assert!(
        helper.ends_with("token reveal --model-token 'alpha'"),
        "{helper}"
    );
    assert!(!helper.contains(VALUE), "{helper}");
    let (_, rest) = helper
        .strip_prefix('\'')
        .and_then(|rest| rest.split_once("' "))
        .unwrap_or_else(|| panic!("the helper starts with no quoted program: {helper}"));
    let binary = common::conductor(case.home()).get_program().to_owned();
    let revealed = Command::new("sh")
        .arg("-c")
        .arg(format!("'{}' {rest}", binary.to_string_lossy()))
        .env_clear()
        .env("HOME", case.home())
        .env("USER", "operator")
        .output()
        .expect("run the helper");
    assert_eq!(revealed.status.code(), Some(0), "{}", said(&revealed));
    assert_eq!(
        String::from_utf8_lossy(&revealed.stdout),
        format!("{VALUE}\n")
    );
    case.assert_nowhere(VALUE);
}

/// Without `daemon.unattended` an empty pool leaves the launch as it was before the model token
/// was read at a placement with no box: an instance at `placement: none` that names a
/// `model_token` starts its agents on the harness's own login, its settings naming no
/// `apiKeyHelper`.
#[test]
fn without_unattended_an_empty_pool_leaves_the_launch_on_the_harness_login() {
    let case = Case::new("model-token-attended", ROLES);
    let secrets = case.dir.join("secrets");
    fs::create_dir_all(&secrets).expect("the secrets directory");
    fs::write(secrets.join("alpha-model"), format!("{VALUE}\n")).expect("mount the token");
    with_model_token(&case, &secrets, "", "none");
    let validated = case
        .conductor()
        .args(["config", "validate"])
        .output()
        .expect("run config validate");
    assert_eq!(validated.status.code(), Some(0), "{}", said(&validated));

    let started = start(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    let settings = started_settings(&case);
    let text = fs::read_to_string(&settings).expect("the settings");
    assert!(!text.contains(token::HELPER_KEY), "{text}");
    case.assert_nowhere(VALUE);
}

// -------------------------------------------------------------------------------------------
// The kind of the model token: the route each kind takes to the harness.
// -------------------------------------------------------------------------------------------

/// A model token's value no output, file or command line may carry.
const MARKER: &str = "marker-subscription-0001";

/// The value the secret holds after it changed.
const CHANGED: &str = "marker-subscription-0002";

/// The variable the harness reads a subscription token from.
const OAUTH: &str = "CLAUDE_CODE_OAUTH_TOKEN";

/// The variable the harness reads an API key from.
const API_KEY: &str = "ANTHROPIC_API_KEY";

/// Makes `case`'s `claude` a harness double that keeps, for its last launch, each argument on a
/// line of `harness/argv`, the value of [`OAUTH`] in its environment in `harness/oauth`, and
/// `set` in `harness/oauth-set` where the variable is set at all. `claude agents` lists no
/// session. It runs shell builtins only: the case's `PATH` holds no other program.
fn harness_double(case: &Case) {
    let harness = case.dir.join("harness");
    fs::create_dir_all(&harness).expect("the harness double's directory");
    fs::create_dir_all(case.dir.join("tmp")).expect("the case's TMPDIR");
    let claude = case.dir.join("bin/claude");
    fs::write(
        &claude,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = agents ]; then echo '[]'; exit 0; fi\n\
             if [ \"$1\" = --version ]; then echo '9.9.9 (Claude Code)'; exit 0; fi\n\
             printf '%s\\n' \"$@\" > '{argv}'\n\
             printf '%s' \"${{{OAUTH}-}}\" > '{oauth}'\n\
             printf '%s' \"${{{OAUTH}+set}}\" > '{set}'\n\
             printf '%s' \"${{{API_KEY}-}}\" > '{apikey}'\n\
             printf '%s' \"${{{API_KEY}+set}}\" > '{apikey_set}'\n\
             echo 'session 0190a1b2-0000-7000-8000-000000000001'\n",
            argv = harness.join("argv").display(),
            oauth = harness.join("oauth").display(),
            set = harness.join("oauth-set").display(),
            apikey = harness.join("apikey").display(),
            apikey_set = harness.join("apikey-set").display(),
        ),
    )
    .expect("write the harness double");
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).expect("runnable");
}

/// The value of [`OAUTH`] the harness double's last launch had, `None` where it was not set.
fn oauth_of(case: &Case) -> Option<String> {
    let set = fs::read_to_string(case.dir.join("harness/oauth-set")).expect("the double ran");
    (set == "set").then(|| {
        fs::read_to_string(case.dir.join("harness/oauth")).expect("the double kept the value")
    })
}

/// The arguments of the harness double's last launch.
fn argv_of(case: &Case) -> String {
    fs::read_to_string(case.dir.join("harness/argv")).expect("the harness double was launched")
}

/// The settings file the harness double's last launch was given with `--settings`.
fn settings_of(case: &Case) -> PathBuf {
    let argv = argv_of(case);
    let words: Vec<&str> = argv.lines().collect();
    let at = words
        .iter()
        .position(|word| *word == "--settings")
        .unwrap_or_else(|| panic!("no --settings in {argv}"));
    PathBuf::from(words[at + 1])
}

/// `conductor session start --role <role>` of the built binary, through the `none` provider: it
/// launches the case's harness double with the launch's environment, the variables of this
/// process taken out but the case's own.
fn start_built(case: &Case, role: &str) -> Output {
    case.conductor()
        .env_remove(OAUTH)
        .env_remove(API_KEY)
        .args(["session", "start", "--role", role])
        .output()
        .expect("run session start")
}

/// [`with_model_token`] at a placement that opens no box, in a file that says
/// `daemon.unattended`, the instance's `model_token_kind` `kind` where one is given, and the
/// director's own settings file `settings`, which names an `apiKeyHelper` of its own.
fn with_kind(case: &Case, secrets: &Path, kind: Option<&str>) {
    with_kind_at(case, secrets, kind, "none");
}

/// [`with_kind`] at the placement `placement`.
fn with_kind_at(case: &Case, secrets: &Path, kind: Option<&str>, placement: &str) {
    let settings = case.dir.join("director-settings.json");
    fs::write(
        &settings,
        serde_json::json!({"apiKeyHelper": "printf helper-of-the-role", "env": {"KEPT": "1"}})
            .to_string(),
    )
    .expect("write the role's settings");
    with_model_token(case, secrets, ", unattended: true", placement);
    let mut text = fs::read_to_string(case.file()).expect("the config file");
    text = text.replace(
        "{role: director, harness: claude, model: opus}",
        &format!(
            "{{role: director, harness: claude, model: opus, settings: {:?}}}",
            settings.display().to_string()
        ),
    );
    if let Some(kind) = kind {
        text = text.replace(
            "\x20   model_token: secret:alpha-model\n",
            &format!("\x20   model_token: secret:alpha-model\n\x20   model_token_kind: {kind}\n"),
        );
    }
    fs::write(case.file(), text).expect("write the config file");
}

/// The model token's secret `alpha-model` in `secrets` holds `value`.
fn mount(secrets: &Path, value: &str) {
    fs::create_dir_all(secrets).expect("the secrets directory");
    fs::write(secrets.join("alpha-model"), format!("{value}\n")).expect("mount the token");
}

/// A `model_token` of kind `subscription_token` reaches the harness as `CLAUDE_CODE_OAUTH_TOKEN`
/// in its process's environment, and the session's settings copy names no `apiKeyHelper`, which
/// would outrank the variable: the one the role's own settings named is taken out of the copy,
/// and one line of the start says so without the helper's text. The value is in no output of the
/// start, no argument of the harness, not in the settings copy and in no file under the
/// instance's state, records or the daemon's directory: the launch's environment is the only
/// place it is handed over, and the `none` provider sets it on the harness's process, which no
/// unit and no command line carries.
#[test]
fn a_subscription_token_reaches_the_harness_in_its_variable_and_its_settings_name_no_helper() {
    let case = Case::new("kind-subscription", ROLES);
    let secrets = case.dir.join("secrets");
    mount(&secrets, MARKER);
    with_kind(&case, &secrets, Some("subscription_token"));
    harness_double(&case);

    let started = start_built(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    assert_eq!(
        oauth_of(&case).as_deref(),
        Some(MARKER),
        "{}",
        said(&started)
    );
    let settings = settings_of(&case);
    let text = fs::read_to_string(&settings).expect("the settings copy");
    let json: Value = serde_json::from_str(&text).expect("the settings copy is JSON");
    assert!(json.get(token::HELPER_KEY).is_none(), "{text}");
    assert_eq!(
        json["env"]["KEPT"], "1",
        "the rest of the settings is kept: {text}"
    );
    assert!(
        settings.starts_with(case.dir.join("state")),
        "the copy is the session's: {}",
        settings.display()
    );
    let stderr = String::from_utf8_lossy(&started.stderr).into_owned();
    let told: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains(token::HELPER_KEY))
        .collect();
    assert_eq!(told.len(), 1, "{}", said(&started));
    assert!(
        told[0].contains("alpha-director") && !told[0].contains("helper-of-the-role"),
        "{}",
        told[0]
    );

    for (what, text) in [
        ("the start's output", said(&started)),
        ("the harness's arguments", argv_of(&case)),
        ("the settings copy", text.clone()),
    ] {
        assert!(!text.contains(MARKER), "{what}: {text}");
    }
    case.assert_nowhere(MARKER);
}

/// story:box-subscription-token: at a placement that opens a box, the runtime in the box holds
/// the model token it read from its sealed slot and hands it, for a `subscription_token`, to the
/// `conductor session start` it runs as `CLAUDE_CODE_OAUTH_TOKEN`. That start puts it into the
/// harness's environment and writes a settings copy with no `apiKeyHelper` (the role's own is
/// taken out), and reads no secrets source for it: the source here holds nothing. For an
/// `api_key` the launch is left as it is: no variable reaches the harness and the role's own
/// settings, with their helper, are the ones it starts with. The value is in no output, argument
/// or settings copy.
#[test]
fn in_a_box_a_subscription_token_the_runtime_holds_reaches_the_harness_in_its_variable() {
    for kind in ["subscription_token", "api_key"] {
        let case = Case::new(&format!("box-{}", kind.replace('_', "-")), ROLES);
        let secrets = case.dir.join("secrets");
        fs::create_dir_all(&secrets).expect("the secrets directory");
        with_kind_at(&case, &secrets, Some(kind), "systemd");
        harness_double(&case);

        // The runtime hands the variable on for a `subscription_token` only
        // (`run/handlers.rs`, `token::boxed_subscription_token`).
        let mut command = case.conductor();
        command.env_remove(OAUTH);
        if kind == "subscription_token" {
            command.env(OAUTH, MARKER);
        }
        let started = command
            .args(["session", "start", "--role", "director"])
            .output()
            .expect("run session start");
        assert_eq!(started.status.code(), Some(0), "{kind}: {}", said(&started));
        let settings = settings_of(&case);
        let text = fs::read_to_string(&settings).expect("the settings");
        let json: Value = serde_json::from_str(&text).expect("the settings are JSON");
        if kind == "subscription_token" {
            assert_eq!(
                oauth_of(&case).as_deref(),
                Some(MARKER),
                "{}",
                said(&started)
            );
            assert!(json.get(token::HELPER_KEY).is_none(), "{text}");
            assert!(
                settings.starts_with(case.dir.join("state")),
                "the copy is the session's: {}",
                settings.display()
            );
        } else {
            assert_eq!(oauth_of(&case), None, "{kind}: {}", said(&started));
            assert_eq!(
                json[token::HELPER_KEY],
                "printf helper-of-the-role",
                "{kind}: the role's own settings are left as they are: {text}"
            );
        }
        for (what, text) in [
            ("the start's output", said(&started)),
            ("the harness's arguments", argv_of(&case)),
            ("the settings", text.clone()),
        ] {
            assert!(!text.contains(MARKER), "{kind}: {what}: {text}");
        }
    }
}

/// A `model_token` of kind `api_key`, and one whose instance names no kind, takes the route it
/// took before the kind existed: the settings copy names `conductor token reveal --model-token
/// <instance>` as its `apiKeyHelper`, and the harness's process has no `CLAUDE_CODE_OAUTH_TOKEN`.
#[test]
fn an_api_key_and_a_model_token_of_no_kind_name_the_helper_and_set_no_variable() {
    for kind in [None, Some("api_key")] {
        let case = Case::new(
            &format!("kind-{}", kind.unwrap_or("absent").replace('_', "-")),
            ROLES,
        );
        let secrets = case.dir.join("secrets");
        mount(&secrets, MARKER);
        with_kind(&case, &secrets, kind);
        harness_double(&case);

        let started = start_built(&case, "director");
        assert_eq!(
            started.status.code(),
            Some(0),
            "{kind:?}: {}",
            said(&started)
        );
        assert_eq!(oauth_of(&case), None, "{kind:?}");
        let helper = helper_of(&settings_of(&case));
        assert!(
            helper.ends_with("token reveal --model-token 'alpha'"),
            "{kind:?}: {helper}"
        );
        assert!(
            !said(&started).contains(MARKER),
            "{kind:?}: {}",
            said(&started)
        );
        assert!(!argv_of(&case).contains(MARKER), "{kind:?}");
        case.assert_nowhere(MARKER);
    }
}

/// What a changed value reaches without a restart of the runtime: a subscription token is read
/// from the secrets source once at each session's start, so a session started after the secret
/// changed runs on the new value and one started before keeps the old; an API key is read by
/// `token reveal --model-token` at each ask of the harness, so the next ask answers the new value.
#[test]
fn a_changed_model_token_reaches_the_next_session_start_or_the_next_ask() {
    let case = Case::new("kind-changed", ROLES);
    let secrets = case.dir.join("secrets");
    mount(&secrets, MARKER);
    with_kind(&case, &secrets, Some("subscription_token"));
    harness_double(&case);

    let first = start_built(&case, "director");
    assert_eq!(first.status.code(), Some(0), "{}", said(&first));
    assert_eq!(oauth_of(&case).as_deref(), Some(MARKER));
    mount(&secrets, CHANGED);
    let second = start_built(&case, "director");
    assert_eq!(second.status.code(), Some(0), "{}", said(&second));
    assert_eq!(oauth_of(&case).as_deref(), Some(CHANGED));

    let reveal = || {
        case.conductor()
            .args(["token", "reveal", "--model-token", "alpha"])
            .output()
            .expect("run token reveal")
    };
    mount(&secrets, MARKER);
    let before = reveal();
    assert_eq!(before.status.code(), Some(0), "{}", said(&before));
    assert_eq!(
        String::from_utf8_lossy(&before.stdout),
        format!("{MARKER}\n")
    );
    mount(&secrets, CHANGED);
    let after = reveal();
    assert_eq!(after.status.code(), Some(0), "{}", said(&after));
    assert_eq!(
        String::from_utf8_lossy(&after.stdout),
        format!("{CHANGED}\n")
    );
}

// -------------------------------------------------------------------------------------------
// A pool token's kind, and the `files` source (story:pool-token-kind-files-source).
// -------------------------------------------------------------------------------------------

/// `case`'s config file on the `files` source over `secrets`.
fn with_files_source(case: &Case, secrets: &Path) {
    fs::create_dir_all(secrets).expect("the secrets directory");
    let text = fs::read_to_string(case.file())
        .expect("the config file")
        .replace(
            "secrets: {source: keychain}",
            &format!("secrets: {{source: files, dir: {secrets:?}}}"),
        );
    fs::write(case.file(), text).expect("write the config file");
}

/// The files under `secrets` the host pool's tokens are kept in.
fn token_files(secrets: &Path) -> Vec<PathBuf> {
    fs::read_dir(secrets.join("claude-token"))
        .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
        .unwrap_or_default()
}

/// Under the `files` source `conductor join` writes the token's file into `daemon.secrets.dir`,
/// owner-only, under the name conductor derives; the token's kind is `subscription_token` by
/// default and the pool lists it; a start leases the token and hands it to the harness as
/// `CLAUDE_CODE_OAUTH_TOKEN`, its settings copy naming no `apiKeyHelper`. The value is in no
/// output, argument, settings copy or file of the instance's state, records or the daemon's
/// directory.
#[test]
fn a_files_source_subscription_pool_token_reaches_the_harness_in_its_variable() {
    let mut case = Case::new("files-subscription", ROLES);
    let secrets = case.dir.join("secrets");
    with_files_source(&case, &secrets);
    harness_double(&case);
    case.serve();

    let joined = case.join("operator@example-org.example", MARKER);
    assert_eq!(
        joined.status.code(),
        Some(0),
        "{}\n{}",
        said(&joined),
        case.daemon_log()
    );
    let files = token_files(&secrets);
    assert_eq!(files.len(), 1, "{files:?}");
    let mode = fs::metadata(&files[0])
        .expect("the token's file")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "{:o}", mode);
    assert_eq!(
        fs::read_to_string(&files[0])
            .expect("the token's file")
            .trim_end(),
        MARKER
    );
    let rows = case.tokens();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0]["kind"], "SubscriptionToken", "{rows:?}");

    let started = start_built(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    let rows = case.tokens();
    assert_eq!(rows[0]["leases"], 1, "{rows:?}");
    assert_eq!(
        oauth_of(&case).as_deref(),
        Some(MARKER),
        "{}",
        said(&started)
    );
    let settings = settings_of(&case);
    let text = fs::read_to_string(&settings).expect("the settings copy");
    let json: Value = serde_json::from_str(&text).expect("the settings copy is JSON");
    assert!(json.get(token::HELPER_KEY).is_none(), "{text}");
    for (what, text) in [
        ("the join's output", said(&joined)),
        ("the start's output", said(&started)),
        ("the harness's arguments", argv_of(&case)),
        ("the settings copy", text.clone()),
    ] {
        assert!(!text.contains(MARKER), "{what}: {text}");
    }
    case.assert_nowhere(MARKER);
}

/// Under the `files` source `conductor leave` withdraws the token and removes its file; an API
/// key joined with `--kind ApiKey` is listed as one.
#[test]
fn leave_with_the_files_source_removes_the_token_file() {
    let mut case = Case::new("files-leave", ROLES);
    let secrets = case.dir.join("secrets");
    with_files_source(&case, &secrets);
    case.serve();

    let joined = case.join_with("operator@example-org.example", VALUE, &["--kind", "ApiKey"]);
    assert_eq!(
        joined.status.code(),
        Some(0),
        "{}\n{}",
        said(&joined),
        case.daemon_log()
    );
    assert_eq!(token_files(&secrets).len(), 1);
    assert_eq!(case.tokens()[0]["kind"], "ApiKey");

    let left = case
        .conductor()
        .args(["leave", "--daemon", &case.url(), "--instance", "alpha"])
        .output()
        .expect("run conductor leave");
    assert_eq!(
        left.status.code(),
        Some(0),
        "{}\n{}",
        said(&left),
        case.daemon_log()
    );
    assert_eq!(case.tokens()[0]["state"], "Withdrawn");
    assert!(
        token_files(&secrets).is_empty(),
        "{:?}",
        token_files(&secrets)
    );
    case.assert_nowhere(VALUE);
}

#[test]
fn token_owner_is_a_controller_s_key_only() {
    let case = Case::new(
        "token-owner",
        "\x20     - {role: director, harness: claude, model: opus, token_owner: operator}\n\
         \x20     - {role: controller, harness: claude, model: opus, token_owner: operator}\n",
    );
    let output = case
        .conductor()
        .args(["config", "validate"])
        .output()
        .expect("run config validate");
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    let text = said(&output);
    assert!(text.contains("instances[0].roles[0].token_owner"), "{text}");
    assert!(
        !text.contains("instances[0].roles[1].token_owner"),
        "{text}"
    );
}

// -------------------------------------------------------------------------------------------
// The host's pool (story:host-token-pool).
// -------------------------------------------------------------------------------------------

/// Two instances, `alpha` and `beta`, with a director each and no `model_token`.
const TWO: &str = "version: conductor.config/2\n\
     daemon: {port: PORT, store: \"sqlite:STORE\", secrets: {source: keychain}}\n\
     instances:\n";

impl Case {
    /// [`Case::new`] with a second instance `beta` beside `alpha`, its own directories, a
    /// director, and neither naming a `model_token`.
    fn two(name: &str) -> Self {
        let case = Self::new(name, ROLES);
        let mut text = TWO
            .replace("PORT", &case.port.to_string())
            .replace("STORE", &case.store_file().display().to_string());
        for (instance, sub) in [("alpha", ""), ("beta", "beta-")] {
            text.push_str(&format!(
                "\x20 - name: {instance}\n\
                 \x20   placement: box-a\n\
                 \x20   session_prefix: {instance}\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n\
                 \x20   cache: {cache}\n\
                 \x20   roles:\n{ROLES}",
                root = case.dir.join(format!("home/{sub}example-org")).display(),
                trees = case.dir.join(format!("{sub}trees")).display(),
                records = case.dir.join(format!("{sub}records")).display(),
                state = case.dir.join(format!("{sub}state")).display(),
                cache = case.dir.join(format!("{sub}cache")).display(),
            ));
        }
        for sub in ["beta-records", "beta-state"] {
            fs::create_dir_all(case.dir.join(sub)).expect("create a case directory");
        }
        fs::write(case.file(), text).expect("write the config file");
        case
    }
}

/// `conductor join` with no `--instance`: the token joins the host's pool, the answer names the
/// instances it serves, and no operator record is written; an agent of each of two instances
/// whose config names no `model_token` starts on it, one lease each, and the config file is byte
/// for byte what it was. Fails on a pool of one instance: `join` asked for `--instance`, a token
/// named its instance, and a Leased token was never leased again.
#[test]
fn a_token_joined_to_the_host_is_leased_by_two_instances_whose_config_names_none() {
    let mut case = Case::two("host-two-instances");
    harness_double(&case);
    let before = fs::read(case.file()).expect("the config file");
    case.serve();

    let mut child = case
        .conductor()
        .args([
            "join",
            "--daemon",
            &case.url(),
            "--seat",
            "operator@example-org.example",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run conductor join");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(format!("{MARKER}\n").as_bytes())
        .expect("write the token");
    let joined = child.wait_with_output().expect("wait for join");
    assert_eq!(
        joined.status.code(),
        Some(0),
        "{}\n{}",
        said(&joined),
        case.daemon_log()
    );
    let shown = String::from_utf8_lossy(&joined.stdout).into_owned();
    assert!(
        shown.contains("alpha") && shown.contains("beta"),
        "join names the instances it serves: {shown}"
    );
    assert!(
        OperatorStorage::list(&case.store()).is_empty(),
        "a join with no instance records no operator"
    );
    let (status, rows) = case.get("/v1/tokens");
    assert_eq!(status, 200, "{rows}");
    assert_eq!(rows.as_array().map(Vec::len), Some(1), "{rows}");

    for instance in ["alpha", "beta"] {
        let started = case
            .conductor()
            .env_remove(OAUTH)
            .env_remove(API_KEY)
            .env(config::INSTANCE_VARIABLE, instance)
            .args(["session", "start", "--role", "director"])
            .output()
            .expect("run session start");
        assert_eq!(
            started.status.code(),
            Some(0),
            "{instance}: {}",
            said(&started)
        );
        assert_eq!(oauth_of(&case).as_deref(), Some(MARKER), "{instance}");
        assert!(!said(&started).contains(MARKER), "{}", said(&started));
    }
    let held: Vec<Value> = leases(&case)
        .into_iter()
        .filter(|row| row["state"] == "Held")
        .collect();
    assert_eq!(held.len(), 2, "{held:?}");
    assert_eq!(held[0]["token_id"], held[1]["token_id"], "{held:?}");
    let mut instances: Vec<&str> = held
        .iter()
        .filter_map(|row| row["instance"].as_str())
        .collect();
    instances.sort_unstable();
    assert_eq!(instances, ["alpha", "beta"]);
    assert_eq!(
        fs::read(case.file()).expect("the config file"),
        before,
        "the config file changed"
    );
    case.assert_nowhere(MARKER);
}

/// The pool's library on its own store: `name`'s directory, emptied; answers the store's URL.
fn library_url(name: &str) -> String {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("token-pool-library")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the case's directory");
    format!("sqlite:{}", dir.join("shared.sqlite").display())
}

fn open(url: &str) -> conductor_cli::store::Store {
    conductor_cli::store::open_url(url, conductor_cli::daemon::TENANT).expect("open the store")
}

fn snapshot(id: &str, owner: &str, kind: Option<ModelTokenKind>) -> PoolTokenSnapshot {
    PoolTokenSnapshot {
        state: PoolTokenState::Available,
        data: PoolTokenData {
            token_id: TokenId(Uuid(id.to_owned())),
            owner: OperatorName(owner.to_owned()),
            seat: Seat(format!("{owner}@example-org.example")),
            secret: SecretName(format!("claude-token/{id}")),
            token_state: TokenState::Ok,
            limited_until: None,
            contributed_at: Timestamp("2026-11-08T09:00:00Z".to_owned()),
            actor: conductor_cli::actor::operator(owner),
            kind,
        },
    }
}

/// A store at `url` holding `tokens`.
fn with_tokens(url: &str, tokens: &[(&str, &str, Option<ModelTokenKind>)]) {
    let mut store = open(url);
    for (id, owner, kind) in tokens {
        PoolTokenStorage::put(&mut store, snapshot(id, owner, *kind));
    }
    store.check().expect("the tokens are stored");
}

fn runtime() -> Actor {
    Actor {
        kind: ActorKind::Daemon,
        operator: None,
        agent: None,
    }
}

fn alpha() -> InstanceName {
    InstanceName("alpha".to_owned())
}

/// The start of `agent` of `instance`, for no owner.
fn start_of(instance: &str, agent: &str) -> token::Start {
    token::Start {
        instance: InstanceName(instance.to_owned()),
        agent: AgentName(agent.to_owned()),
        owner: None,
        actor: runtime(),
        runtime: None,
    }
}

fn lease(url: &str, start: &token::Start) -> token::Lease {
    token::lease(url, token::Caps::default(), start)
        .expect("lease")
        .unwrap_or_else(|error| panic!("no token for {}: {error:?}", start.agent.0))
}

/// The held leases of the store at `url`, by token id.
fn held_of(url: &str, token_id: &str) -> Vec<conductor_model::token::TokenLeaseSnapshot> {
    TokenLeaseStorage::list(&open(url))
        .into_iter()
        .filter(|lease| lease.state == TokenLeaseState::Held && lease.data.token_id.0.0 == token_id)
        .collect()
}

/// The class C decision of story:host-token-pool: one token serves several sessions at once, a
/// lease each. Fails on a pool whose lease was the token's own state, where a second start found
/// the token Leased and was refused NoTokenAvailable.
#[test]
fn one_token_serves_several_sessions_at_once() {
    let url = library_url("several");
    with_tokens(
        &url,
        &[("0001", "alice", Some(ModelTokenKind::SubscriptionToken))],
    );
    for agent in ["alpha-controller", "alpha-implementer", "alpha-director"] {
        let leased = lease(&url, &start_of("alpha", agent));
        assert_eq!(leased.token_id.0.0, "0001");
    }
    let held = held_of(&url, "0001");
    assert_eq!(held.len(), 3, "{held:?}");
    assert_eq!(
        PoolTokenStorage::get(&open(&url), &TokenId(Uuid("0001".to_owned())))
            .expect("the token")
            .state,
        PoolTokenState::Available
    );
}

/// Decision (operator, 2026-11-18): a `subscription_token` serves at most its cap of sessions, 4
/// by default; the fifth start on a pool of one such token is refused NoTokenAvailable, as with an
/// empty pool. An `api_key` has no cap by default and takes a fifth.
#[test]
fn a_subscription_token_serves_at_most_its_cap_of_sessions() {
    let url = library_url("cap-subscription");
    with_tokens(
        &url,
        &[("0001", "alice", Some(ModelTokenKind::SubscriptionToken))],
    );
    for n in 1..=4 {
        lease(&url, &start_of("alpha", &format!("alpha-agent-{n}")));
    }
    let fifth = token::lease(
        &url,
        token::Caps::default(),
        &start_of("alpha", "alpha-agent-5"),
    )
    .expect("lease");
    assert!(
        fifth.is_err(),
        "a fifth session ran on a subscription token"
    );
    assert_eq!(held_of(&url, "0001").len(), 4);

    let url = library_url("cap-api-key");
    with_tokens(&url, &[("0002", "alice", Some(ModelTokenKind::ApiKey))]);
    for n in 1..=5 {
        lease(&url, &start_of("alpha", &format!("alpha-agent-{n}")));
    }
    assert_eq!(held_of(&url, "0002").len(), 5);
}

/// Releasing one of two leases of a token leaves the token in use by the other: its state does
/// not move, the other lease is held, and the pick counts one live lease, not a lifetime's.
#[test]
fn releasing_one_of_two_leases_leaves_the_token_in_use() {
    let url = library_url("release-one");
    with_tokens(
        &url,
        &[
            ("0001", "alice", Some(ModelTokenKind::ApiKey)),
            ("0002", "bob", Some(ModelTokenKind::ApiKey)),
        ],
    );
    let first = lease(&url, &start_of("alpha", "alpha-one"));
    assert_eq!(first.token_id.0.0, "0001", "ties break by id");
    let second = lease(&url, &start_of("alpha", "alpha-two"));
    assert_eq!(
        second.token_id.0.0, "0002",
        "the token with fewer live leases"
    );
    let third = lease(&url, &start_of("alpha", "alpha-three"));
    assert_eq!(third.token_id.0.0, "0001");
    token::release(
        &url,
        &alpha(),
        &AgentName("alpha-one".to_owned()),
        runtime(),
    )
    .expect("release")
    .expect("alpha-one held a lease");
    let held = held_of(&url, "0001");
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0].data.agent.0, "alpha-three");
    assert_eq!(
        PoolTokenStorage::get(&open(&url), &TokenId(Uuid("0001".to_owned())))
            .expect("the token")
            .state,
        PoolTokenState::Available
    );
    let rows = token::rows(&open(&url));
    assert_eq!(rows[0].leases, 1, "{rows:?}");
    assert_eq!(rows[1].leases, 1, "{rows:?}");
    // One live lease each: the next goes to the lower id again, though 0001 was leased twice.
    let fourth = lease(&url, &start_of("alpha", "alpha-four"));
    assert_eq!(fourth.token_id.0.0, "0001");
}

/// A lease racing a leave: whichever wrote last sees the other, so no lease stays held on a
/// withdrawn token.
#[test]
fn a_lease_racing_a_leave_never_runs_on_a_withdrawn_token() {
    for round in 0..5 {
        let url = library_url(&format!("race-{round}"));
        with_tokens(&url, &[("0001", "alice", Some(ModelTokenKind::ApiKey))]);
        let leasing = {
            let url = url.clone();
            thread::spawn(move || {
                let mut answered = Vec::new();
                for n in 0..12 {
                    let start = start_of("alpha", &format!("alpha-agent-{n}"));
                    if let Ok(Ok(lease)) = token::lease(&url, token::Caps::default(), &start) {
                        answered.push(lease.lease_id);
                    }
                }
                answered
            })
        };
        thread::sleep(Duration::from_millis(5 * round));
        let id = TokenId(Uuid("0001".to_owned()));
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match token::withdraw(&url, &id, conductor_cli::actor::operator("alice")) {
                Ok(_) => break,
                Err(error) => {
                    assert!(
                        Instant::now() < deadline,
                        "the withdrawal never wrote: {error:#}"
                    );
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }
        leasing.join().expect("the leasing thread");
        let store = open(&url);
        assert_eq!(
            PoolTokenStorage::get(&store, &id).expect("the token").state,
            PoolTokenState::Withdrawn
        );
        let held = held_of(&url, "0001");
        assert!(
            held.is_empty(),
            "round {round}: held on a withdrawn token: {held:?}"
        );
    }
}

/// Each lease names the runtime start that took it; once the instance runs on another start,
/// the leases of the one that ended are released. A lease of another instance, and one that names
/// no start, are kept.
#[test]
fn leases_of_a_runtime_that_ended_are_reaped() {
    let url = library_url("reap");
    with_tokens(&url, &[("0001", "alice", Some(ModelTokenKind::ApiKey))]);
    let ended = token::runtime_tag("a-start-that-ended");
    let current = token::runtime_tag("the-start-now");
    for (instance, agent, tag) in [
        ("alpha", "alpha-old", Some(ended.clone())),
        ("alpha", "alpha-new", Some(current.clone())),
        ("alpha", "alpha-by-hand", None),
        ("beta", "beta-old", Some(ended.clone())),
    ] {
        let mut start = start_of(instance, agent);
        start.runtime = tag;
        lease(&url, &start);
    }
    let reaped = token::reap(&url, &alpha(), Some(&current)).expect("reap");
    assert_eq!(reaped, [AgentName("alpha-old".to_owned())]);
    let mut kept: Vec<String> = held_of(&url, "0001")
        .into_iter()
        .map(|lease| lease.data.agent.0)
        .collect();
    kept.sort();
    assert_eq!(kept, ["alpha-by-hand", "alpha-new", "beta-old"]);
}

/// A usage limit's end is clamped to a week and an hour from now, and a limit is taken only for
/// the token the agent of that instance holds: an agent that holds no lease limits nothing.
#[test]
fn a_limit_further_than_a_week_ahead_is_clamped() {
    let url = library_url("clamp");
    with_tokens(&url, &[("0001", "alice", Some(ModelTokenKind::ApiKey))]);
    let agent = AgentName("alpha-controller".to_owned());
    lease(&url, &start_of("alpha", &agent.0));
    assert!(
        token::limit(
            &url,
            &InstanceName("beta".to_owned()),
            &agent,
            runtime(),
            None
        )
        .expect("limit")
        .is_none(),
        "another instance's agent limited the token"
    );
    let far = time::OffsetDateTime::now_utc() + time::Duration::days(30);
    token::limit(&url, &alpha(), &agent, runtime(), Some(far))
        .expect("limit")
        .expect("the agent held a lease");
    let found =
        PoolTokenStorage::get(&open(&url), &TokenId(Uuid("0001".to_owned()))).expect("the token");
    assert_eq!(found.state, PoolTokenState::Limited);
    let until = time::OffsetDateTime::parse(
        &found.data.limited_until.expect("an end").0,
        &time::format_description::well_known::Rfc3339,
    )
    .expect("an RFC 3339 end");
    assert!(
        until <= time::OffsetDateTime::now_utc() + token::LIMIT_CEILING,
        "the limit holds until {until}"
    );
    assert!(
        held_of(&url, "0001").is_empty(),
        "the limit released the lease"
    );
}

#[test]
fn a_named_owner_with_no_available_token_is_refused_and_takes_no_other() {
    let url = library_url("named-owner");
    with_tokens(&url, &[("0001", "alice", None), ("0002", "bob", None)]);
    let alice = OperatorName("alice".to_owned());
    let mut start = start_of("alpha", "alpha-controller");
    start.owner = Some(alice.clone());
    let first = lease(&url, &start);
    assert_eq!(first.owner, alice);
    token::limit(&url, &alpha(), &start.agent, runtime(), None)
        .expect("limit")
        .expect("held");
    start.agent = AgentName("alpha-worker".to_owned());
    let refused = token::lease(&url, token::Caps::default(), &start).expect("lease");
    assert!(refused.is_err(), "a named owner took another owner's token");
}

/// `story:limit-aware-token-reassignment`: a Limited token is leased again once its reset has
/// passed and never before: the lease asks no probe of the account's headroom.
#[test]
fn with_one_token_a_limited_token_is_leased_again_once_its_reset_passed() {
    let url = library_url("pool-of-one");
    with_tokens(&url, &[("0001", "alice", None)]);
    let start = start_of("alpha", "alpha-controller");
    lease(&url, &start);
    token::limit(&url, &alpha(), &start.agent, runtime(), None)
        .expect("limit")
        .expect("the agent held it");
    let without = token::lease(&url, token::Caps::default(), &start).expect("lease");
    assert!(
        without.is_err(),
        "a Limited token was leased before its time"
    );
    let mut store = open(&url);
    let mut limited =
        PoolTokenStorage::get(&store, &TokenId(Uuid("0001".to_owned()))).expect("the token");
    limited.data.limited_until = Some(Timestamp(
        (time::OffsetDateTime::now_utc() - time::Duration::minutes(1))
            .format(&time::format_description::well_known::Rfc3339)
            .expect("the clock"),
    ));
    PoolTokenStorage::put(&mut store, limited);
    store.check().expect("the reset passed");
    let again = lease(&url, &start);
    assert_eq!(again.token_id.0.0, "0001");
    let found = PoolTokenStorage::get(&open(&url), &again.token_id).expect("the token");
    assert_eq!(found.state, PoolTokenState::Available);
    assert_eq!(found.data.token_state, TokenState::Ok);
    assert_eq!(found.data.limited_until, None);
}

#[test]
fn only_the_owner_withdraws_a_token_and_the_agent_on_it_is_named() {
    let url = library_url("withdraw");
    with_tokens(&url, &[("0001", "alice", None)]);
    lease(&url, &start_of("alpha", "alpha-controller"));
    let id = TokenId(Uuid("0001".to_owned()));
    let (outcome, _) =
        token::withdraw(&url, &id, conductor_cli::actor::operator("bob")).expect("withdraw");
    assert!(
        matches!(outcome, WithdrawTokenOutcome::NotOwner { .. }),
        "another operator withdrew alice's token"
    );
    let (outcome, held) =
        token::withdraw(&url, &id, conductor_cli::actor::operator("alice")).expect("withdraw");
    assert!(matches!(outcome, WithdrawTokenOutcome::Withdrawn { .. }));
    assert_eq!(held, [(alpha(), AgentName("alpha-controller".to_owned()))]);
    assert!(
        held_of(&url, "0001").is_empty(),
        "the withdrawal released the lease"
    );
    assert_eq!(
        PoolTokenStorage::get(&open(&url), &id)
            .expect("the token")
            .state,
        PoolTokenState::Withdrawn
    );
}

// -------------------------------------------------------------------------------------------
// A runtime in a box: the pool through conductord's dial-in listener.
// -------------------------------------------------------------------------------------------

/// The token the daemon minted for the start of `alpha`'s runtime, which its dialed connections
/// open with.
const ALPHA_START: &str = "alpha-start-token-5f1c";

/// The token the daemon minted for the start of `beta`'s runtime.
const BETA_START: &str = "beta-start-token-9a2e";

/// The dial a runtime holds is one per process ([`token::hold_dial`]): the tests that hold one
/// take this lock first, so no test's calls reach another's conductord.
static DIALED: Mutex<()> = Mutex::new(());

/// A conductord of `case` for the pool: its API with the case's config file and store, `alpha`
/// and `beta` routed by dial-in on [`ALPHA_START`] and [`BETA_START`], and its dial-in listener;
/// answers the listener's address and what it said.
fn dial_conductord(case: &Case) -> (String, Arc<Mutex<Vec<String>>>) {
    let api = Arc::new(
        Api::new("daemon-token".to_owned(), "operator".to_owned(), None).with_config(
            case.file(),
            case.home(),
            case.store_url(),
        ),
    );
    api.publish(
        vec![
            serde_json::json!({"name": "alpha", "state": "Running"}),
            serde_json::json!({"name": "beta", "state": "Running"}),
        ],
        [
            (
                "alpha".to_owned(),
                Route::dialed("alpha", ALPHA_START.to_owned()),
            ),
            (
                "beta".to_owned(),
                Route::dialed("beta", BETA_START.to_owned()),
            ),
        ]
        .into_iter()
        .collect(),
        Vec::new(),
    );
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind the dial-in listener");
    let address = listener.local_addr().expect("its address").to_string();
    let said = Arc::new(Mutex::new(Vec::new()));
    let into = Arc::clone(&said);
    thread::spawn(move || {
        conductor_cli::daemon::dial::serve_saying(
            &listener,
            &api,
            Arc::new(move |line: &str| into.lock().expect("said").push(line.to_owned())),
        );
    });
    (address, said)
}

/// One pool call on a new connection to `address`, opened as `instance` with `token`; the answer,
/// or `None` where the connection was closed unanswered.
fn pool_call(address: &str, instance: &str, token: &str, request: &Value) -> Option<Value> {
    let mut stream = TcpStream::connect(address).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("a read timeout");
    // A connection the daemon closed on the greeting may refuse the request line itself: that is
    // a call closed unanswered too.
    if write!(stream, "CONDUCTOR-POOL/1 {instance} {token}\n{request}\n").is_err() {
        return None;
    }
    let mut answer = String::new();
    let _ = stream.read_to_string(&mut answer);
    serde_json::from_str(answer.trim()).ok()
}

/// The contribution of a token of `kind` worth `value` to the host's pool of `case`, as `alice`,
/// through the library, on the case's `files` source over `secrets`.
fn contribute(case: &Case, secrets: &Path, value: &str, kind: ModelTokenKind) -> String {
    let source = conductor_model::config::SecretsSource {
        source: conductor_model::secret::Source::Files,
        namespace: None,
        url: None,
        dir: Some(secrets.display().to_string()),
    };
    let opened = conductor_cli::secret::open(&source).expect("the files source");
    let outcome = token::contribute(
        case.store(),
        &opened,
        &token::Contribution {
            actor: conductor_cli::actor::operator("alice"),
            owner: OperatorName("alice".to_owned()),
            seat: Seat("alice@example-org.example".to_owned()),
            kind,
        },
        secrets_core::storage::SecretValue::new(value.as_bytes().to_vec()).expect("a value"),
    )
    .expect("contribute")
    .expect("the source took the value");
    match outcome {
        conductor_model::token::ContributeTokenOutcome::Contributed { token_contributed } => {
            token_contributed.token_id.0.0
        }
        other => panic!("not contributed: {other:?}"),
    }
}

/// The launch of `case`'s director of `instance`, its settings in the case's state.
fn launch_of(case: &Case, name: &str) -> Launch {
    let limits = || Limits {
        memory_max: None,
        cpu_weight: None,
        tasks_max: None,
    };
    Launch {
        request: SessionRequest {
            role: "controller".to_owned(),
            session_name: SessionName(name.to_owned()),
            cwd: case.dir.join("work").display().to_string(),
            limits: limits(),
            writable: Vec::new(),
            network: Network::Open,
            forge_token_env: Vec::new(),
        },
        program: "claude".to_owned(),
        args: Vec::new(),
        env: Vec::new(),
        session_prefix: None,
        instance_limits: limits(),
        state: case.dir.join("state"),
    }
}

/// The instance `name` of `case`'s config file, and its controller role.
fn instance_of(
    case: &Case,
    name: &str,
) -> (
    conductor_model::config::Instance,
    conductor_model::config::Role,
) {
    let environment = config::Environment::from_process().expect("the environment");
    let loaded = config::load(Some(&case.file()), &environment).expect("the config loads");
    let instance = loaded
        .config
        .instances
        .into_iter()
        .find(|instance| instance.name.0 == name)
        .expect("the instance");
    let role = instance
        .roles
        .iter()
        .find(|role| role.role == "controller")
        .cloned()
        .expect("a controller role");
    (instance, role)
}

/// A runtime in a box has no daemon database: its config names a store that does not exist where
/// it runs. It holds the address it dials and the token minted for its start, and a token joined
/// after it started reaches its agent's harness through conductord, in its variable, with no
/// restart; its release and its limit go the same way. The daemon's journal and the store carry
/// no value. Fails before the host pool: such a runtime found no store and started every agent on
/// the model token sealed into its box when it opened.
#[test]
fn a_boxed_runtime_leases_a_token_joined_after_its_box_opened() {
    let case = Case::two("boxed-runtime");
    let secrets = case.dir.join("secrets");
    with_files_source(&case, &secrets);
    let _dialed = DIALED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (address, said) = dial_conductord(&case);
    // The runtime: it dials conductord and has no store of its own (`runtime_store` finds none,
    // as the process's config names none that exists).
    token::hold_dial(&address, "alpha", ALPHA_START);

    // Joined after the runtime started.
    let token_id = contribute(&case, &secrets, MARKER, ModelTokenKind::SubscriptionToken);
    let (instance, role) = instance_of(&case, "alpha");
    let mut launch = launch_of(&case, "alpha-controller");
    let leased = token::lease_into(&instance, &role, &mut launch)
        .expect("a lease through conductord")
        .expect("the pool holds a token");
    assert_eq!(leased.token_id.0.0, token_id);
    assert!(
        launch
            .env
            .iter()
            .any(|(key, value)| key == OAUTH && value == MARKER),
        "the harness's environment holds no {OAUTH}"
    );
    assert!(!launch.args.iter().any(|arg| arg.contains(MARKER)));
    let held = held_of(&case.store_url(), &token_id);
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0].data.instance.0, "alpha");
    assert_eq!(
        held[0].data.runtime.as_deref(),
        Some(token::runtime_tag(ALPHA_START).as_str()),
        "the lease names the runtime start that took it"
    );

    token::release_for_stop(&instance, "alpha-controller").expect("release through conductord");
    assert!(held_of(&case.store_url(), &token_id).is_empty());

    let mut launch = launch_of(&case, "alpha-controller");
    token::lease_into(&instance, &role, &mut launch)
        .expect("a lease through conductord")
        .expect("the pool holds a token");
    let answer = token::on_limit(&instance, "alpha-controller", None).expect("limit");
    assert!(!answer.other, "the pool holds no other token");
    assert_eq!(
        answer.limited.as_ref().map(|(token, _)| token.as_str()),
        Some(token_id.as_str())
    );
    assert_eq!(
        PoolTokenStorage::get(&case.store(), &TokenId(Uuid(token_id.clone())))
            .expect("the token")
            .state,
        PoolTokenState::Limited
    );

    let journal = said.lock().expect("said").join("\n");
    assert!(!journal.contains(MARKER), "{journal}");
    case.assert_nowhere(MARKER);
}

/// `story:limit-aware-token-reassignment`: a runtime in a box marks the token its agent holds
/// Limited through conductord, until the reset the limit's line named, and conductord answers
/// the token, its reset, whether another token has headroom, and else the first reset of the
/// pool. Fails before the story: conductord answered `limited` and `other` alone, and the
/// runtime could neither name the token nor wait for the first reset.
#[test]
fn a_boxed_runtime_marks_its_token_limited_through_conductord() {
    let case = Case::two("boxed-limit");
    let secrets = case.dir.join("secrets");
    with_files_source(&case, &secrets);
    let _dialed = DIALED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (address, said) = dial_conductord(&case);
    token::hold_dial(&address, "alpha", ALPHA_START);
    let first = contribute(&case, &secrets, MARKER, ModelTokenKind::SubscriptionToken);
    let second = contribute(&case, &secrets, SECOND, ModelTokenKind::SubscriptionToken);
    let (instance, role) = instance_of(&case, "alpha");
    let whole = |at: time::OffsetDateTime| at.replace_nanosecond(0).expect("a whole second");
    let now = time::OffsetDateTime::now_utc();

    let mut launch = launch_of(&case, "alpha-controller");
    let leased = token::lease_into(&instance, &role, &mut launch)
        .expect("a lease through conductord")
        .expect("the pool holds a token");
    let held = leased.token_id.0.0.clone();
    let other = if held == first { &second } else { &first };
    let late = whole(now + time::Duration::hours(5));
    let answer = token::on_limit(&instance, "alpha-controller", Some(late)).expect("limit");
    assert_eq!(answer.limited, Some((held.clone(), late)), "{answer:?}");
    assert!(answer.other, "the second token has headroom: {answer:?}");
    let found =
        PoolTokenStorage::get(&case.store(), &TokenId(Uuid(held.clone()))).expect("the token");
    assert_eq!(found.state, PoolTokenState::Limited);
    let until = found.data.limited_until.expect("an end").0;
    assert_eq!(
        time::OffsetDateTime::parse(&until, &time::format_description::well_known::Rfc3339)
            .expect("RFC 3339"),
        late
    );

    // Resumed on the other token; its limit resets first, so the session waits for that one.
    let mut launch = launch_of(&case, "alpha-controller");
    let moved = token::lease_into(&instance, &role, &mut launch)
        .expect("a lease through conductord")
        .expect("the pool holds a token");
    assert_eq!(
        &moved.token_id.0.0, other,
        "moved to the token with headroom"
    );
    let soon = whole(now + time::Duration::hours(1));
    let answer = token::on_limit(&instance, "alpha-controller", Some(soon)).expect("limit");
    assert_eq!(answer.limited, Some((other.clone(), soon)), "{answer:?}");
    assert!(!answer.other, "{answer:?}");
    assert_eq!(answer.first_reset, Some(soon), "the first token to reset");

    let journal = said.lock().expect("said").join("\n");
    assert!(
        !journal.contains(MARKER) && !journal.contains(SECOND),
        "{journal}"
    );
    case.assert_nowhere(MARKER);
}

/// A runtime acts on its own instance's leases only: conductord takes the instance from the
/// connection's admission, never from the request, so `alpha`'s runtime neither releases nor
/// limits `beta`'s agent, and a lease it asks for names `alpha`, whatever its request says.
#[test]
fn a_runtime_cannot_lease_release_or_limit_for_another_instance_s_agent() {
    let case = Case::two("other-instance");
    let secrets = case.dir.join("secrets");
    with_files_source(&case, &secrets);
    let (address, _) = dial_conductord(&case);
    let token_id = contribute(&case, &secrets, MARKER, ModelTokenKind::SubscriptionToken);
    let beta_lease = pool_call(
        &address,
        "beta",
        BETA_START,
        &serde_json::json!({"call": "lease", "agent": "beta-director"}),
    )
    .expect("an answer");
    assert!(beta_lease["lease"].is_object(), "{beta_lease}");

    for call in ["release", "limit"] {
        let answer = pool_call(
            &address,
            "alpha",
            ALPHA_START,
            &serde_json::json!({"call": call, "agent": "beta-director", "instance": "beta"}),
        )
        .expect("an answer");
        assert_ne!(answer["released"], true, "{call}: {answer}");
        assert_ne!(answer["limited"], true, "{call}: {answer}");
    }
    let held = held_of(&case.store_url(), &token_id);
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0].data.instance.0, "beta");
    assert_eq!(
        PoolTokenStorage::get(&case.store(), &TokenId(Uuid(token_id.clone())))
            .expect("the token")
            .state,
        PoolTokenState::Available
    );

    let answer = pool_call(
        &address,
        "alpha",
        ALPHA_START,
        &serde_json::json!({"call": "lease", "agent": "beta-director", "instance": "beta"}),
    )
    .expect("an answer");
    assert!(answer["lease"].is_object(), "{answer}");
    let instances: Vec<String> = held_of(&case.store_url(), &token_id)
        .into_iter()
        .map(|lease| lease.data.instance.0)
        .collect();
    assert!(instances.contains(&"alpha".to_owned()), "{instances:?}");
    assert_eq!(instances.len(), 2, "{instances:?}");
}

/// A session's process holds no credential of its runtime: a pool call that does not carry the
/// token the daemon minted for the runtime's start is closed unanswered, so it leases nothing,
/// and no command reveals a token of the host's pool (`token reveal` takes no `--token`).
#[test]
fn a_session_process_cannot_lease_or_reveal_a_token_it_does_not_hold() {
    let case = Case::two("session-process");
    let secrets = case.dir.join("secrets");
    with_files_source(&case, &secrets);
    let (address, _) = dial_conductord(&case);
    let token_id = contribute(&case, &secrets, MARKER, ModelTokenKind::ApiKey);
    for (instance, token) in [
        ("alpha", "a-guess"),
        ("alpha", BETA_START),
        ("gamma", ALPHA_START),
    ] {
        let answer = pool_call(
            &address,
            instance,
            token,
            &serde_json::json!({"call": "lease", "agent": "alpha-director"}),
        );
        assert!(answer.is_none(), "{instance} with {token}: {answer:?}");
    }
    assert!(held_of(&case.store_url(), &token_id).is_empty());

    let revealed = case
        .conductor()
        .args(["token", "reveal", "--token", &token_id])
        .output()
        .expect("run token reveal");
    assert_ne!(revealed.status.code(), Some(0), "{}", said(&revealed));
    assert!(!said(&revealed).contains(MARKER), "{}", said(&revealed));
}

/// No value of the host's pool reaches a log or a record: after a join, a lease, a limit and a
/// release, the daemon's output, the start's output (the runtime's), the store and every file of
/// the instance's state and records carry none.
#[test]
fn no_value_of_the_host_pool_reaches_a_log_or_a_record() {
    let mut case = Case::new("no-value", ROLES);
    harness_double(&case);
    case.serve();
    let joined = case.join("operator@example-org.example", MARKER);
    assert_eq!(
        joined.status.code(),
        Some(0),
        "{}\n{}",
        said(&joined),
        case.daemon_log()
    );
    let started = start_built(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    assert_eq!(oauth_of(&case).as_deref(), Some(MARKER));
    let agent = AgentName("alpha-director".to_owned());
    token::release(&case.store_url(), &alpha(), &agent, runtime())
        .expect("release")
        .expect("the agent held a lease");
    let resumed = start_built(&case, "director");
    assert_eq!(resumed.status.code(), Some(0), "{}", said(&resumed));
    token::limit(&case.store_url(), &alpha(), &agent, runtime(), None)
        .expect("limit")
        .expect("the agent held a lease");
    for (what, text) in [
        ("the join's output", said(&joined)),
        ("the start's output", said(&started)),
        ("the second start's output", said(&resumed)),
        ("the harness's arguments", argv_of(&case)),
    ] {
        assert!(!text.contains(MARKER), "{what}: {text}");
    }
    case.assert_nowhere(MARKER);
}
