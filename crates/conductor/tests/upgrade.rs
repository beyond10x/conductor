//! `story:conductor-upgrade`: `conductor upgrade` checks the releases of the repository the
//! binary was built from, and downloads, verifies and installs one, keeping the old binary as
//! `conductor.previous`. One case per outcome the specification declares
//! (`conductor.config.UpgradeOutcome`, `conductor.config.UpgradeRefusal`).
//!
//! Each case copies the built binary into its own `install/` directory and runs the copy, so the
//! file it replaces is the case's. The environment is cleared: `HOME` names the case's `home/`,
//! `TMPDIR` its `tmp/`, and `PATH` starts with the case's `bin/`, which holds a fake `curl`, and
//! goes on with the test's own `PATH` for `tar`, `sha256sum` (or `shasum`) and `sh`. The fake
//! `curl` serves a fixture release from the case's `release/` directory: the API's JSON for the
//! latest release and for each tag, and each asset by the last two steps of its download URL. It
//! records every URL it was asked for in `curl.log`, and answers 22, as `curl --fail` does, for
//! one it has no file for. A release's binary is a script that prints `conductor <version>`.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::upgrade::TARGET;

/// The version of the binary under test.
const INSTALLED: &str = env!("CARGO_PKG_VERSION");

/// A release newer than any the repository will cut.
const NEWER: &str = "999.0.0";

/// The fake `curl`: the URL is its last word that is not a flag, and the file it writes the word
/// after `--output`.
const FAKE_CURL: &str = r#"#!/bin/sh
out=""
url=""
while [ $# -gt 0 ]; do
  case "$1" in
    --output) out="$2"; shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
printf '%s\n' "$url" >> "$FAKE_CURL_LOG"
case "$url" in
  */releases/latest) file="$FAKE_RELEASE_DIR/latest.json" ;;
  */releases/tags/*) file="$FAKE_RELEASE_DIR/tags/${url##*/}.json" ;;
  */releases/download/*) file="$FAKE_RELEASE_DIR/download/${url##*/releases/download/}" ;;
  *) exit 22 ;;
esac
[ -f "$file" ] || { echo "curl: (22) The requested URL returned error: 404" >&2; exit 22; }
cp "$file" "$out"
"#;

struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        // A read-only install directory is made writable again so it can be removed.
        let _ = fs::set_permissions(self.install(), fs::Permissions::from_mode(0o755));
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

/// What one release of the fixture holds.
struct Release<'a> {
    version: &'a str,
    /// Whether it has an archive for the binary's own target; one for another target it always has.
    archive_for_target: bool,
    /// The body of the archive's `conductor` after its `#!/bin/sh` line.
    script: String,
    /// Whether its `SHA256SUMS` line for the archive is the archive's sum.
    sum_matches: bool,
}

