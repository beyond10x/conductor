//! `story:handover-restart-by-controller-id`: a hand-over the loop asked for is a request to one
//! controller, by its id, and to the session that controller ran then. `<state>/run/handover.json`
//! keeps both with the entry, and a later tick restarts only that controller's session: an entry
//! whose repository runs another controller now, under whatever session name, is dropped; so is
//! one whose session name a younger session holds (another id, or started after the delivery).
//! One that names no controller, as the binary before this story wrote it, stands for the
//! session its line names while the session list names it, and is dropped otherwise. A dropped
//! entry stops no session; the loop's own resume of a session is no start by hand.
//!
//! Each case is a [`Scratch`] directory: `home/`, the `HOME` the program runs with, holding the
//! checkouts root `example-org/` with the checkout `alpha` (a `.git/HEAD`) and `~/.claude.json`
//! marking it trusted; `records/` with the charter `charters/alpha.md`; `state/`; `roles/` with
//! the controller role's profile and settings; `bin/` with the stand-ins below, first on `PATH`;
//! `fake/`, what they read and write; and the config file `conductor.yaml` naming them, under
//! the session prefix `ex`, with the `none` provider and a context threshold of 200k.
//!
//! The stand-ins, written here and never committed: `claude` (`agents --json` prints the entries
//! under `fake/agents/`, `stop <id>` takes one away, a launch prints the id `s000000<n>-0000`;
//! every call is logged one line to `fake/claude.calls`, its first four words), `systemd-run`,
//! `systemctl`, `gh`, `notify-send` and `aep`, which do nothing.
//!
//! The ticks are run through the library in a child of this test binary ([`child`]), with a
//! [`Recorder`] in place of the real deliverer and a headroom read from `fake/headroom`; the
//! controllers are spawned and stopped by hand through the built binary. One case gives the
//! instance a token pool first ([`pool`]), in a child run too.

mod common;
mod scratch;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use conductor_cli::config;
use conductor_cli::run::{self, Headroom, Seams};
use conductor_cli::session::{Deliverer, Delivery};
use conductor_model::behaviour::PoolTokenStorage;
use conductor_model::config::InstanceName;
use conductor_model::operator::OperatorName;
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::secret::SecretName;
use conductor_model::token::{
    PoolTokenData, PoolTokenSnapshot, PoolTokenState, Seat, TokenId, TokenState,
};
use scratch::Scratch;
use serde_json::{Value, json};

/// The variable that holds a child run's instructions, as one JSON object.
const CHILD: &str = "CONDUCTOR_TEST_RUN_HANDOVER_CHILD";

/// The ids the stand-in prints for its first and second launch, and the eight characters the
/// watch names the first by.
const FIRST: &str = "s0000001-0000";
const FIRST_SHORT: &str = "s0000001";
const SECOND: &str = "s0000002-0000";

/// The session name of `alpha`'s controller.
const SESSION: &str = "ex-alpha";

/// Three hours, in nanoseconds: longer than `thresholds.silent_session`.
const LONG_AGO: u128 = 3 * 3600 * 1_000_000_000;

// ---------------------------------------------------------------------------------------------
// The child run
// ---------------------------------------------------------------------------------------------

/// The deliverer of a child run: appends `{"name", "text"}` of each delivery to
/// `fake/deliveries.jsonl`.
struct Recorder {
    dir: PathBuf,
}

