//! `story:workspace-from-config`: `conductor config workspace` writes the instance's
//! `<records>/.engineering/workspace.yaml` (`aep.workspace/1`) from its config, replacing the
//! file, with no network read: one member per git checkout directly under `checkouts.root` for a
//! `github` source, one per git checkout directly under its `path` but its `exclude` for a
//! `local` source, none for a `gitlab` source, and the records directory itself; every `source`
//! relative to that `.engineering/` directory. `--check` writes nothing and exits 1 naming each
//! member that differs from the file; `conductor init` writes the same file into its skeleton.
//!
//! Each case runs the built binary with `HOME` naming the case's `home/` and `CONDUCTOR_CONFIG`
//! a config file the case writes ([`common::conductor`]). When `aep` is on `PATH`, the written
//! workspace is also read back with `aep plan workspace members`, under the same `HOME`.

mod common;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;

/// The workspace file, under the records directory.
const WORKSPACE: &str = ".engineering/workspace.yaml";

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
            .join("config_workspace")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("home")).expect("create home");
        let dir = dir.canonicalize().expect("the case's directory");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn workspace(&self) -> PathBuf {
        self.records().join(WORKSPACE)
    }

    /// A git checkout at `path`, holding an empty planning store, so that `aep` finds one there.
    fn checkout(&self, path: &Path) {
        self.local_checkout(path);
        let output = self.git(Command::new("git").arg("-C").arg(path).args([
            "remote",
            "add",
            "origin",
            "https://example.invalid/example-org/x.git",
        ]));
        assert!(
            output.status.success(),
            "git remote add: {}",
            shown(&output)
        );
    }

    /// A checkout with no `origin` remote, which the collectors cannot read at `origin/main`.
    fn local_checkout(&self, path: &Path) {
        fs::create_dir_all(path.join(".engineering/planning")).expect("create the checkout");
        let output = self.git(Command::new("git").args(["init", "--quiet"]).arg(path));
        assert!(output.status.success(), "git init: {}", shown(&output));
    }

    /// The config file of one instance `alpha`: the records, the checkouts root and `sources`,
    /// each a YAML list item without its `- `.
    fn config(&self, sources: &[String]) -> PathBuf {
        let mut text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n\
             \x20   sources:\n",
            root = self.dir.join("checkouts").display(),
            trees = self.dir.join("trees").display(),
            records = self.records().display(),
        );
        for source in sources {
            text.push_str(&format!("      - {source}\n"));
        }
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// Two checkouts under the checkouts root and a directory that is none; one checkout and one
    /// excluded checkout under a local source; the records directory, a git repository with a
    /// store. Answers the config file, which also names a GitLab group.
    fn three_checkouts(&self) -> PathBuf {
        self.checkout(&self.dir.join("checkouts/alpha-api"));
        // A directory name aep does not take as a member name: the member is `beta-web`.
        self.checkout(&self.dir.join("checkouts/Beta.Web"));
        fs::create_dir_all(self.dir.join("checkouts/notes")).expect("create a plain directory");
        // A local-only checkout: no member (its `origin/main` cannot be fetched).
        self.local_checkout(&self.dir.join("checkouts/scratch-only"));
        self.checkout(&self.dir.join("local/gamma-tools"));
        self.checkout(&self.dir.join("local/archive"));
        self.checkout(&self.records());
        self.config(&[
            "github: example-org".to_owned(),
            format!(
                "local: {}\n        exclude: [archive]",
                self.dir.join("local").display()
            ),
            "gitlab: example-org/platform".to_owned(),
        ])
    }

    fn git(&self, command: &mut Command) -> Output {
        command
            .env("HOME", self.home())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Example Operator")
            .env("GIT_AUTHOR_EMAIL", "operator@example.invalid")
            .env("GIT_COMMITTER_NAME", "Example Operator")
            .env("GIT_COMMITTER_EMAIL", "operator@example.invalid")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(Stdio::null())
            .output()
            .expect("the program runs")
    }

    /// `conductor <args>` under the case's home, with `config` as the config file.
    fn conductor(&self, config: &Path, args: &[&str]) -> Output {
        self.git(
            common::conductor(self.home())
                .args(args)
                .current_dir(&self.dir)
                .env("CONDUCTOR_CONFIG", config),
        )
    }
}

fn shown(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Each member of the workspace file at `file`, as (name, source), in the file's order.
fn members(file: &Path) -> Vec<(String, String)> {
    let text = fs::read_to_string(file).expect("read the workspace file");
    let document: serde_yaml::Value = serde_yaml::from_str(&text).expect("YAML");
    assert_eq!(
        document["version"].as_str(),
        Some("aep.workspace/1"),
        "{text}"
    );
    document["members"]
        .as_sequence()
        .expect("a members list")
        .iter()
        .map(|member| {
            (
                member["name"].as_str().expect("a name").to_owned(),
                member["source"].as_str().expect("a source").to_owned(),
            )
        })
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(name, source)| ((*name).to_owned(), (*source).to_owned()))
        .collect()
}

/// The members of the three-checkout case, each relative to `records/.engineering/`.
const THREE: [(&str, &str); 4] = [
    ("alpha-api", "../../checkouts/alpha-api"),
    ("beta-web", "../../checkouts/Beta.Web"),
    ("gamma-tools", "../../local/gamma-tools"),
    ("records", "../../records"),
];