impl<'a> Release<'a> {
    fn good(version: &'a str) -> Self {
        Self {
            version,
            archive_for_target: true,
            script: format!("printf '%s\\n' 'conductor {version}'\n"),
            sum_matches: true,
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("upgrade")
            .join(name);
        if dir.exists() {
            let _ = fs::set_permissions(dir.join("install"), fs::Permissions::from_mode(0o755));
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in [
            "home",
            "bin",
            "tmp",
            "install",
            "build",
            "release/tags",
            "release/download",
        ] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        let curl = dir.join("bin/curl");
        fs::write(&curl, FAKE_CURL).expect("write the fake curl");
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        let built = PathBuf::from(common::conductor(dir.join("home")).get_program());
        fs::copy(&built, dir.join("install/conductor")).expect("copy the binary");
        Self { dir }
    }

    fn install(&self) -> PathBuf {
        self.dir.join("install")
    }

    fn binary(&self) -> PathBuf {
        self.install().join("conductor")
    }

    fn previous(&self) -> PathBuf {
        self.install().join("conductor.previous")
    }

    fn bytes(&self) -> Vec<u8> {
        fs::read(self.binary()).expect("read the installed binary")
    }

    fn curl_log(&self) -> String {
        fs::read_to_string(self.dir.join("curl.log")).unwrap_or_default()
    }

    /// Writes `release` into the fixture: its archives, its `SHA256SUMS`, and its API JSON as the
    /// tag `v<version>`, and as the latest release when `latest`.
    fn release(&self, release: &Release<'_>, latest: bool) {
        let version = release.version;
        let download = self
            .dir
            .join("release/download")
            .join(format!("v{version}"));
        fs::create_dir_all(&download).expect("create the release's download directory");
        let mut targets = vec!["example-unknown-target".to_owned()];
        if release.archive_for_target {
            targets.push(TARGET.to_owned());
        }
        let mut sums = String::new();
        let mut assets = Vec::new();
        for target in &targets {
            let package = format!("conductor-{version}-{target}");
            let staging = self.dir.join("build").join(&package);
            fs::create_dir_all(&staging).expect("create the package directory");
            let program = staging.join("conductor");
            fs::write(&program, format!("#!/bin/sh\n{}", release.script))
                .expect("write the release's binary");
            fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
                .expect("make it runnable");
            let archive = format!("{package}.tar.gz");
            let status = Command::new("tar")
                .arg("-C")
                .arg(self.dir.join("build"))
                .arg("-czf")
                .arg(download.join(&archive))
                .arg(&package)
                .status()
                .expect("tar runs");
            assert!(status.success(), "tar packs the fixture archive");
            let sum = if release.sum_matches || target != TARGET {
                sha256(&download.join(&archive))
            } else {
                "0".repeat(64)
            };
            sums.push_str(&format!("{sum}  {archive}\n"));
            assets.push(archive);
        }
        fs::write(download.join("SHA256SUMS"), sums).expect("write SHA256SUMS");
        assets.push("SHA256SUMS".to_owned());
        let assets: Vec<serde_json::Value> = assets
            .iter()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "browser_download_url": format!(
                        "https://example.invalid/example-org/conductor/releases/download/v{version}/{name}"
                    ),
                })
            })
            .collect();
        let json = serde_json::json!({ "tag_name": format!("v{version}"), "assets": assets });
        let text = serde_json::to_string_pretty(&json).expect("render the release JSON");
        fs::write(
            self.dir
                .join("release/tags")
                .join(format!("v{version}.json")),
            &text,
        )
        .expect("write the tag's JSON");
        if latest {
            fs::write(self.dir.join("release/latest.json"), &text).expect("write the latest JSON");
        }
    }

    /// Runs the installed copy with `args`.
    fn run(&self, args: &[&str]) -> Output {
        let path = format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        Command::new(self.binary())
            .args(args)
            .current_dir(self.dir.join("home"))
            .env_clear()
            .env("HOME", self.dir.join("home"))
            .env("PATH", path)
            .env("TMPDIR", self.dir.join("tmp"))
            .env("FAKE_RELEASE_DIR", self.dir.join("release"))
            .env("FAKE_CURL_LOG", self.dir.join("curl.log"))
            .stdin(Stdio::null())
            .output()
            .expect("the installed conductor runs")
    }

    fn upgrade(&self, args: &[&str]) -> Output {
        let mut all = vec!["upgrade"];
        all.extend_from_slice(args);
        self.run(&all)
    }
}