impl Deliverer for Recorder {
    fn deliver(&self, name: &str, text: &str) -> Result<Delivery> {
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

/// The child run of this test binary ([`Case::tick`]); a plain run of the suite passes at once.
/// It fixes the case's config file as the process's instance and runs a watch tick over the
/// fixture lines; it prints the tick's summary as JSON and exits 0, or names the error and
/// exits 1.
#[test]
fn child() {
    let Ok(spec) = std::env::var(CHILD) else {
        return;
    };
    let spec: Value = serde_json::from_str(&spec).expect("the child's instructions are JSON");
    let file = PathBuf::from(std::env::var_os(config::CONFIG_VARIABLE).expect("a config file"));
    config::set_active(Some(&file)).expect("the case's config loads");
    if spec["pool"].as_bool() == Some(true) {
        pool();
        std::process::exit(0);
    }
    let fake = PathBuf::from(spec["fake"].as_str().expect("the fake directory"));
    let deliverer = Recorder { dir: fake.clone() };
    let headroom = FileHeadroom { dir: fake };
    let host = conductor_cli::confine::Host;
    let seams = Seams {
        providers: &host,
        deliverer: &deliverer,
        headroom: &headroom,
        handle_request: None,
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
    match run::handle_lines(None, None, &lines, &seams) {
        Ok(summary) => {
            let actions: Vec<&str> = summary
                .actions
                .iter()
                .map(|event| conductor_cli::records::dispatch_event(*event))
                .collect();
            println!(
                "{}",
                json!({"tick_id": summary.tick_id, "actions": actions})
            );
            std::process::exit(0);
        }
        Err(error) => {
            eprintln!("conductor: {error:#}");
            std::process::exit(1);
        }
    }
}

/// Puts two tokens into the instance's pool (`story:token-pool`), in the daemon's store where a
/// config file without a `daemon` key has it, under the case's home: a child run's work, the
/// store being opened as the case's instance.
fn pool() {
    let home = PathBuf::from(std::env::var_os("HOME").expect("the case's home"));
    let file = home.join(".b10x/conductor/conductor.sqlite");
    fs::create_dir_all(file.parent().expect("a parent")).expect("create the store's directory");
    let mut store = conductor_cli::store::open_url(
        &format!("sqlite:{}", file.display()),
        conductor_cli::daemon::TENANT,
    )
    .expect("open the daemon's store");
    for id in ["0001", "0002"] {
        PoolTokenStorage::put(
            &mut store,
            PoolTokenSnapshot {
                state: PoolTokenState::Available,
                data: PoolTokenData {
                    token_id: TokenId(Uuid(id.to_owned())),
                    instance: InstanceName("example-org".to_owned()),
                    owner: OperatorName("operator".to_owned()),
                    seat: Seat("operator@example-org.example".to_owned()),
                    secret: SecretName(format!("claude-token/example-org/{id}")),
                    token_state: TokenState::Ok,
                    limited_until: None,
                    agent: None,
                    contributed_at: Timestamp("2026-11-08T09:00:00Z".to_owned()),
                    actor: conductor_cli::actor::operator("operator"),
                },
            },
        );
    }
    drop(store);
}

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// A case directory of its own; see the module documentation.
struct Case {
    /// Held for the case's life: dropping it removes the directory of a case that passed.
    _scratch: Scratch,
    dir: PathBuf,
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("create a case directory");
    fs::write(path, text).expect("write a case file");
}

fn executable(path: &Path, text: &str) {
    write(path, text);
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Now, in nanoseconds since the epoch, as the loop dates a delivery.
fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("a clock after the epoch")
        .as_nanos()
}

impl Case {
    fn new(name: &str) -> Self {
        let scratch = Scratch::new(Path::new("run-handover").join(name));
        fs::create_dir_all(scratch.path().join("home")).expect("create the case's home");
        let dir = scratch.path().canonicalize().expect("the case's directory");
        let case = Self {
            _scratch: scratch,
            dir,
        };
        case.fixture();
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn checkout(&self) -> PathBuf {
        self.home().join("example-org/alpha")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn fake(&self) -> PathBuf {
        self.dir.join("fake")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn fixture(&self) {
        write(&self.checkout().join(".git/HEAD"), "ref: refs/heads/main\n");
        let mut projects = serde_json::Map::new();
        projects.insert(
            self.checkout().display().to_string(),
            json!({"hasTrustDialogAccepted": true}),
        );
        write(
            &self.home().join(".claude.json"),
            &json!({"numStartups": 3, "projects": projects}).to_string(),
        );
        write(
            &self.records().join("charters/alpha.md"),
            "# Charter: alpha\n\nKeep main green.\n\nmerge: controller\n",
        );
        write(
            &self.dir.join("roles/repo-controller.md"),
            "# Repository controller\n",
        );
        write(
            &self.dir.join("roles/controller-settings.json"),
            &json!({
                "env": {"CONDUCTOR_INSTANCE": "example-org"},
                "hooks": {"PreToolUse": [{"matcher": "*", "hooks": [{
                    "type": "command",
                    "command": "CONDUCTOR_INSTANCE=example-org conductor guard \
                                record-guard-decision --from-pre-tool-use || exit 2",
                }]}]},
            })
            .to_string(),
        );
        fs::create_dir_all(self.state()).expect("create the state directory");
        fs::create_dir_all(self.fake().join("agents")).expect("create the session list");
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
                 \x20   confinement: {{provider: none}}\n\
                 \x20   controllers: {{max_working: 4}}\n",
                root = self.home().join("example-org").display(),
                trees = self.dir.join("trees").display(),
                records = self.records().display(),
                state = self.state().display(),
                cache = self.dir.join("cache").display(),
                profile = self.dir.join("roles/repo-controller.md").display(),
                settings = self.dir.join("roles/controller-settings.json").display(),
            ),
        );
        let fake = self.fake();
        let fake = fake.display();
        let bin = self.dir.join("bin");
        executable(
            &bin.join("claude"),
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
                 printf 's%07d-0000\\n' \"$n\"\n"
            ),
        );
        for (name, text) in [
            ("systemd-run", "#!/bin/sh\nexit 0\n"),
            (
                "systemctl",
                "#!/bin/sh\n\
                 case \"$2\" in\n\
                 show) printf 'LoadState=not-found\\nActiveState=inactive\\n';;\n\
                 is-active) echo inactive; exit 3;;\n\
                 esac\n\
                 exit 0\n",
            ),
            ("gh", "#!/bin/sh\necho '[]'\n"),
            ("notify-send", "#!/bin/sh\nexit 0\n"),
            ("aep", "#!/bin/sh\necho '[]'\n"),
        ] {
            executable(&bin.join(name), text);
        }
    }

    fn path(&self) -> String {
        format!("{}:/usr/bin:/bin", self.dir.join("bin").display())
    }

    /// `conductor <args>` from the records directory, with the case's home, config file and
    /// stand-ins, which must succeed; its standard output.
    fn ok(&self, args: &[&str]) -> String {
        let output = common::conductor(self.home())
            .args(args)
            .current_dir(self.records())
            .env(config::CONFIG_VARIABLE, self.config())
            .env("PATH", self.path())
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs");
        assert!(output.status.success(), "{args:?}: {}", describe(&output));
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    /// A watch tick over `lines`, in a child of this test binary ([`child`]): its summary, and
    /// its journal lines (`conductor run: …`).
    fn tick(&self, lines: &[&str]) -> (Value, Vec<String>) {
        let program = PathBuf::from(common::conductor(self.home()).get_program());
        let spec = json!({"lines": lines, "fake": self.fake(), "program": program});
        let output = self.child(&spec);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let summary = stdout
            .lines()
            .rev()
            .find_map(|line| serde_json::from_str::<Value>(line).ok());
        let journal = String::from_utf8_lossy(&output.stderr)
            .lines()
            .filter(|line| line.starts_with("conductor run: "))
            .map(str::to_owned)
            .collect();
        match (output.status.success(), summary) {
            (true, Some(summary)) => (summary, journal),
            _ => panic!("the child run failed: {}", describe(&output)),
        }
    }

    /// Gives the instance a pool of two tokens ([`pool`]), before its controller is spawned: the
    /// spawn leases one of them.
    fn pool(&self) {
        let output = self.child(&json!({"pool": true}));
        assert!(output.status.success(), "{}", describe(&output));
    }

    /// A child run of this test binary ([`child`]) with the instructions `spec`, the case's
    /// home, config file and stand-ins, and a keychain file under the case's home.
    fn child(&self, spec: &Value) -> Output {
        Command::new(std::env::current_exe().expect("this test binary"))
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
            .env(
                conductor_cli::secret::TEST_KEYCHAIN_VARIABLE,
                self.home().join(".test-keychain.ron"),
            )
            .env(CHILD, spec.to_string())
            .current_dir(self.records())
            .stdin(Stdio::null())
            .output()
            .expect("the child runs")
    }

    /// The events of the lines a tick over `lines` wrote, and its journal lines.
    fn actions(&self, lines: &[&str]) -> (Vec<String>, Vec<String>) {
        let (summary, journal) = self.tick(lines);
        let actions = summary["actions"]
            .as_array()
            .expect("the tick's actions")
            .iter()
            .filter_map(|action| action.as_str().map(str::to_owned))
            .collect();
        (actions, journal)
    }

    /// Spawns `alpha`'s controller through the built binary and lists its session `id` live and
    /// working; the controller's id.
    fn spawn(&self, id: &str) -> String {
        self.ok(&["spawn", "--repository", "alpha"]);
        self.list(id, "busy", None);
        self.running()
            .expect("the spawn recorded a running controller")
    }

    /// Lists the live session `id` under the name [`SESSION`] with the status `status`, and
    /// with `started`, nanoseconds since the epoch, as the instant the list says it started at.
    fn list(&self, id: &str, status: &str, started: Option<u128>) {
        let mut entry = json!({
            "id": id, "sessionId": id, "name": SESSION, "pid": 4242,
            "cwd": self.checkout(), "status": status, "state": status,
        });
        if let Some(started) = started {
            entry["startedAt"] = json!(u64::try_from(started / 1_000_000).expect("milliseconds"));
        }
        write(
            &self.fake().join(format!("agents/{id}.json")),
            &entry.to_string(),
        );
    }

    /// Takes the session `id` out of the session list, as one ended outside conductor.
    fn unlist(&self, id: &str) {
        fs::remove_file(self.fake().join(format!("agents/{id}.json"))).expect("end the session");
    }

    /// The rows of `conductor controller all-controllers`.
    fn controllers(&self) -> Vec<Value> {
        serde_json::from_str(&self.ok(&["controller", "all-controllers", "--format", "json"]))
            .expect("the view prints JSON")
    }

    /// The id of `alpha`'s running controller.
    fn running(&self) -> Option<String> {
        self.controllers()
            .iter()
            .find(|row| row["repository"] == "alpha" && row["state"] == "Running")
            .and_then(|row| row["controller_id"].as_str().map(str::to_owned))
    }

    /// Every call of the `claude` stand-in, its first four words, in order.
    fn claude_calls(&self) -> Vec<String> {
        fs::read_to_string(self.fake().join("claude.calls"))
            .unwrap_or_default()
            .lines()
            .map(|line| line.trim_end().to_owned())
            .collect()
    }

    /// The sessions the stand-in was asked to stop, by id, in order.
    fn stopped(&self) -> Vec<String> {
        self.claude_calls()
            .iter()
            .filter_map(|call| call.strip_prefix("stop "))
            .map(|rest| rest.trim().to_owned())
            .collect()
    }

    /// How many sessions the stand-in launched.
    fn launches(&self) -> usize {
        self.claude_calls()
            .iter()
            .filter(|call| call.starts_with("--bg -n "))
            .count()
    }

    fn handover_file(&self) -> PathBuf {
        self.state().join("run/handover.json")
    }

    /// The hand-overs the ticks keep, `<state>/run/handover.json`.
    fn asked(&self) -> Value {
        fs::read_to_string(self.handover_file())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| json!({}))
    }

    /// Asks the running controller for its hand-over: a tick over a `context:` line at 230k,
    /// which measured the session [`FIRST`].
    fn ask(&self) -> String {
        let line = format!("context: alpha 230k tokens (session {FIRST_SHORT})");
        let (actions, journal) = self.actions(&[&line]);
        assert_eq!(actions, ["handover-sent"], "{journal:?}");
        line
    }

    /// Dates the kept hand-over's delivery back by `by` nanoseconds.
    fn date_back(&self, by: u128) {
        let mut asked = self.asked();
        let delivered: u128 = asked[SESSION]["delivered"]
            .as_str()
            .and_then(|at| at.parse().ok())
            .expect("the delivery's instant");
        asked[SESSION]["delivered"] = json!((delivered - by).to_string());
        write(&self.handover_file(), &asked.to_string());
    }

    /// Writes a hand-over file into the checkout, newer than any delivery made before: it is
    /// dated by the clock [`now`] reads, since the file system dates a write by a coarser one,
    /// which may lie before a delivery dated a moment earlier.
    fn hand_over_written(&self) -> PathBuf {
        let file = self.checkout().join(".git/handoff/2026-11-08-alpha.md");
        write(&file, "# Hand-over\n\nNext: the store migration.\n");
        fs::File::options()
            .write(true)
            .open(&file)
            .and_then(|written| written.set_modified(SystemTime::now()))
            .expect("date the hand-over file");
        file
    }
}

// ---------------------------------------------------------------------------------------------
// The cases
// ---------------------------------------------------------------------------------------------

/// The entry holds the controller's id and the session's, and the session it was asked of is
/// still restarted at its hand-over: what the list says of when it started lies before the
/// delivery.
#[test]
fn the_entry_names_the_controller_and_the_session_asked_and_restarts_that_session() {
    let case = Case::new("asked");
    let before = now();
    let controller = case.spawn(FIRST);
    case.list(FIRST, "busy", Some(before));
    let line = case.ask();

    let asked = case.asked();
    assert_eq!(asked[SESSION]["controller"], json!(controller), "{asked}");
    assert_eq!(asked[SESSION]["session_id"], FIRST_SHORT, "{asked}");
    assert_eq!(asked[SESSION]["repo"], "alpha", "{asked}");
    assert_eq!(asked[SESSION]["line"], json!(line), "{asked}");

    case.hand_over_written();
    case.list(FIRST, "idle", Some(before));
    let (actions, journal) = case.actions(&[]);
    assert_eq!(
        actions,
        ["handover", "sent", "controller-restarted"],
        "{journal:?}"
    );
    assert_eq!(case.stopped(), [FIRST]);
    assert_eq!(case.launches(), 2);
    assert!(case.asked().get(SESSION).is_none(), "{}", case.asked());
}

/// Acceptance 1: an entry for controller A, the repository's controller replaced by B under the
/// same session name. The tick stops nothing and forgets the entry, though B is idle and the
/// checkout holds a hand-over file newer than the delivery, and though `silent_session` passed
/// since the delivery.
#[test]
fn an_entry_of_a_replaced_controller_stops_nothing_and_is_forgotten() {
    for silent in [false, true] {
        let case = Case::new(if silent {
            "replaced-silent"
        } else {
            "replaced-file"
        });
        let first = case.spawn(FIRST);
        case.ask();

        // The restart by hand: the controller stopped, another spawned under the same name.
        case.ok(&[
            "controller",
            "stop-controller",
            "--controller-id",
            &first,
            "--reason",
            "restarted by hand",
        ]);
        let second = case.spawn(SECOND);
        assert_ne!(first, second, "another controller");
        let rows = case.controllers();
        assert!(
            rows.iter().all(|row| row["session_name"] == SESSION),
            "under the same session name: {rows:?}"
        );
        assert_eq!(case.stopped(), [FIRST], "the hand's own stop");
        assert_eq!(
            case.asked()[SESSION]["controller"],
            json!(first),
            "the entry is still the first controller's"
        );

        if silent {
            case.date_back(LONG_AGO);
        } else {
            case.hand_over_written();
        }
        case.list(SECOND, "idle", None);
        let (actions, journal) = case.actions(&[]);

        assert_eq!(actions, Vec::<String>::new(), "{journal:?}");
        assert_eq!(case.stopped(), [FIRST], "the tick stopped nothing");
        assert_eq!(case.launches(), 2, "and started nothing");
        assert_eq!(case.running(), Some(second), "the second runs on");
        assert!(
            case.asked().get(SESSION).is_none(),
            "the entry is forgotten: {}",
            case.asked()
        );
        assert!(
            journal
                .iter()
                .any(|line| line.contains(SESSION) && line.contains(&first)),
            "the journal names the session and the controller the entry was asked of: {journal:?}"
        );
    }
}

/// Acceptance 2: the loop never stops a session started after the entry's delivery. The
/// controller's record is the one asked, still Running; its session was ended and started again
/// outside conductor under the same name. A session of another id, and one the list says
/// started after the delivery, are both left alone, with a hand-over file newer than the
/// delivery and with `silent_session` passed; the entry is forgotten.
#[test]
fn a_session_started_after_the_delivery_is_never_stopped() {
    for (name, id, silent) in [
        ("younger-other-id", SECOND, false),
        ("younger-other-id-silent", SECOND, true),
        ("younger-same-id", FIRST, false),
        ("younger-same-id-silent", FIRST, true),
    ] {
        let case = Case::new(name);
        let controller = case.spawn(FIRST);
        case.ask();
        if silent {
            case.date_back(LONG_AGO);
        } else {
            case.hand_over_written();
        }
        // Ended and started again by hand, outside conductor: no controller record changes.
        case.unlist(FIRST);
        let started = (id == FIRST).then(now);
        case.list(id, "idle", started);

        let (actions, journal) = case.actions(&[]);
        assert_eq!(actions, Vec::<String>::new(), "{name}: {journal:?}");
        assert_eq!(case.stopped(), Vec::<String>::new(), "{name}: none stopped");
        assert_eq!(case.launches(), 1, "{name}: none started");
        assert_eq!(
            case.running(),
            Some(controller),
            "{name}: its controller runs on"
        );
        assert!(
            case.asked().get(SESSION).is_none(),
            "{name}: the entry is forgotten: {}",
            case.asked()
        );
        assert!(
            journal.iter().any(|line| line.contains(SESSION)),
            "{name}: the journal names the session: {journal:?}"
        );
    }
}

/// An entry the binary before this story wrote names no controller; its `line` names the
/// session it measured. After an upgrade the entry stands for that session while the session
/// list names it under the controller's session name: it is restarted at its hand-over, as
/// before the upgrade. Tied to no such session (it is gone, or another one runs under the name)
/// or to one that started after the delivery, the entry is forgotten with one journal line and
/// nothing is stopped, whatever its hand-over file and its age say.
#[test]
fn an_entry_that_names_no_controller_stands_only_for_the_session_its_line_names() {
    // The session that holds the name at the tick, whether the list says it started after the
    // delivery, and what the journal says of a dropped entry; none said: the entry stands.
    for (name, holder, younger, said) in [
        ("no-controller-tied", Some(FIRST), false, None),
        (
            "no-controller-gone",
            None,
            false,
            Some("names no controller"),
        ),
        (
            "no-controller-other",
            Some(SECOND),
            false,
            Some("names no controller"),
        ),
        (
            "no-controller-younger",
            Some(FIRST),
            true,
            Some("started after it was asked"),
        ),
    ] {
        let case = Case::new(name);
        let controller = case.spawn(FIRST);
        let delivered = now() - LONG_AGO;
        write(
            &case.handover_file(),
            &json!({SESSION: {
                "repo": "alpha",
                "delivered": delivered.to_string(),
                "line": format!("context: alpha 230k tokens (session {FIRST_SHORT})"),
            }})
            .to_string(),
        );
        case.hand_over_written();
        case.unlist(FIRST);
        if let Some(holder) = holder {
            case.list(holder, "idle", younger.then(now));
        }

        let (actions, journal) = case.actions(&[]);
        let Some(said) = said else {
            assert_eq!(
                actions,
                ["handover", "sent", "controller-restarted"],
                "{name}: {journal:?}"
            );
            assert_eq!(case.stopped(), [FIRST], "{name}: the session asked");
            assert_eq!(case.launches(), 2, "{name}: started again");
            assert!(
                case.asked().get(SESSION).is_none(),
                "{name}: {}",
                case.asked()
            );
            continue;
        };
        assert_eq!(actions, Vec::<String>::new(), "{name}: {journal:?}");
        assert_eq!(case.stopped(), Vec::<String>::new(), "{name}: none stopped");
        assert_eq!(case.launches(), 1, "{name}: none started");
        assert_eq!(case.running(), Some(controller), "{name}");
        assert!(
            case.asked().get(SESSION).is_none(),
            "{name}: the entry is forgotten: {}",
            case.asked()
        );
        let lines: Vec<&String> = journal
            .iter()
            .filter(|line| line.contains(SESSION) && line.contains(said))
            .collect();
        assert_eq!(lines.len(), 1, "{name}: one journal line: {journal:?}");
    }
}

/// An entry the binary before this story wrote, for a session a usage limit stopped: the list
/// keeps that session without a `pid`, so the entry is still tied to it, and the tick with
/// headroom spawns the controller afresh, as before the upgrade, and does not resume it.
#[test]
fn a_limited_session_asked_by_an_earlier_version_is_spawned_afresh() {
    let case = Case::new("limited-earlier-version");
    case.spawn(FIRST);
    write(
        &case.fake().join(format!("agents/{FIRST}.json")),
        &json!({
            "id": FIRST, "sessionId": FIRST, "name": SESSION, "cwd": case.checkout(),
            "status": "blocked", "state": "blocked",
        })
        .to_string(),
    );
    write(&case.fake().join("headroom"), "no");
    let (marked, _) = case.actions(&["usage limit: You've hit your limit; 1 sessions: ex-alpha"]);
    assert_eq!(marked, ["usage-limit"]);
    write(
        &case.handover_file(),
        &json!({SESSION: {
            "repo": "alpha",
            "delivered": now().to_string(),
            "line": format!("context: alpha 230k tokens (session {FIRST_SHORT})"),
        }})
        .to_string(),
    );

    write(&case.fake().join("headroom"), "yes");
    let (actions, journal) = case.actions(&[]);
    assert_eq!(actions, ["sent", "controller-restarted"], "{journal:?}");
    assert_eq!(case.launches(), 2, "a fresh spawn: {journal:?}");
    assert!(
        !case
            .claude_calls()
            .iter()
            .any(|call| call.starts_with("--bg --resume")),
        "{:?}",
        case.claude_calls()
    );
}

/// The loop's own resume is no start by hand. `conductor leave` names the session asked for the
/// loop (`<state>/run/tokens-withdrawn.json`), which ends it and resumes it under the same id
/// while the hand-over is pending. The list then dates the session by the resumed process,
/// after the delivery: the request stands and the session is restarted at its hand-over. A
/// session the list says started after the loop's resume was started by hand: the request is
/// dropped and the loop stops nothing more.
#[test]
fn the_loops_own_resume_keeps_the_request_and_a_later_start_by_hand_drops_it() {
    for (name, by_hand) in [("own-resume", false), ("own-resume-then-by-hand", true)] {
        let case = Case::new(name);
        let controller = case.spawn(FIRST);
        case.ask();
        write(
            &case.state().join("run/tokens-withdrawn.json"),
            &json!([SESSION]).to_string(),
        );
        let resumed = now();
        let (actions, journal) = case.actions(&[]);
        assert_eq!(actions, ["resumed"], "{name}: {journal:?}");
        assert_eq!(case.stopped(), [FIRST], "{name}: the loop's own end");
        assert!(
            case.claude_calls()
                .iter()
                .any(|call| call == &format!("--bg --resume {FIRST} -n")),
            "{name}: {:?}",
            case.claude_calls()
        );
        assert_eq!(
            case.asked()[SESSION]["controller"],
            json!(controller),
            "{name}: the request is kept across the resume: {journal:?}"
        );

        case.hand_over_written();
        case.list(FIRST, "idle", Some(if by_hand { now() } else { resumed }));
        let (actions, journal) = case.actions(&[]);
        if by_hand {
            assert_eq!(actions, Vec::<String>::new(), "{name}: {journal:?}");
            assert_eq!(case.stopped(), [FIRST], "{name}: none stopped by the tick");
            assert_eq!(case.launches(), 1, "{name}: none started");
            assert!(
                case.asked().get(SESSION).is_none(),
                "{name}: {}",
                case.asked()
            );
        } else {
            assert_eq!(
                actions,
                ["handover", "sent", "controller-restarted"],
                "{name}: {journal:?}"
            );
            assert_eq!(case.stopped(), [FIRST, FIRST], "{name}: restarted");
            assert_eq!(case.launches(), 2, "{name}: started again");
        }
        assert!(case.running().is_some(), "{name}: a controller runs");
    }
}

/// The loop's other resume, on another token of the pool (`story:token-pool`): a usage limit
/// stops the session asked while another token is Available, and the same tick resumes it under
/// its id. The request to it is kept, and the session, dated by the resumed process, is
/// restarted at its hand-over.
#[test]
fn a_resume_on_another_token_keeps_the_request_to_the_session_asked() {
    let case = Case::new("own-resume-other-token");
    case.pool();
    let controller = case.spawn(FIRST);
    case.ask();
    // Stopped at a usage limit: the list keeps it without a pid.
    write(
        &case.fake().join(format!("agents/{FIRST}.json")),
        &json!({
            "id": FIRST, "sessionId": FIRST, "name": SESSION, "cwd": case.checkout(),
            "status": "blocked", "state": "blocked",
        })
        .to_string(),
    );
    let resumed = now();
    let (actions, journal) =
        case.actions(&["usage limit: You've hit your limit; 1 sessions: ex-alpha"]);
    assert_eq!(actions, ["usage-limit", "resumed"], "{journal:?}");
    assert_eq!(
        case.asked()[SESSION]["controller"],
        json!(controller),
        "the request is kept across the resume: {journal:?}"
    );

    case.hand_over_written();
    case.list(FIRST, "idle", Some(resumed));
    let (actions, journal) = case.actions(&[]);
    assert_eq!(
        actions,
        ["handover", "sent", "controller-restarted"],
        "{journal:?}"
    );
    assert_eq!(case.stopped(), [FIRST], "the session asked");
    assert_eq!(case.launches(), 2, "started again");
}

/// A session a usage limit stopped is spawned afresh, not resumed, only when the hand-over was
/// asked of its own controller: with an entry of another controller under its session name it
/// is resumed from its transcript, as one never asked. The entry is written between the ticks,
/// so the tick that has headroom reads it before anything forgot it.
#[test]
fn a_limited_session_with_an_entry_of_another_controller_is_resumed_not_respawned() {
    let case = Case::new("limited-other-controller");
    case.spawn(FIRST);
    // Stopped at a usage limit: the list keeps it without a pid.
    write(
        &case.fake().join(format!("agents/{FIRST}.json")),
        &json!({
            "id": FIRST, "sessionId": FIRST, "name": SESSION, "cwd": case.checkout(),
            "status": "blocked", "state": "blocked",
        })
        .to_string(),
    );
    write(&case.fake().join("headroom"), "no");
    let (marked, _) = case.actions(&["usage limit: You've hit your limit; 1 sessions: ex-alpha"]);
    assert_eq!(marked, ["usage-limit"]);
    write(
        &case.handover_file(),
        &json!({SESSION: {
            "repo": "alpha",
            "delivered": now().to_string(),
            "controller": "00000000-0000-4000-8000-000000000000",
            "session_id": "s0000009",
        }})
        .to_string(),
    );

    write(&case.fake().join("headroom"), "yes");
    let (actions, journal) = case.actions(&[]);
    assert_eq!(actions, ["resumed"], "{journal:?}");
    assert_eq!(case.launches(), 1, "no fresh spawn");
    assert!(
        case.claude_calls()
            .iter()
            .any(|call| call == &format!("--bg --resume {FIRST} -n")),
        "{:?}",
        case.claude_calls()
    );
}

// ---------------------------------------------------------------------------------------------
// Adversary pass 1 (wave 51, unit U2): cases added against the unit, not part of its commit
// ---------------------------------------------------------------------------------------------

impl Case {
    /// The first line of each delivery the ticks made, in order (`fake/deliveries.jsonl`).
    fn delivered(&self) -> Vec<String> {
        fs::read_to_string(self.fake().join("deliveries.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter_map(|delivery| {
                delivery["text"]
                    .as_str()
                    .and_then(|text| text.lines().next())
                    .map(str::to_owned)
            })
            .collect()
    }
}

/// Acceptance 2 through the other handler that acts on an entry: `limited()` runs before
/// `handovers()` in a tick, and restarts a session a usage limit marked when a hand-over was
/// asked of it. The session was asked, then marked by a usage limit; it was ended and started
/// again by hand under the same name, after the delivery, once the limit lifted; the next tick
/// has headroom. The loop stops nothing: the session that holds the name started after the
/// delivery.
#[test]
fn adversary_a_session_started_after_the_delivery_is_not_stopped_by_the_usage_limit_path() {
    let case = Case::new("adversary-limited-younger");
    let controller = case.spawn(FIRST);
    case.ask();
    write(&case.fake().join("headroom"), "no");
    let (marked, journal) =
        case.actions(&["usage limit: You've hit your limit; 1 sessions: ex-alpha"]);
    assert_eq!(marked, ["usage-limit"], "{journal:?}");
    assert_eq!(
        case.asked()[SESSION]["controller"],
        json!(controller),
        "the request stands while the session asked holds the name: {journal:?}"
    );

    // Ended and started again by hand, outside conductor, after the delivery.
    case.unlist(FIRST);
    case.list(FIRST, "busy", Some(now()));
    write(&case.fake().join("headroom"), "yes");
    let (actions, journal) = case.actions(&[]);

    assert_eq!(
        case.stopped(),
        Vec::<String>::new(),
        "a session started after the delivery is never stopped: {actions:?} {journal:?}"
    );
    assert_eq!(case.launches(), 1, "none started: {actions:?} {journal:?}");
    assert_eq!(case.running(), Some(controller), "its controller runs on");
}

/// A request to the controller's own live session is not dropped because another live session
/// carries the same name: one started by hand in a directory outside the instance's checkouts,
/// which the watch never names. The request stands whichever of the two the session list
/// prints first.
#[test]
fn adversary_a_session_of_the_same_name_elsewhere_does_not_drop_the_request() {
    for (name, stray) in [
        ("adversary-stray-listed-after", "z0000000-0000"),
        ("adversary-stray-listed-before", "a0000000-0000"),
    ] {
        let case = Case::new(name);
        let before = now();
        let controller = case.spawn(FIRST);
        case.list(FIRST, "busy", Some(before));
        case.ask();
        write(
            &case.fake().join(format!("agents/{stray}.json")),
            &json!({
                "id": stray, "sessionId": stray, "name": SESSION, "pid": 5151,
                "cwd": case.dir.join("elsewhere"), "status": "idle", "state": "idle",
                "startedAt": u64::try_from(before / 1_000_000).expect("milliseconds"),
            })
            .to_string(),
        );

        let (actions, journal) = case.actions(&[]);
        assert_eq!(actions, Vec::<String>::new(), "{name}: {journal:?}");
        assert_eq!(
            case.asked()[SESSION]["controller"],
            json!(controller),
            "{name}: the request to the controller's own session stands: {journal:?}"
        );
    }
}

/// A request the binary before this story kept, to the session the running controller still
/// runs: its line names that session, the session list says it started before the delivery,
/// its hand-over file is written and it is idle. The watch names a session's context once
/// (`tests/watch.rs`, the cases of `context-reported`), so no later `context:` line asks
/// again: within two ticks the controller is restarted at its hand-over, or asked again.
#[test]
fn adversary_a_request_of_an_earlier_version_to_the_running_session_is_not_lost() {
    let case = Case::new("adversary-earlier-version");
    let before = now();
    case.spawn(FIRST);
    case.list(FIRST, "busy", Some(before));
    write(
        &case.handover_file(),
        &json!({SESSION: {
            "repo": "alpha",
            "delivered": now().to_string(),
            "line": format!("context: alpha 230k tokens (session {FIRST_SHORT})"),
        }})
        .to_string(),
    );
    case.hand_over_written();
    case.list(FIRST, "idle", Some(before));

    let (first, first_journal) = case.actions(&[]);
    let (second, second_journal) = case.actions(&[]);

    let restarted = case.stopped() == [FIRST] && case.launches() == 2;
    let asked_again = case
        .delivered()
        .iter()
        .any(|line| line.starts_with("[HANDOVER alpha]"));
    assert!(
        restarted || asked_again,
        "the controller asked before the upgrade is neither restarted at its hand-over nor asked \
         again: ticks {first:?} {second:?}, journal {first_journal:?} {second_journal:?}, \
         deliveries {:?}, kept {}",
        case.delivered(),
        case.asked()
    );
}

// ---------------------------------------------------------------------------------------------
// Adversary pass 2 (wave 51, unit U2): cases added against the correction, not part of its commit
// ---------------------------------------------------------------------------------------------

impl Case {
    /// Puts a `claude` before the stand-in that fails `agents --json` while `fake/list-fails`
    /// exists, as a session list that cannot be read.
    fn list_may_fail(&self) {
        let bin = self.dir.join("bin");
        fs::rename(bin.join("claude"), bin.join("claude-stand-in")).expect("keep the stand-in");
        executable(
            &bin.join("claude"),
            &format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = agents ] && [ -e '{fails}' ]; then echo 'no list' >&2; exit 1; fi\n\
                 exec '{real}' \"$@\"\n",
                fails = self.fake().join("list-fails").display(),
                real = bin.join("claude-stand-in").display(),
            ),
        );
    }

    /// The sessions a usage limit marked, `<state>/run/limited.json`.
    fn limited(&self) -> Value {
        fs::read_to_string(self.state().join("run/limited.json"))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| json!({}))
    }
}

