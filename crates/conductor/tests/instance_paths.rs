//! Wave 08, W1: the paths come from the instance. With a config file, the state directory is the
//! instance's `state` unless `--state-dir` names another, the dashboard reads its records from the
//! instance's `records` unless `--root` names another, and the watch state from the instance's
//! `<state>/watch/`; the sessions it shows are those under the instance's checkouts root. Without
//! a file, every path is today's.
//!
//! Each case is a directory of its own in this test target's temporary directory: `home/` (the
//! binary's `HOME`, which holds no config file), `work/` (its working directory), an empty `bin/`
//! (the dashboard's `PATH`, so no case runs a real session list, `df` or `worktree`) and the config
//! file `conductor.yaml`, whose one instance names `~/state`, `~/records` and `~/checkouts`. The
//! binary runs with both settings' variables removed, and `CONDUCTOR_CONFIG` naming the file when
//! the case has one, so no case reads the real home's config.

use std::fs::{self, File};
use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use conductor_cli::config::{self, CONFIG_VARIABLE, INSTANCE_VARIABLE};
use serde_json::{Value, json};

/// The instant every decision of these cases is decided at.
const AT: &str = "2026-10-08T09:00:00Z";

/// The id the first decision of [`AT`]'s day gets.
const FIRST: &str = "DEC-20261008-01";

/// One case: `home/`, `work/` and `bin/` under its own directory.
struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("instance_paths")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "work", "bin"] {
            fs::create_dir_all(dir.join(sub)).expect("create a directory of the case");
        }
        Self { dir }
    }

    /// The case's home directory, as `HOME` names it to the binary.
    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    /// The case's working directory.
    fn work(&self) -> PathBuf {
        self.dir.join("work")
    }

    /// Writes `text` to `relative` under the case's directory and answers its path.
    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.dir.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        fs::write(&path, text).expect("write a case file");
        path
    }

    /// The config file: one instance, `acme`, whose state, records and checkouts are under the
    /// case's home.
    fn config(&self) -> PathBuf {
        self.write(
            "conductor.yaml",
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: acme\n\
             \x20   sources: [{github: acme}]\n\
             \x20   checkouts: {root: ~/checkouts, trees: ~/trees}\n\
             \x20   records: ~/records\n\
             \x20   state: ~/state\n",
        )
    }

    /// `conductor <args>` from `cwd`, with `HOME` naming the case's home, both settings'
    /// variables removed, and `CONDUCTOR_CONFIG` naming `config` when there is one.
    fn command(&self, cwd: &Path, config: Option<&Path>) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_conductor"));
        command
            .current_dir(cwd)
            .env("HOME", self.home())
            .env_remove(CONFIG_VARIABLE)
            .env_remove(INSTANCE_VARIABLE)
            .stdin(Stdio::null());
        if let Some(config) = config {
            command.env(CONFIG_VARIABLE, config);
        }
        command
    }

    fn run(&self, cwd: &Path, config: Option<&Path>, args: &[&str]) -> Output {
        self.command(cwd, config)
            .args(args)
            .output()
            .expect("the conductor binary runs")
    }

    /// Runs a command that must succeed, and answers its standard output.
    fn ok(&self, cwd: &Path, config: Option<&Path>, args: &[&str]) -> String {
        let output = self.run(cwd, config, args);
        assert!(
            output.status.success(),
            "`conductor {}` exited {:?}; stderr: {}",
            args.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("stdout is UTF-8")
    }

    /// Records a class C decision from `cwd` and answers the id it printed.
    fn record(&self, cwd: &Path, config: Option<&Path>, more: &[&str]) -> String {
        let mut args: Vec<&str> = more.to_vec();
        args.extend([
            "decision",
            "record-conductor-decision",
            "--decision-class",
            "C",
            "--question",
            "which way?",
            "--options",
            "A: one way; B: the other",
            "--choice",
            "A",
            "--reason",
            "A is reversible",
            "--evidence",
            "the request's own text",
            "--decided-at",
            AT,
        ]);
        self.ok(cwd, config, &args).trim_end().to_owned()
    }

    /// The `decision_id` of each row a `decision decisions --format jsonl` run prints.
    fn decision_ids(&self, cwd: &Path, config: Option<&Path>, more: &[&str]) -> Vec<String> {
        let mut args: Vec<&str> = more.to_vec();
        args.extend(["decision", "decisions", "--format", "jsonl"]);
        self.ok(cwd, config, &args)
            .lines()
            .map(|line| {
                let row: Value = serde_json::from_str(line).expect("a jsonl row");
                row["decision_id"]
                    .as_str()
                    .expect("a decision id")
                    .to_owned()
            })
            .collect()
    }
}

