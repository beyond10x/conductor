//! `story:portable-trust`: `conductor trust` marks every git checkout directly under the active
//! instance's `checkouts.root`, and the instance's records directory by its physical path, as
//! trusted Claude Code workspaces in `~/.claude.json`, with no GNU tool: it refuses a records path
//! holding a line break or naming no directory, keeps every other key, project and the file's
//! mode, writes atomically, and refuses when the file's content changed during the run.
//!
//! The cases ported from `task trust` (`tests/taskfile.rs`) keep their assertions. Each case runs
//! the built binary with `HOME` naming the case's `home/`, `CONDUCTOR_CONFIG` a config file the
//! case writes, `CONDUCTOR_INSTANCE` removed, and works only under its own directory.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::trust::Edit;
use serde_json::Value;

/// One case: `home/`, `records/` and `checkouts/` under its own directory.
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
            .join("trust")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "records", "checkouts"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        Self { dir }
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn claude_json(&self) -> PathBuf {
        self.dir.join("home/.claude.json")
    }

    /// Writes a config file of one instance `alpha` whose `records` is the text `records`,
    /// written as a JSON string (which YAML reads as a double-quoted scalar), and whose
    /// checkouts root is `checkouts/`.
    fn config(&self, records: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   sources:\n\
             \x20     - github: example-org\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n",
            root = self.dir.join("checkouts").display(),
            trees = self.dir.join("trees").display(),
            records = serde_json::to_string(records).expect("records as a JSON string"),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// `conductor trust`, isolated as the module says.
    fn trust(&self, config: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_conductor"))
            .arg("trust")
            .current_dir(&self.dir)
            .env("HOME", self.dir.join("home"))
            .env("CONDUCTOR_CONFIG", config)
            .env_remove("CONDUCTOR_INSTANCE")
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// A git checkout `checkouts/<name>`.
    fn checkout(&self, name: &str) -> PathBuf {
        let checkout = self.dir.join("checkouts").join(name);
        fs::create_dir_all(&checkout).expect("create a checkout");
        let init = Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&checkout)
            .env("HOME", self.dir.join("home"))
            .output()
            .expect("git runs");
        assert!(init.status.success(), "{init:?}");
        checkout
    }

    /// The `projects` keys trust left in the case's `~/.claude.json`, sorted.
    fn trusted(&self) -> Vec<String> {
        let written = self.written();
        let mut keys: Vec<String> = written["projects"]
            .as_object()
            .expect("projects")
            .keys()
            .cloned()
            .collect();
        keys.sort_unstable();
        keys
    }

    fn written(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.claude_json()).expect("read .claude.json"))
            .expect(".claude.json is JSON")
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

/// Ported from `trust_marks_the_records_directory_and_each_checkout_trusted`.
#[test]
fn trust_marks_the_records_directory_and_each_checkout_trusted() {
    let case = Case::new("marks");
    let config = case.config(&case.records().display().to_string());
    let checkout = case.checkout("alpha");
    fs::create_dir_all(case.dir.join("checkouts/not-git")).expect("create a plain directory");
    fs::write(case.claude_json(), "{\"projects\": {}}\n").expect("write .claude.json");

    let output = case.trust(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let written = case.written();
    let projects = written["projects"].as_object().expect("projects");
    let mut keys: Vec<&str> = projects.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let checkout = checkout.display().to_string();
    let records = case.records().display().to_string();
    let mut expected = vec![checkout.as_str(), records.as_str()];
    expected.sort_unstable();
    assert_eq!(keys, expected, "{written:#}");
    for key in keys {
        assert_eq!(
            projects[key]["hasTrustDialogAccepted"],
            Value::Bool(true),
            "{key}"
        );
    }
    // It prints what it trusted.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(&records) && stdout.contains(&checkout),
        "trust prints each path it trusted: {}",
        shown(&output)
    );
}

