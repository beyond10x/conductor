//! `story:store-shared`: one shared database per deployment, opened through Eventlog's sqlite and
//! postgres adapters. Two instance runtimes and the daemon write one SQLite file at once and each
//! instance reads one ordered log of its own; every event names its instance; `conductor store
//! migrate` reads today's eventlog tree once into the database; `conductor doctor` names the
//! backend and whether it answers; a `conductor.config/2` file that writes a per-instance `store:`
//! is refused by its YAML path.
//!
//! The Postgres case runs only when `CONDUCTOR_TEST_POSTGRES_URL` names a database it may write;
//! otherwise it says it skipped, by name.

mod active;
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::thread;

use conductor_cli::store;
use conductor_model::behaviour::GoalStorage;
use conductor_model::direction::{GoalData, GoalId, GoalSnapshot, GoalState};
use conductor_model::operator::{Actor, ActorKind, OperatorName};
use eventlog_core::{
    CommandMeta, EventStore as _, Expected, MAX_READ_LIMIT, NewEvent, StreamId, TenantId,
    new_event_id, request_hash,
};
use eventlog_sqlite::SqliteEventStore;
use eventlog_tree::TreeEventStore;
use serde_json::json;
use time::OffsetDateTime;

/// The variable that names a Postgres database the Postgres case may write.
const POSTGRES_VARIABLE: &str = "CONDUCTOR_TEST_POSTGRES_URL";

/// How many goals each writer records.
const EACH: usize = 12;

fn case(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("store_shared")
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the case");
    }
    fs::create_dir_all(&root).expect("create the case");
    root
}

fn goal(id: &str) -> GoalSnapshot {
    GoalSnapshot {
        state: GoalState::Draft,
        data: GoalData {
            actor: Actor {
                kind: ActorKind::Human,
                operator: Some(OperatorName("operator".to_owned())),
                agent: None,
            },
            goal_id: GoalId(id.to_owned()),
            title: format!("goal {id}"),
            exit_evidence: "a test".to_owned(),
            evidence: None,
            reason: None,
        },
    }
}

/// Records `EACH` goals named `<instance>-<n>` through one handle opened on `url` as `instance`.
fn write_goals(url: &str, instance: &str) {
    let mut handle = store::open_url(url, instance).expect("the shared store opens");
    for n in 0..EACH {
        GoalStorage::put(&mut handle, goal(&format!("{instance}-{n:02}")));
        handle.check().expect("every write is kept");
    }
}

/// The goal ids `instance` reads from the store at `url`, in the order the store answers them.
fn goal_ids(url: &str, instance: &str) -> Vec<String> {
    let handle = store::open_url(url, instance).expect("the shared store opens");
    let ids = GoalStorage::list(&handle)
        .into_iter()
        .map(|goal| goal.data.goal_id.0)
        .collect();
    handle.check().expect("every read succeeds");
    ids
}

fn expected(instance: &str) -> Vec<String> {
    (0..EACH).map(|n| format!("{instance}-{n:02}")).collect()
}

/// Two instance runtimes and the daemon, each its own handle on one SQLite file, write at once;
/// each instance then reads exactly its own goals, in the order it wrote them.
fn writers_at_once(url: &str) {
    thread::scope(|scope| {
        for instance in ["alpha", "beta", "daemon"] {
            scope.spawn(move || write_goals(url, instance));
        }
    });
    for instance in ["alpha", "beta", "daemon"] {
        assert_eq!(goal_ids(url, instance), expected(instance), "{instance}");
    }
}

#[test]
fn two_runtimes_and_the_daemon_on_one_sqlite_file_keep_one_ordered_log_per_instance() {
    active::isolate();
    let root = case("sqlite");
    let file = root.join("shared.sqlite");
    let url = format!("sqlite:{}", file.display());
    writers_at_once(&url);

    // Every event names its instance: its tenant and its subject are the instance's name.
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    let log = runtime
        .block_on(SqliteEventStore::open_existing(
            file.to_str().expect("a UTF-8 path"),
            store::PREFIX,
        ))
        .expect("the database the store wrote opens");
    for instance in ["alpha", "beta", "daemon"] {
        let tenant = TenantId::new(instance).expect("a tenant");
        let page = runtime
            .block_on(log.read_feed(&tenant, 0, MAX_READ_LIMIT))
            .expect("the feed reads");
        assert_eq!(page.events.len(), EACH, "{instance}");
        for event in &page.events {
            assert_eq!(event.tenant.as_str(), instance);
            assert_eq!(event.subject, instance);
        }
    }
}

