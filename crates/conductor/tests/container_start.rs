//! `story:container-start-without-prompt`: one instance starts in a container from a config file
//! and mounted credentials, with no prompt.
//!
//! - `daemon.unattended: true`: `conductor daemon serve`, started with no terminal and its
//!   standard input closed, does at its start what `conductor setup --apply` does for each
//!   instance of the file, each only where it is missing: it writes and commits the records
//!   skeleton, trusts the checkouts and the records directory, and activates the checkouts root
//!   as a `worktree` workspace. A second start overwrites nothing. Without the key the start
//!   creates none of them.
//! - `daemon.secrets.source: files`: a directory of mounted files, one per secret, read and never
//!   written: `conductor secret put` refuses in one line and writes nothing, the daemon's start
//!   and doctor name the directory, and `conductor token reveal --model-token <instance>` prints
//!   the instance's `model_token` from it.
//! - `daemon.address`: the listener on `daemon.port` binds that address; on one that is not
//!   loopback the probes answer and a `/v1/` request without a bearer is refused.
//! - With no user systemd manager (no `$XDG_RUNTIME_DIR/systemd/private`, no `systemd-run`), a
//!   runtime whose placement opens no box runs as a child process of the daemon, ends with it,
//!   and is started again by the reconciler under the failed-start rules; with one, the start is
//!   the user unit it was.
//! - Doctor reads a forge login from the forge CLI's own variable alone (`GH_TOKEN`).
//! - The image built from this tree's binary starts with no terminal (`docker run` without
//!   `-t`), and doctor in it ends `ready` ([`the_image_starts_with_no_terminal_and_doctor_ends_ready`],
//!   ignored by default: `task container-test` runs it).
//!
//! No case reaches the host's service manager: a daemon started here runs on a `PATH` of the
//! case's own, and the variables that name the user's manager are removed.

mod active;
mod common;

use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write as _};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt as _;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use conductor_cli::daemon::reconcile::{Binary, Daemon, unit_name};
use conductor_cli::daemon::units::{self, Children, Units as _};
use conductor_cli::daemon::{api, metrics};
use conductor_cli::systemd::Service;
use conductor_model::config::Limits;
use conductor_model::instance::{ReconcileAction, ReconcileReason, ReconcileResult};
use serde_json::Value;

/// How long a case waits for what a daemon does.
const BOUND: Duration = Duration::from_secs(90);

/// The model token's value a case mounts, which no output but the reveal may carry.
const MODEL_VALUE: &str = "sk-example-org-model-5f2c7e10";

/// A case's directory, short so a socket in it fits a socket address; removed after a passing
/// run, with the binary a daemon copied into it.
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

fn write_program(path: &Path, text: &str) {
    fs::write(path, text).expect("write a stand-in");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make it runnable");
}