#[test]
fn config_workspace_writes_each_checkout_of_the_sources_and_the_records() {
    let case = Case::new("writes");
    let config = case.three_checkouts();

    let output = case.conductor(&config, &["config", "workspace"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(members(&case.workspace()), pairs(&THREE));
    // The blockers collector takes a member's repository from its source's last component.
    for (name, source) in members(&case.workspace()) {
        assert!(
            Path::new(&source).file_name().is_some(),
            "the source {source:?} of {name} ends in a directory name"
        );
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for (name, _) in THREE {
        assert!(stdout.contains(name), "names {name}: {}", shown(&output));
    }

    // `aep` resolves each source from `.engineering/` to a checkout that is there.
    match Command::new("aep")
        .args(["plan", "workspace", "members", "--format", "json"])
        .current_dir(case.records())
        .env("HOME", case.home())
        .stdin(Stdio::null())
        .output()
    {
        Err(error) if error.kind() == ErrorKind::NotFound => {
            println!("note: `aep` is not on PATH; the members are not resolved with it");
        }
        Err(error) => panic!("could not run `aep`: {error}"),
        Ok(output) => {
            assert!(output.status.success(), "aep: {}", shown(&output));
            let listed: Value = serde_json::from_slice(&output.stdout).expect("JSON");
            let listed = listed["members"].as_array().expect("a members array");
            assert_eq!(listed.len(), THREE.len(), "{listed:?}");
            for member in listed {
                assert_ne!(
                    member["state"].as_str(),
                    Some("absent"),
                    "each member resolves to its checkout: {member}"
                );
            }
        }
    }
}

#[test]
fn config_workspace_replaces_the_file() {
    let case = Case::new("replaces");
    let config = case.three_checkouts();
    fs::create_dir_all(case.workspace().parent().expect("a parent")).expect("create");
    fs::write(
        case.workspace(),
        "version: aep.workspace/1\nmembers:\n  - name: alpha-api\n    source: ../../alpha-api\n",
    )
    .expect("write a stale file");

    let output = case.conductor(&config, &["config", "workspace"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(members(&case.workspace()), pairs(&THREE));
}

#[test]
fn config_workspace_check_names_each_difference_and_writes_nothing() {
    let case = Case::new("check");
    let config = case.three_checkouts();
    fs::create_dir_all(case.workspace().parent().expect("a parent")).expect("create");
    let stale = "version: aep.workspace/1\n\
                 members:\n\
                 \x20 - name: alpha-api\n\
                 \x20   source: ../../alpha-api\n\
                 \x20 - name: gamma-tools\n\
                 \x20   source: ../../local/gamma-tools\n\
                 \x20 - name: old-site\n\
                 \x20   source: ../../checkouts/old-site\n\
                 \x20 - name: records\n\
                 \x20   source: ../../records\n";
    fs::write(case.workspace(), stale).expect("write a stale file");

    let output = case.conductor(&config, &["config", "workspace", "--check"]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    let differing: Vec<&str> = stderr
        .lines()
        .filter(|line| line.contains("member "))
        .collect();
    assert_eq!(
        differing.len(),
        3,
        "one line per difference: {}",
        shown(&output)
    );
    for name in ["alpha-api", "beta-web", "old-site"] {
        assert!(
            differing.iter().any(|line| line.contains(name)),
            "names {name}: {}",
            shown(&output)
        );
    }
    assert!(
        !differing
            .iter()
            .any(|line| line.contains("gamma-tools") || line.contains("member records")),
        "names only the members that differ: {}",
        shown(&output)
    );
    assert_eq!(
        fs::read_to_string(case.workspace()).expect("read"),
        stale,
        "--check writes nothing"
    );

    let written = case.conductor(&config, &["config", "workspace"]);
    assert_eq!(written.status.code(), Some(0), "{}", shown(&written));
    let fresh = case.conductor(&config, &["config", "workspace", "--check"]);
    assert_eq!(fresh.status.code(), Some(0), "{}", shown(&fresh));
}

#[test]
fn config_workspace_check_without_a_file_names_every_member() {
    let case = Case::new("check-absent");
    let config = case.three_checkouts();

    let output = case.conductor(&config, &["config", "workspace", "--check"]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    for (name, _) in THREE {
        assert!(stderr.contains(name), "names {name}: {}", shown(&output));
    }
    assert!(!case.workspace().exists(), "--check writes nothing");
}

#[test]
fn two_sources_giving_one_name_are_refused_by_their_yaml_paths() {
    let case = Case::new("collision");
    case.checkout(&case.dir.join("checkouts/alpha-api"));
    case.checkout(&case.dir.join("local/alpha-api"));
    case.checkout(&case.records());
    let config = case.config(&[
        "github: example-org".to_owned(),
        format!("local: {}", case.dir.join("local").display()),
    ]);

    let output = case.conductor(&config, &["config", "workspace"]);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    for path in ["instances[0].sources[0]", "instances[0].sources[1]"] {
        assert!(stderr.contains(path), "names {path}: {}", shown(&output));
    }
    assert!(stderr.contains("alpha-api"), "{}", shown(&output));
    assert!(!case.workspace().exists(), "nothing is written");
}

#[test]
fn init_writes_the_workspace_into_the_skeleton_and_commits_it() {
    let case = Case::new("init");
    case.checkout(&case.dir.join("checkouts/alpha-api"));
    let config = case.config(&["github: example-org".to_owned()]);

    let output = case.conductor(&config, &["init"]);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(
        members(&case.workspace()),
        pairs(&[
            ("alpha-api", "../../checkouts/alpha-api"),
            ("records", "../../records"),
        ])
    );
    let tracked = case.git(
        Command::new("git")
            .arg("-C")
            .arg(case.records())
            .args(["ls-files", "--", WORKSPACE]),
    );
    assert_eq!(
        String::from_utf8_lossy(&tracked.stdout).trim(),
        WORKSPACE,
        "init commits the workspace file: {}",
        shown(&tracked)
    );
    let check = case.conductor(&config, &["config", "workspace", "--check"]);
    assert_eq!(check.status.code(), Some(0), "{}", shown(&check));
}
