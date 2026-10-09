//! `story:grouped-instance`: a second instance whose checkouts sit in groups.
//!
//! - `default: <instance>` at the top of the config file selects the instance when neither
//!   `--instance` nor `CONDUCTOR_INSTANCE` does, and must name an instance of the file.
//! - A repository is the directory that holds `.git` at depth 1 or 2 under `checkouts.root`
//!   (`<root>/<repo>` or `<root>/<group>/<repo>`), and the same under `checkouts.trees` with the
//!   tree below it; its name is its path under the root (`<group>/<repo>`). The guard, the
//!   sessions collector and a `local:` source's listing all read it so.
//! - A `gitlab:` source is read by no collector, and `config show` prints it.
//! - A `local:` or `gitlab:` source may carry `exclude`; an excluded repository is not listed and
//!   not placed.
//!
//! Each case is a directory of its own in this test target's temporary directory, holding the
//! checkouts root `root/`, the managed trees `trees/`, conductor's `records/`, `state/`, a home
//! directory `home/`, `bin/` for the programs the binary finds, and an empty `work/` it runs
//! from. A repository is a directory holding `.git`, or a git repository the case builds with
//! one commit. The names are neutral: group `g`, repositories `g/r`, `g/other` and `s`, and the
//! excluded group `x`. No case reaches the network, a real checkout or the real session list.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::config;
use conductor_cli::guard::{Decision, Places, SessionList, decide_in};
use conductor_model::dispatch::Verdict;
use serde_json::{Value, json};

/// The clock of the snapshots.
const NOW: &str = "2026-10-07T12:00:00Z";

