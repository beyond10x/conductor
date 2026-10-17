//! `story:run-daemon`: `conductor run` is the instance's loop as a program
//! (docs/design/conductor-run.md § 3): the watch pass's lines handled, the mailbox read, the cycle
//! with dispatch v1, the daily metrics and brief, each tick recorded, installed as a user unit.
//!
//! Each case is a directory of its own under this test target's temporary directory: `home/`, the
//! `HOME` the program runs with, holding the checkouts root `example-org/` with the checkouts
//! `alpha`, `beta` and `gamma` (a `.git/HEAD` each) and `~/.claude.json` marking them trusted;
//! `records/` with the charter `charters/alpha.md`; `state/`; `roles/` with the controller role's
//! profile and settings; `bin/` with the stand-ins below, first on `PATH`; `fake/`, what they read
//! and write; and the config file `conductor.yaml` naming them, under the session prefix `ex`,
//! with the `none` provider and a context threshold of 200k.
//!
//! The stand-ins, written here and never committed: `claude` (`agents --json` prints the entries
//! under `fake/agents/`, `stop <id>` takes one away, a launch prints the id `s000000<n>-0000` and
//! keeps its last argument, the charter, in `fake/charter.<n>`; every call is logged one line to
//! `fake/claude.calls`), `systemd-run` and `systemctl` (logged; `systemctl` answers a unit loaded
//! and active while `fake/unit.loaded` exists), `gh` (`[]`), `aep` (`[]`) and `ais` (prints
//! `fake/ais.json`).
//!
//! The handlers, the mailbox, the cycle's dispatch v1 and the daily tick are run through the
//! library in a child of this test binary ([`child`]), with a [`Recorder`] in place of the real
//! deliverer (which runs `conductor session deliver`, `story:mailbox-send-and-deliver`) and a
//! headroom read from `fake/headroom`; the handlers are fed fixture watch lines. `run --once`,
//! `--install`, `--uninstall`, `doctor` and the tick views are run through the built binary.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{Result, bail};
use conductor_cli::collect::Collector;
use conductor_cli::config;
use conductor_cli::run::{self, Headroom, Seams};
use conductor_cli::session::{Deliverer, Delivery};
use conductor_cli::{snapshot, store};
use conductor_model::observation::{
    CommitSha, Mergeability, RecordPullRequest, RecordRepository, RepositoryName, SnapshotId,
    SnapshotState, StartSnapshot, Visibility,
};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The variable that holds a child run's instructions, as one JSON object.
const CHILD: &str = "CONDUCTOR_TEST_RUN_CHILD";

/// The variable the free-disk readings are read from (`gc::reclaim::READINGS_VARIABLE`).
const READINGS: &str = "CONDUCTOR_DISK_READINGS";

/// Free bytes no floor of the instance is under: 500 GiB.
const PLENTY: u64 = 500 << 30;

/// Free bytes under `disk_low` (100 GiB) and over `disk_admit` (20 GiB): 50 GiB.
const LOW: u64 = 50 << 30;

/// The id the stand-in prints for its first launch, and the eight characters the watch names it by.
const FIRST: &str = "s0000001-0000";
const FIRST_SHORT: &str = "s0000001";

/// The check-in a stalled controller is sent.
const CHECK_IN: &str = "[REPORT alpha -] check-in: report your state or blocked";

// ---------------------------------------------------------------------------------------------
// The child run
// ---------------------------------------------------------------------------------------------

/// The deliverer of a child run: appends `{"name", "text"}` to `fake/deliveries.jsonl`, and
/// refuses a session `fake/unreachable/<name>` names, as `session deliver` refuses one with no
/// socket.
struct Recorder {
    dir: PathBuf,
}

impl Deliverer for Recorder {
    fn deliver(&self, name: &str, text: &str) -> Result<Delivery> {
        if self.dir.join("unreachable").join(name).exists() {
            bail!("SessionNotReachable: {name}");
        }
        let mut lines = fs::read_to_string(self.dir.join("deliveries.jsonl")).unwrap_or_default();
        lines.push_str(&format!("{}\n", json!({"name": name, "text": text})));
        fs::write(self.dir.join("deliveries.jsonl"), lines).expect("record a delivery");
        Ok(Delivery {
            session: name.to_owned(),
            first_line: text.lines().next().unwrap_or_default().to_owned(),
        })
    }
}

/// The headroom of a child run: `yes` in `fake/headroom`.
struct FileHeadroom {
    dir: PathBuf,
}

impl Headroom for FileHeadroom {
    fn headroom(&self) -> Result<bool> {
        Ok(fs::read_to_string(self.dir.join("headroom")).is_ok_and(|text| text.trim() == "yes"))
    }
}

