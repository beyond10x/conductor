//! `story:gates-hooks-fresh`: `conductor repository refresh-hooks [--apply]` lists each checkout
//! of the instance whose Gates hook (`.git/b10x-gates-hooks/b10x-gates`) lags the `b10x-gates` on
//! `PATH` or is absent, and with `--apply` runs `b10x-gates install --repo <checkout>
//! --repository <owner>/<name> --policy <gates_policy>` in each and prints the hook's version
//! before (or `absent`) and after.
//!
//! Each case runs the built binary with `HOME` naming the case's `home/`, `CONDUCTOR_CONFIG` a
//! config file the case writes, and `PATH` led by the case's `bin/`, which holds a fake
//! `b10x-gates` this file writes at test time: `--version` prints `b10x-gates 0.1.17`, and
//! `install` appends its arguments to `install.log` and writes a hook printing that version into
//! the checkout `--repo` names. Three checkouts are laid out: `alpha` without a hook, `beta` whose
//! hook prints 0.1.14, and `gamma` whose hook prints 0.1.17.

mod common;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

/// The version the fake `b10x-gates` on `PATH` prints, and the one each hook it installs prints.
const CURRENT: &str = "0.1.17";

/// One case: `home/`, `bin/`, `checkouts/` and the config file under its own directory.
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
    fn new(name: &str, policy: bool) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("gates_hooks")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "bin", "checkouts", "records"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        let case = Self { dir };
        case.fake_gates();
        let policy = if policy {
            format!(
                "\x20   gates_policy: {}\n",
                case.dir.join("policy.json").display()
            )
        } else {
            String::new()
        };
        let text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   sources:\n\
             \x20     - github: example-org\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n\
             {policy}",
            root = case.dir.join("checkouts").display(),
            trees = case.dir.join("trees").display(),
            records = case.dir.join("records").display(),
        );
        fs::write(case.config(), text).expect("write the config file");
        case.checkout("alpha", None);
        case.checkout("beta", Some("0.1.14"));
        case.checkout("gamma", Some(CURRENT));
        case
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    fn install_log(&self) -> PathBuf {
        self.dir.join("install.log")
    }

    /// The fake `b10x-gates` in `bin/`.
    fn fake_gates(&self) {
        let hook = hook_text(CURRENT);
        let script = format!(
            "#!/bin/sh\n\
             if [ \"$1\" = --version ]; then echo 'b10x-gates {CURRENT}'; exit 0; fi\n\
             if [ \"$1\" != install ]; then exit 2; fi\n\
             printf '%s\\n' \"$*\" >> '{log}'\n\
             repo=.\n\
             while [ $# -gt 0 ]; do\n\
             \x20 if [ \"$1\" = --repo ]; then repo=\"$2\"; fi\n\
             \x20 shift\n\
             done\n\
             mkdir -p \"$repo/.git/b10x-gates-hooks\"\n\
             printf '%s' '{hook}' > \"$repo/.git/b10x-gates-hooks/b10x-gates\"\n\
             chmod 755 \"$repo/.git/b10x-gates-hooks/b10x-gates\"\n",
            log = self.install_log().display(),
        );
        executable(&self.dir.join("bin/b10x-gates"), &script);
    }

    /// A git checkout `checkouts/<name>` whose `origin` is `example-org/<name>` on GitHub, with a
    /// hook printing `hook` when one is given.
    fn checkout(&self, name: &str, hook: Option<&str>) -> PathBuf {
        let checkout = self.dir.join("checkouts").join(name);
        fs::create_dir_all(&checkout).expect("create a checkout");
        for args in [
            vec!["init", "--quiet"],
            vec![
                "remote",
                "add",
                "origin",
                &format!("git@github.com:example-org/{name}.git"),
            ],
        ] {
            let output = Command::new("git")
                .args(&args)
                .current_dir(&checkout)
                .env("HOME", self.dir.join("home"))
                .output()
                .expect("git runs");
            assert!(output.status.success(), "{output:?}");
        }
        if let Some(version) = hook {
            let dir = checkout.join(".git/b10x-gates-hooks");
            fs::create_dir_all(&dir).expect("create the hooks directory");
            executable(&dir.join("b10x-gates"), &hook_text(version));
        }
        checkout
    }

    /// `conductor repository refresh-hooks <args>`, isolated as the module says.
    fn refresh(&self, args: &[&str]) -> Output {
        let mut path = OsString::from(self.dir.join("bin"));
        path.push(":");
        path.push(std::env::var_os("PATH").unwrap_or_default());
        common::conductor(self.dir.join("home"))
            .args(["repository", "refresh-hooks"])
            .args(args)
            .current_dir(&self.dir)
            .env("CONDUCTOR_CONFIG", self.config())
            .env("PATH", path)
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// What the fake `install` was called with, one call per line.
    fn installs(&self) -> String {
        fs::read_to_string(self.install_log()).unwrap_or_default()
    }

    fn hook_of(&self, name: &str) -> PathBuf {
        self.dir
            .join("checkouts")
            .join(name)
            .join(".git/b10x-gates-hooks/b10x-gates")
    }
}