fn git_at() -> PathBuf {
    ["/usr/bin/git", "/bin/git", "/usr/local/bin/git"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .expect("git is installed")
}

fn sleep_at() -> PathBuf {
    ["/usr/bin/sleep", "/bin/sleep"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .expect("sleep is installed")
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
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("cs")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        for sub in ["home", "programs", "secrets", "alpha/checkouts"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        Self {
            dir: dir.canonicalize().expect("the case's directory"),
        }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn root(&self) -> PathBuf {
        self.dir.join("alpha/checkouts")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("alpha/records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("alpha/state")
    }

    fn secrets(&self) -> PathBuf {
        self.dir.join("secrets")
    }

    fn worktree_log(&self) -> String {
        fs::read_to_string(self.dir.join("worktree.log")).unwrap_or_default()
    }

    /// The config file: the daemon on `port` with `daemon` added to its section, the `files`
    /// source over [`Case::secrets`], and the one instance `alpha` at `placement`, whose model
    /// token is `secret:alpha-model`.
    fn config(&self, port: u16, daemon: &str, placement: &str) {
        let text = format!(
            "version: conductor.config/2\n\
             daemon: {{port: {port}, store: \"sqlite:{store}\", \
             secrets: {{source: files, dir: {secrets:?}}}{daemon}}}\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: {placement}\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   forge: {{route: direct}}\n\
             \x20   model_token: secret:alpha-model\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n",
            store = self.dir.join("shared.sqlite").display(),
            secrets = self.secrets(),
            root = self.root(),
            trees = self.dir.join("alpha/trees"),
            records = self.records(),
            state = self.state(),
            cache = self.dir.join("alpha/cache"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    /// One git checkout `name` under the checkouts root, its `origin` on the instance's GitHub
    /// owner.
    fn checkout(&self, name: &str) -> PathBuf {
        let dir = self.root().join(name);
        let git = |args: &[&str]| {
            let output = Command::new(git_at())
                .arg("-C")
                .arg(&dir)
                .args(args)
                .env("HOME", self.home())
                .output()
                .expect("run git");
            assert!(output.status.success(), "{}", said(&output));
        };
        fs::create_dir_all(&dir).expect("create the checkout");
        git(&["init", "--quiet"]);
        git(&[
            "remote",
            "add",
            "origin",
            &format!("https://github.com/example-org/{name}.git"),
        ]);
        dir.canonicalize().expect("the checkout's path")
    }

    /// The `PATH` of a daemon of the case: `git`, a `worktree` that logs every call and answers
    /// a root activated once `activate` ran, and, unless `manager` is false, a `systemctl` that
    /// knows no unit and a `systemd-run` that starts none. Where the case's file `inspect-pause`
    /// holds a number of seconds, `worktree inspect` marks `inspect-asked` and takes that long.
    fn programs(&self, manager: bool) -> PathBuf {
        let programs = self.dir.join("programs");
        let _ = std::os::unix::fs::symlink(git_at(), programs.join("git"));
        write_program(
            &programs.join("worktree"),
            &format!(
                "#!/bin/sh\n\
                 printf '%s\\n' \"$*\" >> '{log}'\n\
                 case \"$1\" in\n\
                 \x20 inspect) if [ -f '{pause}' ]; then : > '{asked}'; read -r pause < '{pause}'; \
                 '{sleep}' \"$pause\"; fi; \
                 if [ -f '{mark}' ]; then echo '{{\"ok\":false,\"code\":\"invalid-worktree-id\"}}' >&2; \
                 else echo '{{\"ok\":false,\"code\":\"workspace-not-activated\"}}' >&2; fi; exit 1 ;;\n\
                 \x20 activate) : > '{mark}'; echo '{{\"ok\":true}}' ;;\n\
                 \x20 --version) echo 'worktree 0.18.1' ;;\n\
                 esac\n",
                log = self.dir.join("worktree.log").display(),
                mark = self.dir.join("activated").display(),
                pause = self.dir.join("inspect-pause").display(),
                asked = self.dir.join("inspect-asked").display(),
                sleep = sleep_at().display(),
            ),
        );
        if manager {
            write_program(
                &programs.join("systemctl"),
                "#!/bin/sh\necho LoadState=not-found\necho ActiveState=inactive\n",
            );
            write_program(
                &programs.join("systemd-run"),
                "#!/bin/sh\necho 'this case starts no unit' >&2\nexit 1\n",
            );
        }
        programs
    }

    /// `conductor` of the case: its home, its config file, no terminal and standard input
    /// closed, a git identity for the records' commit, the `PATH` of [`Case::programs`] and
    /// no variable naming the user's service manager.
    fn conductor(&self, manager: bool) -> Command {
        let mut command = common::conductor(self.home());
        command
            .arg("--config")
            .arg(self.file())
            .env("PATH", self.programs(manager))
            .env_remove("XDG_RUNTIME_DIR")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .env("GIT_AUTHOR_NAME", "conductor")
            .env("GIT_AUTHOR_EMAIL", "conductor@example-org.example")
            .env("GIT_COMMITTER_NAME", "conductor")
            .env("GIT_COMMITTER_EMAIL", "conductor@example-org.example")
            .stdin(Stdio::null());
        command
    }

    fn trusted(&self) -> Vec<String> {
        let Ok(text) = fs::read_to_string(self.home().join(".claude.json")) else {
            return Vec::new();
        };
        let document: Value = serde_json::from_str(&text).expect("~/.claude.json is JSON");
        document["projects"]
            .as_object()
            .map(|projects| {
                projects
                    .iter()
                    .filter(|(_, project)| project["hasTrustDialogAccepted"] == Value::Bool(true))
                    .map(|(path, _)| path.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A `daemon serve` of a case, killed when dropped, with what it printed.
struct Served {
    child: Child,
    printed: Arc<Mutex<String>>,
}

impl Served {
    /// Starts `daemon serve` of `case` with no terminal and its standard input closed, and waits
    /// for its serving line, which follows its unattended start.
    fn start(case: &Case, manager: bool, env: &[(&str, &str)]) -> Self {
        let mut served = Self::spawn(case, manager, env);
        served.until("the serving line", |served| {
            served.printed().contains("conductord: config ")
        });
        served
    }

    /// [`Served::start`] without waiting for anything.
    fn spawn(case: &Case, manager: bool, env: &[(&str, &str)]) -> Self {
        let mut command = case.conductor(manager);
        command
            .args(["daemon", "serve", "--state-dir"])
            .arg(case.dir.join("daemon"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, value) in env {
            command.env(name, value);
        }
        let mut child = command.spawn().expect("run daemon serve");
        let printed = Arc::new(Mutex::new(String::new()));
        for mut pipe in [
            Box::new(child.stdout.take().expect("stdout")) as Box<dyn Read + Send>,
            Box::new(child.stderr.take().expect("stderr")),
        ] {
            let printed = Arc::clone(&printed);
            thread::spawn(move || {
                let mut chunk = [0_u8; 1024];
                while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                    if let Ok(mut printed) = printed.lock() {
                        printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                    }
                }
            });
        }
        Self { child, printed }
    }

    fn printed(&self) -> String {
        self.printed
            .lock()
            .map(|printed| printed.clone())
            .unwrap_or_default()
    }

    fn runs(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Waits until `holds`, while the daemon runs, within [`BOUND`].
    fn until(&mut self, what: &str, holds: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + BOUND;
        while !holds(self) {
            assert!(self.runs(), "the daemon exited: {}", self.printed());
            assert!(
                Instant::now() < deadline,
                "never held: {what}: {}",
                self.printed()
            );
            thread::sleep(Duration::from_millis(100));
        }
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Waits until `holds` within [`BOUND`].
fn eventually(what: &str, holds: impl Fn() -> bool) {
    let deadline = Instant::now() + BOUND;
    while !holds() {
        assert!(Instant::now() < deadline, "never held: {what}");
        thread::sleep(Duration::from_millis(100));
    }
}

fn read_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn records_head(case: &Case) -> String {
    let output = Command::new(git_at())
        .arg("-C")
        .arg(case.records())
        .args(["rev-list", "--count", "HEAD"])
        .env("HOME", case.home())
        .output()
        .expect("run git");
    assert!(output.status.success(), "{}", said(&output));
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[test]
fn an_unattended_start_seeds_each_instance_and_a_second_start_overwrites_nothing() {
    let case = Case::new("seed");
    let checkout = case.checkout("widget");
    fs::write(
        case.secrets().join("alpha-model"),
        format!("{MODEL_VALUE}\n"),
    )
    .expect("mount the model token");
    case.config(free_port(), ", unattended: true", "none");

    let served = Served::start(&case, true, &[]);
    let printed = served.printed();
    let records = case
        .records()
        .canonicalize()
        .expect("the records were created");
    for file in ["NORTHSTAR.md", "rules.md", "README.md"] {
        assert!(records.join(file).is_file(), "no {file}: {printed}");
    }
    assert_eq!(records_head(&case), "1", "the skeleton is committed once");
    let trusted = case.trusted();
    for path in [&checkout, &records] {
        let path = path.display().to_string();
        assert!(
            trusted.contains(&path),
            "{path} not trusted: {trusted:?}\n{printed}"
        );
    }
    let profile = case.dir.join("worktree/alpha.toml");
    assert!(profile.is_file(), "no profile template: {printed}");
    let activated = format!(
        "activate --profile {} --workspace {}",
        profile.display(),
        case.root().display()
    );
    assert_eq!(
        case.worktree_log().matches(&activated).count(),
        1,
        "{}\n{printed}",
        case.worktree_log()
    );
    let source = format!(
        "secrets: files {} namespace default (answers)",
        case.secrets().display()
    );
    assert!(printed.contains(&source), "{printed}");
    assert!(!printed.contains("SecretMissing"), "{printed}");
    assert!(!printed.contains(MODEL_VALUE), "{printed}");
    drop(served);

    // What a person or a session changed since is kept; nothing is written again.
    let rules = records.join("rules.md");
    let mut edited = fs::read_to_string(&rules).expect("rules.md");
    edited.push_str("\nAn example-org rule, kept across starts.\n");
    fs::write(&rules, &edited).expect("edit rules.md");
    let before: Vec<(PathBuf, Vec<u8>)> = [
        rules.clone(),
        records.join("NORTHSTAR.md"),
        case.home().join(".claude.json"),
        profile.clone(),
    ]
    .into_iter()
    .map(|path| {
        let bytes = read_bytes(&path);
        (path, bytes)
    })
    .collect();

    let served = Served::start(&case, true, &[]);
    let printed = served.printed();
    for (path, bytes) in &before {
        assert_eq!(
            &read_bytes(path),
            bytes,
            "{} was written again: {printed}",
            path.display()
        );
    }
    assert_eq!(records_head(&case), "1", "a second commit: {printed}");
    assert_eq!(
        case.worktree_log().matches("activate").count(),
        1,
        "activated again: {}\n{printed}",
        case.worktree_log()
    );
    for word in ["wrote:", "trusted:", "ran:"] {
        assert!(
            !printed.contains(word),
            "{word} on a second start: {printed}"
        );
    }
}

/// A runtime alone in a pod (`CONDUCTOR_API_LISTEN` set: no daemon beside it to seed for it)
/// starts `instance serve` of its one instance and runs no daemon.
fn alone(case: &Case) -> Served {
    let mut command = case.conductor(true);
    command
        .current_dir(&case.dir)
        .args(["instance", "serve"])
        .env("CONDUCTOR_INSTANCE", "alpha")
        .env("CONDUCTOR_API_LISTEN", "127.0.0.1:0")
        .env("CONDUCTOR_API_TOKEN", "3".repeat(64))
        .env("CONDUCTOR_RUNTIME_LOOP", "off")
        .env_remove("CONDUCTOR_CONFIG_SOURCE")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("run instance serve");
    let printed = Arc::new(Mutex::new(String::new()));
    for mut pipe in [
        Box::new(child.stdout.take().expect("stdout")) as Box<dyn Read + Send>,
        Box::new(child.stderr.take().expect("stderr")),
    ] {
        let printed = Arc::clone(&printed);
        thread::spawn(move || {
            let mut chunk = [0_u8; 1024];
            while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                if let Ok(mut printed) = printed.lock() {
                    printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                }
            }
        });
    }
    let mut served = Served { child, printed };
    served.until("the seeding's last line", |served| {
        let printed = served.printed();
        printed.contains("seeded its instance") || printed.contains("seeding failed")
    });
    served
}

/// A runtime alone in a pod, with no `daemon.unattended`, seeds its own instance at its start as
/// the unattended daemon seeds each one: the records skeleton written and committed, the
/// checkouts root's checkout and the records trusted, the root activated as a `worktree`
/// workspace. A second start writes none of it again and keeps what was changed since.
#[test]
fn a_runtime_alone_in_a_pod_seeds_its_instance_and_a_second_start_overwrites_nothing() {
    let case = Case::new("pod-seed");
    let checkout = case.checkout("widget");
    fs::write(
        case.secrets().join("alpha-model"),
        format!("{MODEL_VALUE}\n"),
    )
    .expect("mount the model token");
    case.config(free_port(), "", "none");

    let served = alone(&case);
    let printed = served.printed();
    assert!(printed.contains("seeded its instance"), "{printed}");
    let records = case
        .records()
        .canonicalize()
        .expect("the records were created");
    for file in ["NORTHSTAR.md", "rules.md", "README.md"] {
        assert!(records.join(file).is_file(), "no {file}: {printed}");
    }
    assert_eq!(records_head(&case), "1", "the skeleton is committed once");
    let trusted = case.trusted();
    for path in [&checkout, &records] {
        let path = path.display().to_string();
        assert!(
            trusted.contains(&path),
            "{path} not trusted: {trusted:?}\n{printed}"
        );
    }
    let profile = case.dir.join("worktree/alpha.toml");
    assert!(profile.is_file(), "no profile template: {printed}");
    assert_eq!(
        case.worktree_log().matches("activate --profile").count(),
        1,
        "{}\n{printed}",
        case.worktree_log()
    );
    drop(served);

    let rules = records.join("rules.md");
    let mut edited = fs::read_to_string(&rules).expect("rules.md");
    edited.push_str("\nAn example-org rule, kept across starts.\n");
    fs::write(&rules, &edited).expect("edit rules.md");
    let before: Vec<(PathBuf, Vec<u8>)> = [
        rules.clone(),
        records.join("NORTHSTAR.md"),
        case.home().join(".claude.json"),
        profile.clone(),
    ]
    .into_iter()
    .map(|path| {
        let bytes = read_bytes(&path);
        (path, bytes)
    })
    .collect();

    let served = alone(&case);
    let printed = served.printed();
    assert!(printed.contains("seeded its instance"), "{printed}");
    for (path, bytes) in &before {
        assert_eq!(
            &read_bytes(path),
            bytes,
            "{} was written again: {printed}",
            path.display()
        );
    }
    assert_eq!(records_head(&case), "1", "a second commit: {printed}");
    assert_eq!(
        case.worktree_log().matches("activate --profile").count(),
        1,
        "activated again: {}\n{printed}",
        case.worktree_log()
    );
    for word in ["wrote:", "trusted:", "ran:", "created:"] {
        assert!(
            !printed.contains(word),
            "{word} on a second start: {printed}"
        );
    }
}

#[test]
fn without_unattended_the_start_creates_no_record_trust_or_workspace() {
    let case = Case::new("plain");
    case.checkout("widget");
    case.config(free_port(), "", "none");
    let served = Served::start(&case, true, &[]);
    let printed = served.printed();
    assert!(!case.records().exists(), "records created: {printed}");
    assert!(
        !case.home().join(".claude.json").exists(),
        "trust written: {printed}"
    );
    assert!(
        !case.worktree_log().contains("activate"),
        "{}",
        case.worktree_log()
    );
    assert!(!case.dir.join("worktree").exists(), "{printed}");
}

#[test]
fn the_files_source_is_read_and_never_written() {
    let case = Case::new("files");
    case.config(free_port(), "", "none");

    let validated = case
        .conductor(true)
        .args(["config", "validate"])
        .output()
        .expect("run config validate");
    assert_eq!(validated.status.code(), Some(0), "{}", said(&validated));

    // A put is refused in one line before any value is read, and the directory is untouched.
    let mut put = case
        .conductor(true)
        .args(["secret", "put", "--name", "alpha-model"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run secret put");
    put.stdin
        .take()
        .expect("stdin")
        .write_all(MODEL_VALUE.as_bytes())
        .expect("pipe the value");
    let put = put.wait_with_output().expect("secret put ends");
    let refusal = String::from_utf8_lossy(&put.stderr).into_owned();
    assert_eq!(put.status.code(), Some(1), "{}", said(&put));
    assert_eq!(refusal.lines().count(), 1, "{refusal}");
    assert!(refusal.contains("SourceReadOnly"), "{refusal}");
    assert!(
        fs::read_dir(case.secrets())
            .expect("the directory")
            .next()
            .is_none(),
        "the put wrote into the directory"
    );
    assert!(!said(&put).contains(MODEL_VALUE), "{}", said(&put));

    // The instance's model token, from its file, one line end dropped.
    let reveal = |case: &Case| {
        case.conductor(true)
            .args(["token", "reveal", "--model-token", "alpha"])
            .output()
            .expect("run token reveal")
    };
    let missing = reveal(&case);
    assert_eq!(missing.status.code(), Some(1), "{}", said(&missing));
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("SecretMissing"),
        "{}",
        said(&missing)
    );
    fs::write(
        case.secrets().join("alpha-model"),
        format!("{MODEL_VALUE}\n"),
    )
    .expect("mount the model token");
    let revealed = reveal(&case);
    assert_eq!(revealed.status.code(), Some(0), "{}", said(&revealed));
    assert_eq!(
        String::from_utf8_lossy(&revealed.stdout),
        format!("{MODEL_VALUE}\n")
    );

    // Doctor names the directory, and that it answers; never the value.
    let doctor = case
        .conductor(true)
        .arg("doctor")
        .output()
        .expect("run doctor");
    let text = said(&doctor);
    let line = format!(
        "secrets: files {} namespace default (answers)",
        case.secrets().display()
    );
    assert!(text.contains(&line), "{text}");
    assert!(!text.contains(MODEL_VALUE), "{text}");

    // The files source needs its `dir`, and reads no Secrets service.
    for (secrets, path) in [
        ("{source: files}", "daemon.secrets.dir"),
        (
            "{source: files, dir: /run/secrets/conductor, url: \"https://secrets.example\"}",
            "daemon.secrets.url",
        ),
    ] {
        let text = fs::read_to_string(case.file()).expect("the config file");
        let start = text.find("secrets: {").expect("the secrets source");
        let end = start + text[start..].find('}').expect("its end") + 1;
        fs::write(
            case.file(),
            format!("{}secrets: {secrets}{}", &text[..start], &text[end..]),
        )
        .expect("write the config file");
        let output = case
            .conductor(true)
            .args(["config", "validate"])
            .output()
            .expect("run config validate");
        assert_eq!(
            output.status.code(),
            Some(1),
            "{secrets}: {}",
            said(&output)
        );
        assert!(said(&output).contains(path), "{secrets}: {}", said(&output));
        fs::write(case.file(), text).expect("restore the config file");
    }
}

/// One request to `127.0.0.1:port` with `host` as its `Host`: its status and body.
fn ask(port: u16, target: &str, host: &str) -> Option<(u16, String)> {
    ask_at(
        IpAddr::from([127, 0, 0, 1]),
        port,
        target,
        host,
        Duration::from_secs(60),
    )
}

/// One `GET target` to `address:port` with `host` as its `Host`, answered within `wait`: its
/// status and body.
fn ask_at(
    address: IpAddr,
    port: u16,
    target: &str,
    host: &str,
    wait: Duration,
) -> Option<(u16, String)> {
    exchange(
        address,
        port,
        &format!("GET {target} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"),
        wait,
    )
}

/// One request `text` to `address:port`, answered within `wait`: its status and body.
fn exchange(address: IpAddr, port: u16, text: &str, wait: Duration) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect((address, port)).ok()?;
    stream.set_read_timeout(Some(wait)).expect("a read timeout");
    stream.write_all(text.as_bytes()).ok()?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;
    let (head, body) = answer.split_once("\r\n\r\n")?;
    let status = head.split(' ').nth(1)?.parse().ok()?;
    Some((status, body.to_owned()))
}

/// The reason `/readyz` gave in `body`, its one row's `reason`.
fn readiness_reason(body: &str) -> String {
    let rows: Value = serde_json::from_str(body).unwrap_or_else(|_| panic!("no JSON: {body}"));
    rows[0]["reason"].as_str().unwrap_or_default().to_owned()
}

/// A loopback listener bound to `::1` answers the `Host` that names it, bracketed, beside
/// `127.0.0.1` and `localhost`, and no other; a write from a page it served, whose `Origin` is
/// `http://[::1]:<port>`, is taken as one from its own origin. Skipped where this host has no
/// IPv6 loopback address.
#[test]
fn a_loopback_address_answers_the_host_it_is_bound_to_and_no_other() {
    let address = IpAddr::from(std::net::Ipv6Addr::LOCALHOST);
    let Ok(free) = TcpListener::bind((address, 0)) else {
        eprintln!("skipped: this host has no IPv6 loopback address to bind");
        return;
    };
    let port = free.local_addr().expect("its address").port();
    drop(free);
    let case = Case::new("address-v6");
    case.config(port, ", address: \"::1\"", "none");
    let served = Served::start(&case, true, &[]);
    let printed = served.printed();
    assert!(
        printed.contains(&format!("on http://[::1]:{port}/")),
        "{printed}"
    );
    let wait = Duration::from_secs(30);
    for host in [
        format!("[::1]:{port}"),
        "[::1]".to_owned(),
        format!("127.0.0.1:{port}"),
        "localhost".to_owned(),
    ] {
        let answer = ask_at(address, port, "/healthz", &host, wait);
        assert!(
            matches!(answer, Some((200, _))),
            "Host {host}: {answer:?}\n{printed}"
        );
    }
    for host in [
        format!("example.org:{port}"),
        format!("[::2]:{port}"),
        "::1".to_owned(),
        format!("127.0.0.2:{port}"),
    ] {
        let answer = ask_at(address, port, "/healthz", &host, wait);
        assert!(
            matches!(answer, Some((403, _))),
            "Host {host}: {answer:?}\n{printed}"
        );
    }

    let token = fs::read_to_string(case.dir.join("daemon").join(api::TOKEN_FILE))
        .expect("the daemon keeps its token");
    let write = |origin: &str| {
        exchange(
            address,
            port,
            &format!(
                "POST /v1/roles/nothing/start HTTP/1.1\r\nHost: [::1]:{port}\r\nOrigin: {origin}\r\n\
                 {header}: {token}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\
                 Connection: close\r\n\r\n{{}}",
                header = conductor_cli::dashboard::TOKEN_HEADER,
            ),
            wait,
        )
        .unwrap_or_else(|| panic!("no answer to a write from {origin}"))
    };
    let (status, body) = write(&format!("http://[::1]:{port}"));
    assert!(
        !body.contains("own origin"),
        "a write from the listener's own page: {status} {body}"
    );
    let (status, body) = write("http://example.org");
    assert_eq!(status, 403, "a write from another origin: {body}");
    assert!(body.contains("own origin"), "{body}");
}

/// While the unattended start seeds, the listener answers: `/healthz` is `200` and `/readyz`
/// `503` with the reason that the start is seeding; once it has seeded, `/readyz` is `200`.
#[test]
fn while_the_unattended_start_seeds_the_probes_answer_and_readiness_names_the_seeding() {
    let case = Case::new("seeding");
    case.checkout("widget");
    let port = free_port();
    case.config(port, ", unattended: true", "none");
    fs::write(case.dir.join("inspect-pause"), "8\n").expect("pause worktree inspect");
    let mut served = Served::spawn(&case, true, &[]);
    let asked = case.dir.join("inspect-asked");
    served.until("the start asks worktree inspect", |_| asked.exists());
    let loopback = IpAddr::from([127, 0, 0, 1]);
    let host = format!("127.0.0.1:{port}");
    let within = Duration::from_secs(3);
    let live = ask_at(loopback, port, "/healthz", &host, within);
    let ready = ask_at(loopback, port, "/readyz", &host, within);
    served.until("the serving line", |served| {
        served.printed().contains("conductord: config ")
    });
    let printed = served.printed();
    assert!(
        matches!(live, Some((200, _))),
        "/healthz while the start seeds: {live:?}\n{printed}"
    );
    let (status, body) =
        ready.unwrap_or_else(|| panic!("/readyz while the start seeds: no answer\n{printed}"));
    assert_eq!(status, 503, "{body}\n{printed}");
    assert_eq!(readiness_reason(&body), metrics::SEEDING, "{body}");
    served.until("/readyz answers 200 once the start has seeded", |_| {
        ask(port, "/readyz", &host).is_some_and(|(status, _)| status == 200)
    });
}

/// A seeding step that fails, here the records' commit with no git identity, is said in one
/// line naming the instance with git's reason, at every start that finds it so; the daemon goes
/// on serving and reconciling, and `/readyz` stays `503` with the reason that the seeding
/// failed. The skeleton is left as it lies, and the first start with an identity commits it.
#[test]
fn a_seeding_that_fails_keeps_the_daemon_serving_and_a_later_start_commits_the_records() {
    let case = Case::new("seed-fails");
    let port = free_port();
    case.config(port, ", unattended: true", "none");
    let host = format!("127.0.0.1:{port}");
    let records = case.records();
    let skeleton = || -> Vec<(PathBuf, Vec<u8>)> {
        ["NORTHSTAR.md", "rules.md", "README.md"]
            .into_iter()
            .map(|file| {
                let path = records.join(file);
                let bytes = read_bytes(&path);
                (path, bytes)
            })
            .collect()
    };
    let has_head = || {
        Command::new(git_at())
            .arg("-C")
            .arg(case.records())
            .args(["rev-parse", "--verify", "--quiet", "HEAD"])
            .env("HOME", case.home())
            .output()
            .expect("run git")
            .status
            .success()
    };
    let no_identity = [("GIT_AUTHOR_NAME", ""), ("GIT_COMMITTER_NAME", "")];
    let mut written = None;
    for start in 1..=2 {
        let mut served = Served::start(&case, true, &no_identity);
        served.until("the reconciler answers /v1/instances", |_| {
            ask(port, "/v1/instances", &host).is_some_and(|(status, _)| status == 200)
        });
        let printed = served.printed();
        let (status, body) = ask(port, "/readyz", &host).expect("an answer to /readyz");
        assert_eq!(status, 503, "start {start}: {body}\n{printed}");
        assert_eq!(readiness_reason(&body), metrics::SEEDING_FAILED, "{body}");
        let live = ask(port, "/healthz", &host);
        assert!(matches!(live, Some((200, _))), "{live:?}");
        assert!(served.runs(), "the daemon ended: {printed}");
        let said: Vec<&str> = printed
            .lines()
            .filter(|line| line.starts_with("conductor: daemon serve: unattended: alpha:"))
            .collect();
        assert_eq!(said.len(), 1, "start {start}: {printed}");
        assert!(
            said[0].contains("not committed")
                && said[0].contains("git commit")
                && said[0].contains("the next start tries again")
                && !said[0].contains("commits it"),
            "start {start}: {}",
            said[0]
        );
        assert!(
            !printed
                .lines()
                .any(|line| line.trim_start().starts_with("***")),
            "start {start} printed more of git than its reason: {printed}"
        );
        assert!(!has_head(), "start {start} committed: {printed}");
        let now = skeleton();
        if let Some(written) = &written {
            assert_eq!(
                &now, written,
                "start {start} changed the skeleton: {printed}"
            );
        }
        written = Some(now);
    }

    let mut served = Served::start(&case, true, &[]);
    served.until("/readyz answers 200 once the commit is made", |_| {
        ask(port, "/readyz", &host).is_some_and(|(status, _)| status == 200)
    });
    let printed = served.printed();
    assert_eq!(records_head(&case), "1", "{printed}");
    assert_eq!(
        Some(skeleton()),
        written,
        "the commit changed the skeleton: {printed}"
    );
    assert!(
        !printed.contains("conductor: daemon serve: unattended: alpha:"),
        "{printed}"
    );
}

/// A skeleton left uncommitted and outside the index, whose every file git ignores, is not
/// committed, and the start says so in its one line without promising that the next start
/// commits it.
#[test]
fn a_records_skeleton_git_ignores_whole_is_said_and_not_promised() {
    let case = Case::new("seed-ignored");
    let port = free_port();
    case.config(port, ", unattended: true", "none");
    let host = format!("127.0.0.1:{port}");
    let no_identity = [("GIT_AUTHOR_NAME", ""), ("GIT_COMMITTER_NAME", "")];
    drop(Served::start(&case, true, &no_identity));
    // A file of the index is subject to no ignore rule: the skeleton is left outside it.
    fs::remove_file(case.records().join(".git/index")).expect("empty the index");
    let info = case.records().join(".git/info");
    fs::create_dir_all(&info).expect("the repository's info directory");
    fs::write(info.join("exclude"), "*\n").expect("ignore every file");

    let mut served = Served::start(&case, true, &[]);
    served.until("the reconciler answers /v1/instances", |_| {
        ask(port, "/v1/instances", &host).is_some_and(|(status, _)| status == 200)
    });
    let printed = served.printed();
    let said: Vec<&str> = printed
        .lines()
        .filter(|line| line.starts_with("conductor: daemon serve: unattended: alpha:"))
        .collect();
    assert_eq!(said.len(), 1, "{printed}");
    assert!(
        said[0].contains("not committed")
            && said[0].contains("git ignores")
            && !said[0].contains("next start"),
        "{}",
        said[0]
    );
    let head = Command::new(git_at())
        .arg("-C")
        .arg(case.records())
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .env("HOME", case.home())
        .output()
        .expect("run git");
    assert!(!head.status.success(), "the start committed: {printed}");
}

#[test]
fn a_daemon_address_that_is_not_loopback_answers_the_probes_and_takes_bearers_on_v1() {
    let case = Case::new("address");
    let port = free_port();
    case.config(port, ", address: 0.0.0.0", "none");
    let mut served = Served::start(&case, true, &[]);
    let printed = served.printed();
    assert!(
        printed.contains(&format!("on http://0.0.0.0:{port}/")),
        "{printed}"
    );
    // A probe from another address, naming the container's address as its host.
    let host = format!("10.1.2.3:{port}");
    let (status, body) = ask(port, "/healthz", &host).expect("an answer to the probe");
    assert_eq!(status, 200, "{body}");
    served.until("readiness answers 200", |_| {
        ask(port, "/readyz", &host).is_some_and(|(status, _)| status == 200)
    });
    let (status, body) = ask(port, "/v1/instances", &host).expect("an answer");
    assert_eq!(status, 401, "{body}");
    let (status, body) = ask(port, "/v1/instances", "localhost").expect("an answer");
    assert_eq!(
        status, 401,
        "a /v1/ read without a bearer on any host: {body}"
    );
    drop(served);

    case.config(port, ", address: somewhere", "none");
    let output = case
        .conductor(true)
        .args(["config", "validate"])
        .output()
        .expect("run config validate");
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        said(&output).contains("daemon.address"),
        "{}",
        said(&output)
    );
}

/// Whether the instance runtime's socket of `case` answers a connection.
fn runtime_answers(case: &Case) -> bool {
    UnixStream::connect(conductor_cli::instance::socket(&case.state())).is_ok()
}

#[test]
fn without_a_user_manager_the_runtime_is_a_child_of_the_daemon_and_ends_with_it() {
    let case = Case::new("child");
    case.config(free_port(), "", "none");
    // The runtime serves its API alone: no loop reaches a forge.
    let quiet = [(conductor_cli::instance::LOOP_VARIABLE, "off")];

    let mut served = Served::start(&case, false, &quiet);
    served.until("the runtime answers on its socket", |_| {
        runtime_answers(&case)
    });
    // The pass's line follows the start it records.
    served.until("the start recorded", |served| {
        served
            .printed()
            .contains("alpha: Start (Cadence): Succeeded")
    });
    assert!(
        served
            .printed()
            .contains("no user service manager here: each runtime runs as a child process"),
        "{}",
        served.printed()
    );
    // SIGTERM: the daemon stops its runtime and exits 0.
    nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(i32::try_from(served.child.id()).expect("a pid")),
        nix::sys::signal::Signal::SIGTERM,
    )
    .expect("send SIGTERM");
    let deadline = Instant::now() + BOUND;
    let status = loop {
        if let Some(status) = served.child.try_wait().expect("wait") {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "the daemon did not end on SIGTERM: {}",
            served.printed()
        );
        thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(status.code(), Some(0), "{}", served.printed());
    eventually("the runtime ends with the daemon", || {
        !runtime_answers(&case)
    });
    drop(served);

    // SIGKILL leaves the daemon no time: the runtime ends on its own once its parent is gone.
    let mut served = Served::start(&case, false, &quiet);
    served.until("the runtime answers again", |_| runtime_answers(&case));
    served.child.kill().expect("SIGKILL the daemon");
    let _ = served.child.wait();
    eventually("the runtime ends with the killed daemon", || {
        !runtime_answers(&case)
    });
}

#[test]
fn the_user_manager_decides_how_a_runtime_without_a_box_starts() {
    let case = Case::new("manager");
    let runtime = case.dir.join("run");
    let empty = case.dir.join("empty");
    fs::create_dir_all(runtime.join("systemd")).expect("the runtime directory");
    fs::create_dir_all(&empty).expect("an empty PATH directory");
    let bin = case.dir.join("bin");
    fs::create_dir_all(&bin).expect("a PATH directory");
    write_program(&bin.join("systemd-run"), "#!/bin/sh\nexit 0\n");

    // No manager's socket and no `systemd-run`: a child process.
    assert!(!units::has_user_manager(
        Some(runtime.as_os_str()),
        Some(empty.as_os_str())
    ));
    assert!(!units::has_user_manager(None, None));
    // The user manager's socket, or `systemd-run` on the `PATH`: the user unit, as before.
    // A stand-in of the manager's socket: a socket's address would not hold the case's path.
    fs::write(runtime.join("systemd/private"), "").expect("a stand-in of the manager's socket");
    assert!(units::has_user_manager(
        Some(runtime.as_os_str()),
        Some(empty.as_os_str())
    ));
    assert!(units::has_user_manager(None, Some(bin.as_os_str())));
    assert!(matches!(
        units::Manager::chosen(Some(runtime.as_os_str()), None),
        units::Manager::Systemd(_)
    ));
    assert!(matches!(
        units::Manager::chosen(None, Some(OsStr::new(""))),
        units::Manager::Children(_)
    ));
}

fn service(case: &Case, program: &Path) -> Service {
    Service {
        unit: unit_name("alpha"),
        slice: "conductor_alpha.slice".to_owned(),
        limits: Limits {
            memory_max: None,
            cpu_weight: None,
            tasks_max: None,
        },
        cwd: case.dir.clone(),
        properties: Vec::new(),
        env: vec!["EXAMPLE_ORG=1".to_owned()],
        command: vec![program.display().to_string()],
    }
}

#[test]
fn a_child_runtime_is_started_stopped_and_seen_running() {
    let case = Case::new("units");
    let program = case.dir.join("runtime");
    write_program(
        &program,
        &format!(
            "#!/bin/sh\necho \"$EXAMPLE_ORG\" > '{}'\nexec sleep 300\n",
            case.dir.join("ran").display()
        ),
    );
    let mut children = Children::default();
    let unit = unit_name("alpha");
    assert!(!children.active(&unit).expect("active"));
    children.start(&service(&case, &program)).expect("start");
    assert!(children.active(&unit).expect("active"));
    assert!(children.answers(&unit).expect("answers"));
    eventually("the child ran with its variables", || {
        fs::read_to_string(case.dir.join("ran")).is_ok_and(|text| text.trim() == "1")
    });
    assert!(
        children.start(&service(&case, &program)).is_err(),
        "a second start of a running unit"
    );
    children.stop(&unit).expect("stop");
    assert!(!children.active(&unit).expect("active"));
    children.stop(&unit).expect("a stop of a stopped unit");
}

#[test]
fn a_child_runtime_that_exits_is_started_again_under_the_failed_start_rules() {
    let case = Case::new("lost");
    case.config(free_port(), "", "none");
    let program = case.dir.join("conductor");
    write_program(
        &program,
        "#!/bin/sh\necho 'the runtime fails' >&2\nexit 1\n",
    );
    let mut daemon = Daemon::open(
        case.file(),
        case.home(),
        case.dir.join("daemon"),
        Binary {
            path: program,
            version: "1.0.0".to_owned(),
        },
        Children::default(),
    )
    .expect("open the daemon");
    let started = Instant::now();
    let done = daemon
        .pass(ReconcileReason::Cadence, started)
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Start);
    assert_eq!(done[0].result, ReconcileResult::Succeeded, "{done:?}");
    let unit = unit_name("alpha");
    eventually("the child exits", || {
        let mut probe = daemon.units.clone();
        !probe.active(&unit).unwrap_or(true)
    });
    let now = Instant::now();
    assert_eq!(
        daemon.due(now).expect("due"),
        Some(ReconcileReason::LostRuntime)
    );
    let done = daemon
        .pass(ReconcileReason::LostRuntime, now)
        .expect("a pass");
    assert_eq!(done.len(), 1, "{done:?}");
    assert_eq!(done[0].action, ReconcileAction::Start);
    assert_eq!(done[0].reason, ReconcileReason::LostRuntime);
    assert_eq!(done[0].result, ReconcileResult::Refused, "{done:?}");
    let detail = done[0].detail.as_deref().unwrap_or_default();
    assert!(detail.contains("failed start 1 in a row"), "{detail}");
}

#[test]
fn doctor_reads_a_forge_login_from_the_cli_s_own_variable() {
    let case = Case::new("login");
    case.config(free_port(), "", "none");
    let programs = case.programs(true);
    write_program(
        &programs.join("gh"),
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo 'gh version 2.103.3 (2026-01-01)'; exit 0; fi\n\
         if [ \"$1 $2\" = 'auth status' ] && [ -n \"$GH_TOKEN\" ]; then\n\
         \x20 echo '  Logged in to github.com account example-bot (GH_TOKEN)'; exit 0\n\
         fi\n\
         echo 'You are not logged into any GitHub hosts.' >&2; exit 1\n",
    );
    let doctor = |token: Option<&str>| {
        let mut command = case.conductor(true);
        command.arg("doctor").env_remove("GH_TOKEN");
        if let Some(token) = token {
            command.env("GH_TOKEN", token);
        }
        said(&command.output().expect("run doctor"))
    };
    let without = doctor(None);
    assert!(
        without.contains("gh: not logged in to github.com"),
        "{without}"
    );
    let with = doctor(Some("example-org-forge-token"));
    assert!(
        with.contains("gh: logged in to github.com as example-bot"),
        "{with}"
    );
    assert!(!with.contains("example-org-forge-token"), "{with}");
}

/// Runs `docker <args>`, its output.
fn docker(args: &[&str]) -> Output {
    Command::new("docker")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run docker")
}

fn docker_ok(args: &[&str]) -> String {
    let output = docker(args);
    assert!(
        output.status.success(),
        "docker {}: {}",
        args.join(" "),
        said(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// The container of [`the_image_starts_with_no_terminal_and_doctor_ends_ready`], removed when
/// dropped, and the image with it unless `CONDUCTOR_CONTAINER_KEEP` is set.
struct Container {
    name: String,
    image: String,
}

impl Drop for Container {
    fn drop(&mut self) {
        let _ = docker(&["rm", "--force", &self.name]);
        if std::env::var_os("CONDUCTOR_CONTAINER_KEEP").is_none() {
            let _ = docker(&["image", "rm", "--force", &self.image]);
        }
    }
}

/// Waits until `/readyz` of `name` answers 200 inside the container, within [`BOUND`].
fn until_ready(name: &str) {
    let deadline = Instant::now() + BOUND;
    loop {
        let output = docker(&[
            "exec",
            name,
            "curl",
            "-s",
            "-o",
            "/dev/null",
            "-w",
            "%{http_code}",
            "http://127.0.0.1:7300/readyz",
        ]);
        if String::from_utf8_lossy(&output.stdout).trim() == "200" {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "/readyz never answered 200: {}",
            said(&docker(&["logs", name]))
        );
        thread::sleep(Duration::from_millis(500));
    }
}

/// The image of this tree's binary starts with no terminal: `docker run` without `-t` or `-i`,
/// a config file and a directory of secrets mounted read-only, and nothing typed. It reads
/// `/readyz` inside the container and through the published port, the records and the trust the
/// start created, and doctor's last line, once a checkout is under the root and the container
/// started again. It builds the image itself, tagged `CONDUCTOR_CONTAINER_TAG` (else
/// `conductor:check`), and removes the container and the image when it ends. Ignored by default:
/// it needs `docker` and the network the image's build downloads from (`task container-test`).
#[test]
#[ignore = "needs docker and the network: task container-test"]
fn the_image_starts_with_no_terminal_and_doctor_ends_ready() {
    let case = Case::new("image");
    let image =
        std::env::var("CONDUCTOR_CONTAINER_TAG").unwrap_or_else(|_| "conductor:check".to_owned());
    let context = case.dir.join("context");
    fs::create_dir_all(&context).expect("the build context");
    let binary = PathBuf::from(common::conductor(case.home()).get_program());
    fs::copy(&binary, context.join("conductor")).expect("the binary in the build context");
    let dockerfile = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Dockerfile");
    let built = Command::new("docker")
        .args(["build", "--tag", &image, "--file"])
        .arg(&dockerfile)
        .arg(&context)
        .stdin(Stdio::null())
        .status()
        .expect("run docker build");
    assert!(built.success(), "docker build failed");
    let name = format!("conductor-check-{}", std::process::id());
    let container = Container {
        name: name.clone(),
        image: image.clone(),
    };

    let home = "/var/lib/conductor";
    let config = format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 port: 7300\n\
         \x20 address: 0.0.0.0\n\
         \x20 unattended: true\n\
         \x20 store: sqlite:{home}/conductor.sqlite\n\
         \x20 secrets: {{source: files, dir: /run/secrets/conductor}}\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{{local: {home}/alpha/checkouts}}]\n\
         \x20   forge: {{route: direct}}\n\
         \x20   model_token: secret:alpha-model\n\
         \x20   checkouts: {{root: {home}/alpha/checkouts, trees: {home}/alpha/trees}}\n\
         \x20   records: {home}/alpha/records\n\
         \x20   state: {home}/alpha/state\n\
         \x20   cache: {home}/alpha/cache\n"
    );
    let file = case.dir.join("conductor.yaml");
    fs::write(&file, config).expect("write the config file");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("readable");
    let secret = case.secrets().join("alpha-model");
    fs::write(&secret, format!("{MODEL_VALUE}\n")).expect("the model token");
    fs::set_permissions(&secret, fs::Permissions::from_mode(0o644)).expect("readable");
    fs::set_permissions(case.secrets(), fs::Permissions::from_mode(0o755)).expect("readable");

    let mounted_config = format!("{}:{home}/conductor.yaml:ro", file.display());
    let mounted_secrets = format!("{}:/run/secrets/conductor:ro", case.secrets().display());
    docker_ok(&[
        "run",
        "--detach",
        "--init",
        "--name",
        &name,
        "--publish",
        "127.0.0.1::7300",
        "--env",
        &format!("CONDUCTOR_CONFIG={home}/conductor.yaml"),
        "--env",
        "GIT_AUTHOR_NAME=conductor",
        "--env",
        "GIT_AUTHOR_EMAIL=conductor@example-org.example",
        "--env",
        "GIT_COMMITTER_NAME=conductor",
        "--env",
        "GIT_COMMITTER_EMAIL=conductor@example-org.example",
        "--volume",
        &mounted_config,
        "--volume",
        &mounted_secrets,
        &image,
        "conductor",
        "daemon",
        "serve",
    ]);
    let terminal = docker_ok(&[
        "inspect",
        "--format",
        "{{.Config.Tty}} {{.Config.OpenStdin}}",
        &name,
    ]);
    assert_eq!(
        terminal.trim(),
        "false false",
        "the container has a terminal or a standard input"
    );
    until_ready(&name);

    // The published port reaches the listener on daemon.address.
    let published = docker_ok(&["port", &name, "7300/tcp"]);
    let port: u16 = published
        .lines()
        .next()
        .and_then(|line| line.rsplit(':').next())
        .and_then(|port| port.trim().parse().ok())
        .unwrap_or_else(|| panic!("no published port: {published}"));
    let (status, body) = ask(port, "/readyz", "127.0.0.1").expect("the published port answers");
    assert_eq!(status, 200, "{body}");

    let logs = said(&docker(&["logs", &name]));
    assert!(
        logs.contains("secrets: files /run/secrets/conductor namespace default (answers)"),
        "{logs}"
    );
    assert!(!logs.contains("SecretMissing"), "{logs}");
    assert!(!logs.contains(MODEL_VALUE), "{logs}");
    docker_ok(&[
        "exec",
        &name,
        "test",
        "-f",
        &format!("{home}/alpha/records/NORTHSTAR.md"),
    ]);
    let trust = docker_ok(&["exec", &name, "cat", &format!("{home}/.claude.json")]);
    assert!(trust.contains(&format!("{home}/alpha/records")), "{trust}");

    // A checkout appears under the root; the next start trusts it and activates the workspace.
    docker_ok(&[
        "exec",
        &name,
        "sh",
        "-c",
        &format!(
            "git init --quiet {home}/alpha/checkouts/widget && \
             git -C {home}/alpha/checkouts/widget commit --quiet --allow-empty -m first"
        ),
    ]);
    docker_ok(&["restart", &name]);
    until_ready(&name);
    let trust = docker_ok(&["exec", &name, "cat", &format!("{home}/.claude.json")]);
    assert!(
        trust.contains(&format!("{home}/alpha/checkouts/widget")),
        "{trust}"
    );
    let doctor = docker(&["exec", &name, "conductor", "doctor"]);
    let text = said(&doctor);
    assert_eq!(
        String::from_utf8_lossy(&doctor.stdout).lines().last(),
        Some("ready"),
        "{text}\n{}",
        said(&docker(&["logs", &name]))
    );
    assert!(
        text.contains(&format!(
            "worktree workspace: {home}/alpha/checkouts activated"
        )),
        "{text}"
    );
    drop(container);
}

/// `program` on this process's `PATH`, if it is there.
fn which(program: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(program))
        .find(|path| path.is_file())
}

/// The check workflow's `container` job, run here step by step as a runner runs it: each `run:`
/// script under `bash -eo pipefail` in the workspace root, with the workflow's `env`, against a
/// `gh` that serves a `task` release archive and its checksums and a `cargo` that records what
/// it is asked. The job installs that `task`, at the workflow's `TASK_VERSION`, and its last step
/// runs `task container-test` from this tree's `Taskfile.yml` with the real `task`; the `cargo`
/// it runs selects, in this very test binary, the image case and only it, among the ignored
/// cases. The job writes nothing of the repository: no permission of its own, the workflow's
/// `contents: read`. Neither the job nor a step of it carries `if:` or `continue-on-error:`, so
/// it runs on every event of the workflow and its failure fails the run, and no step names a
/// `shell:` or `defaults:` other than the runner's `bash -eo pipefail` this case runs. What reads
/// the workflow runs everywhere; only the part that runs the steps is skipped where `task`
/// (go-task) is not on `PATH`.
#[test]
fn the_check_workflow_s_container_job_runs_the_image_case() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root");
    let text = fs::read_to_string(workspace.join(".github/workflows/check.yml"))
        .expect("read the check workflow");
    let workflow: serde_yaml::Value = serde_yaml::from_str(&text).expect("the workflow is YAML");
    let job = &workflow["jobs"]["container"];
    assert!(job.is_mapping(), "check.yml has no job `container`");
    assert_eq!(
        serde_yaml::to_string(&workflow["permissions"]).expect("render"),
        "contents: read\n",
        "the workflow's own permission"
    );
    assert!(
        job.get("permissions").is_none(),
        "the job asks for a permission of its own"
    );
    for key in ["if", "continue-on-error", "defaults"] {
        assert!(
            job.get(key).is_none(),
            "the job carries `{key}:`: {:?}",
            job.get(key)
        );
    }
    assert!(
        workflow.get("defaults").is_none(),
        "the workflow carries `defaults:`: {:?}",
        workflow.get("defaults")
    );
    let steps = job["steps"].as_sequence().expect("the job's steps");
    for (index, step) in steps.iter().enumerate() {
        for key in ["if", "continue-on-error", "shell"] {
            assert!(
                step.get(key).is_none(),
                "step {index} of the job carries `{key}:`: {step:?}"
            );
        }
    }
    let scripts: Vec<&str> = steps
        .iter()
        .filter_map(|step| step.get("run").and_then(serde_yaml::Value::as_str))
        .collect();
    assert!(!scripts.is_empty(), "the job runs nothing");
    assert!(
        scripts
            .last()
            .is_some_and(|script| script.trim() == "task container-test"),
        "the job's last script is not `task container-test`: {scripts:?}"
    );

    let Some(real_task) = which("task") else {
        eprintln!("skipped: `task` (go-task) is not on PATH, so the job's steps are not run");
        return;
    };
    let case = Case::new("job");
    let runner = case.dir.join("runner");
    let bin = case.dir.join("bin");
    fs::create_dir_all(&runner).expect("the runner's temporary directory");
    fs::create_dir_all(&bin).expect("the stand-ins' directory");
    let github_path = case.dir.join("github_path");
    fs::write(&github_path, "").expect("GITHUB_PATH");
    let archive = case.dir.join("archive");
    fs::create_dir_all(&archive).expect("the archive's directory");
    write_program(
        &archive.join("task"),
        &format!("#!/bin/sh\nexec '{}' \"$@\"\n", real_task.display()),
    );
    let downloads = case.dir.join("downloads.log");
    write_program(
        &bin.join("gh"),
        &format!(
            "#!/bin/sh\n\
             echo \"$*\" >> '{downloads}'\n\
             [ \"$1 $2\" = 'release download' ] || exit 2\n\
             tar -czf task_linux_amd64.tar.gz -C '{archive}' task\n\
             sha256sum task_linux_amd64.tar.gz > task_checksums.txt\n",
            downloads = downloads.display(),
            archive = archive.display(),
        ),
    );
    let asked = case.dir.join("cargo.log");
    write_program(
        &bin.join("cargo"),
        &format!(
            "#!/bin/sh\nfor word in \"$@\"; do echo \"$word\"; done > '{}'\n",
            asked.display()
        ),
    );

    let env: Vec<(String, String)> = workflow["env"]
        .as_mapping()
        .expect("the workflow's env")
        .iter()
        .map(|(name, value)| {
            let value = match value {
                serde_yaml::Value::String(text) => text.clone(),
                other => serde_yaml::to_string(other)
                    .expect("render")
                    .trim()
                    .to_owned(),
            };
            (name.as_str().expect("a name").to_owned(), value)
        })
        .collect();
    let system = std::env::var("PATH").unwrap_or_default();
    for (index, script) in scripts.iter().enumerate() {
        let added: Vec<String> = fs::read_to_string(&github_path)
            .expect("GITHUB_PATH")
            .lines()
            .rev()
            .map(str::to_owned)
            .collect();
        let path = added
            .into_iter()
            .chain([bin.display().to_string(), system.clone()])
            .collect::<Vec<_>>()
            .join(":");
        let file = case.dir.join(format!("step-{index}.sh"));
        fs::write(&file, script).expect("write the step's script");
        let output = Command::new("bash")
            .args(["--noprofile", "--norc", "-eo", "pipefail"])
            .arg(&file)
            .current_dir(&workspace)
            .envs(env.iter().map(|(name, value)| (name, value)))
            .env("PATH", path)
            .env("RUNNER_TEMP", &runner)
            .env("GITHUB_PATH", &github_path)
            .env("GH_TOKEN", "example-org-runner-token")
            .stdin(Stdio::null())
            .output()
            .expect("run the step");
        assert!(output.status.success(), "{script}\n{}", said(&output));
    }

    let version = env
        .iter()
        .find(|(name, _)| name == "TASK_VERSION")
        .map(|(_, value)| value.clone())
        .expect("the workflow pins TASK_VERSION");
    let downloaded = fs::read_to_string(&downloads).unwrap_or_default();
    assert!(
        downloaded.starts_with(&format!("release download {version} -R go-task/task ")),
        "{downloaded}"
    );
    let words: Vec<String> = fs::read_to_string(&asked)
        .expect("the job's task ran cargo")
        .lines()
        .map(str::to_owned)
        .collect();
    let split = words
        .iter()
        .position(|word| word == "--")
        .unwrap_or_else(|| panic!("cargo was given no test arguments: {words:?}"));
    let (cargo, harness) = (&words[..split], &words[split + 1..]);
    assert_eq!(cargo.first().map(String::as_str), Some("test"), "{words:?}");
    let after = |flag: &str| {
        cargo
            .iter()
            .position(|word| word == flag)
            .and_then(|at| cargo.get(at + 1))
            .map(String::as_str)
    };
    assert_eq!(after("-p"), Some("conductor-cli"), "{words:?}");
    assert_eq!(after("--test"), Some("container_start"), "{words:?}");
    let this = std::env::current_exe().expect("this test binary");
    assert!(
        this.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("container_start")),
        "{}",
        this.display()
    );
    let listed = Command::new(&this)
        .arg("--list")
        .args(harness.iter().filter(|word| *word != "--nocapture"))
        .stdin(Stdio::null())
        .output()
        .expect("list the cases the job selects");
    let selected: Vec<String> = String::from_utf8_lossy(&listed.stdout)
        .lines()
        .filter_map(|line| line.strip_suffix(": test"))
        .map(str::to_owned)
        .collect();
    assert!(harness.iter().any(|word| word == "--ignored"), "{words:?}");
    assert_eq!(
        selected,
        ["the_image_starts_with_no_terminal_and_doctor_ends_ready"],
        "{}",
        said(&listed)
    );
}