#[test]
fn one_runtime_on_postgres_when_conductor_test_postgres_url_is_set() {
    active::isolate();
    let Ok(url) = std::env::var(POSTGRES_VARIABLE) else {
        eprintln!(
            "skipped one_runtime_on_postgres_when_conductor_test_postgres_url_is_set: \
             {POSTGRES_VARIABLE} is not set"
        );
        return;
    };
    let alpha = format!("alpha-{}", std::process::id());
    write_goals(&url, &alpha);
    assert_eq!(goal_ids(&url, &alpha), expected(&alpha));
}

#[test]
fn a_store_url_that_is_neither_sqlite_nor_postgres_is_refused() {
    active::isolate();
    let Err(error) = store::open_url("mysql://localhost/x", "alpha") else {
        panic!("a mysql URL opened a store");
    };
    assert!(
        format!("{error:#}").contains("sqlite:<path> or postgres://"),
        "{error:#}"
    );
}

fn run(home: &Path, args: &[&str]) -> Output {
    common::conductor(home)
        .current_dir(home)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

fn said(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout {}\nstderr {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Writes one goal into a tree store at `state/tree/`, as the build before this one kept it.
fn tree_with_a_goal(state: &Path) {
    let root = state.join("tree");
    fs::create_dir_all(&root).expect("create the tree");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    let log = runtime
        .block_on(TreeEventStore::open(&root))
        .expect("a tree store");
    let tenant = TenantId::new("conductor").expect("a tenant");
    let stream =
        StreamId::new(tenant, "conductor.direction.Goal", "migrated-goal").expect("a stream");
    let events = [NewEvent::new(
        "stored",
        1,
        json!({
            "actor": {"kind": "Human", "operator": "operator"},
            "state": "Draft",
            "goal_id": "migrated-goal",
            "title": "a goal kept in the tree",
            "exit_evidence": "a test",
            "evidence": null,
            "reason": null,
        }),
    )
    .expect("an event")];
    let id = new_event_id();
    let meta = CommandMeta {
        idempotency_key: id.clone(),
        request_hash: request_hash(&events).expect("a hash"),
        subject: "conductor".to_owned(),
        actor: "conductor".to_owned(),
        request_id: id.clone(),
        trace_id: id,
        causation_id: None,
        causation_depth: 0,
        occurred_at: OffsetDateTime::now_utc(),
        claim: None,
    };
    runtime
        .block_on(log.append(&stream, Expected::NoStream, &events, &meta))
        .expect("the goal is kept in the tree");
}

#[test]
fn store_migrate_reads_the_tree_once_into_the_database() {
    let root = case("migrate");
    let state = root.join("state");
    tree_with_a_goal(&state);
    let state_arg = state.to_str().expect("a UTF-8 path");

    let migrated = run(&root, &["--state-dir", state_arg, "store", "migrate"]);
    assert!(migrated.status.success(), "{}", said(&migrated));
    assert!(
        state.join(store::STORE_FILE).is_file(),
        "the database is written: {}",
        said(&migrated)
    );

    // The database holds the goal: the tree is not read again.
    fs::remove_dir_all(state.join("tree")).expect("remove the tree");
    let goals = run(
        &root,
        &[
            "--state-dir",
            state_arg,
            "goal",
            "goals",
            "--format",
            "json",
        ],
    );
    assert!(goals.status.success(), "{}", said(&goals));
    assert!(
        String::from_utf8_lossy(&goals.stdout).contains("migrated-goal"),
        "{}",
        said(&goals)
    );

    // Read once: a second run is refused and copies nothing.
    tree_with_a_goal(&state);
    let again = run(&root, &["--state-dir", state_arg, "store", "migrate"]);
    assert_eq!(again.status.code(), Some(1), "{}", said(&again));
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("AlreadyMigrated"),
        "{}",
        said(&again)
    );
}

#[test]
fn store_migrate_without_a_tree_names_source_missing() {
    let root = case("migrate-missing");
    let state = root.join("state");
    fs::create_dir_all(&state).expect("create the state directory");
    let output = run(
        &root,
        &[
            "--state-dir",
            state.to_str().expect("a UTF-8 path"),
            "store",
            "migrate",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("SourceMissing"),
        "{}",
        said(&output)
    );
}

fn config_with(root: &Path, daemon_store: &str, instance_extra: &str) -> PathBuf {
    let file = root.join("conductor.yaml");
    fs::write(
        &file,
        format!(
            "version: conductor.config/2\n\
             daemon: {{port: 7400, store: \"{daemon_store}\"}}\n\
             globals: {{}}\n\
             instances:\n\
             \x20 - name: one\n\
             \x20   session_prefix: one\n\
             \x20   placement: example-box\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: ~/one, trees: ~/one-trees}}\n\
             {instance_extra}"
        ),
    )
    .expect("write the config");
    file
}

#[test]
fn doctor_names_the_store_backend_and_whether_it_answers() {
    let root = case("doctor");
    let file = root.join("shared.sqlite");
    let config = config_with(&root, &format!("sqlite:{}", file.display()), "");

    // Doctor creates nothing: a file that is not there does not answer.
    let output = run(
        &root,
        &["--config", config.to_str().expect("a UTF-8 path"), "doctor"],
    );
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.lines().any(|line| line.starts_with("store: sqlite ")
            && line.contains("(does not answer: no database at")),
        "{}",
        said(&output)
    );
    assert!(!file.exists(), "doctor created the database");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a runtime");
    drop(
        runtime
            .block_on(SqliteEventStore::open(
                file.to_str().expect("a UTF-8 path"),
                store::PREFIX,
            ))
            .expect("the database is created"),
    );
    let output = run(
        &root,
        &["--config", config.to_str().expect("a UTF-8 path"), "doctor"],
    );
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.lines()
            .any(|line| line.starts_with("store: sqlite ") && line.ends_with("(answers)")),
        "{}",
        said(&output)
    );

    let unreachable = case("doctor-postgres");
    let config = config_with(
        &unreachable,
        "postgres://conductor@127.0.0.1:9/conductor",
        "",
    );
    let output = run(
        &unreachable,
        &["--config", config.to_str().expect("a UTF-8 path"), "doctor"],
    );
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.lines()
            .any(|line| line.starts_with("store: postgres ") && line.contains("(does not answer")),
        "{}",
        said(&output)
    );
}