/// The child run of this test binary ([`Case::child`]); a plain run of the suite passes at once.
/// It fixes the case's config file as the process's instance, builds what `reviews` asks for, and
/// runs `do`: `lines`, a watch tick over the fixture lines; `cycle`; or `daily` for `day`. With
/// `decide`, the tick hands a decision request to `run::handle_request`, whose decider is the fake
/// one ([`run::DECIDER_VARIABLE`]). It prints the tick's summary as JSON and exits 0, or names
/// the error and exits 1.
#[test]
fn child() {
    let Ok(spec) = std::env::var(CHILD) else {
        return;
    };
    let spec: Value = serde_json::from_str(&spec).expect("the child's instructions are JSON");
    let file = PathBuf::from(std::env::var_os("CONDUCTOR_CONFIG").expect("a config file"));
    config::set_active(Some(&file)).expect("the case's config loads");
    let fake = PathBuf::from(spec["fake"].as_str().expect("the fake directory"));
    if let Some(reviews) = spec["reviews"].as_object() {
        let state = PathBuf::from(&config::active().instance.state);
        let waiting: Vec<(String, i64)> = reviews
            .iter()
            .map(|(repo, count)| (repo.clone(), count.as_i64().unwrap_or(0)))
            .collect();
        take_reviews(&state, &waiting);
    }
    let deliverer = Recorder { dir: fake.clone() };
    let file_headroom = FileHeadroom { dir: fake.clone() };
    let headroom: &dyn Headroom = if spec["ais"] == true {
        &run::Ais
    } else {
        &file_headroom
    };
    let host = conductor_cli::confine::Host::default();
    let seams = Seams {
        providers: &host,
        deliverer: &deliverer,
        headroom,
        handle_request: (spec["decide"] == true)
            .then_some(run::handle_request as run::HandleRequest),
        program: PathBuf::from(spec["program"].as_str().expect("the conductor binary")),
        gc: conductor_cli::gc::gc,
    };
    let lines: Vec<String> = spec["lines"]
        .as_array()
        .map(|lines| {
            lines
                .iter()
                .filter_map(|line| line.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let summary = match spec["do"].as_str() {
        Some("lines") => run::handle_lines(None, None, &lines, &seams),
        Some("cycle") => run::cycle_tick(None, None, &seams),
        Some("daily") => {
            run::daily_tick(None, None, spec["day"].as_str().expect("the day"), &seams)
        }
        other => panic!("no such child run: {other:?}"),
    };
    match summary {
        Ok(summary) => {
            let actions: Vec<&str> = summary
                .actions
                .iter()
                .map(|event| conductor_cli::records::dispatch_event(*event))
                .collect();
            println!(
                "{}",
                json!({
                    "tick_id": summary.tick_id,
                    "lines_handled": summary.lines_handled,
                    "actions": actions,
                })
            );
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("conductor: {error:#}");
            std::process::exit(1);
        }
    }
}

/// Takes a complete snapshot into the store under `state` that observes each repository of
/// `waiting` active, with that many pull requests waiting for review.
fn take_reviews(state: &Path, waiting: &[(String, i64)]) {
    let waiting = waiting.to_vec();
    let collector = Collector::new("reviews", move |recorder| {
        let snapshot = recorder.snapshot().clone();
        for (repository, count) in &waiting {
            recorder.repository(RecordRepository {
                snapshot_id: snapshot.clone(),
                ..observation(repository)
            })?;
            for number in 1..=*count {
                recorder.pull_request(RecordPullRequest {
                    snapshot_id: snapshot.clone(),
                    repository: RepositoryName(repository.clone()),
                    number,
                    title: format!("Change {number}"),
                    draft: false,
                    mergeability: Mergeability::Mergeable,
                    failing_checks: 0,
                    failing_check_names: Vec::new(),
                    opened_at: Timestamp("2026-10-07T09:00:00Z".to_owned()),
                    updated_at: Timestamp("2026-10-07T09:00:00Z".to_owned()),
                    url: None,
                })?;
            }
        }
        Ok(())
    });
    let store = store::open(state).expect("open the case's store");
    let start = StartSnapshot {
        started_at: Timestamp("2026-10-17T08:00:00Z".to_owned()),
        disk_free_bytes: PLENTY.try_into().unwrap_or(i64::MAX),
        budget: None,
    };
    let taken = snapshot::take_from(store, start, &[collector]).expect("the snapshot ends");
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
}

/// A repository observed active: three real commits in seven days, not archived.
fn observation(name: &str) -> RecordRepository {
    RecordRepository {
        snapshot_id: SnapshotId(Uuid("00000000-0000-0000-0000-000000000000".to_owned())),
        repository: RepositoryName(name.to_owned()),
        visibility: Visibility::Private,
        archived: false,
        local_checkout: true,
        main_head: CommitSha("0a1b2c3".to_owned()),
        main_committed_at: Timestamp("2026-10-01T00:00:00Z".to_owned()),
        main_commits_7d: 5,
        real_commits_7d: 3,
        behind_main: 0,
        dirty_files: 0,
        worktrees: 1,
        open_issues: 0,
        oldest_open_issue_at: None,
        latest_release: None,
        latest_release_at: None,
        unreleased_commits: None,
        planning_store_version: None,
        in_catalog: Some(true),
        gates_hooks_version: None,
        gates_version: None,
        wave_width: None,
        wave_ready: None,
        wave_reason: None,
    }
}

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// A case directory of its own; see the module documentation.
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

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

fn executable(path: &Path, text: &str) {
    write(path, text);
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

/// Settings that pin the instance (`story:instance-isolation`), as the shipped ones are pinned.
fn pinned(instance: &str) -> String {
    json!({
        "env": {"CONDUCTOR_INSTANCE": instance},
        "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
            "type": "command",
            "command": format!(
                "CONDUCTOR_INSTANCE={instance} conductor guard record-guard-decision \
                 --from-pre-tool-use || exit 2"
            ),
        }]}]},
    })
    .to_string()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("run")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        case.fixture();
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn root(&self) -> PathBuf {
        self.home().join("example-org")
    }

    fn checkout(&self, repo: &str) -> PathBuf {
        self.root().join(repo)
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn charter(&self, repo: &str) -> PathBuf {
        self.records().join(format!("charters/{repo}.md"))
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn fake(&self) -> PathBuf {
        self.dir.join("fake")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// Writes the config file with `extra` lines under the instance.
    fn write_config(&self, extra: &str) {
        write(
            &self.config(),
            &format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n\
                 \x20   cache: {cache}\n\
                 \x20   roles:\n\
                 \x20     - {{role: controller, harness: claude, model: sonnet, profile: {profile}, \
                 settings: {settings}}}\n\
                 \x20   thresholds: {{context_handover: 200000}}\n\
                 {extra}",
                root = self.root().display(),
                trees = self.dir.join("trees").display(),
                records = self.records().display(),
                state = self.state().display(),
                cache = self.dir.join("cache").display(),
                profile = self.dir.join("roles/repo-controller.md").display(),
                settings = self.dir.join("roles/controller-settings.json").display(),
            ),
        );
    }

    /// The `confinement` and `controllers` the config file writes until a case sets its own.
    fn configure(&self, confinement: &str, controllers: &str) {
        self.write_config(&format!(
            "\x20   confinement: {confinement}\n\x20   controllers: {controllers}\n"
        ));
    }

    fn fixture(&self) {
        let mut projects = serde_json::Map::new();
        for repo in ["alpha", "beta", "gamma"] {
            write(
                &self.checkout(repo).join(".git/HEAD"),
                "ref: refs/heads/main\n",
            );
            projects.insert(
                self.checkout(repo).display().to_string(),
                json!({"hasTrustDialogAccepted": true}),
            );
        }
        write(
            &self.home().join(".claude.json"),
            &json!({"numStartups": 3, "projects": projects}).to_string(),
        );
        write(
            &self.charter("alpha"),
            "# Charter: alpha\n\nKeep main green.\n\nmerge: controller\n",
        );
        write(
            &self.dir.join("roles/repo-controller.md"),
            "# Repository controller\n",
        );
        write(
            &self.dir.join("roles/controller-settings.json"),
            &pinned("example-org"),
        );
        fs::create_dir_all(self.state()).expect("create the state directory");
        fs::create_dir_all(self.fake().join("agents")).expect("create the session list");
        self.configure("{provider: none}", "{max_working: 4}");
        let fake = self.fake();
        let fake = fake.display();
        executable(
            &self.bin().join("claude"),
            &format!(
                "#!/bin/sh\n\
                 D='{fake}'\n\
                 printf '%s\\n' \"$1 $2 $3 $4\" >> \"$D/claude.calls\"\n\
                 case \"$1\" in\n\
                 agents)\n\
                 \x20 printf '['; sep=''\n\
                 \x20 for f in \"$D\"/agents/*.json; do\n\
                 \x20   [ -e \"$f\" ] || continue\n\
                 \x20   printf '%s' \"$sep\"; cat \"$f\"; sep=','\n\
                 \x20 done\n\
                 \x20 printf ']\\n'; exit 0;;\n\
                 stop) rm -f \"$D/agents/$2.json\"; exit 0;;\n\
                 rm) exit 0;;\n\
                 esac\n\
                 n=$(( $(cat \"$D/launches\" 2>/dev/null || echo 0) + 1 ))\n\
                 echo \"$n\" > \"$D/launches\"\n\
                 if [ \"$2\" = --resume ]; then echo \"$3\"; exit 0; fi\n\
                 for arg; do last=\"$arg\"; done\n\
                 printf '%s' \"$last\" > \"$D/charter.$n\"\n\
                 printf 's%07d-0000\\n' \"$n\"\n"
            ),
        );
        executable(
            &self.bin().join("systemd-run"),
            &format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{fake}/systemd-run.args'\nexit 0\n"),
        );
        executable(
            &self.bin().join("systemctl"),
            &format!(
                "#!/bin/sh\n\
                 D='{fake}'\n\
                 printf '%s\\n' \"$*\" >> \"$D/systemctl.calls\"\n\
                 case \"$2\" in\n\
                 show) if [ -e \"$D/unit.loaded\" ]; then \
                 printf 'LoadState=loaded\\nActiveState=active\\n'; else \
                 printf 'LoadState=not-found\\nActiveState=inactive\\n'; fi;;\n\
                 is-active) if [ -e \"$D/unit.loaded\" ]; then echo active; exit 0; \
                 else echo inactive; exit 3; fi;;\n\
                 esac\n\
                 exit 0\n"
            ),
        );
        executable(&self.bin().join("gh"), "#!/bin/sh\necho '[]'\n");
        executable(&self.bin().join("aep"), "#!/bin/sh\necho '[]'\n");
        executable(
            &self.bin().join("ais"),
            &format!("#!/bin/sh\ncat '{fake}/ais.json'\n"),
        );
        // The cycle's steps, for a cycle run against the store alone: each is logged, none runs.
        executable(
            &self.bin().join("conductor-steps"),
            &format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{fake}/steps'\nexit 0\n"),
        );
        self.readings(PLENTY);
    }

    /// The free bytes the watch and the snapshot read.
    fn readings(&self, bytes: u64) {
        write(&self.fake().join("readings"), &format!("{bytes}\n"));
    }

    fn path(&self) -> String {
        format!("{}:/usr/bin:/bin", self.bin().display())
    }

    /// The built binary's path, which a child run's cycle and daily tick run.
    fn program(&self) -> PathBuf {
        PathBuf::from(common::conductor(self.home()).get_program())
    }

    /// `conductor <args>` from the records directory, with the case's home, config file and
    /// stand-ins.
    fn conductor(&self, args: &[&str]) -> Output {
        common::conductor(self.home())
            .args(args)
            .current_dir(self.records())
            .env("CONDUCTOR_CONFIG", self.config())
            .env("PATH", self.path())
            .env(READINGS, self.fake().join("readings"))
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// [`Case::conductor`], which must succeed; its standard output.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.conductor(args);
        assert!(output.status.success(), "{args:?}: {}", describe(&output));
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// A child run of this test binary ([`child`]) with `spec` and the case's fake directory and
    /// built binary; its summary.
    fn child(&self, mut spec: Value) -> Value {
        spec["fake"] = json!(self.fake());
        if spec["program"].is_null() {
            spec["program"] = json!(self.program());
        }
        let output = Command::new(std::env::current_exe().expect("this test binary"))
            .args([
                "--exact",
                "child",
                "--nocapture",
                "--test-threads",
                "1",
                "-q",
            ])
            .env("HOME", self.home())
            .env_remove(config::INSTANCE_VARIABLE)
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .env(config::CONFIG_VARIABLE, self.config())
            .env("PATH", self.path())
            .env(READINGS, self.fake().join("readings"))
            .env(CHILD, spec.to_string())
            .env(run::DECIDER_VARIABLE, "fake")
            .current_dir(self.records())
            .stdin(Stdio::null())
            .output()
            .expect("the child runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let summary = stdout
            .lines()
            .rev()
            .find_map(|line| serde_json::from_str::<Value>(line).ok());
        match (output.status.success(), summary) {
            (true, Some(summary)) => summary,
            _ => panic!("the child run failed: {}", describe(&output)),
        }
    }

    /// A watch tick over `lines`.
    fn tick(&self, lines: &[&str]) -> Value {
        self.child(json!({"do": "lines", "lines": lines}))
    }

    /// Spawns `repo`'s controller through the built binary and lists its session live.
    fn spawn(&self, repo: &str, id: &str) {
        self.ok(&["spawn", "--repository", repo]);
        self.live(repo, id, true);
    }

    /// Lists the session `id` of `repo` in the session list: with a pid when `live`, else as the
    /// list keeps a session stopped at a usage limit.
    fn live(&self, repo: &str, id: &str, live: bool) {
        let mut entry = json!({
            "id": id, "sessionId": id, "name": format!("ex-{repo}"),
            "cwd": self.checkout(repo), "status": if live { "busy" } else { "blocked" },
            "state": if live { "working" } else { "blocked" },
        });
        if live {
            entry["pid"] = json!(4242);
        }
        write(
            &self.fake().join(format!("agents/{id}.json")),
            &entry.to_string(),
        );
    }

    /// Every delivery the recorder kept, `(session, text)`, in order.
    fn deliveries(&self) -> Vec<(String, String)> {
        fs::read_to_string(self.fake().join("deliveries.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| {
                let value: Value = serde_json::from_str(line).expect("a recorded delivery");
                (
                    value["name"].as_str().unwrap_or_default().to_owned(),
                    value["text"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    }

    /// Every call of the `claude` stand-in, its first four words, in order.
    fn claude_calls(&self) -> Vec<String> {
        fs::read_to_string(self.fake().join("claude.calls"))
            .unwrap_or_default()
            .lines()
            .map(|line| line.trim_end().to_owned())
            .collect()
    }

    /// The launches among [`Case::claude_calls`]: the session name each started.
    fn launched(&self) -> Vec<String> {
        self.claude_calls()
            .iter()
            .filter_map(|call| call.strip_prefix("--bg -n "))
            .map(|rest| rest.split(' ').next().unwrap_or_default().to_owned())
            .collect()
    }

    /// Every line of the records' dispatch log, in order.
    fn log(&self) -> Vec<Value> {
        let dir = self.records().join("dispatches");
        let mut files: Vec<PathBuf> = fs::read_dir(&dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default();
        files.sort();
        files
            .iter()
            .flat_map(|file| {
                fs::read_to_string(file)
                    .expect("read the dispatch log")
                    .lines()
                    .map(|line| serde_json::from_str(line).expect("a dispatch line"))
                    .collect::<Vec<Value>>()
            })
            .collect()
    }

    /// The lines of the dispatch log written by the tick `tick`, as its summary names it.
    fn lines_of(&self, tick: &Value) -> Vec<Value> {
        self.log()
            .into_iter()
            .filter(|line| line["tick_id"] == tick["tick_id"])
            .collect()
    }

    /// The rows of a view, rendered as JSON.
    fn view(&self, args: &[&str]) -> Vec<Value> {
        let mut args = args.to_vec();
        args.extend(["--format", "json"]);
        serde_json::from_str(&self.ok(&args)).expect("the view prints JSON")
    }

    fn controllers(&self) -> Vec<(String, String)> {
        self.view(&["controller", "all-controllers"])
            .iter()
            .map(|row| {
                (
                    row["repository"].as_str().unwrap_or_default().to_owned(),
                    row["state"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    }

    /// Sends a dispatch for `repo` and answers its id.
    fn dispatch(&self, repo: &str) -> String {
        let proposed = self.dir.join("goal.proposed");
        if !proposed.exists() {
            write(&proposed, "");
            self.ok(&[
                "goal",
                "propose-goal",
                "--goal-id",
                "G1",
                "--title",
                "Keep main green",
                "--exit-evidence",
                "main is green",
            ]);
        }
        self.ok(&[
            "dispatch",
            "send-dispatch",
            "--repository",
            repo,
            "--goal-id",
            "G1",
            "--brief",
            "Keep main green",
            "--sent-at",
            "2026-10-17T08:00:00Z",
            "--priority",
            "1",
        ])
        .trim()
        .to_owned()
    }

    fn dispatch_state(&self, id: &str) -> String {
        self.view(&["dispatch", "dispatches"])
            .iter()
            .find(|row| row["dispatch_id"] == id)
            .and_then(|row| row["state"].as_str())
            .unwrap_or("none")
            .to_owned()
    }

    /// Leaves `text` in conductor's mailbox as `<name>.md`.
    fn mail(&self, name: &str, text: &str) {
        write(
            &self.state().join(format!("mailbox/conductor/{name}.md")),
            text,
        );
    }
}

fn actions(summary: &Value) -> Vec<String> {
    summary["actions"]
        .as_array()
        .map(|actions| {
            actions
                .iter()
                .filter_map(|action| action.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn events(lines: &[Value]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line["event"].as_str().unwrap_or_default().to_owned())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------------------------

/// `context:` over `thresholds.context_handover` delivers `[HANDOVER <repo>] context <n>k` once
/// per session, and writes `handover-sent` with the session and the line.
#[test]
fn context_over_the_threshold_delivers_one_handover_per_session() {
    let case = Case::new("context-handover");
    case.spawn("alpha", FIRST);
    let line = format!("context: alpha 230k tokens (session {FIRST_SHORT})");

    let first = case.tick(&[&line]);
    assert_eq!(
        case.deliveries(),
        [(
            "ex-alpha".to_owned(),
            "[HANDOVER alpha] context 230k".to_owned()
        )]
    );
    assert_eq!(actions(&first), ["handover-sent"]);
    let written = case.lines_of(&first);
    assert_eq!(events(&written), ["handover-sent"], "{written:?}");
    assert_eq!(written[0]["repo"], "alpha");
    assert_eq!(written[0]["session"], "ex-alpha");
    assert_eq!(written[0]["line"], line);

    let second = case.tick(&[&line]);
    assert_eq!(case.deliveries().len(), 1, "once per session");
    assert_eq!(actions(&second), Vec::<String>::new());
    assert_eq!(case.launched(), ["ex-alpha"], "nothing restarted");
}

/// Once `<checkout>/.git/handoff/` holds a file newer than the delivery, the session is stopped,
/// the hand-over path appended to the charter as its resume point and the controller spawned
/// again from it: `handover`, `sent`, `controller-restarted`.
#[test]
fn a_hand_over_file_newer_than_the_delivery_restarts_the_controller_from_its_charter() {
    let case = Case::new("handover-file");
    case.spawn("alpha", FIRST);
    case.tick(&[&format!(
        "context: alpha 230k tokens (session {FIRST_SHORT})"
    )]);
    assert!(case.lines_of(&case.tick(&[])).is_empty(), "no file yet");

    let handoff = case
        .checkout("alpha")
        .join(".git/handoff/2026-10-17-alpha.md");
    write(
        &handoff,
        "# Hand-over\n\nNext: finish the store migration.\n",
    );
    let tick = case.tick(&[]);

    assert_eq!(
        actions(&tick),
        ["handover", "sent", "controller-restarted"],
        "{tick}"
    );
    let calls = case.claude_calls();
    assert!(
        calls.iter().any(|call| call == &format!("stop {FIRST}")),
        "{calls:?}"
    );
    assert_eq!(case.launched(), ["ex-alpha", "ex-alpha"]);
    let charter = fs::read_to_string(case.charter("alpha")).expect("read the charter");
    assert!(charter.starts_with("# Charter: alpha\n"), "{charter}");
    assert!(
        charter.contains(&format!("## Resume point\n\n{}\n", handoff.display())),
        "{charter}"
    );
    let relaunched = fs::read_to_string(case.fake().join("charter.2")).expect("the charter sent");
    assert!(relaunched.contains("## Resume point"), "{relaunched}");
    let written = case.lines_of(&tick);
    assert_eq!(written[0]["event"], "handover");
    assert_eq!(written[0]["path"], json!(handoff));
    assert_eq!(written[1]["event"], "controller-restarted");
    assert_eq!(written[1]["reason"], "handover");
    let mut states: Vec<String> = case
        .controllers()
        .into_iter()
        .map(|(_, state)| state)
        .collect();
    states.sort();
    assert_eq!(states, ["Running", "Stopped"]);
}

/// Over 1.25 times the threshold the session is stopped and spawned again at once, with no
/// hand-over to wait for.
#[test]
fn context_over_a_quarter_more_than_the_threshold_restarts_at_once() {
    let case = Case::new("context-over");
    case.spawn("alpha", FIRST);
    let tick = case.tick(&[&format!(
        "context: alpha 260k tokens (session {FIRST_SHORT})"
    )]);
    assert_eq!(
        actions(&tick),
        ["handover", "sent", "controller-restarted"],
        "{tick}"
    );
    assert!(case.deliveries().is_empty());
    let written = case.lines_of(&tick);
    assert_eq!(written[0]["reason"], "context over 1.25x");
    assert_eq!(case.launched(), ["ex-alpha", "ex-alpha"]);
}

/// `usage limit:` marks the sessions it names (`usage-limit`); a later tick whose headroom is
/// back resumes each (`resumed`), and none before.
#[test]
fn a_usage_limit_marks_the_sessions_and_a_tick_with_headroom_resumes_them() {
    let case = Case::new("usage-limit");
    case.ok(&["spawn", "--repository", "alpha"]);
    case.live("alpha", FIRST, false);
    write(&case.fake().join("headroom"), "no");

    let marked = case.tick(&["usage limit: You've hit your limit; 1 sessions: ex-alpha"]);
    assert_eq!(actions(&marked), ["usage-limit"]);
    assert_eq!(case.lines_of(&marked)[0]["session"], "ex-alpha");
    assert_eq!(
        actions(&case.tick(&[])),
        Vec::<String>::new(),
        "no headroom"
    );

    write(&case.fake().join("headroom"), "yes");
    let resumed = case.tick(&[]);
    assert_eq!(actions(&resumed), ["resumed"]);
    let calls = case.claude_calls();
    assert!(
        calls
            .iter()
            .any(|call| call == &format!("--bg --resume {FIRST} -n")),
        "{calls:?}"
    );
    assert_eq!(
        actions(&case.tick(&[])),
        Vec::<String>::new(),
        "resumed once"
    );
}

/// A marked session that was over the context threshold is spawned again from its charter, not
/// resumed into the same context.
#[test]
fn a_limited_session_over_the_threshold_is_respawned_not_resumed() {
    let case = Case::new("usage-limit-over");
    case.ok(&["spawn", "--repository", "alpha"]);
    case.live("alpha", FIRST, false);
    write(&case.fake().join("headroom"), "no");
    case.tick(&[
        &format!("context: alpha 210k tokens (session {FIRST_SHORT})"),
        "usage limit: You've hit your limit; 1 sessions: ex-alpha",
    ]);
    write(&case.fake().join("headroom"), "yes");
    let tick = case.tick(&[]);
    assert_eq!(actions(&tick), ["sent", "controller-restarted"], "{tick}");
    assert_eq!(
        case.lines_of(&tick)[0]["reason"],
        "limit over the context threshold"
    );
    assert!(
        !case
            .claude_calls()
            .iter()
            .any(|call| call.contains("--resume"))
    );
}

/// `PR checks done`, `CI red on main` and `CI green again` reach the live controller as
/// `[REPORT <repo> -] watch: <line>`, each a `relayed` line.
#[test]
fn checks_and_ci_lines_are_relayed_to_the_live_controller() {
    let case = Case::new("relay");
    case.spawn("alpha", FIRST);
    let lines = [
        "PR checks done: alpha#7 green (head 0a1b2c3)",
        "CI red on main: alpha / check (run 9)",
        "CI green again on main: alpha / check",
    ];
    let tick = case.tick(&lines);
    assert_eq!(actions(&tick), ["relayed", "relayed", "relayed"]);
    let expected: Vec<(String, String)> = lines
        .iter()
        .map(|line| {
            (
                "ex-alpha".to_owned(),
                format!("[REPORT alpha -] watch: {line}"),
            )
        })
        .collect();
    assert_eq!(case.deliveries(), expected);
    assert_eq!(case.lines_of(&tick)[0]["session"], "ex-alpha");
}

/// A red `main` with no controller spawns one from the charter (`red-ci`, `spawned`); without a
/// charter it is a deviation line (`red-ci`, `no charter`) and nothing starts.
#[test]
fn red_main_without_a_controller_spawns_one_where_there_is_a_charter() {
    let case = Case::new("red-ci");
    let tick = case.tick(&[
        "CI red on main: alpha / check (run 9)",
        "CI red on main: beta / check (run 3)",
    ]);
    assert_eq!(actions(&tick), ["sent", "red-ci", "red-ci"], "{tick}");
    assert_eq!(case.launched(), ["ex-alpha"]);
    let written = case.lines_of(&tick);
    assert_eq!(written[0]["repo"], "alpha");
    assert_eq!(written[0]["reason"], "spawned");
    assert_eq!(written[0]["session"], "ex-alpha");
    assert_eq!(written[1]["repo"], "beta");
    assert_eq!(written[1]["reason"], "no charter");
    assert!(case.deliveries().is_empty());
}

/// A stalled or silent session is first asked to check in (`stalled`); the second time it is
/// stopped and spawned again (`controller-restarted`).
#[test]
fn a_stalled_session_is_asked_to_check_in_then_restarted() {
    let case = Case::new("stalled");
    case.spawn("alpha", FIRST);
    let first = case.tick(&[&format!("session stalled: ex-alpha {FIRST_SHORT} 45m")]);
    assert_eq!(actions(&first), ["stalled"]);
    assert_eq!(
        case.deliveries(),
        [("ex-alpha".to_owned(), CHECK_IN.to_owned())]
    );

    let second = case.tick(&[&format!("session silent: ex-alpha {FIRST_SHORT} 50m")]);
    assert_eq!(
        actions(&second),
        ["sent", "controller-restarted"],
        "{second}"
    );
    assert_eq!(case.lines_of(&second)[0]["reason"], "stalled again");
    assert!(
        case.claude_calls()
            .iter()
            .any(|call| call == &format!("stop {FIRST}"))
    );
    assert_eq!(case.launched(), ["ex-alpha", "ex-alpha"]);
    assert_eq!(case.deliveries().len(), 1, "no second check-in");
}

/// A gone or exited session whose repository has an open dispatch is spawned again; one with no
/// open dispatch is left.
#[test]
fn a_gone_session_with_an_open_dispatch_is_respawned() {
    let case = Case::new("gone");
    case.ok(&["spawn", "--repository", "alpha"]);
    let dispatch = case.dispatch("alpha");
    let tick = case.tick(&[&format!("session gone: ex-alpha {FIRST_SHORT}")]);
    assert_eq!(actions(&tick), ["sent", "controller-restarted"], "{tick}");
    assert_eq!(case.lines_of(&tick)[0]["reason"], "gone");
    assert_eq!(case.launched(), ["ex-alpha", "ex-alpha"]);

    case.ok(&["dispatch", "report-started", "--dispatch-id", &dispatch]);
    case.ok(&[
        "dispatch",
        "report-done",
        "--dispatch-id",
        &dispatch,
        "--evidence",
        "main is green",
    ]);
    let idle = case.tick(&["session exited (blocked): ex-alpha s0000002"]);
    assert_eq!(actions(&idle), Vec::<String>::new());
    assert_eq!(
        case.launched(),
        ["ex-alpha", "ex-alpha"],
        "no open dispatch"
    );
}

/// `disk low` runs `gc` and writes `cleanup` with the line.
#[test]
fn disk_low_runs_gc_and_writes_cleanup() {
    let case = Case::new("disk-low");
    let line = "disk low: 50G free on /";
    let tick = case.tick(&[line]);
    assert_eq!(actions(&tick), ["cleanup"], "{tick}");
    let written = case.lines_of(&tick);
    assert_eq!(written[0]["repo"], "-");
    assert_eq!(written[0]["line"], line);
    assert!(
        written[0]["evidence"]
            .as_str()
            .is_some_and(|evidence| evidence.starts_with("gc exited")),
        "{written:?}"
    );
}

/// `answered <id> <choice>` delivers `[DECISION <id>] <choice> <reason> (<decision id>)` to the
/// controller of the repository that raised the request (`decision-sent`).
#[test]
fn an_answer_is_delivered_to_the_controller_that_raised_the_request() {
    let case = Case::new("answered");
    case.spawn("alpha", FIRST);
    case.ok(&[
        "decision",
        "raise-decision-request",
        "--request-id",
        "R-1",
        "--repository",
        "alpha",
        "--question",
        "Which store layout?",
        "--options",
        "A tree, B files",
        "--recommendation",
        "A",
        "--raised-at",
        "2026-10-17T08:00:00Z",
    ]);
    let decision = case
        .ok(&[
            "decision",
            "record-conductor-decision",
            "--decision-class",
            "C",
            "--question",
            "Which store layout?",
            "--options",
            "A tree, B files",
            "--choice",
            "A",
            "--reason",
            "one migration, then one layout",
            "--evidence",
            "the store design",
            "--decided-at",
            "2026-10-17T08:05:00Z",
            "--repository",
            "alpha",
        ])
        .trim()
        .to_owned();
    case.ok(&[
        "decision",
        "answer-request",
        "--request-id",
        "R-1",
        "--decision-id",
        &decision,
    ]);
    let tick = case.tick(&["answered R-1 A"]);
    assert_eq!(actions(&tick), ["decision-sent"]);
    assert_eq!(
        case.deliveries(),
        [(
            "ex-alpha".to_owned(),
            format!("[DECISION R-1] A one migration, then one layout ({decision})")
        )]
    );
    let written = case.lines_of(&tick);
    assert_eq!(written[0]["id"], "R-1");
    assert_eq!(written[0]["grant"], json!(decision));
    assert_eq!(written[0]["session"], "ex-alpha");
}

// ---------------------------------------------------------------------------------------------
// Mailbox
// ---------------------------------------------------------------------------------------------

/// A mailbox `[REPORT … ] started` moves its dispatch to Started.
#[test]
fn a_mailbox_report_started_moves_the_dispatch() {
    let case = Case::new("mail-started");
    let id = case.dispatch("alpha");
    case.mail(
        "0001",
        &format!("[REPORT alpha {id}] started on the brief\n"),
    );
    let tick = case.tick(&[]);
    assert_eq!(case.dispatch_state(&id), "Started");
    assert_eq!(actions(&tick), ["started"]);
    assert_eq!(tick["lines_handled"], 1);
}

/// A mailbox `[REPORT … ] blocked` moves its dispatch to Blocked with the rest as the reason.
#[test]
fn a_mailbox_report_blocked_moves_the_dispatch() {
    let case = Case::new("mail-blocked");
    let id = case.dispatch("alpha");
    case.mail("0001", &format!("[REPORT alpha {id}] started\n"));
    case.mail(
        "0002",
        &format!("[REPORT alpha {id}] blocked on beta's API\n"),
    );
    let tick = case.tick(&[]);
    assert_eq!(case.dispatch_state(&id), "Blocked");
    assert_eq!(actions(&tick), ["started", "blocked"]);
    let blocked = case
        .log()
        .into_iter()
        .find(|line| line["event"] == "blocked")
        .expect("a blocked line");
    assert_eq!(blocked["reason"], "on beta's API");
}

/// A mailbox `[REPORT … ] unblocked` moves a blocked dispatch back to Started.
#[test]
fn a_mailbox_report_unblocked_moves_the_dispatch() {
    let case = Case::new("mail-unblocked");
    let id = case.dispatch("alpha");
    case.mail("0001", &format!("[REPORT alpha {id}] started\n"));
    case.mail("0002", &format!("[REPORT alpha {id}] blocked on beta\n"));
    case.mail("0003", &format!("[REPORT alpha {id}] unblocked\n"));
    case.tick(&[]);
    assert_eq!(case.dispatch_state(&id), "Started");
}

/// A mailbox `[REPORT … ] done` moves its dispatch to Done with the rest as the evidence.
#[test]
fn a_mailbox_report_done_moves_the_dispatch() {
    let case = Case::new("mail-done");
    let id = case.dispatch("alpha");
    case.mail("0001", &format!("[REPORT alpha {id}] started\n"));
    case.mail("0002", &format!("[REPORT alpha {id}] done PR 12 merged\n"));
    case.tick(&[]);
    assert_eq!(case.dispatch_state(&id), "Done");
}

/// A mailbox `[REPORT … ] failed` moves its dispatch to Failed.
#[test]
fn a_mailbox_report_failed_moves_the_dispatch() {
    let case = Case::new("mail-failed");
    let id = case.dispatch("alpha");
    case.mail("0001", &format!("[REPORT alpha {id}] started\n"));
    case.mail(
        "0002",
        &format!("[REPORT alpha {id}] failed the gate never passes\n"),
    );
    case.tick(&[]);
    assert_eq!(case.dispatch_state(&id), "Failed");
}

/// `[HANDOVER <repo>]` from a controller, and `[REPORT … ] blocked handover <path>`, are recorded
/// as reports: the dispatch is Blocked with the hand-over as its reason.
#[test]
fn a_mailbox_handover_is_recorded_as_a_report() {
    let case = Case::new("mail-handover");
    let id = case.dispatch("alpha");
    case.mail("0001", &format!("[REPORT alpha {id}] started\n"));
    case.mail(
        "0002",
        &format!("[REPORT alpha {id}] blocked handover docs/handoff/2026-10-17-alpha.md\n"),
    );
    case.tick(&[]);
    assert_eq!(case.dispatch_state(&id), "Blocked");

    case.mail("0003", &format!("[REPORT alpha {id}] unblocked\n"));
    case.mail(
        "0004",
        "[HANDOVER alpha] context 210k: docs/handoff/2026-10-17-alpha-2.md\n",
    );
    case.tick(&[]);
    assert_eq!(case.dispatch_state(&id), "Blocked");
    let reasons: Vec<String> = case
        .log()
        .iter()
        .filter(|line| line["event"] == "blocked")
        .filter_map(|line| line["reason"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(
        reasons,
        [
            "handover docs/handoff/2026-10-17-alpha.md",
            "handover: context 210k: docs/handoff/2026-10-17-alpha-2.md"
        ]
    );
}

/// A `[NEED]` is delivered to the live controller of the repository it asks; with none it is
/// kept, and delivered once that repository's controller has started.
#[test]
fn a_mailbox_need_waits_for_the_other_repository_s_controller() {
    let case = Case::new("mail-need");
    let need = "[NEED alpha N-20261017-01] beta export the store id\n\nalpha reads it on start.\n";
    case.mail("0001", need);
    let first = case.tick(&[]);
    assert!(case.deliveries().is_empty(), "beta has no controller");
    assert_eq!(first["lines_handled"], 1);

    write(
        &case.charter("beta"),
        "# Charter: beta\n\nExport the store id.\n",
    );
    case.spawn("beta", FIRST);
    let second = case.tick(&[]);
    assert_eq!(actions(&second), ["relayed"]);
    assert_eq!(case.deliveries(), [("ex-beta".to_owned(), need.to_owned())]);
    assert_eq!(case.lines_of(&second)[0]["repo"], "beta");

    case.tick(&[]);
    assert_eq!(case.deliveries().len(), 1, "delivered once");
}

/// A `[DECISION-REQUEST]` is raised and, with no decider seam, escalated as class O
/// with the options its body lists, its recommendation as the default and a deadline 48 hours
/// out.
#[test]
fn a_mailbox_decision_request_is_raised_and_escalated() {
    let case = Case::new("mail-decision-request");
    case.mail(
        "0001",
        "[DECISION-REQUEST alpha R-7] Which store layout?\n\n\
         A) tree layout: one migration\n\
         B) keep the eventlog files: no cost now\n\n\
         Recommendation: B, it needs no migration\n",
    );
    let before = OffsetDateTime::now_utc();
    let tick = case.tick(&[]);
    assert_eq!(tick["lines_handled"], 1);
    let requests = case.view(&["decision", "requests"]);
    let request = requests
        .iter()
        .find(|row| row["request_id"] == "R-7")
        .unwrap_or_else(|| panic!("R-7 is raised: {requests:?}"));
    assert_eq!(request["state"], "Escalated");
    assert_eq!(request["escalation_class"], "O");
    assert_eq!(request["repository"], "alpha");
    assert_eq!(request["question"], "Which store layout?");
    assert_eq!(request["recommended_letters"], "B");
    assert_eq!(request["default_letters"], "B");
    let letters: Vec<&str> = request["lettered_options"]
        .as_array()
        .expect("lettered options")
        .iter()
        .filter_map(|option| option["letter"].as_str())
        .collect();
    assert_eq!(letters, ["A", "B"]);
    let deadline =
        OffsetDateTime::parse(request["deadline"].as_str().expect("a deadline"), &Rfc3339)
            .expect("an RFC 3339 deadline");
    let hours = (deadline - before).whole_hours();
    assert!((47..=48).contains(&hours), "{hours} hours");
}

/// With the decider seam (`story:decide-model-adapter`), a `[DECISION-REQUEST]` reaches
/// `decide::handle_request` whole: class C is raised with its rule, answered by the fake decider
/// and recorded with its name, and the `[DECISION]` line is delivered to the controller of the
/// repository that raised it (`decision-sent`).
#[test]
fn a_mailbox_decision_request_is_decided_and_sent_to_the_controller() {
    let case = Case::new("mail-decision-decided");
    case.spawn("alpha", FIRST);
    case.mail(
        "0001",
        "[DECISION-REQUEST alpha R-8] Which store layout?\n\n\
         A) tree layout: one migration\n\
         B) keep the eventlog files: no cost now\n\n\
         Recommendation: B, it needs no migration\n",
    );
    let tick = case.child(json!({"do": "lines", "decide": true}));
    assert_eq!(tick["lines_handled"], 1);
    assert_eq!(actions(&tick), ["decision-sent"]);
    let requests = case.view(&["decision", "requests"]);
    let request = requests
        .iter()
        .find(|row| row["request_id"] == "R-8")
        .unwrap_or_else(|| panic!("R-8 is raised: {requests:?}"));
    assert_eq!(request["state"], "Answered");
    assert_eq!(request["classification_rule"], "none");
    let decision = request["decision_id"]
        .as_str()
        .expect("the request names its decision")
        .to_owned();
    let decisions = case.view(&["decision", "decisions"]);
    let recorded = decisions
        .iter()
        .find(|row| row["decision_id"] == json!(decision))
        .unwrap_or_else(|| panic!("{decision} is recorded: {decisions:?}"));
    assert_eq!(recorded["choice"], "B");
    assert_eq!(recorded["decider_name"], "fake");
    assert_eq!(
        case.deliveries(),
        [(
            "ex-alpha".to_owned(),
            format!(
                "[DECISION R-8] B the fake decider answers the request's recommendation \
                 ({decision})"
            )
        )]
    );
    let written = case.lines_of(&tick);
    assert_eq!(written[0]["event"], "decision-sent");
    assert_eq!(written[0]["id"], "R-8");
    assert_eq!(written[0]["grant"], json!(decision));
    assert_eq!(written[0]["session"], "ex-alpha");
}

/// A `[RESOURCE]` is answered "use conductor build run", once.
#[test]
fn a_mailbox_resource_request_is_answered_use_conductor_build_run() {
    let case = Case::new("mail-resource");
    case.spawn("alpha", FIRST);
    case.mail("0001", "[RESOURCE alpha] disk 40G\n");
    let tick = case.tick(&[]);
    assert_eq!(actions(&tick), ["relayed"]);
    let deliveries = case.deliveries();
    assert_eq!(deliveries.len(), 1, "{deliveries:?}");
    assert_eq!(deliveries[0].0, "ex-alpha");
    assert!(
        deliveries[0].1.contains("use conductor build run"),
        "{deliveries:?}"
    );
    case.tick(&[]);
    assert_eq!(case.deliveries().len(), 1, "each file is handled once");
}

// ---------------------------------------------------------------------------------------------
// Cycle and daily
// ---------------------------------------------------------------------------------------------

/// The cycle runs `snapshot start-snapshot`, `publish-board` and `records check`, then dispatch
/// v1 spawns each Active repository with a charter and no running controller, up to
/// `controllers.max_working`, and writes one `cycle` line.
#[test]
fn run_once_runs_a_watch_tick_and_a_cycle_that_spawns_up_to_max_working() {
    let case = Case::new("once");
    case.configure("{provider: none}", "{max_working: 2}");
    for repo in ["alpha", "beta", "gamma"] {
        write(
            &case.charter(repo),
            &format!("# Charter: {repo}\n\nKeep main green.\n"),
        );
        case.ok(&[
            "repository",
            "mark-repository",
            "--repository",
            repo,
            "--activity",
            "Active",
            "--marked-by",
            "Operator",
            "--reason",
            "pilot",
        ]);
    }
    let output = case.conductor(&["run", "--once"]);
    assert!(output.status.success(), "{}", describe(&output));

    let ticks = case.view(&["session", "run-ticks"]);
    let kinds: Vec<&str> = ticks
        .iter()
        .filter_map(|tick| tick["kind"].as_str())
        .collect();
    assert_eq!(kinds, ["watch", "cycle"], "{ticks:?}");
    assert_eq!(case.launched(), ["ex-alpha", "ex-beta"]);
    let cycle = case
        .log()
        .into_iter()
        .find(|line| line["event"] == "cycle")
        .expect("a cycle line");
    assert_eq!(cycle["repo"], "-");
    assert_eq!(cycle["tick_id"], ticks[1]["tick_id"]);
    let summary = cycle["line"].as_str().expect("the cycle's line");
    for step in ["snapshot", "board", "records check", "spawned alpha, beta"] {
        assert!(summary.contains(step), "{summary}");
    }
    assert!(
        ticks[1]["actions"]
            .as_array()
            .is_some_and(|actions| actions.last() == Some(&json!("cycle"))),
        "{ticks:?}"
    );
    assert!(
        !case.view(&["snapshot", "snapshots"]).is_empty(),
        "the snapshot ran"
    );
}

/// The watch pass's own lines are handled: a `disk low` it reads writes `cleanup` in the watch
/// tick.
#[test]
fn run_once_handles_the_lines_of_its_watch_pass() {
    let case = Case::new("once-watch");
    case.readings(LOW);
    let output = case.conductor(&["run", "--once"]);
    assert!(output.status.success(), "{}", describe(&output));
    let ticks = case.view(&["session", "run-ticks"]);
    let watch = ticks
        .iter()
        .find(|tick| tick["kind"] == "watch")
        .expect("a watch tick");
    assert_eq!(watch["actions"], json!(["cleanup"]), "{ticks:?}");
    assert_eq!(watch["lines_handled"], 1);
    let cleanup = case
        .log()
        .into_iter()
        .find(|line| line["event"] == "cleanup")
        .expect("a cleanup line");
    assert_eq!(cleanup["tick_id"], watch["tick_id"]);
    assert_eq!(cleanup["line"], "disk low: 50G free on /");
}

/// A `merge: human` repository at `controllers.max_open_reviews` pull requests waiting for review
/// waits; the cycle spawns the others.
#[test]
fn a_merge_human_repository_at_its_review_limit_is_not_spawned() {
    let case = Case::new("merge-human");
    case.configure("{provider: none}", "{max_working: 4, max_open_reviews: 2}");
    write(
        &case.charter("alpha"),
        "# Charter: alpha\n\nKeep main green.\n\nmerge: human\n",
    );
    write(
        &case.charter("beta"),
        "# Charter: beta\n\nKeep main green.\n\nmerge: human\n",
    );
    let tick = case.child(json!({
        "do": "cycle",
        "reviews": {"alpha": 2, "beta": 1},
        "program": case.bin().join("conductor-steps"),
    }));
    assert_eq!(case.launched(), ["ex-beta"], "{tick}");
    let steps = fs::read_to_string(case.fake().join("steps")).expect("the steps ran");
    assert_eq!(
        steps.lines().collect::<Vec<_>>(),
        [
            "snapshot start-snapshot",
            "snapshot publish-board",
            "records check"
        ]
    );
    let cycle = case.lines_of(&tick);
    assert_eq!(events(&cycle), ["cycle"]);
    let summary = cycle[0]["line"].as_str().expect("the cycle's line");
    assert!(
        summary.contains("alpha waits: 2/2 pull requests"),
        "{summary}"
    );
    assert!(summary.contains("spawned beta"), "{summary}");
}

/// The daily tick computes the previous day's metrics and writes `REPORT.md` with deviations
/// only.
#[test]
fn the_daily_tick_computes_metrics_and_writes_the_brief() {
    let case = Case::new("daily");
    let tick = case.child(json!({"do": "daily", "day": "2026-10-16"}));
    let report = fs::read_to_string(case.records().join("REPORT.md")).expect("REPORT.md");
    assert!(report.starts_with("# Report 2026-10-16\n"), "{report}");
    assert!(!report.contains("meets its target"), "{report}");
    assert!(!case.view(&["metric", "metrics"]).is_empty(), "computed");
    let ticks = case.view(&["session", "run-ticks"]);
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0]["kind"], "daily");
    assert_eq!(ticks[0]["tick_id"], tick["tick_id"]);
}

// ---------------------------------------------------------------------------------------------
// Ticks, unit, doctor, config
// ---------------------------------------------------------------------------------------------

/// `session record-run-tick` records a tick, `session run-ticks` lists it, and a negative count
/// is refused naming `InvalidLineCount`.
#[test]
fn a_tick_is_recorded_and_listed() {
    let case = Case::new("record-tick");
    let id = case.ok(&[
        "session",
        "record-run-tick",
        "--kind",
        "cycle",
        "--started-at",
        "2026-10-17T08:00:00Z",
        "--lines-handled",
        "0",
        "--actions",
        r#"["cycle"]"#,
    ]);
    let ticks = case.view(&["session", "run-ticks"]);
    assert_eq!(ticks.len(), 1);
    assert_eq!(ticks[0]["tick_id"], id.trim());
    assert_eq!(ticks[0]["state"], "Recorded");
    assert_eq!(ticks[0]["kind"], "cycle");
    assert_eq!(ticks[0]["started_at"], "2026-10-17T08:00:00Z");
    assert_eq!(ticks[0]["lines_handled"], 0);
    assert_eq!(ticks[0]["actions"], json!(["cycle"]));

    let refused = case.conductor(&[
        "session",
        "record-run-tick",
        "--kind",
        "watch",
        "--started-at",
        "2026-10-17T08:00:00Z",
        "--lines-handled",
        "-1",
        "--actions",
        "[]",
    ]);
    assert_eq!(refused.status.code(), Some(1), "{}", describe(&refused));
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("InvalidLineCount"),
        "{}",
        describe(&refused)
    );
    assert_eq!(case.view(&["session", "run-ticks"]).len(), 1);
}

/// `run --install` starts `conductor_<instance>-run.service` through `systemd-run`, in the
/// instance's slice, running this binary's `run` with the instance's config.
#[test]
fn install_starts_the_user_unit_through_systemd_run() {
    let case = Case::new("install");
    case.configure("{provider: systemd}", "{max_working: 4}");
    let output = case.conductor(&["run", "--install"]);
    assert!(output.status.success(), "{}", describe(&output));
    let args = fs::read_to_string(case.fake().join("systemd-run.args")).expect("systemd-run ran");
    let args: Vec<&str> = args.lines().collect();
    let config = format!("CONDUCTOR_CONFIG={}", case.config().display());
    for expected in [
        "--user",
        "--unit",
        "conductor_example-org-run.service",
        "--slice",
        "conductor_ex.slice",
        "CONDUCTOR_INSTANCE=example-org",
        config.as_str(),
    ] {
        assert!(args.contains(&expected), "{expected}: {args:?}");
    }
    assert_eq!(args.last(), Some(&"run"), "{args:?}");
    assert_eq!(
        args[args.len() - 2],
        case.program().display().to_string(),
        "{args:?}"
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("conductor_example-org-run.service"),
        "{}",
        describe(&output)
    );
}

/// Under the `none` provider `run --install` starts nothing and prints the command to run.
#[test]
fn install_under_the_none_provider_prints_the_command() {
    let case = Case::new("install-none");
    let output = case.conductor(&["run", "--install"]);
    assert!(output.status.success(), "{}", describe(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&format!("CONDUCTOR_CONFIG={}", case.config().display())),
        "{stdout}"
    );
    assert!(stdout.trim_end().ends_with(" run"), "{stdout}");
    assert!(!case.fake().join("systemd-run.args").exists());
}

/// `run --uninstall` stops the unit; with no unit loaded it says so and stops nothing.
#[test]
fn uninstall_stops_the_unit() {
    let case = Case::new("uninstall");
    let idle = case.conductor(&["run", "--uninstall"]);
    assert!(idle.status.success(), "{}", describe(&idle));
    assert!(
        String::from_utf8_lossy(&idle.stdout).contains("not installed"),
        "{}",
        describe(&idle)
    );
    let calls = fs::read_to_string(case.fake().join("systemctl.calls")).unwrap_or_default();
    assert!(!calls.contains(" stop "), "{calls}");

    write(&case.fake().join("unit.loaded"), "");
    let output = case.conductor(&["run", "--uninstall"]);
    assert!(output.status.success(), "{}", describe(&output));
    let calls = fs::read_to_string(case.fake().join("systemctl.calls")).expect("systemctl ran");
    assert!(
        calls
            .lines()
            .any(|call| call == "--user stop conductor_example-org-run.service"),
        "{calls}"
    );
}

/// `doctor` reports the run unit's state.
#[test]
fn doctor_reports_the_run_unit() {
    let case = Case::new("doctor");
    let line = |output: &Output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.starts_with("run unit:"))
            .map(str::to_owned)
    };
    let stopped = case.conductor(&["doctor"]);
    assert_eq!(
        line(&stopped).as_deref(),
        Some("run unit: conductor_example-org-run.service not running (inactive)"),
        "{}",
        describe(&stopped)
    );
    write(&case.fake().join("unit.loaded"), "");
    let running = case.conductor(&["doctor"]);
    assert_eq!(
        line(&running).as_deref(),
        Some("run unit: conductor_example-org-run.service running"),
        "{}",
        describe(&running)
    );
}

/// At most one of `--once`, `--install` and `--uninstall` is given.
#[test]
fn run_refuses_two_modes() {
    let case = Case::new("two-modes");
    let output = case.conductor(&["run", "--once", "--install"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(!case.fake().join("systemd-run.args").exists());
    assert!(case.view_or_none().is_none(), "nothing ran");
}

impl Case {
    /// The ticks recorded, or none when there is no store.
    fn view_or_none(&self) -> Option<Vec<Value>> {
        let output = self.conductor(&["session", "run-ticks", "--format", "json"]);
        output
            .status
            .success()
            .then(|| serde_json::from_slice(&output.stdout).expect("JSON"))
            .filter(|rows: &Vec<Value>| !rows.is_empty())
    }
}

/// The run daemon retires the conductor session: an instance with no `conductor` role is valid.
#[test]
fn an_instance_without_a_conductor_role_is_valid() {
    let case = Case::new("no-conductor-role");
    let output = case.conductor(&["config", "validate"]);
    assert!(output.status.success(), "{}", describe(&output));
}

/// The `ais` reader: headroom while the active account has reached no limit and no window is
/// full; none once one is.
#[test]
fn ais_reads_headroom_from_the_active_account() {
    let case = Case::new("ais");
    case.ok(&["spawn", "--repository", "alpha"]);
    case.live("alpha", FIRST, false);
    case.tick(&["usage limit: You've hit your limit; 1 sessions: ex-alpha"]);
    let account = |active: bool, used: f64| {
        json!({"active": active, "provider": "claude", "health": "ready",
               "usage": {"limit_reached": used >= 100.0,
                         "windows": [{"name": "five_hour", "used_percent": used},
                                     {"name": "seven_day", "used_percent": 40.0}]}})
    };
    write(
        &case.fake().join("ais.json"),
        &json!({"version": 1, "accounts": [account(true, 100.0), account(false, 0.0)]}).to_string(),
    );
    assert_eq!(
        actions(&case.child(json!({"do": "lines", "ais": true}))),
        Vec::<String>::new()
    );
    write(
        &case.fake().join("ais.json"),
        &json!({"version": 1, "accounts": [account(false, 100.0), account(true, 12.0)]})
            .to_string(),
    );
    assert_eq!(
        actions(&case.child(json!({"do": "lines", "ais": true}))),
        ["resumed"]
    );
}
