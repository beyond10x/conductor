//! `story:config-globals`: `conductor.config/2`'s precedence (a key an instance or a role writes
//! overrides `globals`, which overrides conductor's built-in default), `conductor config show
//! --effective` with the level each key came from, `conductor config migrate` from
//! `conductor.config/1`, the daemon refusing a `/1` file, the `secret:<name>` rule for a
//! secret-bearing key, and `conductor config put`, the CLI path of `PUT /v1/config`.
//!
//! Each case is a directory of its own in this test target's temporary directory, holding a home
//! directory `home/` the binary runs with; no case reads the real home's config.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

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
            .join("config_globals")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.dir.join(relative);
        fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        fs::write(&path, text).expect("write a case file");
        path
    }

    /// `conductor --config <file> <words>` with the case's home and a git identity.
    fn run(&self, file: &Path, words: &[&str]) -> Output {
        common::conductor(self.home())
            .current_dir(&self.dir)
            .arg("--config")
            .arg(file)
            .args(words)
            .envs(identity())
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }
}

fn identity() -> [(&'static str, &'static str); 4] {
    [
        ("GIT_AUTHOR_NAME", "example"),
        ("GIT_AUTHOR_EMAIL", "example@example.invalid"),
        ("GIT_COMMITTER_NAME", "example"),
        ("GIT_COMMITTER_EMAIL", "example@example.invalid"),
    ]
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A `conductor.config/2` file whose globals, instance and role each set a key.
fn layered(case: &Case) -> PathBuf {
    case.write(
        "conductor.yaml",
        "version: conductor.config/2\n\
         daemon: {store: \"sqlite:~/daemon/conductor.sqlite\"}\n\
         globals:\n\
         \x20 cadence: {cycle: 3h, watch: 9m}\n\
         \x20 thresholds: {disk_low: 20G}\n\
         \x20 roles:\n\
         \x20   - {role: controller, harness: claude, model: global-model, profile: ~/lead.md}\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   placement: systemd\n\
         \x20   session_prefix: one\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n\
         \x20   cadence: {watch: 7m}\n\
         \x20   thresholds: {disk_low: 30G}\n\
         \x20   roles:\n\
         \x20     - {role: controller, model: role-model}\n",
    )
}

/// The effective rows of `instance` as `config show --effective --format json` prints them.
fn effective(case: &Case, file: &Path, instance: &str) -> Vec<Value> {
    let output = case.run(
        file,
        &[
            "config",
            "show",
            "--effective",
            "--instance",
            instance,
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let rows: Value = serde_json::from_slice(&output.stdout).expect("the rows are JSON");
    rows.as_array().expect("a list of rows").clone()
}

/// The row of `path` for `role` (none for an instance-wide key).
fn row<'r>(rows: &'r [Value], role: Option<&str>, path: &str) -> &'r Value {
    rows.iter()
        .find(|row| row["path"] == path && row["role"].as_str() == role)
        .unwrap_or_else(|| panic!("no row {role:?} {path}: {rows:#?}"))
}

#[test]
fn effective_shows_every_key_with_the_level_it_came_from() {
    let case = Case::new("effective");
    let file = layered(&case);
    let rows = effective(&case, &file, "one");
    let home = case.home();
    for (role, path, value, level) in [
        (None, "cadence.cycle", "3h".to_owned(), "Global"),
        (None, "cadence.watch", "7m".to_owned(), "Instance"),
        (None, "cadence.ci", "900s".to_owned(), "Default"),
        (None, "thresholds.disk_low", "30G".to_owned(), "Instance"),
        (None, "placement", "systemd".to_owned(), "Instance"),
        (Some("controller"), "model", "role-model".to_owned(), "Role"),
        (Some("controller"), "harness", "claude".to_owned(), "Global"),
        (
            Some("controller"),
            "profile",
            home.join("lead.md").display().to_string(),
            "Global",
        ),
        (
            Some("controller"),
            "kind",
            "claude-code".to_owned(),
            "Default",
        ),
    ] {
        let found = row(&rows, role, path);
        assert_eq!(found["value"], value, "{role:?} {path}: {found}");
        assert_eq!(found["level"], level, "{role:?} {path}: {found}");
        assert_eq!(found["instance"], "one", "{found}");
        let key = format!("one/{}/{path}", role.unwrap_or("-"));
        assert_eq!(found["setting_key"], key, "{found}");
    }

    // The text names each key once, its value and its level.
    let output = case.run(&file, &["config", "show", "--effective"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let text = stdout(&output);
    for line in [
        "cadence.cycle: 3h (global)",
        "cadence.watch: 7m (instance)",
        "cadence.ci: 900s (default)",
        "roles.controller.model: role-model (role)",
    ] {
        assert!(
            text.lines().any(|written| written == line),
            "no line {line:?}:\n{text}"
        );
    }

    // The effective config every command reads is the same: the global cadence reaches the
    // instance.
    let output = case.run(&file, &["config", "show", "--format", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let shown: Value = serde_json::from_slice(&output.stdout).expect("json");
    let instance = &shown["instances"][0];
    assert_eq!(instance["cadence"]["cycle"], "3h", "{instance:#}");
    assert_eq!(instance["roles"][0]["model"], "role-model", "{instance:#}");
    assert_eq!(instance["roles"][0]["harness"], "claude", "{instance:#}");
}

#[test]
fn migrate_converts_a_first_version_file_once_keeping_its_steps() {
    let case = Case::new("migrate");
    let file = case.write(
        "conductor.yaml",
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   dashboard_port: 7320\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n\
         \x20   store: tree\n\
         \x20   confinement: {mode: strict, provider: systemd, limits: {memory_max: 8G}}\n",
    );
    case.write(
        "home/.b10x/conductor/host/host.yaml",
        "version: conductor.host-config/1\n\
         roles:\n\
         \x20 - {role: maintainer, harness: claude, model: opus}\n",
    );
    let before = fs::read_to_string(&file).expect("read");

    let checked = case.run(&file, &["config", "migrate", "--check"]);
    assert_eq!(checked.status.code(), Some(0), "{}", describe(&checked));
    assert_eq!(
        fs::read_to_string(&file).expect("read"),
        before,
        "--check wrote"
    );

    let output = case.run(&file, &["config", "migrate"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let notes = stdout(&output);
    for note in [
        "version: conductor.config/2",
        "instances[0].dashboard_port: dropped",
        "instances[0].confinement.mode: dropped",
        "instances[0].confinement.provider: dropped",
        "instances[0].confinement.limits: moved to instances[0].limits",
        "instances[0].store: dropped",
        "host.roles: moved to globals.roles",
        "one: session_prefix one",
    ] {
        assert!(notes.contains(note), "no note {note:?}:\n{notes}");
    }
    let written: Value =
        serde_yaml::from_str(&fs::read_to_string(&file).expect("read")).expect("yaml");
    let instance = &written["instances"][0];
    assert_eq!(written["version"], "conductor.config/2", "{written:#}");
    assert_eq!(instance["placement"], "systemd", "{written:#}");
    assert_eq!(instance["limits"]["memory_max"], "8G", "{written:#}");
    assert_eq!(instance["session_prefix"], "one", "{written:#}");
    for gone in ["dashboard_port", "confinement", "store"] {
        assert!(instance.get(gone).is_none(), "{gone} kept: {written:#}");
    }
    assert_eq!(
        written["globals"]["roles"][0]["role"], "maintainer",
        "{written:#}"
    );
    let validated = case.run(&file, &["config", "validate"]);
    assert_eq!(validated.status.code(), Some(0), "{}", describe(&validated));

    // Once: a second run finds a `/2` file and converts nothing.
    let again = case.run(&file, &["config", "migrate"]);
    assert_eq!(again.status.code(), Some(0), "{}", describe(&again));
    assert!(!stdout(&again).contains("dropped"), "{}", describe(&again));
}

#[test]
fn the_daemon_refuses_a_first_version_file_naming_migrate() {
    let case = Case::new("daemon-first");
    let file = case.write(
        "conductor.yaml",
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   session_prefix: one\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n",
    );
    let output = common::conductor(case.home())
        .arg("--config")
        .arg(&file)
        .arg("--state-dir")
        .arg(case.dir.join("daemon"))
        .args(["daemon", "serve"])
        .stdin(Stdio::null())
        .output()
        .expect("run conductor");
    let said = stderr(&output);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert_eq!(said.lines().count(), 1, "{said}");
    assert!(said.contains("conductor config migrate"), "{said}");
    assert!(said.contains("conductor.config/1"), "{said}");
}

#[test]
fn a_secret_bearing_key_takes_only_a_secret_name() {
    let case = Case::new("secret");
    let instance = "instances:\n\
                    \x20 - name: one\n\
                    \x20   placement: systemd\n\
                    \x20   session_prefix: one\n\
                    \x20   sources: [{github: example-org}]\n\
                    \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n";
    let plain = case.write(
        "plain.yaml",
        &format!(
            "version: conductor.config/2\n\
             daemon: {{store: \"postgres://conductor:hunter2@db.example/conductor\"}}\n\
             {instance}"
        ),
    );
    let output = case.run(&plain, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = stderr(&output);
    assert!(said.contains("daemon.store: "), "{said}");
    assert!(said.contains("secret:<name>"), "{said}");
    assert!(!said.contains("hunter2"), "the value is echoed: {said}");

    let named = case.write(
        "named.yaml",
        &format!(
            "version: conductor.config/2\n\
             daemon: {{store: \"postgres://conductor:secret:db-password@db.example/conductor\"}}\n\
             {instance}"
        ),
    );
    let output = case.run(&named, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));

    // Any key named as a secret holder takes only `secret:<name>`, wherever it is.
    let keyed = case.write(
        "keyed.yaml",
        &format!("version: conductor.config/2\n{instance}\x20   forge_token: plain-words\n"),
    );
    let output = case.run(&keyed, &["config", "validate"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = stderr(&output);
    assert!(
        said.lines()
            .any(|line| line.contains("instances[0].forge_token: ")
                && line.contains("secret:<name>")),
        "{said}"
    );
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .envs(identity())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        describe(&output)
    );
    stdout(&output).trim().to_owned()
}

#[test]
fn put_validates_writes_commits_and_records_the_change() {
    let case = Case::new("put");
    let file = layered(&case);
    let repo = case.dir.clone();
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "conductor.yaml"]);
    git(&repo, &["commit", "-q", "-m", "the config"]);
    let first = git(&repo, &["rev-parse", "HEAD"]);

    let output = case.run(
        &file,
        &[
            "config",
            "put",
            "--level",
            "instance",
            "--instance",
            "one",
            "--path",
            "thresholds.silent_session",
            "--value",
            "45m",
            "--reason",
            "silence alarms fire on long gates",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let written: Value =
        serde_yaml::from_str(&fs::read_to_string(&file).expect("read")).expect("yaml");
    assert_eq!(
        written["instances"][0]["thresholds"]["silent_session"], "45m",
        "{written:#}"
    );
    let head = git(&repo, &["rev-parse", "HEAD"]);
    assert_ne!(head, first, "the change was not committed");
    let message = git(&repo, &["log", "-1", "--format=%B"]);
    assert!(
        message.contains("silence alarms fire on long gates"),
        "{message}"
    );
    assert!(message.contains("operator"), "{message}");
    assert!(message.contains("thresholds.silent_session"), "{message}");

    let rows = effective(&case, &file, "one");
    let found = row(&rows, None, "thresholds.silent_session");
    assert_eq!(found["value"], "45m", "{found}");
    assert_eq!(found["level"], "Instance", "{found}");

    let url = format!(
        "sqlite:{}",
        case.home().join("daemon/conductor.sqlite").display()
    );
    let store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the daemon's store opens");
    let revisions = store.list_config_revisions();
    assert_eq!(revisions.len(), 1, "{revisions:#?}");
    let revision = &revisions[0].data;
    assert_eq!(revision.commit, head);
    assert_eq!(revision.path, "thresholds.silent_session");
    assert_eq!(revision.value.as_deref(), Some("45m"));
    assert_eq!(revision.reason, "silence alarms fire on long gates");
    assert_eq!(
        revision.actor.operator.as_ref().map(|name| name.0.as_str()),
        Some("operator")
    );
    drop(store);

    // An invalid value changes nothing and names the key.
    let text = fs::read_to_string(&file).expect("read");
    let output = case.run(
        &file,
        &[
            "config",
            "put",
            "--level",
            "global",
            "--path",
            "cadence.watch",
            "--value",
            "soon",
            "--reason",
            "faster",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let said = stderr(&output);
    assert_eq!(said.lines().count(), 1, "{said}");
    assert!(said.contains("globals.cadence.watch"), "{said}");
    assert_eq!(fs::read_to_string(&file).expect("read"), text);
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), head);

    // A level that does not fit its scope changes nothing.
    let output = case.run(
        &file,
        &[
            "config",
            "put",
            "--level",
            "global",
            "--instance",
            "one",
            "--path",
            "cadence.watch",
            "--value",
            "5m",
            "--reason",
            "faster",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        stderr(&output).contains("LevelScopeMismatch"),
        "{}",
        describe(&output)
    );
    assert_eq!(fs::read_to_string(&file).expect("read"), text);

    let store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the daemon's store opens");
    assert_eq!(store.list_config_revisions().len(), 1);
}

/// story:instance-rules-text: an instance's `standing_rules` are written by `POST /v1/instances`
/// and `PUT /v1/instances/{name}` alone. `conductor config put` refuses the key, set or removed,
/// at the instance level and at the global and role levels, with one line that names the key's
/// path and those routes and quotes no part of the offered value; the file, its commit and the
/// store's revisions stay as they were. Another key of the instance still goes through.
#[test]
fn config_put_refuses_the_standing_rules_and_takes_another_instance_key() {
    let case = Case::new("put-standing-rules");
    let layered = fs::read_to_string(layered(&case)).expect("read");
    let file = case.write(
        "conductor.yaml",
        &format!("{layered}\x20   standing_rules: \"No private name in a commit.\\n\"\n"),
    );
    let repo = case.dir.clone();
    git(&repo, &["init", "-q"]);
    git(&repo, &["add", "conductor.yaml"]);
    git(&repo, &["commit", "-q", "-m", "the config"]);
    let head = git(&repo, &["rev-parse", "HEAD"]);
    let text = fs::read_to_string(&file).expect("read");
    let offered = "Zq9 marker rule offered through the config put";
    let instance = ["--level", "instance", "--instance", "one"];
    let role = [
        "--level",
        "role",
        "--instance",
        "one",
        "--role",
        "controller",
    ];
    let global = ["--level", "global"];
    for (what, level, value) in [
        ("set at the instance", &instance[..], Some(offered)),
        ("removed at the instance", &instance[..], None),
        ("set at a role", &role[..], Some(offered)),
        ("set under globals", &global[..], Some(offered)),
    ] {
        let mut words = vec!["config", "put"];
        words.extend_from_slice(level);
        words.extend_from_slice(&["--path", "standing_rules", "--reason", "a second writer"]);
        if let Some(value) = value {
            words.extend_from_slice(&["--value", value]);
        }
        let output = case.run(&file, &words);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{what}: {}",
            describe(&output)
        );
        let said = stderr(&output);
        assert_eq!(said.lines().count(), 1, "{what}: {said}");
        for words in [
            "standing_rules",
            "POST /v1/instances",
            "PUT /v1/instances/{name}",
        ] {
            assert!(
                said.contains(words),
                "{what}: the refusal names {words:?}: {said}"
            );
        }
        assert!(
            !said.contains("Zq9"),
            "{what}: the refusal quotes the value: {said}"
        );
        assert_eq!(fs::read_to_string(&file).expect("read"), text, "{what}");
        assert_eq!(git(&repo, &["rev-parse", "HEAD"]), head, "{what}");
    }
    let url = format!(
        "sqlite:{}",
        case.home().join("daemon/conductor.sqlite").display()
    );
    let store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the daemon's store opens, so that nothing recorded is shown");
    assert_eq!(
        store.list_config_revisions().len(),
        0,
        "nothing is recorded"
    );
    drop(store);

    let output = case.run(
        &file,
        &[
            "config",
            "put",
            "--level",
            "instance",
            "--instance",
            "one",
            "--path",
            "thresholds.silent_session",
            "--value",
            "45m",
            "--reason",
            "another key",
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let written: Value =
        serde_yaml::from_str(&fs::read_to_string(&file).expect("read")).expect("yaml");
    assert_eq!(
        written["instances"][0]["thresholds"]["silent_session"],
        "45m"
    );
    assert_eq!(
        written["instances"][0]["standing_rules"], "No private name in a commit.\n",
        "{written:#}"
    );
    // The store read above is the one a change is recorded in: the change taken is there.
    let store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the daemon's store opens");
    assert_eq!(
        store.list_config_revisions().len(),
        1,
        "the change taken is recorded in the store read above"
    );
}

/// The function `PUT /v1/config` calls: an agent changes only a key the tuning grant names, to a
/// value inside its range, and no change is written at `Default`.
#[test]
fn put_config_holds_an_agent_to_the_tuning_grant() {
    use conductor_cli::config::put::put_config;
    use conductor_model::agent::{AgentKindName, AgentName};
    use conductor_model::config::{InstanceName, Level, PutConfig, PutConfigOutcome, RoleName};
    use conductor_model::operator::{Actor, ActorKind, AgentActor};

    let case = Case::new("put-agent");
    let file = case.write(
        "conductor.yaml",
        "version: conductor.config/2\n\
         globals:\n\
         \x20 tuning: {keys: [{path: thresholds.cpu_reserve, min: 0, max: 4}]}\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   placement: systemd\n\
         \x20   session_prefix: one\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n",
    );
    let url = format!("sqlite:{}", case.dir.join("daemon.sqlite").display());
    let mut store = conductor_cli::store::open_url(&url, conductor_cli::daemon::TENANT)
        .expect("the store opens");
    let agent = Actor {
        kind: ActorKind::Agent,
        operator: None,
        agent: Some(AgentActor {
            instance: InstanceName("one".to_owned()),
            role: RoleName("tuner".to_owned()),
            name: AgentName("one-tuner".to_owned()),
            kind: AgentKindName("claude-code".to_owned()),
        }),
    };
    let change = |level: Level, path: &str, value: &str| PutConfig {
        actor: agent.clone(),
        level,
        instance: None,
        role: None,
        path: path.to_owned(),
        value: Some(value.to_owned()),
        reason: "tuning".to_owned(),
        changed_at: conductor_model::primitives::Timestamp("2026-11-08T09:00:00Z".to_owned()),
    };
    let text = fs::read_to_string(&file).expect("read");
    for (input, refused) in [
        (
            change(Level::Global, "cadence.watch", "5m"),
            "ActorNotHuman",
        ),
        (
            change(Level::Global, "thresholds.cpu_reserve", "9"),
            "ActorNotHuman",
        ),
        (
            change(Level::Default, "thresholds.cpu_reserve", "2"),
            "LevelScopeMismatch",
        ),
    ] {
        let answer = put_config(&file, &case.home(), &mut store, &input).expect("no fault");
        let refusal = answer.expect_err("refused");
        assert!(
            refusal.to_string().starts_with(refused),
            "{input:?}: {refusal}"
        );
        assert!(
            !matches!(refusal.outcome, PutConfigOutcome::Changed { .. }),
            "{refusal:?}"
        );
        assert_eq!(fs::read_to_string(&file).expect("read"), text);
    }
    let revision = put_config(
        &file,
        &case.home(),
        &mut store,
        &change(Level::Global, "thresholds.cpu_reserve", "2"),
    )
    .expect("no fault")
    .expect("inside the grant");
    assert_eq!(revision.commit, "", "the file is in no git checkout");
    let written: Value =
        serde_yaml::from_str(&fs::read_to_string(&file).expect("read")).expect("yaml");
    assert_eq!(written["globals"]["thresholds"]["cpu_reserve"], 2);
    assert_eq!(store.list_config_revisions().len(), 1);
}

/// `ConfigRevisionStorage::list`, named for the cases.
trait Revisions {
    fn list_config_revisions(&self) -> Vec<conductor_model::config::ConfigRevisionSnapshot>;
}

impl Revisions for conductor_cli::store::Store {
    fn list_config_revisions(&self) -> Vec<conductor_model::config::ConfigRevisionSnapshot> {
        conductor_model::behaviour::ConfigRevisionStorage::list(self)
    }
}
