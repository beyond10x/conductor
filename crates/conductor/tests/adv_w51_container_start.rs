//! Adversary cases of wave 51, unit U7 (`story:container-start-without-prompt`).
//!
//! - `token reveal --model-token` is documented for an instance whose placement opens no box;
//!   it prints the model token of an instance whose placement opens one.
//! - `daemon.address` accepts any IP address; a loopback address other than `127.0.0.1` makes
//!   its own probes answer 403.
//! - The unattended start writes the records skeleton and commits it; a commit that failed at
//!   one start is never made at a later one.
//! - The listener is bound before the unattended seeding and served only after it, so the
//!   probes do not answer while the start seeds.
//!
//! The second pass (`adv2_`), over the correction of those four:
//!
//! - The records commit of a later start is made where the repository's `HEAD` has no commit,
//!   not where the repository has none: a repository with a commit whose `HEAD` is on a branch
//!   with none yet gets one.
//! - That commit adds every file of `docs/handoff/`; one that git ignores fails the commit at
//!   every start, so the skeleton stays uncommitted.
//! - The case of the check workflow's `container` job reads neither the workflow's events nor
//!   the job's `needs:`, `runs-on:` or `timeout-minutes:`.
//!
//! No case reaches the host's service manager: a daemon started here runs on a `PATH` of the
//! case's own, which holds a `systemctl` that knows no unit and a `systemd-run` that starts
//! none, and the variables that name the user's manager are removed.

mod common;

use std::fs;
use std::io::{Read, Write as _};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long a case waits for what a daemon does.
const BOUND: Duration = Duration::from_secs(90);

/// The model token's value a case mounts.
const MODEL_VALUE: &str = "sk-example-org-adv-model-81d0";

/// A case's directory, removed after a passing run.
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

fn found(names: &[&str]) -> PathBuf {
    names
        .iter()
        .map(PathBuf::from)
        .find(|path| path.exists())
        .unwrap_or_else(|| panic!("none of {names:?} is installed"))
}

fn git_at() -> PathBuf {
    found(&["/usr/bin/git", "/bin/git", "/usr/local/bin/git"])
}