/// The date of every commit the cases make.
const OLD: &str = "2026-09-01T09:00:00Z";

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

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
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("grouped_instance")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in [
            "home",
            "root",
            "trees",
            "records/.engineering",
            "work",
            "bin",
        ] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        fs::write(
            dir.join("records/.engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers: []\n",
        )
        .expect("write the records' workspace file");
        let dir = fs::canonicalize(&dir).expect("the case's directory exists");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn root(&self) -> PathBuf {
        self.dir.join("root")
    }

    fn trees(&self) -> PathBuf {
        self.dir.join("trees")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn log(&self, program: &str) -> PathBuf {
        self.dir.join(format!("{program}.log"))
    }

    /// A checkout `<root>/<name>`: a directory holding a `.git` directory.
    fn checkout(&self, name: &str) -> PathBuf {
        let dir = self.root().join(name);
        fs::create_dir_all(dir.join(".git")).expect("create the checkout");
        dir
    }

    /// A managed tree `<trees>/<name>/<tree>`: a directory holding a `.git` file, as `git worktree
    /// add` leaves one.
    fn tree(&self, name: &str, tree: &str) -> PathBuf {
        let dir = self.trees().join(name).join(tree);
        fs::create_dir_all(&dir).expect("create the tree");
        fs::write(dir.join(".git"), "gitdir: /nowhere\n").expect("write the tree's .git");
        dir
    }

    /// A git repository `<root>/<name>` with one commit.
    fn repository(&self, name: &str) -> PathBuf {
        let dir = self.root().join(name);
        fs::create_dir_all(&dir).expect("create the repository");
        git(&dir, &["init", "--quiet", "--initial-branch=main"]);
        fs::write(dir.join("README.md"), format!("# {name}\n")).expect("write a file");
        git(&dir, &["add", "--all"]);
        git(&dir, &["commit", "--quiet", "-m", "Start the repository"]);
        dir
    }

    /// The config file of one instance `acme` whose `sources` are `sources` (a YAML flow
    /// sequence), with the case's checkouts root, trees, records, state and cache.
    fn config(&self, sources: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: {sources}\n    \
             checkouts:\n      root: {}\n      trees: {}\n    records: {}\n    state: {}\n    \
             cache: {}\n",
            quoted(&self.root()),
            quoted(&self.trees()),
            quoted(&self.dir.join("records")),
            quoted(&self.state()),
            quoted(&self.dir.join("cache")),
        );
        self.write("conductor.yaml", &text)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let file = self.dir.join(name);
        fs::write(&file, text).expect("write the case file");
        file
    }

    /// The guard's places for the instance `file` names, as the hook resolves them when
    /// `CONDUCTOR_CONFIG` names it.
    fn places(&self, file: &Path) -> Places {
        let environment = config::Environment {
            home: Some(self.home()),
            cwd: self.home(),
            config: Some(file.as_os_str().to_owned()),
            instance: None,
        };
        let active = config::resolve(None, &environment).expect("the config file loads");
        assert!(active.from_file);
        Places::active(&self.home(), &active).with_config_file(file)
    }

    /// The program `name` on the binary's `PATH`: it appends its arguments to its log, then runs
    /// `body`, shell built-ins only.
    fn program(&self, name: &str, body: &str) {
        let file = self.bin().join(name);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{body}\n",
                text(&self.log(name))
            ),
        )
        .expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// `gh` answering every list empty, `aep` an empty workspace and listing, `claude` the
    /// session list `sessions`, and the real `git`.
    fn programs(&self, sessions: &Value) {
        self.program("gh", "printf '%s' '[]'");
        self.program(
            "aep",
            "case \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) printf '%s' \
             '{\"artifacts\": []}' ;;\nesac",
        );
        self.program("claude", &format!("printf '%s' '{sessions}'"));
        let git = which("git");
        std::os::unix::fs::symlink(git, self.bin().join("git")).expect("link git");
    }

    /// The built binary run from `work/` with `--state-dir` naming `state/`, `CONDUCTOR_CONFIG`
    /// naming `config`, `HOME` naming `home/` and `PATH` only `bin/`.
    fn conductor(&self, config: &Path, args: &[&str]) -> Output {
        common::conductor(self.home())
            .current_dir(self.dir.join("work"))
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    fn start_snapshot(&self, config: &Path) -> Output {
        self.conductor(
            config,
            &[
                "snapshot",
                "start-snapshot",
                "--started-at",
                NOW,
                "--disk-free-bytes",
                "100000000000",
            ],
        )
    }

    /// The rows of the view `view`, without the identities the store assigns.
    fn rows(&self, config: &Path, view: &str) -> Vec<Value> {
        let output = self.conductor(config, &["snapshot", view, "--format", "jsonl"]);
        assert!(output.status.success(), "{view}: {}", describe(&output));
        String::from_utf8(output.stdout)
            .expect("UTF-8")
            .lines()
            .map(|line| {
                let mut row: Value = serde_json::from_str(line).expect("one JSON object a line");
                let fields = row.as_object_mut().expect("a row is an object");
                fields.remove("observation_id");
                fields.remove("snapshot_id");
                row
            })
            .collect()
    }

    fn lines(&self, program: &str) -> Vec<String> {
        fs::read_to_string(self.log(program))
            .map(|log| log.lines().map(str::to_owned).collect())
            .unwrap_or_default()
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

/// Runs git in `dir` at [`OLD`], with no configuration of the user's.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", OLD)
        .env("GIT_COMMITTER_DATE", OLD)
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A Claude session of the list `claude agents --json` prints.
fn live(id: &str, cwd: &Path) -> Value {
    json!({"id": id, "pid": 100, "status": "busy", "cwd": text(cwd)})
}

/// A sessions row: the session `id` at `cwd`, bound to `repository`.
fn placed(id: &str, cwd: &Path, repository: Option<&str>) -> Value {
    json!({
        "harness": "Claude",
        "session_ref": id,
        "name": null,
        "cwd": text(cwd),
        "repository": repository,
        "role": null,
        "activity": "Busy",
    })
}

/// A sessions row of a session that is not placed: its harness and activity only.
fn unplaced() -> Value {
    json!({
        "harness": "Claude",
        "session_ref": "",
        "name": null,
        "cwd": "",
        "repository": null,
        "role": null,
        "activity": "Busy",
    })
}

// ---------------------------------------------------------------------------------------------
// Acceptance 1: `default` selects the instance; the setting beats it; a default naming no
// instance is refused with its YAML path.
// ---------------------------------------------------------------------------------------------

/// Two instances, `a` and `b`, and `default: <default>`; `b` carries the session prefix `b`, since
/// at most one instance keeps the bare session names (story:instance-session-names).
fn two_instances(default: &str) -> String {
    format!(
        "version: conductor.config/1\n\
         default: {default}\n\
         instances:\n\
         \x20 - name: a\n\
         \x20   sources: [{{github: acme}}]\n\
         \x20   checkouts: {{root: /fixture-home/a, trees: /fixture-home/a-trees}}\n\
         \x20 - name: b\n\
         \x20   session_prefix: b\n\
         \x20   sources: [{{gitlab: acme-group}}]\n\
         \x20   checkouts: {{root: /fixture-home/b, trees: /fixture-home/b-trees}}\n"
    )
}

fn environment(case: &Case, file: &Path, instance: Option<&str>) -> config::Environment {
    config::Environment {
        home: Some(case.home()),
        cwd: case.home(),
        config: Some(file.as_os_str().to_owned()),
        instance: instance.map(Into::into),
    }
}

#[test]
fn the_default_selects_the_instance_and_the_setting_beats_it() {
    let case = Case::new("default");
    let file = case.write("two.yaml", &two_instances("a"));

    let active = config::resolve(None, &environment(&case, &file, None))
        .expect("two instances and a default resolve");
    assert_eq!(active.instance.name.0, "a");
    let active = config::resolve(None, &environment(&case, &file, Some("b")))
        .expect("the setting names an instance");
    assert_eq!(active.instance.name.0, "b");

    // Through the binary: `config show` shows the default, the variable beats it, and the flag
    // beats both.
    let show = |extra: &[&str], variable: Option<&str>| {
        let mut command = common::conductor(case.home());
        command
            .current_dir(case.dir.join("work"))
            .env_clear()
            .env("HOME", case.home())
            .args([
                "--config",
                &text(&file),
                "config",
                "show",
                "--format",
                "json",
            ])
            .args(extra)
            .stdin(Stdio::null());
        if let Some(variable) = variable {
            command.env(config::INSTANCE_VARIABLE, variable);
        }
        let output = command.output().expect("the binary runs");
        assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
        let shown: Value = serde_json::from_slice(&output.stdout).expect("JSON");
        shown["instances"][0]["name"]
            .as_str()
            .expect("an instance name")
            .to_owned()
    };
    assert_eq!(show(&[], None), "a");
    assert_eq!(show(&[], Some("b")), "b");
    assert_eq!(show(&["--instance", "a"], Some("b")), "a");
}

#[test]
fn a_default_naming_no_instance_is_refused_with_its_yaml_path() {
    let case = Case::new("default-unknown");
    let text_of = two_instances("c");
    let problems = config::parse(&text_of, &case.home()).expect_err("default c names no instance");
    assert_eq!(
        problems
            .iter()
            .map(|problem| problem.path.as_str())
            .collect::<Vec<_>>(),
        ["default"],
        "{problems:?}"
    );
    assert!(
        problems[0].message.contains("a, b"),
        "the problem names the instances: {problems:?}"
    );
    let file = case.write("two.yaml", &text_of);
    let output = common::conductor(case.home())
        .current_dir(case.dir.join("work"))
        .env_clear()
        .env("HOME", case.home())
        .args(["--config", &text(&file), "config", "validate"])
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(": default: "),
        "{}",
        describe(&output)
    );
}

// ---------------------------------------------------------------------------------------------
// Acceptance 2: the guard reads a repository at depth 1 or 2.
// ---------------------------------------------------------------------------------------------

/// The guard's decision on `tool` with `input`, for a session whose cwd is `cwd`.
fn decide(places: &Places, cwd: &Path, tool: &str, input: Value) -> Decision {
    let payload = json!({
        "session_id": "00000000-0000-4000-8000-000000000001",
        "cwd": text(cwd),
        "tool_name": tool,
        "tool_input": input,
    });
    decide_in(&payload, places, &SessionList::default())
}

fn write(places: &Places, cwd: &Path, target: &Path) -> Decision {
    decide(
        places,
        cwd,
        "Write",
        json!({"file_path": text(target), "content": ""}),
    )
}

fn bash(places: &Places, cwd: &Path, command: &str) -> Decision {
    decide(places, cwd, "Bash", json!({"command": command}))
}

/// The case's groups: `g/r` and `g/other` in group `g`, `s` directly under the root, each with
/// a managed tree, and `x/y` in the group `x`.
fn grouped(case: &Case) {
    for name in ["g/r", "g/other", "s", "x/y"] {
        case.checkout(name);
    }
    case.tree("g/r", "t1");
    case.tree("g/other", "t2");
    case.tree("s", "t3");
}

#[test]
fn a_controller_in_a_grouped_checkout_writes_only_its_own_repository() {
    let case = Case::new("guard");
    grouped(&case);
    let places = case.places(&case.config("[{gitlab: acme-group, exclude: [x]}]"));
    let root = case.root();
    let trees = case.trees();
    let here = root.join("g/r");

    let allowed = write(&places, &here, &here.join("x"));
    assert_eq!(allowed.verdict, Verdict::Allow, "{allowed:?}");
    assert_eq!(allowed.repository, "g/r");
    for inside in [here.join("src/lib.rs"), trees.join("g/r/t1/x")] {
        let decision = write(&places, &here, &inside);
        assert_eq!(decision.verdict, Verdict::Allow, "{inside:?}: {decision:?}");
    }
    for outside in [
        root.join("g/other/x"),
        root.join("g/x"),
        root.join("g/ghost/x"),
        root.join("s/x"),
        root.join("x/y/x"),
        trees.join("g/other/t2/x"),
        trees.join("g/x"),
        trees.join("s/t3/x"),
    ] {
        let decision = write(&places, &here, &outside);
        assert_eq!(decision.verdict, Verdict::Deny, "{outside:?}: {decision:?}");
    }

    // Bash reaches another repository of the group, or one at depth 1, only to be denied.
    for (command, place) in [
        ("cd ../other", "g/other"),
        (&*format!("cd {}", text(&root.join("s"))), "s"),
        ("git -C ../other status", "g/other"),
        (
            &*format!("git -C {} log", text(&trees.join("g/other/t2"))),
            "g/other",
        ),
    ] {
        let decision = bash(&places, &here, command);
        assert_eq!(decision.verdict, Verdict::Deny, "{command}: {decision:?}");
        assert!(
            decision
                .reason
                .contains(&format!("another repository, {place}")),
            "{command}: {decision:?}"
        );
    }
    let decision = bash(&places, &here, "cd src && git status");
    assert_eq!(decision.verdict, Verdict::Allow, "{decision:?}");

    // A worker in the repository's managed tree is the controller's own.
    let worker = trees.join("g/r/t1");
    let decision = write(&places, &worker, &root.join("g/r/x"));
    assert_eq!(decision.verdict, Verdict::Allow, "{decision:?}");
    assert_eq!(decision.repository, "g/r");
    let decision = write(&places, &worker, &root.join("g/other/x"));
    assert_eq!(decision.verdict, Verdict::Deny, "{decision:?}");

    // A repository directly under the root reads as today.
    let alone = root.join("s");
    let decision = write(&places, &alone, &alone.join("x"));
    assert_eq!(decision.verdict, Verdict::Allow, "{decision:?}");
    assert_eq!(decision.repository, "s");
    let decision = write(&places, &alone, &root.join("g/r/x"));
    assert_eq!(decision.verdict, Verdict::Deny, "{decision:?}");
}

#[test]
fn a_group_directory_and_an_excluded_repository_are_no_controller_s() {
    let case = Case::new("guard-group");
    grouped(&case);
    let places = case.places(&case.config("[{gitlab: acme-group, exclude: [x]}]"));
    let root = case.root();

    // The group directory holds no `.git`: a session there is in no repository.
    let group = root.join("g");
    for target in [root.join("g/r/x"), root.join("g/x")] {
        let decision = write(&places, &group, &target);
        assert_eq!(decision.verdict, Verdict::Deny, "{target:?}: {decision:?}");
        assert_eq!(decision.repository, "", "{decision:?}");
    }
    // An excluded repository is not placed: a session there is denied every ruled tool.
    let excluded = root.join("x/y");
    let decision = write(&places, &excluded, &excluded.join("x"));
    assert_eq!(decision.verdict, Verdict::Deny, "{decision:?}");
    // ... and stays another repository to a controller elsewhere.
    let decision = bash(&places, &root.join("s"), &format!("cd {}", text(&excluded)));
    assert_eq!(decision.verdict, Verdict::Deny, "{decision:?}");
}

/// What a controller can write cannot widen it: a `.git` it writes in its own managed worktrees,
/// `<trees>/g/r/.git`, or in its own checkout, leaves its workers in repository `g/r`. A group
/// with no directory under the root is read from its trees, the deeper reading first.
#[test]
fn a_controller_cannot_widen_itself_by_writing_a_git_marker() {
    let case = Case::new("guard-widen");
    grouped(&case);
    let places = case.places(&case.config("[{gitlab: acme-group}]"));
    let root = case.root();
    let trees = case.trees();
    let worker = trees.join("g/r/t1");
    let marker = trees.join("g/r/.git");
    let decision = write(&places, &worker, &marker);
    assert_eq!(decision.verdict, Verdict::Allow, "{decision:?}");
    fs::write(&marker, "gitdir: /nowhere\n").expect("write the marker the guard allowed");
    fs::create_dir_all(root.join("g/r/sub/.git")).expect("a nested .git in the checkout");
    for cwd in [worker.clone(), root.join("g/r/sub")] {
        let decision = write(&places, &cwd, &root.join("g/other/x"));
        assert_eq!(decision.verdict, Verdict::Deny, "{cwd:?}: {decision:?}");
        assert_eq!(decision.repository, "g/r", "{decision:?}");
    }

    // A group only the trees hold.
    let lone = case.tree("h/q", "t4");
    case.tree("h/p", "t5");
    let decision = write(&places, &lone, &trees.join("h/q/t4/x"));
    assert_eq!(decision.verdict, Verdict::Allow, "{decision:?}");
    assert_eq!(decision.repository, "h/q", "{decision:?}");
    let decision = write(&places, &lone, &trees.join("h/p/t5/x"));
    assert_eq!(decision.verdict, Verdict::Deny, "{decision:?}");
}

// ---------------------------------------------------------------------------------------------
// Acceptance 2 and 3, through the binary: sessions are placed by the same rule, a `local:`
// source lists repositories at depth 1 and 2, an excluded group's are not listed, and a
// `gitlab:` source asks nothing.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_local_source_lists_grouped_repositories_and_places_their_sessions() {
    let case = Case::new("local");
    for name in ["g/r", "s", "x/y"] {
        case.repository(name);
    }
    fs::create_dir_all(case.root().join("g/notes")).expect("a directory that is no checkout");
    fs::write(case.root().join("README.md"), "").expect("a file beside the checkouts");
    case.tree("g/r", "t1");
    let root = case.root();
    let trees = case.trees();
    case.programs(&json!([
        live("s-gr", &root.join("g/r/src")),
        live("s-tree", &trees.join("g/r/t1")),
        live("s-s", &root.join("s")),
        live("s-x", &root.join("x/y")),
        live("s-group-x", &root.join("x")),
        live("s-group", &root.join("g")),
    ]));
    let config = case.config(&format!("[{{local: {}, exclude: [x]}}]", quoted(&root)));
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));

    let names: Vec<Value> = case
        .rows(&config, "repositories")
        .into_iter()
        .map(|row| row["repository"].clone())
        .collect();
    assert_eq!(names, [json!("g/r"), json!("s")]);
    assert_eq!(
        case.rows(&config, "sessions"),
        [
            placed("s-gr", &root.join("g/r/src"), Some("g/r")),
            placed("s-tree", &trees.join("g/r/t1"), Some("g/r")),
            placed("s-s", &root.join("s"), Some("s")),
            unplaced(),
            unplaced(),
            placed("s-group", &root.join("g"), None),
        ]
    );
}

