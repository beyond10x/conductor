//! Adversary cases for `story:box-runtime-starts` (wave 49, U12, pass 1).
//!
//! Each case reads a config file as the docs write it (`config::parse`), asks `placement::request`
//! for the box as conductord does (the store URL as `daemon.store` writes it, `HOME`, a `PATH`
//! that finds nothing), and holds the request against substrate's lexical admission rules
//! (`substrate-wire` at the pinned revision: `validate_read_only_roots`,
//! `validate_read_only_files`, `validate_writable_roots`) or against what the docs of this unit
//! promise about the box. No daemon, box or unit runs.

mod active;

use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::placement::{self, BoxRequest};
use conductor_model::config::Instance;

fn case(name: &str) -> PathBuf {
    active::isolate();
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_box_mounts")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("home")).expect("create the case's home");
    dir
}

/// The instances of the config `text`, `~` read as `home`.
fn instances(text: &str, home: &Path) -> Vec<Instance> {
    conductor_cli::config::parse(text, home)
        .unwrap_or_else(|problems| panic!("the config is valid: {problems:?}"))
        .instances
}

/// The box conductord asks for: `HOME`, a `PATH` that finds no program, and the store URL as the
/// file writes it (`Daemon::service` hands `daemon.store` on unchanged).
fn request(instance: &Instance, file: &Path, home: &Path, store: &str) -> BoxRequest {
    let env = vec![
        ("HOME".to_owned(), home.display().to_string()),
        ("PATH".to_owned(), home.join("no-bin").display().to_string()),
        (
            conductor_cli::store::STORE_VARIABLE.to_owned(),
            store.to_owned(),
        ),
    ];
    placement::request(instance, file, Path::new("/usr/bin/conductor"), env)
}

/// Whether `path` is at or under a writable root of `request`.
fn writes(request: &BoxRequest, path: &Path) -> bool {
    request
        .writable_roots
        .iter()
        .any(|root| path.starts_with(&root.path))
}