fn sleep_at() -> PathBuf {
    found(&["/usr/bin/sleep", "/bin/sleep"])
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
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("csadv")
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

    fn secrets(&self) -> PathBuf {
        self.dir.join("secrets")
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
            state = self.dir.join("alpha/state"),
            cache = self.dir.join("alpha/cache"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    /// One git checkout `name` under the checkouts root, its `origin` on the instance's owner.
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

    /// The `PATH` of a daemon of the case: `git`, a `worktree` whose `inspect` marks that it was
    /// asked and then takes `inspect_for` seconds, a `systemctl` that knows no unit and a
    /// `systemd-run` that starts none.
    fn programs(&self, inspect_for: u32) -> PathBuf {
        let programs = self.dir.join("programs");
        let _ = std::os::unix::fs::symlink(git_at(), programs.join("git"));
        write_program(
            &programs.join("worktree"),
            &format!(
                "#!/bin/sh\n\
                 case \"$1\" in\n\
                 \x20 inspect) : > '{asked}'; '{sleep}' {inspect_for}; \
                 echo '{{\"ok\":false,\"code\":\"workspace-not-activated\"}}' >&2; exit 1 ;;\n\
                 \x20 activate) echo '{{\"ok\":true}}' ;;\n\
                 \x20 --version) echo 'worktree 0.18.1' ;;\n\
                 esac\n",
                asked = self.dir.join("inspect-asked").display(),
                sleep = sleep_at().display(),
            ),
        );
        write_program(
            &programs.join("systemctl"),
            "#!/bin/sh\necho LoadState=not-found\necho ActiveState=inactive\n",
        );
        write_program(
            &programs.join("systemd-run"),
            "#!/bin/sh\necho 'this case starts no unit' >&2\nexit 1\n",
        );
        programs
    }

    /// `conductor` of the case: its home, its config file, standard input closed, a git
    /// identity, the `PATH` of [`Case::programs`] and no variable naming the user's manager.
    fn conductor(&self, inspect_for: u32) -> Command {
        let mut command = common::conductor(self.home());
        command
            .arg("--config")
            .arg(self.file())
            .env("PATH", self.programs(inspect_for))
            .env_remove("XDG_RUNTIME_DIR")
            .env_remove("DBUS_SESSION_BUS_ADDRESS")
            .env("GIT_AUTHOR_NAME", "conductor")
            .env("GIT_AUTHOR_EMAIL", "conductor@example-org.example")
            .env("GIT_COMMITTER_NAME", "conductor")
            .env("GIT_COMMITTER_EMAIL", "conductor@example-org.example")
            .stdin(Stdio::null());
        command
    }

    fn git_in_records(&self, args: &[&str]) -> Output {
        Command::new(git_at())
            .arg("-C")
            .arg(self.records())
            .args(args)
            .env("HOME", self.home())
            .output()
            .expect("run git")
    }
}

/// A `daemon serve` of a case, killed when dropped, with what it printed.
struct Served {
    child: Child,
    printed: Arc<Mutex<String>>,
}

impl Served {
    /// Starts `daemon serve` of `case` with its standard input closed, `env` set, and returns
    /// at once.
    fn spawn(case: &Case, inspect_for: u32, env: &[(&str, &str)]) -> Self {
        let mut command = case.conductor(inspect_for);
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

    /// [`Served::spawn`], then waits for its serving line.
    fn start(case: &Case, env: &[(&str, &str)]) -> Self {
        let mut served = Self::spawn(case, 0, env);
        served.until("the serving line", |served| {
            served.printed().contains("conductord: config ")
        });
        served
    }

    fn printed(&self) -> String {
        self.printed
            .lock()
            .map(|printed| printed.clone())
            .unwrap_or_default()
    }

    fn until(&mut self, what: &str, holds: impl Fn(&Self) -> bool) {
        let deadline = Instant::now() + BOUND;
        while !holds(self) {
            assert!(
                matches!(self.child.try_wait(), Ok(None)),
                "the daemon exited: {}",
                self.printed()
            );
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

/// One `GET target` to `address:port` with `host` as its `Host`, read within `wait`: its
/// status and body, or why there is none.
fn ask(
    address: IpAddr,
    port: u16,
    target: &str,
    host: &str,
    wait: Duration,
) -> Result<(u16, String), String> {
    let mut stream =
        TcpStream::connect((address, port)).map_err(|error| format!("connect: {error}"))?;
    stream
        .set_read_timeout(Some(wait))
        .map_err(|error| format!("timeout: {error}"))?;
    stream
        .write_all(
            format!("GET {target} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .map_err(|error| format!("write: {error}"))?;
    let mut answer = Vec::new();
    stream
        .read_to_end(&mut answer)
        .map_err(|error| format!("no answer within {wait:?}: {error}"))?;
    let answer = String::from_utf8_lossy(&answer).into_owned();
    let (head, body) = answer
        .split_once("\r\n\r\n")
        .ok_or_else(|| format!("no head: {answer:?}"))?;
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| format!("no status: {head:?}"))?;
    Ok((status, body.to_owned()))
}

/// The CLI's help (`token reveal --model-token`: "the model_token of an instance whose placement
/// opens no box") and `spec/domains/token.yaml` (`RevealToken.model_token`: "the token an agent
/// at a placement with no box runs on") name the instances it serves. At a placement that opens
/// a box the model token is the box's, sealed through a slot; the reveal prints it to any
/// process that has the config file.
#[test]
fn adv_reveal_model_token_refuses_an_instance_whose_placement_opens_a_box() {
    let case = Case::new("reveal-boxed");
    fs::write(
        case.secrets().join("alpha-model"),
        format!("{MODEL_VALUE}\n"),
    )
    .expect("mount the model token");
    let reveal = |case: &Case| {
        case.conductor(0)
            .args(["token", "reveal", "--model-token", "alpha"])
            .output()
            .expect("run token reveal")
    };

    // At a placement that opens no box the reveal prints the value: the case reaches it.
    case.config(free_port(), "", "none");
    let unboxed = reveal(&case);
    assert_eq!(unboxed.status.code(), Some(0), "{}", said(&unboxed));
    assert_eq!(
        String::from_utf8_lossy(&unboxed.stdout),
        format!("{MODEL_VALUE}\n")
    );

    // At a placement that opens a box it is the box's token, and nothing prints it.
    case.config(free_port(), "", "systemd");
    let boxed = reveal(&case);
    assert!(
        !String::from_utf8_lossy(&boxed.stdout).contains(MODEL_VALUE),
        "the model token of a boxed instance was printed: {}",
        said(&boxed).replace(MODEL_VALUE, "<the value>")
    );
    assert_eq!(
        boxed.status.code(),
        Some(1),
        "{}",
        said(&boxed).replace(MODEL_VALUE, "<the value>")
    );
}

/// `daemon.address` is "an IP address" (`spec/domains/config.yaml`); `127.0.0.2` is a loopback
/// address, served as the loopback listener. A client of that address names it as its `Host`,
/// and the loopback listener answers only `127.0.0.1` and `localhost`: the probes answer 403.
#[test]
fn adv_a_loopback_address_other_than_127_0_0_1_answers_its_own_probes() {
    let case = Case::new("address-loopback");
    let port = free_port();
    case.config(port, ", address: 127.0.0.2", "none");
    let served = Served::start(&case, &[]);
    let address: IpAddr = "127.0.0.2".parse().expect("an address");
    let answer = ask(
        address,
        port,
        "/healthz",
        &format!("127.0.0.2:{port}"),
        Duration::from_secs(30),
    );
    assert!(
        matches!(answer, Ok((200, _))),
        "GET /healthz on 127.0.0.2:{port} with its own Host: {answer:?}\n{}",
        served.printed()
    );
}

/// The unattended start "writes the records skeleton and commits it", each step "only where it
/// is missing" (`website/docs/guides/run-in-a-container.md`, `daemon/unattended.rs`). A first
/// start whose commit fails (here: no git identity) leaves the skeleton written and
/// uncommitted; the records step of every later start sees `NORTHSTAR.md` and does nothing, so
/// the commit is never made and no later start says so.
#[test]
fn adv_a_records_commit_that_failed_at_the_first_start_is_made_at_the_next() {
    let case = Case::new("records-commit");
    case.config(free_port(), ", unattended: true", "none");

    let no_identity = [("GIT_AUTHOR_NAME", ""), ("GIT_COMMITTER_NAME", "")];
    let first = Served::start(&case, &no_identity);
    let first_printed = first.printed();
    drop(first);
    assert!(
        case.records().join("NORTHSTAR.md").is_file(),
        "the first start wrote no skeleton: {first_printed}"
    );
    let head = case.git_in_records(&["rev-parse", "--verify", "HEAD"]);
    assert!(
        !head.status.success(),
        "the case means a first start whose commit failed, and it committed: {first_printed}"
    );

    let second = Served::start(&case, &[]);
    let second_printed = second.printed();
    drop(second);
    let count = case.git_in_records(&["rev-list", "--count", "HEAD"]);
    assert_eq!(
        String::from_utf8_lossy(&count.stdout).trim(),
        "1",
        "after a second start with a git identity the skeleton is still not committed: {}\n\
         first start:\n{first_printed}\nsecond start:\n{second_printed}",
        said(&count)
    );
}

/// The daemon serves before its store answers (`daemon/mod.rs`, `story:daemon-probes-and-
/// metrics`): "the listeners take requests as soon as the config is read, `GET /healthz`
/// answers `200`". With `daemon.unattended: true` the listener is bound, then the start seeds
/// (records, trust, `worktree inspect` and `activate`, each bounded at 10 s or 20 s), and only
/// then is the listener served: a probe sent while the start seeds is not answered.
#[test]
fn adv_healthz_answers_while_the_unattended_start_seeds() {
    let case = Case::new("seed-probe");
    case.checkout("widget");
    let port = free_port();
    case.config(port, ", unattended: true", "none");
    let mut served = Served::spawn(&case, 8, &[]);
    let asked = case.dir.join("inspect-asked");
    served.until("the start asks worktree inspect", |_| asked.exists());
    let loopback: IpAddr = "127.0.0.1".parse().expect("an address");
    let host = format!("127.0.0.1:{port}");
    let during = ask(loopback, port, "/healthz", &host, Duration::from_secs(3));
    served.until("the serving line", |served| {
        served.printed().contains("conductord: config ")
    });
    let after = ask(loopback, port, "/healthz", &host, Duration::from_secs(30));
    assert!(
        matches!(after, Ok((200, _))),
        "the probe the case sends is not answered 200 once the daemon serves: {after:?}\n{}",
        served.printed()
    );
    assert!(
        matches!(during, Ok((200, _))),
        "GET /healthz sent while the unattended start seeds: {during:?}\n{}",
        served.printed()
    );
}

/// How many commits the records repository of `case` holds, on any branch.
fn commits_anywhere(case: &Case) -> String {
    let count = case.git_in_records(&["rev-list", "--all", "--count"]);
    assert!(count.status.success(), "{}", said(&count));
    String::from_utf8_lossy(&count.stdout).trim().to_owned()
}

/// The correction's rule (`daemon/unattended.rs`, `init::commit_lying`): the start commits the
/// skeleton "where `records` is a git repository of its own with no commit yet", and "a
/// repository that has a commit is left alone, whatever its work tree holds". `commit_lying`
/// asks whether `HEAD` resolves, not whether the repository holds a commit: in a records
/// repository with a commit whose `HEAD` is on a branch that has none yet, as after `git checkout
/// --orphan`, the start commits the skeleton onto that branch, an edit of the operator's with it.
#[test]
fn adv2_a_records_repository_with_a_commit_is_left_alone_while_its_head_has_none() {
    let case = Case::new("records-orphan");
    case.config(free_port(), ", unattended: true", "none");
    let first = Served::start(&case, &[]);
    let first_printed = first.printed();
    drop(first);
    assert_eq!(
        commits_anywhere(&case),
        "1",
        "the case means a first start that committed the skeleton: {first_printed}"
    );

    let orphan = case.git_in_records(&["checkout", "--quiet", "--orphan", "draft"]);
    assert!(orphan.status.success(), "{}", said(&orphan));
    let rules = case.records().join("rules.md");
    let mut edited = fs::read_to_string(&rules).expect("rules.md");
    edited.push_str("\nAn example-org rule, not committed yet.\n");
    fs::write(&rules, &edited).expect("edit rules.md");

    let second = Served::start(&case, &[]);
    let second_printed = second.printed();
    drop(second);
    let draft = case.git_in_records(&["rev-parse", "--verify", "--quiet", "refs/heads/draft"]);
    let commits = commits_anywhere(&case);
    assert!(
        !draft.status.success() && commits == "1",
        "a start committed into a records repository that has a commit: {commits} commits, \
         branch draft at {:?}\n{second_printed}",
        String::from_utf8_lossy(&draft.stdout).trim()
    );
    assert_eq!(
        fs::read_to_string(&rules).expect("rules.md"),
        edited,
        "{second_printed}"
    );
}

/// The correction (`init::lying`) commits "the skeleton files that are there, plus every file in
/// `docs/handoff/`", added by name. `git add` refuses a path that git ignores, so one such file
/// in `docs/handoff/` (here an editor's backup of the hand-over, ignored by the repository's own
/// `info/exclude`) fails the commit at every start: the skeleton the first start wrote is never
/// committed, though git has an identity and every skeleton file can be.
#[test]
fn adv2_a_file_git_ignores_in_the_hand_over_directory_does_not_keep_the_records_uncommitted() {
    let case = Case::new("records-ignored");
    case.config(free_port(), ", unattended: true", "none");
    let no_identity = [("GIT_AUTHOR_NAME", ""), ("GIT_COMMITTER_NAME", "")];
    let first = Served::start(&case, &no_identity);
    let first_printed = first.printed();
    drop(first);
    let head = case.git_in_records(&["rev-parse", "--verify", "--quiet", "HEAD"]);
    assert!(
        !head.status.success(),
        "the case means a first start whose commit failed, and it committed: {first_printed}"
    );
    let handoff = case.records().join("docs/handoff");
    let written: Vec<PathBuf> = fs::read_dir(&handoff)
        .expect("docs/handoff")
        .flatten()
        .map(|entry| entry.path())
        .collect();
    assert_eq!(written.len(), 1, "{written:?}\n{first_printed}");
    let mut backup = written[0].clone().into_os_string();
    backup.push("~");
    fs::write(&backup, "an editor's backup of the hand-over\n").expect("write the backup");
    let info = case.records().join(".git/info");
    fs::create_dir_all(&info).expect("the repository's info directory");
    fs::write(info.join("exclude"), "*~\n").expect("ignore editor backups");

    let second = Served::start(&case, &[]);
    let second_printed = second.printed();
    drop(second);
    let count = case.git_in_records(&["rev-list", "--count", "HEAD"]);
    assert_eq!(
        String::from_utf8_lossy(&count.stdout).trim(),
        "1",
        "a start with a git identity left the skeleton uncommitted: {}\nsecond start:\n\
         {second_printed}",
        said(&count)
    );
}

/// Why the check workflow `text` would not run its `container` job on every pull request and
/// every push to `main`, at once, on a runner the `check` job runs on, within a bound; none when
/// it would.
fn container_job_unheld(text: &str) -> Option<String> {
    const FILTERS: [&str; 5] = [
        "branches",
        "branches-ignore",
        "paths",
        "paths-ignore",
        "types",
    ];
    let workflow: serde_yaml::Value = match serde_yaml::from_str(text) {
        Ok(workflow) => workflow,
        Err(error) => return Some(format!("the workflow is no YAML: {error}")),
    };
    let on = &workflow["on"];
    let Some(pull) = on.get("pull_request") else {
        return Some(format!("the workflow runs on no pull request: on: {on:?}"));
    };
    if let Some(key) = FILTERS.into_iter().find(|key| pull.get(*key).is_some()) {
        return Some(format!("its pull requests are filtered by `{key}:`"));
    }
    let push = &on["push"];
    let to_main = push
        .get("branches")
        .and_then(serde_yaml::Value::as_sequence)
        .is_some_and(|branches| {
            branches
                .iter()
                .any(|branch| branch.as_str() == Some("main"))
        });
    if !to_main {
        return Some(format!("the workflow runs on no push to main: on: {on:?}"));
    }
    if let Some(key) = ["paths", "paths-ignore"]
        .into_iter()
        .find(|key| push.get(*key).is_some())
    {
        return Some(format!("its pushes are filtered by `{key}:`"));
    }
    let job = &workflow["jobs"]["container"];
    if let Some(needs) = job.get("needs") {
        return Some(format!("the job waits on other jobs: needs: {needs:?}"));
    }
    let runner = &workflow["jobs"]["check"]["runs-on"];
    if runner.is_null() || job.get("runs-on") != Some(runner) {
        return Some(format!(
            "the job runs on {:?}, not on the check job's runner {runner:?}",
            job.get("runs-on")
        ));
    }
    match job
        .get("timeout-minutes")
        .and_then(serde_yaml::Value::as_u64)
    {
        Some(minutes) if minutes > 0 => None,
        other => Some(format!("the job's timeout-minutes is {other:?}")),
    }
}

/// "A test runs the start in a container without a terminal" (the story's acceptance), as the
/// check workflow's `container` job. `the_check_workflow_s_container_job_runs_the_image_case`
/// reads the job's steps and its `if:`, but not the workflow's events nor the job's `needs:`,
/// `runs-on:` or `timeout-minutes:`: a job that waits on a job that is skipped, that runs on no
/// pull request, or on a runner no one has, never runs, and that case stays green. This case
/// holds the tree's workflow to them, and shows each such change of it refused.
#[test]
fn adv2_the_check_workflow_runs_the_container_job_on_every_pull_request() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = fs::read_to_string(workspace.join(".github/workflows/check.yml"))
        .expect("read the check workflow");
    assert_eq!(container_job_unheld(&text), None, "the check workflow");

    let job = "  container:\n";
    let head = "    name: container start\n    runs-on: ubuntu-latest\n    timeout-minutes: 45\n";
    for anchor in [job, head, "  pull_request:\n", "branches: [main]"] {
        assert_eq!(text.matches(anchor).count(), 1, "{anchor:?} in check.yml");
    }
    let skipped = format!(
        "  gate:\n    if: false\n    runs-on: ubuntu-latest\n    steps:\n      - run: \"true\"\n{job}"
    );
    let mutants = [
        (
            "needs a job that is skipped",
            text.replace(job, &skipped)
                .replace(head, &format!("{head}    needs: gate\n")),
        ),
        (
            "runs on no pull request",
            text.replace("  pull_request:\n", ""),
        ),
        (
            "runs on pull requests of docs only",
            text.replace(
                "  pull_request:\n",
                "  pull_request:\n    paths: [docs/**]\n",
            ),
        ),
        (
            "runs on no push to main",
            text.replace("branches: [main]", "branches: [release]"),
        ),
        (
            "runs on a runner no one has",
            text.replace(
                head,
                "    name: container start\n    runs-on: no-such-runner\n    timeout-minutes: 45\n",
            ),
        ),
        (
            "times out at once",
            text.replace(
                head,
                "    name: container start\n    runs-on: ubuntu-latest\n    timeout-minutes: 0\n",
            ),
        ),
    ];
    for (what, mutant) in &mutants {
        assert_ne!(mutant, &text, "{what}: the change applies");
        assert!(
            container_job_unheld(mutant).is_some(),
            "a check workflow whose container job {what} is taken"
        );
    }
}