/// The names in `dir`, sorted.
fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read the directory")
        .map(|entry| {
            entry
                .expect("read an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn text(path: &Path) -> &str {
    path.to_str().expect("the case's path is UTF-8")
}

#[test]
fn with_a_config_file_a_decision_lands_in_the_instance_state_and_its_views_read_it_there() {
    let case = Case::new("decision");
    let config = case.config();
    let work = case.work();
    let state = case.home().join("state");

    assert_eq!(case.record(&work, Some(&config), &[]), FIRST);
    assert_eq!(
        entries(&work),
        Vec::<String>::new(),
        "with a config file nothing is kept under the working directory"
    );
    assert!(
        state.is_dir(),
        "the instance's state directory holds the store"
    );
    assert_eq!(
        case.decision_ids(&work, None, &["--state-dir", text(&state)]),
        [FIRST],
        "the store is the one in the instance's state directory"
    );

    assert_eq!(
        case.decision_ids(&work, Some(&config), &[]),
        [FIRST],
        "a view reads the instance's state directory"
    );
    assert_eq!(
        case.ok(
            &work,
            Some(&config),
            &["dispatch", "dispatches", "--format", "jsonl"]
        ),
        "",
        "the dispatch view opens the instance's store, which holds no dispatch"
    );
    assert_eq!(
        entries(&work),
        Vec::<String>::new(),
        "a view creates nothing"
    );
}

#[test]
fn the_state_dir_flag_beats_the_instance_state() {
    let case = Case::new("flag");
    let config = case.config();
    let work = case.work();
    let flag = case.dir.join("flag");

    assert_eq!(
        case.record(&work, Some(&config), &["--state-dir", text(&flag)]),
        FIRST
    );
    assert!(flag.is_dir(), "the flag's directory holds the store");
    assert!(
        !case.home().join("state").exists(),
        "the instance's state directory is not created"
    );
    assert_eq!(entries(&work), Vec::<String>::new());
    assert_eq!(
        case.decision_ids(&work, Some(&config), &["--state-dir", text(&flag)]),
        [FIRST]
    );
}

#[test]
fn without_a_config_file_the_state_is_state_under_the_working_directory() {
    let case = Case::new("no-file");
    let work = case.work();

    assert_eq!(case.record(&work, None, &[]), FIRST);
    assert_eq!(
        entries(&work),
        ["state"],
        "today's state/ under the working directory"
    );
    assert!(!case.home().join("state").exists());
    assert_eq!(case.decision_ids(&work, None, &[]), [FIRST]);
}

/// A running `conductor dashboard serve`, stopped when dropped.
struct Served {
    child: Child,
    port: u16,
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts `conductor dashboard serve --port 0 <more>` from `cwd`, with `PATH` naming the case's
/// empty `bin/`, and reads the port from its first line.
fn serve(case: &Case, cwd: &Path, config: Option<&Path>, more: &[&str]) -> Served {
    let errors = case.dir.join("serve.stderr");
    let mut child = case
        .command(cwd, config)
        .env("PATH", case.dir.join("bin"))
        .args(["dashboard", "serve", "--port", "0"])
        .args(more)
        .stdout(Stdio::piped())
        .stderr(File::create(&errors).expect("create the server's stderr file"))
        .spawn()
        .expect("the conductor binary starts");
    let stdout = child.stdout.take().expect("a piped stdout");
    let (lines, first) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = BufReader::new(stdout).read_line(&mut line);
        let _ = lines.send(line);
    });
    let line = first
        .recv_timeout(Duration::from_secs(30))
        .unwrap_or_default();
    let port = line
        .trim_end()
        .strip_prefix("listening on http://127.0.0.1:")
        .and_then(|port| port.parse().ok());
    let Some(port) = port else {
        let _ = child.kill();
        let status = child.wait();
        panic!(
            "dashboard serve printed {line:?} and ended {status:?}; stderr: {}",
            fs::read_to_string(&errors).unwrap_or_default()
        );
    };
    Served { child, port }
}

/// The body of `GET <path>` to `served`, which must answer 200.
fn get(served: &Served, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", served.port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nConnection: close\r\n\r\n",
        served.port
    )
    .expect("send the request");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read the answer");
    let (head, body) = raw.split_once("\r\n\r\n").expect("a head and a body");
    assert!(head.starts_with("HTTP/1.1 200 "), "GET {path}: {head}");
    body.to_owned()
}

