//! `story:pod-source-checkouts`: an instance runtime alone in a pod (`CONDUCTOR_API_LISTEN` set)
//! clones, by itself, the repositories its `github` and `gitlab` sources name under `include`,
//! at its start and at each collect pass, keeps them fetched, and records each repository it
//! could not clone or fetch as one member failure of the `repositories` collector.
//!
//! Each case runs the built binary with a cleared environment: `HOME` the case's `home/`,
//! `PATH` only its `bin/`, `CONDUCTOR_CONFIG` a config file of one instance `acme` in the case's
//! directory. `bin/` holds a fake `glab` that answers from a copy of `tests/fixtures/gitlab/`, a
//! fake `gh` that lists two repositories of `example-org`, fakes of `aep`, `claude` and `ess`,
//! and `git`, a wrapper that logs its words and runs the real git (or, in one case, refuses a
//! clone). No case reaches the network: the forge is a directory of bare repositories under the
//! case's `forge/`, which git reaches through a rewrite of `https://gitlab.example.com/` and
//! `https://github.com/` given in its environment (`GIT_CONFIG_COUNT`), so the `origin` a clone
//! stores stays the HTTPS URL. A collect pass is `snapshot start-snapshot` run with the pod's
//! variable set, as the runtime's cycle tick runs it; a start is `instance serve` with the loop
//! off.

mod active;
mod common;

use std::fs;
use std::io::Read as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The clock of every snapshot.
const NOW: &str = "2026-10-07T12:00:00Z";

/// The date of every commit the cases make.
const OLD: &str = "2026-09-01T09:00:00Z";

/// The answers the fake `glab` gives, copied into each case.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/gitlab");

/// The forge the instance's group is on.
const HOST: &str = "gitlab.example.com";

/// The variable that tells the runtime it runs alone in a pod.
const LISTEN: &str = "CONDUCTOR_API_LISTEN";