/// The case the correction names as untested: the session asked is resumed by hand under its id
/// after the delivery, and the loop ends and resumes it (`conductor leave`) before any tick saw
/// the hand's start. The stamp makes the request stand; the session the loop then stops at its
/// hand-over is the one asked, by its id, so the conversation that holds the request.
#[test]
fn adversary2_a_start_by_hand_the_loop_resumes_before_a_tick_keeps_the_request() {
    let case = Case::new("adversary2-hand-then-loop");
    let before = now();
    let controller = case.spawn(FIRST);
    case.list(FIRST, "busy", Some(before));
    case.ask();
    // Resumed by hand under the same id, after the delivery.
    case.list(FIRST, "busy", Some(now()));
    write(
        &case.state().join("run/tokens-withdrawn.json"),
        &json!([SESSION]).to_string(),
    );
    let resumed = now();
    let (actions, journal) = case.actions(&[]);
    assert_eq!(actions, ["resumed"], "{journal:?}");
    assert_eq!(
        case.asked()[SESSION]["controller"],
        json!(controller),
        "the request stands across the loop's resume: {journal:?}"
    );
    assert!(case.asked()[SESSION]["resumed"].is_string(), "{}", case.asked());

    case.hand_over_written();
    case.list(FIRST, "idle", Some(resumed));
    let (actions, journal) = case.actions(&[]);
    assert_eq!(
        actions,
        ["handover", "sent", "controller-restarted"],
        "{journal:?}"
    );
    assert_eq!(
        case.stopped(),
        [FIRST, FIRST],
        "only the session asked, by its id: the loop's end, then its hand-over"
    );
}