fn data(served: &Served) -> Value {
    serde_json::from_str(&get(served, "/data.json")).expect("/data.json is JSON")
}

fn ids(rows: &Value) -> Vec<&str> {
    rows.as_array()
        .unwrap_or_else(|| panic!("a list of rows: {rows}"))
        .iter()
        .map(|row| row["id"].as_str().expect("a row id"))
        .collect()
}

/// A decision line as conductor writes it by hand into `decisions/YYYY-MM.jsonl`.
fn decision_line(id: &str) -> String {
    format!(
        "{}\n",
        json!({"id": id, "class": "C", "question": "Which first?", "options": "A or B",
            "choice": "A", "reason": "reversible", "decided_by": "Conductor",
            "decided_at": "2026-10-08T00:00:00Z"})
    )
}

/// The records of two roots and the watch state of two places, each telling which was read: the
/// instance's records under `~/records` and its watch state under `~/state/watch/`, and a
/// conductor repository `decoy/` holding the working directory, with the watch stopgap
/// `~/.cache/conductor-watch/`.
fn two_roots(case: &Case) -> PathBuf {
    case.write(
        "home/records/decisions/2026-10.jsonl",
        &decision_line("DEC-20261008-07"),
    );
    case.write("home/records/dispatches/2026-10.jsonl", "");
    case.write(
        "home/state/watch/ci.state",
        "acme-web|CI|7|failure|2026-10-08T08:00:00Z\n",
    );
    case.write("home/state/watch/limit.new", "Instance limit reached\ts7\n");
    case.write(
        "decoy/decisions/2026-10.jsonl",
        &decision_line("DEC-20261008-09"),
    );
    case.write("decoy/dispatches/2026-10.jsonl", "");
    case.write(
        "home/.cache/conductor-watch/ci.state",
        "decoy|CI|9|failure|2026-10-08T08:00:00Z\n",
    );
    case.write(
        "home/.cache/conductor-watch/limit.new",
        "Stopgap limit reached\ts9\n",
    );
    let work = case.dir.join("decoy/work");
    fs::create_dir_all(&work).expect("create the decoy's working directory");
    work
}

fn red_repositories(data: &Value) -> Vec<&str> {
    data["red_ci"]
        .as_array()
        .unwrap_or_else(|| panic!("red CI rows: {data:#}"))
        .iter()
        .map(|row| row["repo"].as_str().expect("a repository"))
        .collect()
}

fn limits(data: &Value) -> Vec<&str> {
    data["watch"]["usage_limit"]
        .as_array()
        .unwrap_or_else(|| panic!("usage-limit rows: {data:#}"))
        .iter()
        .map(|row| row["text"].as_str().expect("a limit"))
        .collect()
}