/// The SHA-256 of `file`, as `sha256sum`, or `shasum -a 256` where it is missing, prints it.
fn sha256(file: &Path) -> String {
    let output = Command::new("sha256sum")
        .arg(file)
        .output()
        .or_else(|_| {
            Command::new("shasum")
                .arg("-a")
                .arg("256")
                .arg(file)
                .output()
        })
        .expect("sha256sum or shasum runs");
    assert!(output.status.success(), "the sum is computed");
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .expect("a sum")
        .to_owned()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Asserts a refusal: exit 1, `what` on standard error, the copy byte-identical to `before` and
/// no `conductor.previous` written.
fn assert_refused(case: &Case, output: &Output, what: &str, before: &[u8]) {
    let stderr = text(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "refused, exit 1: {stderr}");
    assert!(
        stderr.contains(what),
        "standard error names `{what}`: {stderr}"
    );
    assert!(case.bytes() == before, "the installed binary is unchanged");
    assert!(
        !case.previous().exists(),
        "no conductor.previous is written"
    );
}

#[test]
fn check_with_a_newer_release_says_upgrade_available() {
    let case = Case::new("check-newer");
    case.release(&Release::good(NEWER), true);
    let before = case.bytes();

    let output = case.upgrade(&["--check"]);

    let stdout = text(&output.stdout);
    assert!(output.status.success(), "exit 0: {}", text(&output.stderr));
    assert!(
        stdout.contains(INSTALLED),
        "names the installed version: {stdout}"
    );
    assert!(stdout.contains(NEWER), "names the newest version: {stdout}");
    assert!(stdout.contains("upgrade available"), "{stdout}");
    assert!(case.bytes() == before, "--check changes no file");
    assert!(!case.previous().exists());
    assert!(
        !case.curl_log().contains(".tar.gz"),
        "--check downloads no archive: {}",
        case.curl_log()
    );
}

#[test]
fn check_at_the_newest_release_says_none_available() {
    let case = Case::new("check-current");
    case.release(&Release::good(INSTALLED), true);

    let output = case.upgrade(&["--check"]);

    let stdout = text(&output.stdout);
    assert!(output.status.success(), "exit 0: {}", text(&output.stderr));
    assert!(stdout.contains(INSTALLED), "{stdout}");
    assert!(stdout.contains("none available"), "{stdout}");
}

#[test]
fn upgrade_replaces_the_binary_and_keeps_the_previous_one() {
    let case = Case::new("upgraded");
    case.release(&Release::good(NEWER), true);
    let before = case.bytes();

    let output = case.upgrade(&[]);

    let stdout = text(&output.stdout);
    assert!(output.status.success(), "exit 0: {}", text(&output.stderr));
    assert!(stdout.contains("upgraded"), "{stdout}");
    assert!(
        stdout.contains(INSTALLED) && stdout.contains(NEWER),
        "{stdout}"
    );
    let version = case.run(&["--version"]);
    assert!(
        text(&version.stdout).contains(&format!("conductor {NEWER}")),
        "the installed binary is the release's: {}",
        text(&version.stdout)
    );
    assert!(
        fs::read(case.previous()).expect("conductor.previous is kept") == before,
        "conductor.previous holds the old bytes"
    );
    let leftovers: Vec<String> = fs::read_dir(case.install())
        .expect("read the install directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name != "conductor" && name != "conductor.previous")
        .collect();
    assert!(
        leftovers.is_empty(),
        "no staging file is left: {leftovers:?}"
    );
}

#[test]
fn upgrade_at_the_newest_release_says_already_current() {
    let case = Case::new("already-current");
    case.release(&Release::good(INSTALLED), true);
    let before = case.bytes();

    let output = case.upgrade(&[]);

    let stdout = text(&output.stdout);
    assert!(output.status.success(), "exit 0: {}", text(&output.stderr));
    assert!(stdout.contains("already current"), "{stdout}");
    assert!(case.bytes() == before, "the installed binary is unchanged");
    assert!(!case.previous().exists());
}

#[test]
fn a_release_without_this_targets_archive_is_refused() {
    let case = Case::new("no-archive");
    case.release(
        &Release {
            archive_for_target: false,
            ..Release::good(NEWER)
        },
        true,
    );
    let before = case.bytes();

    let output = case.upgrade(&[]);

    assert_refused(&case, &output, "no archive for this target", &before);
}

#[test]
fn a_sum_that_does_not_match_is_refused() {
    let case = Case::new("checksum-mismatch");
    case.release(
        &Release {
            sum_matches: false,
            ..Release::good(NEWER)
        },
        true,
    );
    let before = case.bytes();

    let output = case.upgrade(&[]);

    assert_refused(&case, &output, "checksum mismatch", &before);
}

#[test]
fn a_new_binary_that_does_not_run_is_refused() {
    let case = Case::new("does-not-run");
    case.release(
        &Release {
            script: "exit 3\n".to_owned(),
            ..Release::good(NEWER)
        },
        true,
    );
    let before = case.bytes();

    let output = case.upgrade(&[]);

    assert_refused(&case, &output, "new binary does not run", &before);
}

#[test]
fn a_binary_in_a_read_only_directory_is_refused() {
    let case = Case::new("not-writable");
    case.release(&Release::good(NEWER), true);
    let before = case.bytes();
    fs::set_permissions(case.install(), fs::Permissions::from_mode(0o555))
        .expect("make the install directory read-only");

    let output = case.upgrade(&[]);

    assert_refused(&case, &output, "binary not writable", &before);
}

#[test]
fn release_installs_the_named_release() {
    let case = Case::new("rollback");
    case.release(&Release::good(NEWER), true);
    case.release(&Release::good("0.6.0"), false);

    let output = case.upgrade(&["--release", "0.6.0"]);

    assert!(output.status.success(), "exit 0: {}", text(&output.stderr));
    let log = case.curl_log();
    assert!(
        log.contains(&format!("conductor-0.6.0-{TARGET}.tar.gz")),
        "the named release's archive is downloaded: {log}"
    );
    assert!(
        !log.contains(NEWER),
        "the latest release is not read: {log}"
    );
    let version = case.run(&["--version"]);
    assert!(
        text(&version.stdout).contains("conductor 0.6.0"),
        "the named release is installed: {}",
        text(&version.stdout)
    );
    assert!(case.previous().exists(), "conductor.previous is kept");
}

#[test]
fn a_release_that_cannot_be_read_is_refused() {
    let case = Case::new("release-unread");
    let before = case.bytes();

    let output = case.upgrade(&[]);

    assert_refused(&case, &output, "release unread", &before);
}