#[test]
fn a_gitlab_source_asks_nothing_and_config_show_prints_it() {
    let case = Case::new("gitlab");
    grouped(&case);
    let root = case.root();
    case.programs(&json!([
        live("s-gr", &root.join("g/r")),
        live("s-s", &root.join("s/src")),
        live("s-x", &root.join("x/y")),
    ]));
    let config = case.config("[{gitlab: acme-group, exclude: [x]}]");
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.lines("gh"),
        Vec::<String>::new(),
        "gh is asked nothing"
    );
    assert_eq!(case.rows(&config, "repositories"), Vec::<Value>::new());
    assert_eq!(
        case.rows(&config, "sessions"),
        [
            placed("s-gr", &root.join("g/r"), Some("g/r")),
            placed("s-s", &root.join("s/src"), Some("s")),
            unplaced(),
        ]
    );

    let shown = case.conductor(&config, &["config", "show", "--format", "json"]);
    assert_eq!(shown.status.code(), Some(0), "{}", describe(&shown));
    let shown: Value = serde_json::from_slice(&shown.stdout).expect("JSON");
    assert_eq!(
        shown["instances"][0]["sources"],
        json!([{"gitlab": "acme-group", "exclude": ["x"]}]),
        "{shown:#}"
    );
    let yaml = case.conductor(&config, &["config", "show", "--format", "yaml"]);
    let written = case.write("shown.yaml", &String::from_utf8_lossy(&yaml.stdout));
    let reread = case.conductor(&written, &["config", "show", "--format", "json"]);
    assert_eq!(
        serde_json::from_slice::<Value>(&reread.stdout).expect("JSON"),
        shown,
        "the shown file reads back the same"
    );
}