#[test]
fn with_a_config_file_the_dashboard_serves_the_instance_records_and_its_watch_state() {
    let case = Case::new("dashboard");
    let config = case.config();
    let work = two_roots(&case);

    let served = serve(&case, &work, Some(&config), &[]);
    let data = data(&served);
    assert_eq!(data["reading"]["root"], "~/records", "{data:#}");
    assert_eq!(
        data["reading"]["ci_state"], "~/state/watch/ci.state",
        "{data:#}"
    );
    assert_eq!(ids(&data["decisions"]), ["DEC-20261008-07"], "{data:#}");
    assert_eq!(red_repositories(&data), ["acme-web"], "{data:#}");
    assert_eq!(limits(&data), ["Instance limit reached"], "{data:#}");
    let errors = data["errors"].to_string();
    assert!(
        !errors.contains("records"),
        "the instance's records read: {errors}"
    );

    let page = get(&served, "/");
    assert!(
        page.contains("<h2>Sessions under ~/checkouts ("),
        "the page names the instance's checkouts root:\n{page}"
    );
}

#[test]
fn the_root_flag_beats_the_instance_records_and_the_watch_state_stays_the_instance() {
    let case = Case::new("dashboard-root");
    let config = case.config();
    let work = two_roots(&case);
    let decoy = case.dir.join("decoy");

    let served = serve(&case, &work, Some(&config), &["--root", text(&decoy)]);
    let data = data(&served);
    assert_eq!(data["reading"]["root"], text(&decoy), "{data:#}");
    assert_eq!(ids(&data["decisions"]), ["DEC-20261008-09"], "{data:#}");
    assert_eq!(
        data["reading"]["ci_state"], "~/state/watch/ci.state",
        "{data:#}"
    );
    assert_eq!(red_repositories(&data), ["acme-web"], "{data:#}");
}

#[test]
fn without_a_config_file_the_dashboard_reads_today_s_root_and_watch_stopgap() {
    let case = Case::new("dashboard-no-file");
    let work = two_roots(&case);
    let decoy = case.dir.join("decoy");

    let served = serve(&case, &work, None, &[]);
    let data = data(&served);
    assert_eq!(data["reading"]["root"], text(&decoy), "{data:#}");
    assert_eq!(ids(&data["decisions"]), ["DEC-20261008-09"], "{data:#}");
    assert_eq!(
        data["reading"]["ci_state"], "~/.cache/conductor-watch/ci.state",
        "{data:#}"
    );
    assert_eq!(red_repositories(&data), ["decoy"], "{data:#}");
    assert_eq!(limits(&data), ["Stopgap limit reached"], "{data:#}");

    let home = case.home();
    let built_in = config::built_in(&home, &work).checkouts.root;
    let shown = built_in.replacen(text(&home), "~", 1);
    let page = get(&served, "/");
    assert!(
        page.contains(&format!("<h2>Sessions under {shown} (")),
        "the page names the built-in checkouts root {shown}:\n{page}"
    );
}

/// The files of this unit name no organization: the instance does. Test fixtures are exempt;
/// this file builds the two words from parts so that it is not one.
#[test]
fn the_files_of_this_unit_name_no_organization() {
    let words = [["beyond", "10x"].concat(), ["b", "10x"].concat()];
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hits = Vec::new();
    for file in [
        "src/state.rs",
        "src/decide.rs",
        "src/dispatch.rs",
        "src/dashboard.rs",
        "tests/instance_paths.rs",
    ] {
        let text = fs::read_to_string(crate_dir.join(file)).expect("read a file of this unit");
        for (number, line) in text.lines().enumerate() {
            if words.iter().any(|word| line.contains(word.as_str())) {
                hits.push(format!("{file}:{}: {line}", number + 1));
            }
        }
    }
    assert!(hits.is_empty(), "{}", hits.join("\n"));
}