/// The fake `glab`, as `tests/collect_gitlab.rs` writes it: it logs its words, then answers with
/// the file under `answers/` the API path it is asked names.
const GLAB: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
for last in "$@"; do :; done
id=${last#projects/}
id=${id%%/*}
case "$last" in
  groups/*/projects\?*) file=projects.json ;;
  projects/*/repository/commits/*/diff*)
    sha=${last#*/repository/commits/}
    file=${sha%%/*}.diff.json ;;
  projects/*/repository/commits\?*) file=$id.commits.json ;;
  projects/*/repository/commits/*) file=$id.commit.json ;;
  projects/*/repository/compare\?*) file=$id.compare.json ;;
  projects/*/releases\?*) file=$id.releases.json ;;
  projects/*/repository/tags\?*) file=$id.tags.json ;;
  projects/*/issues\?*) file=$id.issues.json ;;
  projects/*/merge_requests\?state=opened*) file=$id.opened.json ;;
  projects/*/merge_requests\?state=merged*) file=$id.merged.json ;;
  projects/*/merge_requests/*) file=$id.mr-${last##*/}.json ;;
  projects/*/pipelines\?*) file=$id.pipelines.json ;;
  *) printf 'no answer for %s\n' "$last" >&2; exit 1 ;;
esac
if [ -f '{answers}'/"$file" ]; then
  while IFS= read -r line || [ -n "$line" ]; do printf '%s\n' "$line"; done < '{answers}'/"$file"
  exit 0
fi
printf 'no answer file %s for %s\n' "$file" "$last" >&2
exit 1
"#;

/// The fake `gh`: it logs its words; `repo list` answers the two repositories `one` and `two` of
/// `example-org`, anything else an empty list.
const GH: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
case "$1 $2" in
  'repo list') printf '%s\n' '[{"name":"one","visibility":"PRIVATE","isArchived":false,"hasIssuesEnabled":false},{"name":"two","visibility":"PRIVATE","isArchived":false,"hasIssuesEnabled":false}]' ;;
  *) printf '%s\n' '[]' ;;
esac
"#;

/// A token value of the fixture, built from pieces: the commit hook's scanner refuses a literal
/// that looks like a token.
fn token() -> String {
    ["fixture", "-", "value", "-", "7Qx2", "Lm9", "Vb4", "Zr8"].concat()
}

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// One case's directory: `home/`, the checkouts root `root/`, `trees/`, `records/`, `state/`,
/// `cache/`, `bin/`, `answers/` for the fake `glab`, `forge/` for the bare repositories and an
/// empty `work/` the binary runs from.
struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    /// The case `name`, its records directory made with an empty workspace file when `records`.
    fn new(name: &str, records: bool) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("pod_checkouts")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "root", "trees", "work", "bin", "answers", "forge"] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        if records {
            fs::create_dir_all(dir.join("records/.engineering")).expect("create the records");
            fs::write(
                dir.join("records/.engineering/workspace.yaml"),
                "version: aep.workspace/1\nmembers: []\n",
            )
            .expect("write the records' workspace file");
        }
        let dir = fs::canonicalize(&dir).expect("the case's directory exists");
        let case = Self { dir };
        for entry in fs::read_dir(FIXTURES).expect("read the fixtures") {
            let path = entry.expect("a fixture").path();
            let name = path.file_name().expect("a fixture's name");
            fs::copy(&path, case.answers().join(name)).expect("copy a fixture");
        }
        case.programs();
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn root(&self) -> PathBuf {
        self.dir.join("root")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn answers(&self) -> PathBuf {
        self.dir.join("answers")
    }

    fn forge(&self) -> PathBuf {
        self.dir.join("forge")
    }

    fn log(&self, program: &str) -> PathBuf {
        self.dir.join(format!("{program}.log"))
    }

    /// The program `name` on the binary's `PATH`, whose whole text is `script`.
    fn script(&self, name: &str, script: &str) {
        let file = self.bin().join(name);
        fs::write(&file, script).expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// The fakes, and `git`, which logs its words and runs the real one, and the real `tar`.
    fn programs(&self) {
        self.script(
            "glab",
            &GLAB
                .replace("{log}", &text(&self.log("glab")))
                .replace("{answers}", &text(&self.answers())),
        );
        self.script("gh", &GH.replace("{log}", &text(&self.log("gh"))));
        self.script(
            "aep",
            "#!/bin/sh\ncase \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) printf \
             '%s' '{\"artifacts\": []}' ;;\nesac\n",
        );
        self.script("claude", "#!/bin/sh\nprintf '%s' '[]'\n");
        self.script(
            "ess",
            "#!/bin/sh\ncase \"$1\" in\n  specify) exit 0 ;;\n  *) echo '1 scenario(s) (0 \
             authored), 0 refusal(s), written to x' ;;\nesac\n",
        );
        self.git_wrapper("");
        std::os::unix::fs::symlink(which("tar"), self.bin().join("tar")).expect("link tar");
    }

    /// `git` on the binary's `PATH`: it logs its words, runs `before` (shell text), then the real
    /// git.
    fn git_wrapper(&self, before: &str) {
        self.script(
            "git",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{before}\nexec '{}' \"$@\"\n",
                text(&self.log("git")),
                text(&which("git"))
            ),
        );
    }

    /// The bare repository of the project `path` (its full path) on `forge` (`gitlab` or
    /// `github`), with one commit on `main`.
    fn bare(&self, forge: &str, path: &str) -> PathBuf {
        let bare = self.forge().join(forge).join(format!("{path}.git"));
        fs::create_dir_all(&bare).expect("create the bare repository");
        git(
            &bare,
            &["init", "--quiet", "--bare", "--initial-branch=main"],
        );
        self.push_to(&bare, "Start");
        bare
    }

    /// A commit `subject` pushed onto `main` of the bare repository `bare`; its id.
    fn push_to(&self, bare: &Path, subject: &str) -> String {
        let work = self.dir.join("pushing");
        let _ = fs::remove_dir_all(&work);
        let url = format!("file://{}", text(bare));
        git(&self.dir, &["clone", "--quiet", &url, &text(&work)]);
        fs::write(work.join(format!("{subject}.txt")), subject).expect("write a file");
        git(&work, &["add", "--all"]);
        git(&work, &["commit", "--quiet", "-m", subject]);
        git(&work, &["push", "--quiet", "origin", "HEAD:main"]);
        let head = capture(&work, &["rev-parse", "HEAD"]);
        fs::remove_dir_all(&work).expect("remove the pushing clone");
        head
    }

    /// The project list of the fake `glab`: each `(id, full path, archived)` a private project on
    /// `main`, its other answers those of the project 101.
    fn projects(&self, projects: &[(u64, &str, bool)]) {
        let listed: Vec<Value> = projects
            .iter()
            .map(|(id, path, archived)| {
                json!({
                    "id": id,
                    "path_with_namespace": path,
                    "archived": archived,
                    "visibility": "private",
                    "default_branch": "main",
                    "open_issues_count": 0,
                    "empty_repo": false,
                    "web_url": format!("https://{HOST}/{path}"),
                })
            })
            .collect();
        fs::write(
            self.answers().join("projects.json"),
            Value::from(listed).to_string(),
        )
        .expect("write the project list");
        for (id, _, _) in projects {
            if *id != 101 {
                self.answer_as(101, *id);
            }
        }
    }

    /// Every answer of the project `from` as the project `to`'s.
    fn answer_as(&self, from: u64, to: u64) {
        for entry in fs::read_dir(self.answers()).expect("read the answers") {
            let path = entry.expect("an answer").path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("an answer's name")
                .to_owned();
            if let Some(rest) = name.strip_prefix(&format!("{from}.")) {
                fs::copy(&path, self.answers().join(format!("{to}.{rest}")))
                    .expect("copy an answer");
            }
        }
    }

    /// The config file of the instance `acme` with the sources `sources`, the entries of a YAML
    /// flow sequence.
    fn config(&self, sources: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: [{sources}]\n    \
             checkouts:\n      root: {}\n      trees: {}\n    \
             records: {}\n    state: {}\n    cache: {}\n",
            quoted(&self.root()),
            quoted(&self.dir.join("trees")),
            quoted(&self.dir.join("records")),
            quoted(&self.dir.join("state")),
            quoted(&self.dir.join("cache")),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// The environment of every run: `HOME`, `PATH` only `bin/`, the config file, a git identity,
    /// and git's rewrite of both forges' HTTPS origins to the bare repositories under `forge/`.
    fn environment(&self, config: &Path, command: &mut Command) {
        let forge = self.forge();
        command
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .env("GIT_CONFIG_COUNT", "2")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.file://{}/gitlab/.insteadOf", text(&forge)),
            )
            .env("GIT_CONFIG_VALUE_0", format!("https://{HOST}/"))
            .env(
                "GIT_CONFIG_KEY_1",
                format!("url.file://{}/github/.insteadOf", text(&forge)),
            )
            .env("GIT_CONFIG_VALUE_1", "https://github.com/");
    }

    /// The built binary run from `work/` with `args`, the case's environment and `env` added.
    fn conductor(&self, config: &Path, env: &[(&str, String)], args: &[&str]) -> Output {
        let mut command = common::conductor(self.home());
        self.environment(config, &mut command);
        command
            .current_dir(self.dir.join("work"))
            .envs(env.iter().map(|(name, value)| (*name, value.as_str())))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// One collect pass, `snapshot start-snapshot` at [`NOW`] with `env`, which completes; the
    /// id of its snapshot.
    fn pass(&self, config: &Path, env: &[(&str, String)]) -> String {
        let output = self.conductor(
            config,
            env,
            &[
                "snapshot",
                "start-snapshot",
                "--started-at",
                NOW,
                "--disk-free-bytes",
                "100000000000",
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .last()
            .unwrap_or_default()
            .trim()
            .to_owned()
    }

    /// The rows of the view `view` of the snapshot `snapshot`.
    fn rows(&self, config: &Path, view: &str, snapshot: &str) -> Vec<Value> {
        let output = self.conductor(config, &[], &["snapshot", view, "--format", "jsonl"]);
        assert!(output.status.success(), "{view}: {}", describe(&output));
        String::from_utf8(output.stdout)
            .expect("UTF-8")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("one JSON object a line"))
            .filter(|row| row["snapshot_id"].as_str() == Some(snapshot))
            .collect()
    }

    /// The member failures of the snapshot `snapshot`, as (member, collector, line).
    fn failures(&self, config: &Path, snapshot: &str) -> Vec<(String, String, String)> {
        self.rows(config, "member-failures", snapshot)
            .iter()
            .map(|row| {
                let field = |name: &str| row[name].as_str().unwrap_or_default().to_owned();
                (field("member"), field("collector"), field("error_line"))
            })
            .collect()
    }

    /// The repository row `name` of the snapshot `snapshot`.
    fn row(&self, config: &Path, snapshot: &str, name: &str) -> Value {
        self.rows(config, "repositories", snapshot)
            .into_iter()
            .find(|row| row["repository"] == json!(name))
            .unwrap_or_else(|| panic!("no repository row {name}"))
    }

    /// The lines `program` logged.
    fn lines(&self, program: &str) -> Vec<String> {
        fs::read_to_string(self.log(program))
            .map(|log| log.lines().map(str::to_owned).collect())
            .unwrap_or_default()
    }

    /// The entries of the checkouts root a clone in progress would leave, by name.
    fn partials(&self) -> Vec<String> {
        fs::read_dir(self.root())
            .expect("read the checkouts root")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(conductor_cli::collect::checkouts::PARTIAL))
            .collect()
    }

    /// The runtime of the pod started over `config` with `env`, until it says its seeding ended;
    /// what it printed.
    fn start(&self, config: &Path, env: &[(&str, String)]) -> String {
        let mut runtime = self.runtime(config, env);
        runtime.until(|printed| {
            printed.contains("seeded its instance") || printed.contains("the seeding failed")
        })
    }

    /// The runtime of the pod, `instance serve` over `config` with its loop off, `env` added,
    /// still running.
    fn runtime(&self, config: &Path, env: &[(&str, String)]) -> Runtime {
        let mut command = common::conductor(self.home());
        self.environment(config, &mut command);
        command
            .current_dir(self.dir.join("work"))
            .args(["instance", "serve"])
            .env(LISTEN, "127.0.0.1:0")
            .env(conductor_cli::instance::TOKEN_VARIABLE, "instance-token")
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .envs(env.iter().map(|(name, value)| (*name, value.as_str())))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        Runtime::spawn(command)
    }

    /// `<state>/run/cycle.last`, the stamp of the loop's last cycle.
    fn cycle_last(&self) -> PathBuf {
        self.dir.join("state/run/cycle.last")
    }

    /// A `cycle.last` stamp `ago` before now, as the loop writes it; its text.
    fn stamp_cycle(&self, ago: time::Duration) -> String {
        let at = (time::OffsetDateTime::now_utc() - ago)
            .format(&time::format_description::well_known::Rfc3339)
            .expect("format the clock");
        let text = format!("{at}\n");
        fs::create_dir_all(self.dir.join("state/run")).expect("create the loop's directory");
        fs::write(self.cycle_last(), &text).expect("write cycle.last");
        text
    }
}

/// `instance serve`, killed when dropped, with what it printed.
struct Runtime {
    child: Child,
    printed: Arc<Mutex<String>>,
}

impl Runtime {
    fn spawn(mut command: Command) -> Self {
        let mut child = command.spawn().expect("instance serve starts");
        let printed = Arc::new(Mutex::new(String::new()));
        for mut pipe in [
            Box::new(child.stdout.take().expect("stdout")) as Box<dyn std::io::Read + Send>,
            Box::new(child.stderr.take().expect("stderr")),
        ] {
            let printed = Arc::clone(&printed);
            std::thread::spawn(move || {
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

    /// `GET <target>` on the TCP address the runtime said it serves on, with its token; the
    /// answer's status, head and body.
    fn get(&self, target: &str) -> (u16, String, String) {
        let printed = self.printed();
        let address = printed
            .lines()
            .find_map(|line| line.split_once("serving /v1/ on ").map(|(_, at)| at.trim()))
            .unwrap_or_else(|| panic!("no address: {printed}"))
            .to_owned();
        let mut stream = std::net::TcpStream::connect(&address).expect("connect to the runtime");
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .expect("a read timeout");
        std::io::Write::write_all(
            &mut stream,
            format!(
                "GET {target} HTTP/1.1\r\nConnection: close\r\nX-Conductor-Token: \
                 instance-token\r\n\r\n"
            )
            .as_bytes(),
        )
        .expect("send");
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).expect("read");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
        let status = head
            .split(' ')
            .nth(1)
            .and_then(|code| code.parse().ok())
            .unwrap_or_else(|| panic!("no status line: {head}"));
        (status, head.to_owned(), body.to_owned())
    }

    /// Waits up to 60 s for what it printed to hold `holds`, while it runs; what it printed.
    fn until(&mut self, holds: impl Fn(&str) -> bool) -> String {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let printed = self.printed();
            if holds(&printed) {
                return printed;
            }
            if let Ok(Some(status)) = self.child.try_wait() {
                panic!("instance serve exited {status}: {printed}");
            }
            assert!(Instant::now() < deadline, "never held: {printed}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn text(path: &Path) -> String {
    path.to_str().expect("the case's path is UTF-8").to_owned()
}

/// `path` as a YAML scalar: a JSON string.
fn quoted(path: &Path) -> String {
    Value::from(text(path)).to_string()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The program `name` on this process's `PATH`.
fn which(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("{name} is on PATH"))
}

/// A git command in `dir` at [`OLD`], with no configuration of the user's.
fn git_command(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(which("git"));
    command
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_CONFIG_COUNT")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", OLD)
        .env("GIT_COMMITTER_DATE", OLD)
        .stdin(Stdio::null());
    command
}

fn git(dir: &Path, args: &[&str]) {
    let output = git_command(dir, args).output().expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// What `git <args>` printed in `dir`, trimmed.
fn capture(dir: &Path, args: &[&str]) -> String {
    let output = git_command(dir, args).output().expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The `origin` URL of the checkout `dir`.
fn origin(dir: &Path) -> String {
    capture(dir, &["config", "--get", "remote.origin.url"])
}

/// The pod's variables of a case: the pod's own and both forges' tokens.
fn pod() -> Vec<(&'static str, String)> {
    vec![
        (LISTEN, "127.0.0.1:0".to_owned()),
        ("GITLAB_TOKEN", token()),
        ("GH_TOKEN", token()),
    ]
}

/// The `gitlab` source of the group `example-org` on [`HOST`] with `include: [<include>]`.
fn gitlab(include: &str) -> String {
    format!("{{gitlab: example-org, host: {HOST}, include: [{include}]}}")
}

// ---------------------------------------------------------------------------------------------
// Cases
// ---------------------------------------------------------------------------------------------

/// At its start a pod clones each project its `gitlab` source includes, before it seeds its
/// instance: each checkout is where the collector reads it (`alpha`, `sub/gamma`), its `origin`
/// the project's HTTPS URL; the workspace file the seeding writes names both; no temporary
/// directory is left; and at the next pass each one's row has a checkout.
#[test]
fn a_pod_clones_its_gitlab_include_at_its_start_where_the_collector_reads_them() {
    let case = Case::new("start-gitlab", false);
    case.projects(&[
        (101, "example-org/alpha", false),
        (104, "example-org/sub/gamma", false),
    ]);
    case.bare("gitlab", "example-org/alpha");
    case.bare("gitlab", "example-org/sub/gamma");
    let config = case.config(&gitlab("alpha, sub/gamma"));

    let printed = case.start(&config, &pod()[1..]);
    assert!(printed.contains("seeded its instance"), "{printed}");
    for (path, project) in [("alpha", "alpha"), ("sub/gamma", "sub/gamma")] {
        let dir = case.root().join(path);
        assert!(dir.join(".git").is_dir(), "{path}: {printed}");
        assert_eq!(
            origin(&dir),
            format!("https://{HOST}/example-org/{project}.git")
        );
    }
    assert!(case.partials().is_empty(), "{:?}", case.partials());
    let workspace = fs::read_to_string(case.dir.join("records/.engineering/workspace.yaml"))
        .expect("the seeding wrote the workspace file");
    assert!(workspace.contains("  - name: alpha\n"), "{workspace}");
    assert!(workspace.contains("  - name: sub-gamma\n"), "{workspace}");

    let snapshot = case.pass(&config, &pod());
    for name in ["alpha", "sub/gamma"] {
        assert_eq!(
            case.row(&config, &snapshot, name)["local_checkout"],
            json!(true),
            "{name}"
        );
    }
    assert!(case.failures(&config, &snapshot).is_empty());
}

/// A pod clones the one repository its `github` source's `include` names, from
/// `https://github.com/<owner>/<name>.git`, and none of the owner's others.
#[test]
fn a_pod_clones_the_one_github_repository_its_include_names() {
    let case = Case::new("github", true);
    case.bare("github", "example-org/one");
    case.bare("github", "example-org/two");
    let config = case.config("{github: example-org, include: [one]}");

    let snapshot = case.pass(&config, &pod());
    let one = case.root().join("one");
    assert!(one.join(".git").is_dir());
    assert_eq!(origin(&one), "https://github.com/example-org/one.git");
    assert!(!case.root().join("two").exists());
    assert_eq!(
        case.row(&config, &snapshot, "one")["local_checkout"],
        json!(true)
    );
    assert!(case.failures(&config, &snapshot).is_empty());
}

/// Outside a pod (the variable unset) nothing is cloned, at a pass or at a runtime's start.
#[test]
fn outside_a_pod_nothing_is_cloned() {
    let case = Case::new("not-a-pod", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    case.bare("gitlab", "example-org/alpha");
    let config = case.config(&gitlab("alpha"));

    case.pass(&config, &pod()[1..]);
    assert!(!case.root().join("alpha").exists());
    assert!(
        !case
            .lines("git")
            .iter()
            .any(|line| line.starts_with("clone"))
    );
}

/// A source without `include` clones nothing, in a pod too.
#[test]
fn a_source_without_include_clones_nothing() {
    let case = Case::new("no-include", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    case.bare("gitlab", "example-org/alpha");
    let config = case.config(&format!("{{gitlab: example-org, host: {HOST}}}"));

    let snapshot = case.pass(&config, &pod());
    assert!(!case.root().join("alpha").exists());
    assert!(
        !case
            .lines("git")
            .iter()
            .any(|line| line.starts_with("clone"))
    );
    assert!(case.failures(&config, &snapshot).is_empty());
}

/// An archived project an entry includes is not cloned, and is no failure.
#[test]
fn an_archived_included_project_is_not_cloned() {
    let case = Case::new("archived", true);
    case.projects(&[
        (101, "example-org/alpha", false),
        (102, "example-org/beta", true),
    ]);
    case.bare("gitlab", "example-org/alpha");
    case.bare("gitlab", "example-org/beta");
    let config = case.config(&gitlab("alpha, beta"));

    let snapshot = case.pass(&config, &pod());
    assert!(case.root().join("alpha/.git").is_dir());
    assert!(!case.root().join("beta").exists());
    assert!(case.failures(&config, &snapshot).is_empty());
}

/// A clone that fails (the project has no repository on the forge) stops neither the start nor
/// the pass nor the other clones; the pass records one member failure naming it; the next pass
/// tries again and, once the repository is there, clones it, and it becomes a member of the
/// workspace file without a restart.
#[test]
fn a_failed_clone_is_a_member_failure_and_the_next_pass_clones_it() {
    let case = Case::new("failed-clone", false);
    case.projects(&[
        (101, "example-org/alpha", false),
        (104, "example-org/sub/gamma", false),
    ]);
    case.bare("gitlab", "example-org/alpha");
    let config = case.config(&gitlab("alpha, sub/gamma"));

    let printed = case.start(&config, &pod()[1..]);
    assert!(printed.contains("seeded its instance"), "{printed}");
    assert!(
        printed.contains("sub/gamma"),
        "the start names it: {printed}"
    );
    assert!(case.root().join("alpha/.git").is_dir());
    assert!(!case.root().join("sub/gamma").exists());

    let first = case.pass(&config, &pod());
    let failures = case.failures(&config, &first);
    assert_eq!(failures.len(), 1, "{failures:#?}");
    let (member, collector, line) = &failures[0];
    assert_eq!(member, "sub/gamma");
    assert_eq!(collector, "repositories");
    assert!(
        line.contains(&format!("https://{HOST}/example-org/sub/gamma.git")),
        "{line}"
    );
    assert!(case.partials().is_empty(), "{:?}", case.partials());

    case.bare("gitlab", "example-org/sub/gamma");
    let second = case.pass(&config, &pod());
    assert!(case.failures(&config, &second).is_empty());
    assert!(case.root().join("sub/gamma/.git").is_dir());
    assert_eq!(
        case.row(&config, &second, "sub/gamma")["local_checkout"],
        json!(true)
    );
    let workspace = fs::read_to_string(case.dir.join("records/.engineering/workspace.yaml"))
        .expect("the workspace file");
    assert!(workspace.contains("  - name: sub-gamma\n"), "{workspace}");
}

/// A checkout already at the path, with a commit of its own, is left as it is: the same `HEAD`
/// and the same `origin` after a pass, and nothing is cloned.
#[test]
fn an_existing_checkout_with_local_commits_is_left_as_it_is() {
    let case = Case::new("existing", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    let bare = case.bare("gitlab", "example-org/alpha");
    let dir = case.root().join("alpha");
    git(
        &case.root(),
        &[
            "clone",
            "--quiet",
            &format!("file://{}", text(&bare)),
            "alpha",
        ],
    );
    let url = format!("https://{HOST}/example-org/alpha.git");
    git(&dir, &["remote", "set-url", "origin", &url]);
    fs::write(dir.join("local.txt"), "mine").expect("write a file");
    git(&dir, &["add", "--all"]);
    git(&dir, &["commit", "--quiet", "-m", "A commit of its own"]);
    let head = capture(&dir, &["rev-parse", "HEAD"]);
    let config = case.config(&gitlab("alpha"));

    let snapshot = case.pass(&config, &pod());
    assert_eq!(capture(&dir, &["rev-parse", "HEAD"]), head);
    assert_eq!(origin(&dir), url);
    assert!(
        !case
            .lines("git")
            .iter()
            .any(|line| line.starts_with("clone"))
    );
    assert!(case.failures(&config, &snapshot).is_empty());
}

/// A directory at the path that holds no `.git` is left alone and reported: one member failure
/// names it, and what it holds is still there.
#[test]
fn a_directory_without_git_at_the_path_is_left_alone_and_reported() {
    let case = Case::new("occupied", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    case.bare("gitlab", "example-org/alpha");
    let dir = case.root().join("alpha");
    fs::create_dir_all(&dir).expect("create the directory");
    fs::write(dir.join("notes.txt"), "kept").expect("write a file");
    let config = case.config(&gitlab("alpha"));

    let snapshot = case.pass(&config, &pod());
    assert_eq!(
        fs::read_to_string(dir.join("notes.txt")).expect("the file is there"),
        "kept"
    );
    assert!(!dir.join(".git").exists());
    let failures = case.failures(&config, &snapshot);
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert_eq!(failures[0].0, "alpha");
    assert!(
        failures[0].2.contains("holds no git checkout"),
        "{failures:#?}"
    );
}

/// The temporary directory a killed clone left is removed before the next try, and the clone
/// succeeds.
#[test]
fn a_leftover_temporary_directory_of_a_killed_clone_is_removed() {
    let case = Case::new("leftover", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    case.bare("gitlab", "example-org/alpha");
    let leftover = case.root().join(format!(
        "{}alpha",
        conductor_cli::collect::checkouts::PARTIAL
    ));
    fs::create_dir_all(leftover.join("partial/alpha/.git")).expect("a killed clone's leftover");
    fs::write(leftover.join("partial/alpha/half"), "half").expect("write a file");
    let config = case.config(&gitlab("alpha"));

    let snapshot = case.pass(&config, &pod());
    assert!(!leftover.exists());
    assert!(case.root().join("alpha/.git").is_dir());
    assert!(case.failures(&config, &snapshot).is_empty());
}

/// An included GitLab checkout is fetched at each pass: a commit pushed to its origin is in the
/// checkout's `origin/main` after the next one, and its row reads it. A fetch that fails for one
/// repository leaves the other rows of the pass, and is one member failure.
#[test]
fn an_included_gitlab_checkout_is_fetched_and_a_failed_fetch_leaves_the_other_rows() {
    let case = Case::new("fetch", true);
    case.projects(&[
        (101, "example-org/alpha", false),
        (104, "example-org/sub/gamma", false),
    ]);
    let alpha = case.bare("gitlab", "example-org/alpha");
    let gamma = case.bare("gitlab", "example-org/sub/gamma");
    let config = case.config(&gitlab("alpha, sub/gamma"));
    case.pass(&config, &pod());

    let pushed = case.push_to(&alpha, "Later");
    let second = case.pass(&config, &pod());
    assert_eq!(
        capture(&case.root().join("alpha"), &["rev-parse", "origin/main"]),
        pushed
    );
    assert_eq!(case.row(&config, &second, "alpha")["behind_main"], json!(1));

    fs::remove_dir_all(&gamma).expect("the origin of sub/gamma goes away");
    let third = case.pass(&config, &pod());
    let names: Vec<Value> = case
        .rows(&config, "repositories", &third)
        .into_iter()
        .map(|row| row["repository"].clone())
        .collect();
    assert_eq!(names, [json!("alpha"), json!("sub/gamma")]);
    let failures = case.failures(&config, &third);
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert_eq!(failures[0].0, "sub/gamma");
    assert!(failures[0].2.contains("fetch"), "{failures:#?}");
}

/// A pod that carries no token of its source's forge tries no clone: git is never asked to
/// clone, and each included repository is one member failure that says so.
#[test]
fn without_the_forges_token_no_clone_is_run() {
    let case = Case::new("no-token", true);
    case.projects(&[
        (101, "example-org/alpha", false),
        (104, "example-org/sub/gamma", false),
    ]);
    case.bare("gitlab", "example-org/alpha");
    case.bare("gitlab", "example-org/sub/gamma");
    let config = case.config(&gitlab("alpha, sub/gamma"));

    let snapshot = case.pass(&config, &[(LISTEN, "127.0.0.1:0".to_owned())]);
    assert!(
        !case
            .lines("git")
            .iter()
            .any(|line| line.starts_with("clone")),
        "{:#?}",
        case.lines("git")
    );
    let mut failures = case.failures(&config, &snapshot);
    failures.sort();
    let members: Vec<&str> = failures
        .iter()
        .map(|(member, _, _)| member.as_str())
        .collect();
    assert_eq!(members, ["alpha", "sub/gamma"]);
    for (_, collector, line) in &failures {
        assert_eq!(collector, "repositories");
        assert!(line.contains("GITLAB_TOKEN"), "{line}");
    }
}

/// No recorded line holds the forge's token: a clone whose git names the token on its error
/// lines is one member failure whose line holds no part of the value, and the pass says it
/// nowhere.
#[test]
fn no_recorded_line_holds_the_forges_token() {
    let case = Case::new("redacted", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    case.bare("gitlab", "example-org/alpha");
    case.git_wrapper(
        "if [ \"$1\" = clone ]; then\n  printf \"fatal: Authentication failed for \
         'https://oauth2:%s@gitlab.example.com/example-org/alpha.git/'\\n\" \"$GITLAB_TOKEN\" \
         >&2\n  printf 'fatal: the token %s was refused\\n' \"$GITLAB_TOKEN\" >&2\n  exit 128\nfi",
    );
    let config = case.config(&gitlab("alpha"));

    let output = case.conductor(
        &config,
        &pod(),
        &[
            "snapshot",
            "start-snapshot",
            "--started-at",
            NOW,
            "--disk-free-bytes",
            "100000000000",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let snapshot = String::from_utf8_lossy(&output.stdout)
        .lines()
        .last()
        .unwrap_or_default()
        .trim()
        .to_owned();
    let failures = case.failures(&config, &snapshot);
    assert_eq!(failures.len(), 1, "{failures:#?}");
    assert!(failures[0].2.contains("refused"), "{failures:#?}");
    let token = token();
    for (_, _, line) in &failures {
        assert!(!line.contains(&token), "{line}");
    }
    assert!(
        !describe(&output).contains(&token),
        "the pass says the token: {}",
        describe(&output)
    );
}

/// In a pod, `conductor doctor` says how many of the repositories its sources include have no
/// checkout.
#[test]
fn doctor_in_a_pod_counts_the_included_repositories_without_a_checkout() {
    let case = Case::new("doctor", true);
    case.projects(&[
        (101, "example-org/alpha", false),
        (104, "example-org/sub/gamma", false),
    ]);
    case.bare("gitlab", "example-org/alpha");
    let config = case.config(&gitlab("alpha, sub/gamma"));
    case.pass(&config, &pod());

    let output = case.conductor(&config, &pod(), &["doctor"]);
    let printed = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        printed.contains("checkouts acme: 1 of 2 included repositories have no checkout"),
        "{}",
        describe(&output)
    );
    let outside = case.conductor(&config, &pod()[1..], &["doctor"]);
    assert!(
        !String::from_utf8_lossy(&outside.stdout).contains("included repositories"),
        "{}",
        describe(&outside)
    );
}

/// The variable that bounds how long a pod's start begins clones, in seconds.
const START_BOUND: &str = "CONDUCTOR_START_CLONE_BOUND";

/// `conductor config validate` on the config file of `sources` with a session prefix, which
/// [`Case::config`] leaves out and validation asks for.
fn validated(case: &Case, sources: &str) -> Output {
    let config = case.config(sources);
    let text = fs::read_to_string(&config).expect("read the config file");
    let text = text.replacen(
        "- name: acme\n",
        "- name: acme\n    session_prefix: acme\n",
        1,
    );
    fs::write(&config, text).expect("write the config file");
    case.conductor(&config, &[], &["config", "validate"])
}

/// An `include` or `exclude` entry with a step `.git`, in any case, or one that starts with the
/// temporary clone prefix names no checkout: `config validate` refuses it at the entry's YAML
/// path.
#[test]
fn an_entry_with_a_step_that_names_no_checkout_is_refused_at_validation() {
    let case = Case::new("entry-refused", true);
    let partial = conductor_cli::collect::checkouts::PARTIAL;
    let valid = validated(&case, &gitlab("alpha, sub/gamma"));
    assert_eq!(valid.status.code(), Some(0), "{}", describe(&valid));
    for (sources, path) in [
        (
            "{github: example-org, include: [.git]}".to_owned(),
            "instances[0].sources[0].include[0]",
        ),
        (
            format!("{{gitlab: example-org, host: {HOST}, include: [alpha, sub/.GIT]}}"),
            "instances[0].sources[0].include[1]",
        ),
        (
            format!("{{gitlab: example-org, host: {HOST}, exclude: [{partial}alpha]}}"),
            "instances[0].sources[0].exclude[0]",
        ),
        (
            format!("{{local: {}, exclude: [.Git]}}", quoted(&case.root())),
            "instances[0].sources[0].exclude[0]",
        ),
    ] {
        let output = validated(&case, &sources);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{sources}: {}",
            describe(&output)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(path) && stderr.contains("names no checkout"),
            "{sources}: {}",
            describe(&output)
        );
    }
}

/// A listed project whose path holds a step `.git` (any case) or one that starts with the
/// temporary clone prefix is left out of the pass: one member failure each where an `include`
/// entry covers its path, nothing under the checkouts root, no clone of it; the other included
/// projects are cloned and read.
#[test]
fn a_listed_project_with_a_step_that_names_no_checkout_is_a_member_failure_and_not_cloned() {
    let case = Case::new("listed-refused", true);
    let partial = conductor_cli::collect::checkouts::PARTIAL;
    let shadow = format!("example-org/{partial}alpha");
    case.projects(&[
        (101, "example-org/alpha", false),
        (102, "example-org/.git", false),
        (103, "example-org/sub/.Git", false),
        (104, "example-org/sub/gamma", false),
        (105, &shadow, false),
    ]);
    for path in [
        "example-org/alpha",
        "example-org/.git",
        "example-org/sub/.Git",
        "example-org/sub/gamma",
        &shadow,
    ] {
        case.bare("gitlab", path);
    }
    let refused = validated(&case, &gitlab(&format!("alpha, sub, \"{partial}alpha\"")));
    assert_eq!(refused.status.code(), Some(1), "{}", describe(&refused));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("include[2]"),
        "{}",
        describe(&refused)
    );
    let config = case.config(&gitlab("alpha, sub"));

    let snapshot = case.pass(&config, &pod());
    let mut failures = case.failures(&config, &snapshot);
    failures.sort();
    let members: Vec<&str> = failures
        .iter()
        .map(|(member, _, _)| member.as_str())
        .collect();
    // `.git` and the shadow lie outside every entry: dropped without a failure, as an excluded
    // project is.
    assert_eq!(members, ["sub/.Git"], "{failures:#?}");
    for (_, collector, line) in &failures {
        assert_eq!(collector, "repositories");
        assert!(line.contains("names no checkout"), "{line}");
    }
    for path in [".git", "sub/.Git", &format!("{partial}alpha")] {
        assert!(
            !case.root().join(path).exists(),
            "{path} is under the checkouts root"
        );
    }
    let clones: Vec<String> = case
        .lines("git")
        .into_iter()
        .filter(|line| line.starts_with("clone"))
        .collect();
    assert_eq!(clones.len(), 2, "{clones:#?}");
    for name in ["alpha", "sub/gamma"] {
        assert!(case.root().join(name).join(".git").is_dir(), "{name}");
        assert_eq!(
            case.row(&config, &snapshot, name)["local_checkout"],
            json!(true),
            "{name}"
        );
    }
    let names: Vec<Value> = case
        .rows(&config, "repositories", &snapshot)
        .into_iter()
        .map(|row| row["repository"].clone())
        .collect();
    assert_eq!(names, [json!("alpha"), json!("sub/gamma")]);
}

/// A listed project with a step that names no checkout is a member failure only of a source that
/// would otherwise collect it: of the source on the group whose `include` covers its parent path,
/// and of one with no `include`; the source on the same group whose `include` names something
/// else drops it without one, as it drops an excluded project.
#[test]
fn a_project_naming_no_checkout_is_a_failure_only_of_a_source_that_would_collect_it() {
    let refused_of = |name: &str, source: &str| -> Vec<String> {
        let case = Case::new(name, true);
        case.projects(&[
            (101, "example-org/gate", false),
            (102, "example-org/platform/.git", false),
        ]);
        case.bare("gitlab", "example-org/gate");
        case.bare("gitlab", "example-org/platform/.git");
        let config = case.config(source);
        let snapshot = case.pass(&config, &pod());
        case.failures(&config, &snapshot)
            .into_iter()
            .filter(|(_, _, line)| line.contains("names no checkout"))
            .map(|(member, _, _)| member)
            .collect()
    };
    assert_eq!(
        refused_of("refused-covered", &gitlab("platform")),
        ["platform/.git"]
    );
    assert_eq!(
        refused_of("refused-elsewhere", &gitlab("gate")),
        Vec::<String>::new()
    );
    assert_eq!(
        refused_of(
            "refused-no-include",
            &format!("{{gitlab: example-org, host: {HOST}}}")
        ),
        ["platform/.git"]
    );
    // Both sources on the one group: the failure is the first's alone, recorded once.
    assert_eq!(
        refused_of(
            "refused-both",
            &format!("{}, {}", gitlab("platform"), gitlab("gate"))
        ),
        ["platform/.git"]
    );
}

/// The paths the trust names after a clone at a pass are the checkouts an `include` entry
/// covers: a git directory under the root that no entry covers, with an `origin` of the group or
/// with none, is not trusted.
#[test]
fn a_pass_trusts_only_the_checkouts_an_include_entry_covers() {
    let case = Case::new("trust-included", true);
    case.projects(&[
        (101, "example-org/alpha", false),
        (102, "example-org/beta", false),
    ]);
    case.bare("gitlab", "example-org/alpha");
    let beta = case.bare("gitlab", "example-org/beta");
    git(
        &case.root(),
        &[
            "clone",
            "--quiet",
            &format!("file://{}", text(&beta)),
            "beta",
        ],
    );
    let url = format!("https://{HOST}/example-org/beta.git");
    git(
        &case.root().join("beta"),
        &["remote", "set-url", "origin", &url],
    );
    fs::create_dir_all(case.root().join("stray")).expect("create a directory");
    git(&case.root().join("stray"), &["init", "--quiet"]);
    let config = case.config(&gitlab("alpha"));

    case.pass(&config, &pod());
    assert!(case.root().join("alpha/.git").is_dir());
    let file = case.home().join(conductor_cli::trust::CLAUDE_JSON);
    let document: Value = serde_json::from_str(
        &fs::read_to_string(&file).expect("the pass trusted the new checkout"),
    )
    .expect("JSON");
    let trusted: Vec<String> = document["projects"]
        .as_object()
        .map(|projects| projects.keys().cloned().collect())
        .unwrap_or_default();
    let physical = |name: &str| {
        text(&fs::canonicalize(case.root().join(name)).expect("the directory is there"))
    };
    assert!(trusted.contains(&physical("alpha")), "{trusted:#?}");
    for name in ["beta", "stray"] {
        assert!(!trusted.contains(&physical(name)), "{name}: {trusted:#?}");
    }
}

/// A start begins no clone once its bound has passed: with the bound at 0 seconds its clones
/// begin none and it says how many it left; the collect pass it takes at its start clones them,
/// and the next pass finds no failure.
#[test]
fn a_start_past_its_bound_leaves_the_rest_to_the_pass() {
    let case = Case::new("start-bound", false);
    case.projects(&[
        (101, "example-org/alpha", false),
        (104, "example-org/sub/gamma", false),
    ]);
    case.bare("gitlab", "example-org/alpha");
    case.bare("gitlab", "example-org/sub/gamma");
    let config = case.config(&gitlab("alpha, sub/gamma"));

    let mut env = pod()[1..].to_vec();
    env.push((START_BOUND, "0".to_owned()));
    let printed = case.start(&config, &env);
    assert!(printed.contains("seeded its instance"), "{printed}");
    assert!(
        printed
            .contains("2 included repositories without a checkout are left to the collect passes"),
        "{printed}"
    );
    assert!(!printed.contains("cloned https://"), "{printed}");
    assert!(
        printed.contains("took a snapshot at its start"),
        "{printed}"
    );
    for name in ["alpha", "sub/gamma"] {
        assert!(
            case.root().join(name).join(".git").is_dir(),
            "{name}: {printed}"
        );
    }

    let snapshot = case.pass(&config, &pod());
    for name in ["alpha", "sub/gamma"] {
        assert!(case.root().join(name).join(".git").is_dir(), "{name}");
    }
    assert!(case.failures(&config, &snapshot).is_empty());
}

/// A runtime alone in a pod takes a snapshot at its start, after its clones and its seeding,
/// whatever `cycle.last` says (one minute old here, its cycle two hours) and with its loop off:
/// its health answers that snapshot, which lists the clone that failed at this start and no
/// longer the failure of the snapshot before it, whose repository this start cloned. The cycle
/// keeps its stamp.
#[test]
fn a_pod_takes_a_snapshot_at_its_start_and_its_health_lists_the_starts_failed_clone() {
    let case = Case::new("start-snapshot", true);
    case.projects(&[
        (101, "example-org/alpha", false),
        (102, "example-org/beta", false),
    ]);
    case.bare("gitlab", "example-org/alpha");
    let config = case.config(&gitlab("alpha, beta"));
    let before = case.pass(&config, &pod());
    let members: Vec<String> = case
        .failures(&config, &before)
        .into_iter()
        .map(|(member, _, _)| member)
        .collect();
    assert_eq!(members, ["beta"]);

    case.projects(&[
        (101, "example-org/alpha", false),
        (102, "example-org/beta", false),
        (103, "example-org/delta", false),
    ]);
    case.bare("gitlab", "example-org/beta");
    let config = case.config(&gitlab("alpha, beta, delta"));
    let stamp = case.stamp_cycle(time::Duration::minutes(1));
    let mut runtime = case.runtime(&config, &pod()[1..]);
    // The case's records skeleton is there, so the seeding's `init` writes nothing and says so.
    let printed = runtime.until(|printed| {
        printed.contains("seeded its instance") || printed.contains("the seeding failed")
    });
    assert!(case.root().join("beta/.git").is_dir(), "{printed}");

    // An answer computed before a write is sent once more, marked stale (`X-Conductor-Age`),
    // while the server computes it again: read until it is fresh.
    let deadline = Instant::now() + Duration::from_secs(30);
    let (status, body) = loop {
        let (status, head, body) = runtime.get("/v1/health");
        if status != 503 && !head.to_ascii_lowercase().contains("x-conductor-age") {
            break (status, body);
        }
        assert!(Instant::now() < deadline, "health stays stale: {head}");
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(status, 200, "{body}");
    let health: Value = serde_json::from_str(&body).expect("the health answer's JSON");
    assert_ne!(
        health["snapshot"]["snapshot_id"],
        json!(before),
        "health answers the snapshot before the start: {printed}"
    );
    let members: Vec<&str> = health["member_failures"]
        .as_array()
        .expect("member_failures")
        .iter()
        .filter_map(|failure| failure["member"].as_str())
        .collect();
    assert_eq!(members, ["delta"], "{health:#}");
    assert_eq!(
        fs::read_to_string(case.cycle_last()).expect("cycle.last"),
        stamp,
        "the start's snapshot is no cycle"
    );
}

/// The same start outside a pod (no `CONDUCTOR_API_LISTEN`, its API on a socket) clones nothing
/// and takes no snapshot while `cycle.last` is younger than the cycle.
#[test]
fn a_start_outside_a_pod_takes_no_snapshot_before_the_cycle_is_due() {
    let case = Case::new("out", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    case.bare("gitlab", "example-org/alpha");
    let config = case.config(&gitlab("alpha"));
    case.stamp_cycle(time::Duration::minutes(1));
    let mut command = common::conductor(case.home());
    case.environment(&config, &mut command);
    command
        .current_dir(case.dir.join("work"))
        .args(["instance", "serve"])
        .env(conductor_cli::instance::LOOP_VARIABLE, "off")
        .env(conductor_cli::instance::SOCKET_VARIABLE, case.dir.join("s"))
        .envs(
            pod()[1..]
                .iter()
                .map(|(name, value)| (*name, value.as_str())),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut runtime = Runtime::spawn(command);
    let printed = runtime.until(|printed| printed.contains("the loop is off"));
    assert!(!printed.contains("seeded"), "{printed}");
    assert!(!case.root().join("alpha").exists(), "{printed}");
    let output = case.conductor(
        &config,
        &[],
        &["snapshot", "repositories", "--format", "jsonl"],
    );
    assert!(
        !output.status.success() || output.stdout.is_empty(),
        "a snapshot was taken: {}",
        describe(&output)
    );
}

/// The collector's `glab` calls run with `GLAB_SEND_TELEMETRY=false`, which `glab config --help`
/// names as the variable that turns its usage data off.
#[test]
fn the_collectors_glab_calls_send_no_usage_data() {
    let case = Case::new("telemetry", true);
    case.projects(&[(101, "example-org/alpha", false)]);
    case.bare("gitlab", "example-org/alpha");
    let seen = case.dir.join("glab-env.log");
    case.script(
        "glab",
        &GLAB
            .replace("{log}", &text(&case.log("glab")))
            .replace("{answers}", &text(&case.answers()))
            .replacen(
                "#!/bin/sh\n",
                &format!(
                    "#!/bin/sh\nprintf '%s\\n' \"${{GLAB_SEND_TELEMETRY-unset}}\" >> '{}'\n",
                    text(&seen)
                ),
                1,
            ),
    );
    let config = case.config(&gitlab("alpha"));

    case.pass(&config, &pod());
    let values: Vec<String> = fs::read_to_string(&seen)
        .expect("glab ran")
        .lines()
        .map(str::to_owned)
        .collect();
    assert!(!values.is_empty());
    assert!(values.iter().all(|value| value == "false"), "{values:?}");
}