/// Acceptance 1 against the stamp: an entry for controller A, the repository's controller
/// replaced by B under the same session name, and B's session resumed by the loop, so the entry
/// carries a stamp newer than B's start. The hand-over path stops nothing and forgets the entry.
#[test]
fn adversary2_a_replaced_controller_resumed_by_the_loop_stops_nothing() {
    let case = Case::new("adversary2-replaced-resumed");
    let first = case.spawn(FIRST);
    case.ask();
    case.ok(&[
        "controller",
        "stop-controller",
        "--controller-id",
        &first,
        "--reason",
        "restarted by hand",
    ]);
    let second = case.spawn(SECOND);
    case.list(SECOND, "busy", Some(now()));
    write(
        &case.state().join("run/tokens-withdrawn.json"),
        &json!([SESSION]).to_string(),
    );
    let (actions, journal) = case.actions(&[]);
    assert_eq!(actions, ["resumed"], "{journal:?}");
    assert_eq!(case.stopped(), [FIRST, SECOND], "the hand's stop, the loop's end");
    assert!(
        case.asked().get(SESSION).is_none(),
        "the entry of the first controller is forgotten: {}",
        case.asked()
    );
    assert!(
        journal
            .iter()
            .any(|line| line.contains(SESSION) && line.contains(&first)),
        "{journal:?}"
    );

    case.hand_over_written();
    case.list(SECOND, "idle", None);
    let (actions, journal) = case.actions(&[]);
    assert_eq!(actions, Vec::<String>::new(), "{journal:?}");
    assert_eq!(case.stopped(), [FIRST, SECOND], "the tick stopped nothing");
    assert_eq!(case.running(), Some(second), "the second runs on");
}