#[test]
fn an_exclude_is_a_path_under_the_root_of_a_local_or_gitlab_source() {
    let home = Path::new("/fixture-home");
    let file = |sources: &str| {
        format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: {sources}\n    \
             checkouts: {{root: /fixture-home/acme, trees: /fixture-home/trees}}\n"
        )
    };
    let parsed = config::parse(
        &file("[{local: /fixture-home/acme, exclude: [x, g/r]}, {gitlab: acme-group}]"),
        home,
    )
    .expect("a local and a gitlab source with exclude");
    assert_eq!(parsed.instances[0].sources.len(), 2);
    for (sources, path) in [
        (
            "[{github: acme, exclude: [x]}]",
            "instances[0].sources[0].exclude",
        ),
        (
            "[{gitlab: acme-group, exclude: [/x]}]",
            "instances[0].sources[0].exclude[0]",
        ),
        (
            "[{gitlab: acme-group, exclude: [../x]}]",
            "instances[0].sources[0].exclude[0]",
        ),
        (
            "[{gitlab: acme-group, exclude: [g/./r]}]",
            "instances[0].sources[0].exclude[0]",
        ),
        (
            "[{gitlab: acme-group, exclude: [\"\"]}]",
            "instances[0].sources[0].exclude[0]",
        ),
        (
            "[{gitlab: acme-group, exclude: x}]",
            "instances[0].sources[0].exclude",
        ),
        (
            "[{gitlab: acme-group, local: /x}]",
            "instances[0].sources[0]",
        ),
        ("[{exclude: [x]}]", "instances[0].sources[0]"),
    ] {
        let problems = config::parse(&file(sources), home).expect_err(sources);
        assert!(
            problems.iter().any(|problem| problem.path == path),
            "{sources}: {problems:?}"
        );
    }
}