#[test]
fn a_v2_file_writing_a_per_instance_store_is_refused_by_its_yaml_path() {
    let root = case("instance-store");
    let config = config_with(
        &root,
        "sqlite:~/conductor.sqlite",
        "\x20   store: \"sqlite:~/one.sqlite\"\n",
    );
    let output = run(
        &root,
        &[
            "--config",
            config.to_str().expect("a UTF-8 path"),
            "config",
            "validate",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("instances[0].store") && text.contains("daemon.store"),
        "{}",
        said(&output)
    );
}

#[test]
fn config_migrate_drops_the_per_instance_store_key_with_a_note() {
    let root = case("migrate-config");
    let file = root.join("conductor.yaml");
    fs::write(
        &file,
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: one\n\
         \x20   session_prefix: one\n\
         \x20   store: \"sqlite:~/one.sqlite\"\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: ~/one, trees: ~/one-trees}\n",
    )
    .expect("write the config");
    let output = run(
        &root,
        &[
            "--config",
            file.to_str().expect("a UTF-8 path"),
            "config",
            "migrate",
        ],
    );
    assert!(output.status.success(), "{}", said(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("instances[0].store: dropped"),
        "{}",
        said(&output)
    );
    let text = fs::read_to_string(&file).expect("read the config back");
    assert!(!text.contains("store:"), "{text}");
    assert!(text.contains("session_prefix: one"), "{text}");
}

/// No per-instance copy, no sync, no restore: the binding places no `instance restore` and no
/// `store sync`, no source names a `store/events` route, and `cadence.sync` is no key of the file.
#[test]
fn no_sync_no_restore_and_no_store_events_route_exist() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let binding = fs::read_to_string(manifest.join("cli.yaml")).expect("read the binding");
    for path in ["[instance, restore]", "[store, sync]"] {
        assert!(!binding.contains(path), "the binding places {path}");
    }
    fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read a source directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    sources(&manifest.join("src"), &mut files);
    let routed: Vec<&PathBuf> = files
        .iter()
        .filter(|file| fs::read_to_string(file).is_ok_and(|text| text.contains("store/events")))
        .collect();
    assert!(routed.is_empty(), "a store/events route: {routed:?}");

    let root = case("no-sync");
    let config = config_with(&root, "sqlite:~/conductor.sqlite", "");
    let file = fs::read_to_string(&config).expect("read the config");
    fs::write(
        &config,
        file.replace("globals: {}", "globals: {cadence: {sync: PT60S}}"),
    )
    .expect("write the config");
    let output = run(
        &root,
        &[
            "--config",
            config.to_str().expect("a UTF-8 path"),
            "config",
            "validate",
        ],
    );
    assert_eq!(output.status.code(), Some(1), "{}", said(&output));
    assert!(
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .contains("cadence.sync"),
        "{}",
        said(&output)
    );
}