/// The correction's claim for the usage-limit path: with headroom back and a hand-over kept for
/// the session, a session list that cannot be read stops nothing, starts nothing and leaves the
/// session marked and its request kept; the first tick that reads the list restarts it.
#[test]
fn adversary2_a_limited_session_whose_request_cannot_be_told_stays_marked() {
    let case = Case::new("adversary2-limited-list-fails");
    case.list_may_fail();
    let controller = case.spawn(FIRST);
    case.ask();
    write(
        &case.fake().join(format!("agents/{FIRST}.json")),
        &json!({
            "id": FIRST, "sessionId": FIRST, "name": SESSION, "cwd": case.checkout(),
            "status": "blocked", "state": "blocked",
        })
        .to_string(),
    );
    write(&case.fake().join("headroom"), "no");
    let (marked, journal) =
        case.actions(&["usage limit: You've hit your limit; 1 sessions: ex-alpha"]);
    assert_eq!(marked, ["usage-limit"], "{journal:?}");

    write(&case.fake().join("list-fails"), "");
    write(&case.fake().join("headroom"), "yes");
    let (actions, journal) = case.actions(&[]);
    assert_eq!(actions, Vec::<String>::new(), "{journal:?}");
    assert_eq!(case.launches(), 1, "none started: {journal:?}");
    assert!(
        case.limited().get(SESSION).is_some(),
        "still marked: {} {journal:?}",
        case.limited()
    );
    assert_eq!(
        case.asked()[SESSION]["controller"],
        json!(controller),
        "the request is kept: {journal:?}"
    );
    assert!(
        journal
            .iter()
            .any(|line| line.contains(&format!("hand-over of {SESSION}"))),
        "one journal line says why: {journal:?}"
    );

    fs::remove_file(case.fake().join("list-fails")).expect("the list reads again");
    let (actions, journal) = case.actions(&[]);
    assert_eq!(actions, ["sent", "controller-restarted"], "{journal:?}");
    assert_eq!(case.launches(), 2, "spawned afresh: {journal:?}");
    assert!(case.limited().get(SESSION).is_none(), "{}", case.limited());
}