/// A hook that prints `b10x-gates <version>` for `--version`, as the installed one does.
fn hook_text(version: &str) -> String {
    format!("#!/bin/sh\necho \"b10x-gates {version}\"\n")
}

fn executable(file: &Path, text: &str) {
    fs::write(file, text).expect("write an executable");
    fs::set_permissions(file, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8 output")
}

fn describe(output: &Output) -> String {
    format!(
        "{}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Without `--apply`, the absent and the lagging checkout are listed, the current one is not,
/// and nothing is installed.
#[test]
fn the_absent_and_the_lagging_checkout_are_listed_and_nothing_is_installed() {
    let case = Case::new("list", true);
    let output = case.refresh(&[]);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(
        stdout(&output),
        "example-org/alpha: absent\nexample-org/beta: 0.1.14\n",
        "{}",
        describe(&output)
    );
    assert_eq!(case.installs(), "", "a listing installs nothing");
    assert!(!case.hook_of("alpha").exists());

    let output = case.refresh(&["--format", "json"]);
    assert!(output.status.success(), "{}", describe(&output));
    let rows: Value = serde_json::from_slice(&output.stdout).expect("JSON rows");
    let checkouts = case.dir.join("checkouts");
    assert_eq!(
        rows,
        json!([
            {
                "repository": "example-org/alpha",
                "checkout": checkouts.join("alpha").display().to_string(),
                "before": null,
                "after": null,
                "outcome": "Absent",
                "detail": null,
            },
            {
                "repository": "example-org/beta",
                "checkout": checkouts.join("beta").display().to_string(),
                "before": "0.1.14",
                "after": null,
                "outcome": "Lagging",
                "detail": null,
            },
        ])
    );
}

/// With `--apply`, `b10x-gates install` runs in the absent and the lagging checkout with the
/// instance's policy, and each line names the version before and after; the current checkout is
/// left alone, and a second run finds nothing.
#[test]
fn apply_installs_the_absent_and_the_lagging_hooks_and_prints_before_and_after() {
    let case = Case::new("apply", true);
    let output = case.refresh(&["--apply"]);
    assert!(output.status.success(), "{}", describe(&output));
    assert_eq!(
        stdout(&output),
        format!("example-org/alpha: absent -> {CURRENT}\nexample-org/beta: 0.1.14 -> {CURRENT}\n"),
        "{}",
        describe(&output)
    );
    let checkouts = case.dir.join("checkouts");
    let policy = case.dir.join("policy.json");
    assert_eq!(
        case.installs(),
        format!(
            "install --repo {alpha} --repository example-org/alpha --policy {policy}\n\
             install --repo {beta} --repository example-org/beta --policy {policy}\n",
            alpha = checkouts.join("alpha").display(),
            beta = checkouts.join("beta").display(),
            policy = policy.display(),
        )
    );

    let again = case.refresh(&["--apply"]);
    assert!(again.status.success(), "{}", describe(&again));
    assert_eq!(stdout(&again), "", "every hook is current now");
}

/// `--apply` without the instance's `gates_policy` installs nothing and names the key.
#[test]
fn apply_without_a_gates_policy_installs_nothing_and_names_the_key() {
    let case = Case::new("no-policy", false);
    let output = case.refresh(&["--apply"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("instances[0].gates_policy"),
        "{}",
        describe(&output)
    );
    assert_eq!(case.installs(), "");
}
