//! `story:token-pool` (design architecture-next § 5.3, § 17): an engineer joins an instance and
//! contributes their Claude token to its pool; the `claude-code` kind leases from the pool.
//!
//! - `conductor join --daemon <url> --instance <i>` against a running `conductor daemon serve`
//!   records the caller as an operator of the instance with the grants `daemon.operators` gives
//!   them and puts their token, read from standard input, into the pool: `GET
//!   /v1/instances/{name}/tokens` lists its owner, seat and state, never the value nor the
//!   secret's name; `conductor leave` withdraws it.
//! - `conductor session start` leases a token: the session's settings name `conductor token
//!   reveal --token <id>` as their `apiKeyHelper`, which prints the value, and nothing else
//!   carries it; after a usage limit marks the token Limited, the next start runs on another
//!   token. No line of the daemon's output, of the harness's arguments or of any file under the
//!   instance's state and records carries the value.
//! - The pool's rules on the library: the least-leased Available token, a named owner's own
//!   token only (`NoTokenAvailable` otherwise), a Limited token taken again on headroom with a
//!   pool of one, only the owner withdraws.
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
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::config;
use conductor_cli::token;
use conductor_model::agent::AgentName;
use conductor_model::behaviour::{OperatorStorage, PoolTokenStorage};
use conductor_model::config::InstanceName;
use conductor_model::operator::{Actor, ActorKind, OperatorName, OperatorState};
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::secret::SecretName;
use conductor_model::token::{
    PoolTokenData, PoolTokenSnapshot, PoolTokenState, Seat, TokenId, TokenState,
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

    /// `conductor join` with `value` on standard input.
    fn join(&self, seat: &str, value: &str) -> Output {
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
        conductor_cli::store::open_url(
            &format!("sqlite:{}", self.store_file().display()),
            conductor_cli::daemon::TENANT,
        )
        .expect("open the shared store")
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

#[test]
fn a_start_leases_a_token_and_a_limit_resumes_the_next_start_on_another() {
    let mut case = Case::new("lease-resume", ROLES);
    case.serve();
    for (seat, value) in [
        ("first@example-org.example", VALUE),
        ("second@example-org.example", SECOND),
    ] {
        let joined = case.join(seat, value);
        assert_eq!(
            joined.status.code(),
            Some(0),
            "{}\n{}",
            said(&joined),
            case.daemon_log()
        );
    }

    let started = start(&case, "director");
    assert_eq!(started.status.code(), Some(0), "{}", said(&started));
    let rows = case.tokens();
    let leased: Vec<&Value> = rows.iter().filter(|row| row["state"] == "Leased").collect();
    assert_eq!(leased.len(), 1, "{rows:?}");
    assert_eq!(leased[0]["agent"], "alpha-director", "{rows:?}");
    let first = leased[0]["token_id"].as_str().expect("an id").to_owned();

    let settings = started_settings(&case);
    let helper = helper_of(&settings);
    assert!(
        helper.contains(&format!("token reveal --token '{first}'")),
        "{helper}"
    );
    assert!(
        !helper.contains(VALUE) && !helper.contains(SECOND),
        "{helper}"
    );

    // The harness asks the helper for its credential: the leased token's value, and only that.
    // The helper names the binary that started the session, here this test binary's child run;
    // it is asked of the built binary.
    let (program, rest) = helper
        .strip_prefix('\'')
        .and_then(|rest| rest.split_once("' "))
        .unwrap_or_else(|| panic!("the helper starts with no quoted program: {helper}"));
    assert!(!program.is_empty(), "{helper}");
    let binary = common::conductor(case.home()).get_program().to_owned();
    let asked = format!("'{}' {rest}", binary.to_string_lossy());
    let revealed = Command::new("sh")
        .arg("-c")
        .arg(&asked)
        .env_clear()
        .env("HOME", case.home())
        .env("USER", "operator")
        .env(
            conductor_cli::secret::TEST_KEYCHAIN_VARIABLE,
            case.home().join(".test-keychain.ron"),
        )
        .output()
        .expect("run the helper");
    assert_eq!(revealed.status.code(), Some(0), "{}", said(&revealed));
    let value = String::from_utf8_lossy(&revealed.stdout);
    assert!(
        value.trim() == VALUE || value.trim() == SECOND,
        "{}",
        said(&revealed)
    );

    // A usage limit marks the token Limited; the agent's next start runs on the other token.
    let marked = token::limit(
        case.store(),
        &AgentName("alpha-director".to_owned()),
        conductor_cli::actor::daemon(),
    )
    .expect("mark the token limited");
    assert!(marked.is_some(), "the agent held no token");
    let resumed = start(&case, "director");
    assert_eq!(resumed.status.code(), Some(0), "{}", said(&resumed));
    let rows = case.tokens();
    let limited = rows
        .iter()
        .find(|row| row["token_id"] == first.as_str())
        .expect("the first token");
    assert_eq!(limited["state"], "Limited", "{rows:?}");
    assert_eq!(limited["token_state"], "Limited", "{rows:?}");
    let now_on = rows
        .iter()
        .find(|row| row["state"] == "Leased")
        .unwrap_or_else(|| panic!("no token is leased: {rows:?}"));
    assert_ne!(now_on["token_id"], first.as_str(), "{rows:?}");
    let helper = helper_of(&started_settings(&case));
    assert!(
        helper.contains(now_on["token_id"].as_str().expect("an id")),
        "{helper}"
    );

    // The owner leaves: both tokens are withdrawn, and the agent on one is named for the loop to
    // resume on another token.
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
    assert!(
        String::from_utf8_lossy(&left.stdout).contains("alpha-director"),
        "{}",
        said(&left)
    );
    assert!(case.tokens().iter().all(|row| row["state"] == "Withdrawn"));
    let named = fs::read_to_string(case.dir.join("state/run").join(token::WITHDRAWN_FILE))
        .expect("the agents to resume are named");
    assert!(named.contains("alpha-director"), "{named}");

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

/// `story:container-start-without-prompt`: an instance whose placement opens no box and whose
/// pool leases no token runs its agents on its `model_token`: the settings name `conductor token
/// reveal --model-token <instance>` as their `apiKeyHelper`, which prints the value from the
/// daemon's secrets source, here a directory of mounted files; no argument, setting or output of
/// the start carries the value.
#[test]
fn with_no_token_in_the_pool_an_agent_runs_on_its_instance_s_model_token() {
    let case = Case::new("model-token", ROLES);
    let secrets = case.dir.join("secrets");
    fs::create_dir_all(&secrets).expect("the secrets directory");
    fs::write(secrets.join("alpha-model"), format!("{VALUE}\n")).expect("mount the token");
    let text = fs::read_to_string(case.file())
        .expect("the config file")
        .replace(
            "secrets: {source: keychain}",
            &format!("secrets: {{source: files, dir: {secrets:?}}}"),
        )
        .replace(
            "\x20   session_prefix: alpha\n",
            "\x20   session_prefix: alpha\n\x20   model_token: secret:alpha-model\n",
        );
    fs::write(case.file(), text).expect("write the config file");

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
// The pool's rules on the library.
// -------------------------------------------------------------------------------------------

fn snapshot(id: &str, owner: &str) -> PoolTokenSnapshot {
    PoolTokenSnapshot {
        state: PoolTokenState::Available,
        data: PoolTokenData {
            token_id: TokenId(Uuid(id.to_owned())),
            instance: InstanceName("alpha".to_owned()),
            owner: OperatorName(owner.to_owned()),
            seat: Seat(format!("{owner}@example-org.example")),
            secret: SecretName(format!("claude-token/alpha/{id}")),
            token_state: TokenState::Ok,
            limited_until: None,
            agent: None,
            contributed_at: Timestamp("2026-11-08T09:00:00Z".to_owned()),
            actor: conductor_cli::actor::operator(owner),
        },
    }
}

fn library_store(name: &str) -> conductor_cli::store::Store {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("token-pool-library")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the case's directory");
    conductor_cli::store::open_url(
        &format!("sqlite:{}", dir.join("shared.sqlite").display()),
        conductor_cli::daemon::TENANT,
    )
    .expect("open the store")
}

fn runtime() -> Actor {
    Actor {
        kind: ActorKind::Daemon,
        operator: None,
        agent: None,
    }
}

#[test]
fn a_named_owner_with_no_available_token_is_refused_and_takes_no_other() {
    let mut store = library_store("named-owner");
    PoolTokenStorage::put(&mut store, snapshot("0001", "alice"));
    PoolTokenStorage::put(&mut store, snapshot("0002", "bob"));
    let instance = InstanceName("alpha".to_owned());
    let alice = OperatorName("alice".to_owned());
    let first = token::lease(
        store,
        &instance,
        &AgentName("alpha-controller".to_owned()),
        Some(&alice),
        runtime(),
        false,
    )
    .expect("lease")
    .expect("alice's token");
    assert_eq!(first.owner, alice);
    let store = library_store_reopen("named-owner");
    let refused = token::lease(
        store,
        &instance,
        &AgentName("alpha-worker".to_owned()),
        Some(&alice),
        runtime(),
        false,
    )
    .expect("lease");
    assert!(refused.is_err(), "a named owner took another owner's token");
}

fn library_store_reopen(name: &str) -> conductor_cli::store::Store {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("token-pool-library")
        .join(name);
    conductor_cli::store::open_url(
        &format!("sqlite:{}", dir.join("shared.sqlite").display()),
        conductor_cli::daemon::TENANT,
    )
    .expect("open the store")
}

#[test]
fn with_one_token_a_limited_token_is_leased_again_on_headroom() {
    let mut store = library_store("pool-of-one");
    PoolTokenStorage::put(&mut store, snapshot("0001", "alice"));
    let instance = InstanceName("alpha".to_owned());
    let agent = AgentName("alpha-controller".to_owned());
    token::lease(store, &instance, &agent, None, runtime(), false)
        .expect("lease")
        .expect("the one token");
    token::limit(library_store_reopen("pool-of-one"), &agent, runtime())
        .expect("limit")
        .expect("the agent held it");
    let without = token::lease(
        library_store_reopen("pool-of-one"),
        &instance,
        &agent,
        None,
        runtime(),
        false,
    )
    .expect("lease");
    assert!(
        without.is_err(),
        "a Limited token was leased before its time"
    );
    let again = token::lease(
        library_store_reopen("pool-of-one"),
        &instance,
        &agent,
        None,
        runtime(),
        true,
    )
    .expect("lease")
    .expect("the one token again, on headroom");
    assert_eq!(again.token_id.0.0, "0001");
    let found = PoolTokenStorage::get(&library_store_reopen("pool-of-one"), &again.token_id)
        .expect("the token");
    assert_eq!(found.state, PoolTokenState::Leased);
    assert_eq!(found.data.token_state, TokenState::Ok);
    assert_eq!(found.data.limited_until, None);
}

#[test]
fn the_least_leased_token_has_the_most_headroom() {
    let mut store = library_store("headroom");
    PoolTokenStorage::put(&mut store, snapshot("0001", "alice"));
    PoolTokenStorage::put(&mut store, snapshot("0002", "bob"));
    let instance = InstanceName("alpha".to_owned());
    let agent = AgentName("alpha-controller".to_owned());
    let first = token::lease(store, &instance, &agent, None, runtime(), false)
        .expect("lease")
        .expect("a token");
    assert_eq!(first.token_id.0.0, "0001", "ties break by id");
    token::release(library_store_reopen("headroom"), &agent, runtime()).expect("release");
    let second = token::lease(
        library_store_reopen("headroom"),
        &instance,
        &agent,
        None,
        runtime(),
        false,
    )
    .expect("lease")
    .expect("a token");
    assert_eq!(
        second.token_id.0.0, "0002",
        "the least-leased token goes first"
    );
}

#[test]
fn only_the_owner_withdraws_a_token_and_the_agent_on_it_is_named() {
    let mut store = library_store("withdraw");
    PoolTokenStorage::put(&mut store, snapshot("0001", "alice"));
    let instance = InstanceName("alpha".to_owned());
    let agent = AgentName("alpha-controller".to_owned());
    token::lease(store, &instance, &agent, None, runtime(), false)
        .expect("lease")
        .expect("a token");
    let id = TokenId(Uuid("0001".to_owned()));
    let (outcome, _) = token::withdraw(
        library_store_reopen("withdraw"),
        &id,
        conductor_cli::actor::operator("bob"),
    )
    .expect("withdraw");
    assert!(
        matches!(outcome, WithdrawTokenOutcome::NotOwner { .. }),
        "another operator withdrew alice's token"
    );
    let (outcome, held) = token::withdraw(
        library_store_reopen("withdraw"),
        &id,
        conductor_cli::actor::operator("alice"),
    )
    .expect("withdraw");
    assert!(matches!(outcome, WithdrawTokenOutcome::Withdrawn { .. }));
    assert_eq!(held, Some(agent));
    assert_eq!(
        PoolTokenStorage::get(&library_store_reopen("withdraw"), &id)
            .expect("the token")
            .state,
        PoolTokenState::Withdrawn
    );
}