/// Ported from `adv_trust_writes_a_records_directory_with_a_trailing_slash_by_the_key_its_session_has`:
/// a session's working directory never ends in `/`, and Claude Code keys `projects` by it.
#[test]
fn trust_writes_a_records_directory_with_a_trailing_slash_by_the_key_its_session_has() {
    let case = Case::new("trailing-slash");
    let records = case.records().display().to_string();
    let config = case.config(&format!("{records}/"));
    fs::write(case.claude_json(), "{\"projects\": {}}\n").expect("write .claude.json");

    let output = case.trust(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(case.trusted(), vec![records], "{}", shown(&output));
}

/// Ported from `adv_trust_trusts_no_directory_the_records_path_only_contains`: a records path
/// holding a line break is refused, with a message, and nothing is trusted in that run.
#[test]
fn trust_trusts_no_directory_the_records_path_only_contains() {
    let case = Case::new("newline");
    let outside = case.dir.join("outside");
    fs::create_dir_all(&outside).expect("create a directory outside");
    let records = format!("{}\nrecords", outside.display());
    fs::create_dir_all(&records).expect("create the records directory");
    let config = case.config(&records);
    fs::write(case.claude_json(), "{\"projects\": {}}\n").expect("write .claude.json");

    let output = case.trust(&config);
    assert_ne!(output.status.code(), Some(0), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("line break"),
        "the refusal says why: {}",
        shown(&output)
    );
    let trusted = case.trusted();
    assert!(
        !trusted.contains(&outside.display().to_string()),
        "trust marked {} trusted, which is neither the records directory nor a checkout: \
         {trusted:?}",
        outside.display()
    );
    assert!(trusted.is_empty(), "trusted in a refused run: {trusted:?}");
}

/// Ported from `trust_writes_a_symlinked_records_directory_by_its_physical_path`.
#[test]
fn trust_writes_a_symlinked_records_directory_by_its_physical_path() {
    let case = Case::new("symlinked-records");
    let link = case.dir.join("records-link");
    std::os::unix::fs::symlink(case.records(), &link).expect("link the records");
    let config = case.config(&link.display().to_string());
    fs::write(case.claude_json(), "{\"projects\": {}}\n").expect("write .claude.json");

    let output = case.trust(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    assert_eq!(
        case.trusted(),
        vec![case.records().display().to_string()],
        "{}",
        shown(&output)
    );
}

/// Ported from `trust_covers_the_records_directory`, which read the task's `test -d`
/// precondition: a records directory that is not there is refused, and nothing is written.
#[test]
fn trust_refuses_a_records_directory_that_is_not_there() {
    let case = Case::new("records-missing");
    let config = case.config(&case.dir.join("no-such-records").display().to_string());
    case.checkout("alpha");
    let before = "{\"projects\": {}}\n";
    fs::write(case.claude_json(), before).expect("write .claude.json");

    let output = case.trust(&config);
    assert_eq!(output.status.code(), Some(1), "{}", shown(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("not a directory"),
        "the refusal says why: {}",
        shown(&output)
    );
    assert_eq!(
        fs::read_to_string(case.claude_json()).expect("read .claude.json"),
        before
    );
}

/// Every other key, every other project and every other field of a trusted project is kept, in
/// its order, and so is the file's mode.
#[test]
fn trust_keeps_every_other_key_project_and_the_files_mode() {
    let case = Case::new("keeps");
    let config = case.config(&case.records().display().to_string());
    let records = case.records().display().to_string();
    let before = format!(
        "{{\n  \"zeta\": 1,\n  \"numStartups\": 12345678901234,\n  \"projects\": {{\n    \
         \"/elsewhere\": {{\"allowedTools\": [\"Bash\"], \"hasTrustDialogAccepted\": false}},\n    \
         {records:?}: {{\"history\": [{{\"display\": \"x\"}}], \"lastCost\": 0.25}}\n  }},\n  \
         \"alpha\": {{\"nested\": null}}\n}}\n"
    );
    fs::write(case.claude_json(), &before).expect("write .claude.json");
    fs::set_permissions(case.claude_json(), fs::Permissions::from_mode(0o600))
        .expect("chmod .claude.json");

    let output = case.trust(&config);
    assert_eq!(output.status.code(), Some(0), "{}", shown(&output));
    let text = fs::read_to_string(case.claude_json()).expect("read .claude.json");
    let written: Value = serde_json::from_str(&text).expect(".claude.json is JSON");
    let mut expected: Value = serde_json::from_str(&before).expect("the case's JSON");
    expected["projects"][&records]["hasTrustDialogAccepted"] = Value::Bool(true);
    assert_eq!(written, expected, "{text}");
    // The top-level keys keep their order.
    let order: Vec<usize> = ["\"zeta\"", "\"numStartups\"", "\"projects\"", "\"alpha\""]
        .iter()
        .map(|key| text.find(key).expect("the key is kept"))
        .collect();
    assert!(order.is_sorted(), "the keys keep their order: {text}");
    let mode = fs::metadata(case.claude_json())
        .expect("stat .claude.json")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "the file keeps its mode");
    // No temporary file is left beside it.
    let left: Vec<String> = fs::read_dir(case.dir.join("home"))
        .expect("read home")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec![".claude.json".to_owned()]);
}

/// The content-hash refusal: a `~/.claude.json` that changed between the read and the write is
/// not overwritten, whatever its modification time says, and no temporary file is left.
#[test]
fn trust_refuses_when_the_file_changed_between_read_and_write() {
    let case = Case::new("changed");
    let file = case.claude_json();
    fs::write(&file, "{\"projects\": {}}\n").expect("write .claude.json");
    let modified = fs::metadata(&file)
        .expect("stat")
        .modified()
        .expect("mtime");

    let mut edit = Edit::read(&file).expect("read .claude.json");
    edit.trust("/somewhere");
    // Claude Code writes the file while the edit is open, within the same second.
    let changed = "{\"projects\": {}, \"numStartups\": 2}\n";
    fs::write(&file, changed).expect("change .claude.json");
    let times = fs::FileTimes::new().set_modified(modified);
    fs::File::options()
        .write(true)
        .open(&file)
        .expect("open")
        .set_times(times)
        .expect("restore the mtime");

    let refused = edit.write().expect_err("a changed file is refused");
    assert!(
        refused.to_string().contains("changed"),
        "the refusal says why: {refused:#}"
    );
    assert_eq!(fs::read_to_string(&file).expect("read"), changed);
    let left: Vec<String> = fs::read_dir(case.dir.join("home"))
        .expect("read home")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, vec![".claude.json".to_owned()]);
}