/// substrate-wire's `is_absolute_canonical`.
fn canonical(path: &str) -> bool {
    path.starts_with('/')
        && !(path.len() > 1 && path.ends_with('/'))
        && !path.contains("//")
        && path
            .split('/')
            .skip(1)
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

/// substrate-wire's `is_path_beneath`.
fn beneath(path: &str, ancestor: &str) -> bool {
    path.strip_prefix(ancestor)
        .is_some_and(|rest| rest.starts_with('/'))
}

fn nested(a: &str, b: &str) -> bool {
    a == b || beneath(a, b) || beneath(b, a)
}

/// The refusal substrate's lexical rules answer `request` with, by its published code, or `None`
/// when they admit it. The rules are those of `substrate-wire` at the revision `Cargo.toml` pins.
fn substrate_refusal(request: &BoxRequest) -> Option<String> {
    for (index, root) in request.read_only_roots.iter().enumerate() {
        if !canonical(root) {
            return Some(format!("exec.read-only-root-invalid: {root}"));
        }
        if let Some(other) = request.read_only_roots[..index]
            .iter()
            .find(|other| nested(root, other))
        {
            return Some(format!("exec.read-only-root-invalid: {root} and {other}"));
        }
    }
    for (index, file) in request.read_only_files.iter().enumerate() {
        if !canonical(&file.host_path) || !canonical(&file.mount) {
            return Some(format!(
                "exec.read-only-file-path-invalid: {}",
                file.mount
            ));
        }
        if let Some(other) = request
            .read_only_roots
            .iter()
            .chain(
                request.read_only_files[..index]
                    .iter()
                    .map(|other| &other.mount),
            )
            .find(|other| nested(&file.mount, other))
        {
            return Some(format!(
                "exec.read-only-file-mount-overlap: {} and {other}",
                file.mount
            ));
        }
    }
    for (index, root) in request.writable_roots.iter().enumerate() {
        if !canonical(&root.path) {
            return Some(format!(
                "exec.writable-root-mount-invalid: {} at {:?}",
                root.name, root.path
            ));
        }
        if let Some(other) = request
            .read_only_roots
            .iter()
            .chain(request.read_only_files.iter().map(|file| &file.mount))
            .chain(
                request.writable_roots[..index]
                    .iter()
                    .map(|other| &other.path),
            )
            .find(|other| nested(&root.path, other))
        {
            return Some(format!(
                "exec.writable-root-mount-overlap: {} at {} and {other}",
                root.name, root.path
            ));
        }
    }
    None
}

/// The config of `website/docs/guides/configure-an-instance.md` § 1: the records under the
/// checkouts root, the trees elsewhere, `placement: linux` (a box).
const CONFIGURE_AN_INSTANCE: &str = "\
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/conductor.sqlite
globals:
  cadence: {cycle: 2h}
default: personal
instances:
  - name: personal
    placement: linux
    session_prefix: personal
    sources:
      - github: example-org
    checkouts:
      root: ~/example-org
      trees: ~/.local/state/worktree/trees/example-org
    records: ~/example-org/records
";

/// The config of `website/docs/getting-started.md` § 3.
const GETTING_STARTED: &str = "\
version: conductor.config/2
daemon:
  store: sqlite:~/.b10x/conductor/conductor.sqlite
instances:
  - name: home
    placement: systemd
    session_prefix: home
    sources:
      - local: ~/src
    checkouts:
      root: ~/src
      trees: ~/.local/state/worktree/trees/home
";

const STORE: &str = "sqlite:~/.b10x/conductor/conductor.sqlite";

/// The getting-started layout under `home`: the config file where § 3 writes it, and its instance.
fn getting_started(home: &Path) -> (PathBuf, Instance) {
    let top = home.join(".b10x/conductor");
    fs::create_dir_all(&top).expect("create the directory");
    let file = top.join("conductor.yaml");
    fs::write(&file, GETTING_STARTED).expect("write the config file");
    let instance = instances(GETTING_STARTED, home).remove(0);
    (file, instance)
}

/// The guide's own config puts the records (written) under the checkouts root (read): the box
/// request names both, and substrate refuses a writable root beneath a read-only root.
#[test]
fn the_box_of_the_configure_an_instance_guide_is_admitted_by_substrate() {
    let dir = case("guide");
    let home = dir.join("home");
    let file = home.join("example-org/conductor.yaml");
    fs::create_dir_all(file.parent().expect("a parent")).expect("create the directory");
    fs::write(&file, CONFIGURE_AN_INSTANCE).expect("write the config file");
    let instance = instances(CONFIGURE_AN_INSTANCE, &home).remove(0);
    let request = request(&instance, &file, &home, STORE);
    assert_eq!(
        substrate_refusal(&request),
        None,
        "read-only roots {:?}, writable roots {:?}",
        request.read_only_roots,
        request.writable_roots
    );
}

/// `security/sessions.md` and `getting-started.md` say the box reads the config file. In the
/// getting-started layout the one writable root holds it, so the runtime in the box can rewrite
/// the file that names its own placement and limits.
#[test]
fn the_getting_started_box_cannot_write_the_config_file_it_is_said_to_read() {
    let dir = case("config-writable");
    let home = dir.join("home");
    let (file, instance) = getting_started(&home);
    let request = request(&instance, &file, &home, STORE);
    assert!(
        !writes(&request, &file),
        "{} is under a writable root: {:?}",
        file.display(),
        request.writable_roots
    );
}

/// conductord keeps its own files in `~/.b10x/conductor/daemon`: `runtimes.json` holds the token
/// of every instance's runtime, which `box_request` strips from the box's environment, and
/// `binaries.json` and `binaries/<version>/conductor` the binary it starts. In the
/// getting-started layout the one writable root holds them.
#[test]
fn the_getting_started_box_cannot_write_the_daemons_own_directory() {
    let dir = case("daemon-dir");
    let home = dir.join("home");
    let (file, instance) = getting_started(&home);
    let request = request(&instance, &file, &home, STORE);
    let daemon = home.join(conductor_cli::daemon::DIR);
    for kept in [
        daemon.join(conductor_cli::daemon::reconcile::RUNTIMES_FILE),
        daemon.join(conductor_cli::daemon::reconcile::BINARIES_FILE),
        daemon
            .join(conductor_cli::daemon::upgrade::BINARIES_DIR)
            .join("1.0.1/conductor"),
    ] {
        assert!(
            !writes(&request, &kept),
            "{} is under a writable root: {:?}",
            kept.display(),
            request.writable_roots
        );
    }
}

/// The loop exports what the collectors read into `<cache>/exports` each pass
/// (`collect::blockers::Sources::of`) and the decider runs in `<cache>/decide`: the cache is
/// something the loop writes, and the acceptance asks for what the loop writes.
#[test]
fn the_box_writes_the_cache_directory_the_loop_exports_into() {
    let dir = case("cache");
    let home = dir.join("home");
    let (file, instance) = getting_started(&home);
    assert_eq!(
        instance.cache,
        home.join(".cache/b10x/conductor/home").display().to_string()
    );
    let request = request(&instance, &file, &home, STORE);
    let exports = Path::new(&instance.cache).join("exports");
    assert!(
        writes(&request, &exports),
        "{} is under no writable root: {:?}",
        exports.display(),
        request.writable_roots
    );
}

/// `config::parse` keeps an absolute path as written, so `records: /…/records/` reaches the
/// request with its slash, and substrate refuses a mount that is not canonical.
#[test]
fn a_records_path_written_with_a_trailing_slash_is_a_mount_substrate_admits() {
    let dir = case("slash");
    let home = dir.join("home");
    let records = dir.join("own/records");
    let text = format!(
        "version: conductor.config/2\ndaemon:\n  store: {STORE}\ninstances:\n  - name: alpha\n    \
         placement: systemd\n    session_prefix: alpha\n    sources:\n      - local: ~/src\n    \
         checkouts:\n      root: ~/src\n      trees: ~/trees\n    records: {}/\n",
        records.display()
    );
    let file = home.join(".b10x/conductor/conductor.yaml");
    let instance = instances(&text, &home).remove(0);
    let request = request(&instance, &file, &home, STORE);
    assert_eq!(
        substrate_refusal(&request),
        None,
        "writable roots {:?}",
        request.writable_roots
    );
}

/// The config file a link in the writable root (a dotfiles checkout holds the file): the link is
/// in the box, and the file it names must be too, as a program's is.
#[test]
fn a_config_file_that_is_a_link_in_a_writable_root_is_read_in_the_box() {
    let dir = case("config-link");
    let home = dir.join("home");
    let top = home.join(".b10x/conductor");
    fs::create_dir_all(&top).expect("create the directory");
    let held = home.join("dotfiles/conductor.yaml");
    fs::create_dir_all(held.parent().expect("a parent")).expect("create the directory");
    fs::write(&held, GETTING_STARTED).expect("write the config file");
    let file = top.join("conductor.yaml");
    std::os::unix::fs::symlink(&held, &file).expect("link");
    let instance = instances(GETTING_STARTED, &home).remove(0);
    let request = request(&instance, &file, &home, STORE);
    let held = fs::canonicalize(&held).expect("the file is there");
    let seen = request
        .read_only_files
        .iter()
        .any(|known| Path::new(&known.mount) == held)
        || request
            .read_only_roots
            .iter()
            .chain(request.writable_roots.iter().map(|root| &root.path))
            .any(|root| held.starts_with(root));
    assert!(
        seen,
        "{} is the file the link in the box names and no mount holds it: files {:?}, roots {:?} \
         {:?}",
        held.display(),
        request.read_only_files,
        request.read_only_roots,
        request.writable_roots
    );
}

/// `config validate` takes `a.b` and `a_b` as two instances; substrate-daemon refuses a root name
/// declared twice, so two instances need two names.
#[test]
fn two_instances_of_one_file_name_two_state_roots() {
    let dir = case("names");
    let home = dir.join("home");
    let mut text = format!("version: conductor.config/2\ndaemon:\n  store: {STORE}\ninstances:\n");
    for name in ["a.b", "a_b"] {
        text.push_str(&format!(
            "  - name: {name}\n    placement: systemd\n    session_prefix: {prefix}\n    \
             sources:\n      - local: ~/src\n    checkouts:\n      root: ~/src\n      \
             trees: ~/trees\n    state: {state}\n    records: {records}\n",
            prefix = if name == "a.b" { "first" } else { "second" },
            state = dir.join(name).join("state").display(),
            records = dir.join(name).join("records").display(),
        ));
    }
    let file = home.join(".b10x/conductor/conductor.yaml");
    let both = instances(&text, &home);
    let first = request(&both[0], &file, &home, STORE);
    let second = request(&both[1], &file, &home, STORE);
    assert_ne!(first.writable_roots[0].path, second.writable_roots[0].path);
    assert_ne!(
        first.writable_roots[0].name, second.writable_roots[0].name,
        "{} and {} are two directories under one root name",
        first.writable_roots[0].path, second.writable_roots[0].path
    );
}
