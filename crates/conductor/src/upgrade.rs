//! `conductor upgrade` (`story:conductor-upgrade`): checks for a newer release of the repository
//! the binary was built from, and downloads, verifies and installs one, the binding's `upgrade`
//! callable over `conductor.config.UpgradeBinary`.
//!
//! The repository is the crate's `repository` (`CARGO_PKG_REPOSITORY`), a GitHub repository; its
//! releases are read from the GitHub API, `releases/latest`, or `releases/tags/v<x.y.z>` for
//! `--release`. Each release carries `conductor-<version>-<target>.tar.gz` per target and one
//! `SHA256SUMS` (`.github/workflows/release.yml`); the target is the one the binary was built for
//! ([`TARGET`], from `build.rs`).
//!
//! The network is `curl`, the sum `sha256sum`, or `shasum -a 256` where that is missing, and the
//! unpacking `tar -xzf`, into a temporary directory under `TMPDIR`, removed afterwards. The
//! unpacked binary's `--version` must exit 0 before anything is installed. The binary's file,
//! symbolic links resolved, is then replaced by renames in its own directory: the old file
//! becomes `<name>.previous`, and the new one, staged beside it, takes its name.
//!
//! A refusal (`conductor.config.UpgradeRefused`) changes no file, names its
//! `conductor.config.UpgradeRefusal` on one line of standard error, and exits 1.

use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Result, bail};
use conductor_model::config::{UpgradeOutcome, UpgradeRefusal, UpgradeRefused, UpgradeReport};
use serde_json::Value;

use crate::cli::UpgradeArgs;

/// The target the binary was built for, as Cargo names it: the `<target>` of the release archive
/// it installs.
pub const TARGET: &str = env!("CONDUCTOR_BUILD_TARGET");

/// The repository the binary was built from, whose releases it reads.
pub const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The installed version: the crate's.
const INSTALLED: &str = env!("CARGO_PKG_VERSION");

const COMMAND: &str = "upgrade";

/// What a refusal answers before it is rendered.
type Refusal = (UpgradeRefusal, String);

/// One release, as the GitHub API describes it: its version and its assets by name.
struct Release {
    version: String,
    assets: Vec<(String, String)>,
}

impl Release {
    fn asset(&self, name: &str) -> Option<&str> {
        self.assets
            .iter()
            .find(|(asset, _)| asset == name)
            .map(|(_, url)| url.as_str())
    }
}

/// Runs `conductor upgrade`.
///
/// # Errors
///
/// A refusal, naming its `conductor.config.UpgradeRefusal`, or a failure to find the running
/// binary.
pub fn upgrade(args: &UpgradeArgs) -> Result<ExitCode> {
    let asked = args
        .release
        .as_deref()
        .map(|text| text.trim_start_matches('v'));
    let release_name = asked.unwrap_or("latest").to_owned();
    let refuse = |(refusal, detail): Refusal, release: &str| {
        refused(&UpgradeRefused {
            release: release.to_owned(),
            target: TARGET.to_owned(),
            refusal,
            detail,
        })
    };
    if let Some(asked) = asked
        && parse_version(asked).is_none()
    {
        return refuse(
            (
                UpgradeRefusal::ReleaseUnread,
                format!("--release {asked:?} is no x.y.z"),
            ),
            &release_name,
        );
    }
    let binary = installed_binary()?;
    let work = WorkDir::create()?;
    let release = match read_release(&work.path, asked) {
        Ok(release) => release,
        Err(refusal) => return refuse(refusal, &release_name),
    };
    let report = |outcome: UpgradeOutcome| UpgradeReport {
        installed: INSTALLED.to_owned(),
        release: release.version.clone(),
        outcome,
        binary: binary.display().to_string(),
    };
    if args.check.unwrap_or(false) {
        let newer = match (parse_version(&release.version), parse_version(INSTALLED)) {
            (Some(release), Some(installed)) => release > installed,
            _ => release.version != INSTALLED,
        };
        let outcome = if newer {
            UpgradeOutcome::UpgradeAvailable
        } else {
            UpgradeOutcome::NoneAvailable
        };
        println!("{}", render(&report(outcome)));
        return Ok(ExitCode::SUCCESS);
    }
    if release.version == INSTALLED {
        println!("{}", render(&report(UpgradeOutcome::AlreadyCurrent)));
        return Ok(ExitCode::SUCCESS);
    }
    if let Err(refusal) = install(&work.path, &release, &binary) {
        return refuse(refusal, &release.version);
    }
    println!("{}", render(&report(UpgradeOutcome::Upgraded)));
    Ok(ExitCode::SUCCESS)
}

/// Downloads, verifies, unpacks and runs `release`'s archive for [`TARGET`] under `work`, and
/// puts its binary in place of `binary`.
fn install(work: &Path, release: &Release, binary: &Path) -> Result<(), Refusal> {
    let package = format!("conductor-{}-{TARGET}", release.version);
    let archive_name = format!("{package}.tar.gz");
    let Some(archive_url) = release.asset(&archive_name) else {
        return Err((
            UpgradeRefusal::NoArchiveForTarget,
            format!("release {} has no {archive_name}", release.version),
        ));
    };
    let Some(sums_url) = release.asset("SHA256SUMS") else {
        return Err((
            UpgradeRefusal::ReleaseUnread,
            format!("release {} has no SHA256SUMS", release.version),
        ));
    };
    let directory = binary.parent().unwrap_or_else(|| Path::new("/"));
    let name = binary.file_name().map_or_else(
        || "conductor".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let staged = directory.join(format!(".{name}.upgrade-{}", std::process::id()));
    // Found before anything is downloaded: the directory takes a new file.
    if let Err(error) = fs::write(&staged, b"") {
        return Err(not_writable(directory, &error));
    }
    let _ = fs::remove_file(&staged);

    let archive = work.join(&archive_name);
    download(archive_url, &archive)?;
    let sums = work.join("SHA256SUMS");
    download(sums_url, &sums)?;
    let sums = fs::read_to_string(&sums).map_err(|error| {
        (
            UpgradeRefusal::ReleaseUnread,
            format!("SHA256SUMS: {error}"),
        )
    })?;
    let Some(expected) = expected_sum(&sums, &archive_name) else {
        return Err((
            UpgradeRefusal::ChecksumMismatch,
            format!("SHA256SUMS has no line for {archive_name}"),
        ));
    };
    let actual = sha256(&archive)?;
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err((
            UpgradeRefusal::ChecksumMismatch,
            format!("{archive_name} is {actual}, SHA256SUMS says {expected}"),
        ));
    }

    let unpacked = work.join("unpacked");
    fs::create_dir_all(&unpacked).map_err(|error| {
        (
            UpgradeRefusal::ReleaseUnread,
            format!("{}: {error}", unpacked.display()),
        )
    })?;
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&unpacked)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| (UpgradeRefusal::ReleaseUnread, format!("tar: {error}")))?;
    if !output.status.success() {
        return Err((
            UpgradeRefusal::ReleaseUnread,
            format!("tar -xzf {archive_name}: {}", first_line(&output.stderr)),
        ));
    }
    let new = unpacked.join(&package).join("conductor");
    let ran = Command::new(&new)
        .arg("--version")
        .stdin(Stdio::null())
        .output();
    match ran {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            return Err((
                UpgradeRefusal::NewBinaryDoesNotRun,
                format!(
                    "{package}/conductor --version exited {}: {}",
                    output
                        .status
                        .code()
                        .map_or_else(|| "on a signal".to_owned(), |code| code.to_string()),
                    first_line(&output.stderr)
                ),
            ));
        }
        Err(error) => {
            return Err((
                UpgradeRefusal::NewBinaryDoesNotRun,
                format!("{package}/conductor --version: {error}"),
            ));
        }
    }

    replace(&new, &staged, binary, directory, &name)
}

/// Stages `new` as `staged` beside `binary`, then renames `binary` to `<name>.previous` and
/// `staged` to `binary`. A failure removes the staged file and puts the old binary back.
fn replace(
    new: &Path,
    staged: &Path,
    binary: &Path,
    directory: &Path,
    name: &str,
) -> Result<(), Refusal> {
    let abandon = |error: &std::io::Error| {
        let _ = fs::remove_file(staged);
        not_writable(directory, error)
    };
    fs::copy(new, staged).map_err(|error| abandon(&error))?;
    let mode = fs::metadata(binary)
        .map(|metadata| metadata.permissions().mode())
        .unwrap_or(0o755)
        | 0o111;
    fs::set_permissions(staged, fs::Permissions::from_mode(mode))
        .map_err(|error| abandon(&error))?;
    let previous = directory.join(format!("{name}.previous"));
    fs::rename(binary, &previous).map_err(|error| abandon(&error))?;
    if let Err(error) = fs::rename(staged, binary) {
        let _ = fs::rename(&previous, binary);
        return Err(abandon(&error));
    }
    Ok(())
}

fn not_writable(directory: &Path, error: &std::io::Error) -> Refusal {
    (
        UpgradeRefusal::BinaryNotWritable,
        format!("{}: {error}", directory.display()),
    )
}

/// The running binary's file, symbolic links resolved.
fn installed_binary() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|error| anyhow::anyhow!("{COMMAND}: the running binary is not found: {error}"))?;
    Ok(fs::canonicalize(&exe).unwrap_or(exe))
}

/// The GitHub API's URL of the releases of [`REPOSITORY`]: `https://api.github.com/repos/<owner>/<repo>/releases`.
fn releases_url() -> Result<String, Refusal> {
    let path = REPOSITORY
        .strip_prefix("https://github.com/")
        .map(|rest| rest.trim_end_matches('/').trim_end_matches(".git"))
        .filter(|rest| {
            let steps: Vec<&str> = rest.split('/').collect();
            steps.len() == 2 && steps.iter().all(|step| !step.is_empty())
        });
    match path {
        Some(path) => Ok(format!("https://api.github.com/repos/{path}/releases")),
        None => Err((
            UpgradeRefusal::ReleaseUnread,
            format!("the binary was built from {REPOSITORY:?}, which is no GitHub repository"),
        )),
    }
}

/// Reads the release named `asked`, or the latest one.
fn read_release(work: &Path, asked: Option<&str>) -> Result<Release, Refusal> {
    let base = releases_url()?;
    let url = match asked {
        Some(version) => format!("{base}/tags/v{version}"),
        None => format!("{base}/latest"),
    };
    let file = work.join("release.json");
    download(&url, &file)?;
    let unread = |detail: String| (UpgradeRefusal::ReleaseUnread, detail);
    let text = fs::read_to_string(&file).map_err(|error| unread(format!("{url}: {error}")))?;
    let json: Value =
        serde_json::from_str(&text).map_err(|error| unread(format!("{url}: {error}")))?;
    let Some(tag) = json.get("tag_name").and_then(Value::as_str) else {
        return Err(unread(format!("{url}: no tag_name")));
    };
    let version = tag.trim_start_matches('v').to_owned();
    if parse_version(&version).is_none() {
        return Err(unread(format!("{url}: tag {tag:?} names no x.y.z")));
    }
    let assets = json
        .get("assets")
        .and_then(Value::as_array)
        .map(|assets| {
            assets
                .iter()
                .filter_map(|asset| {
                    let name = asset.get("name")?.as_str()?;
                    let url = asset.get("browser_download_url")?.as_str()?;
                    Some((name.to_owned(), url.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Release { version, assets })
}

/// Downloads `url` into `file` with `curl`.
fn download(url: &str, file: &Path) -> Result<(), Refusal> {
    let output = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--output",
        ])
        .arg(file)
        .arg(url)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| (UpgradeRefusal::ReleaseUnread, format!("curl: {error}")))?;
    if output.status.success() {
        Ok(())
    } else {
        Err((
            UpgradeRefusal::ReleaseUnread,
            format!("curl {url}: {}", first_line(&output.stderr)),
        ))
    }
}

/// The SHA-256 of `file` in hex, from `sha256sum`, or `shasum -a 256` where that is missing.
fn sha256(file: &Path) -> Result<String, Refusal> {
    let output = match Command::new("sha256sum")
        .arg(file)
        .stdin(Stdio::null())
        .output()
    {
        Err(error) if error.kind() == ErrorKind::NotFound => Command::new("shasum")
            .args(["-a", "256"])
            .arg(file)
            .stdin(Stdio::null())
            .output(),
        other => other,
    }
    .map_err(|error| {
        (
            UpgradeRefusal::ChecksumMismatch,
            format!("no sum computed: {error}"),
        )
    })?;
    let text = String::from_utf8_lossy(&output.stdout);
    match text.split_whitespace().next() {
        Some(sum) if output.status.success() => Ok(sum.to_owned()),
        _ => Err((
            UpgradeRefusal::ChecksumMismatch,
            format!("no sum computed: {}", first_line(&output.stderr)),
        )),
    }
}

/// The sum `SHA256SUMS` gives for `name`: a line `<sum>  <name>`, or `<sum> *<name>`.
fn expected_sum(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        let sum = words.next()?;
        let file = words.next()?.trim_start_matches('*');
        (file == name && words.next().is_none()).then(|| sum.to_owned())
    })
}

/// `x.y.z` as three numbers.
fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((major, minor, patch))
}

fn first_line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// The report's line on standard output.
fn render(report: &UpgradeReport) -> String {
    let UpgradeReport {
        installed,
        release,
        binary,
        ..
    } = report;
    match report.outcome {
        UpgradeOutcome::UpgradeAvailable => {
            format!("installed {installed}, newest {release}: upgrade available")
        }
        UpgradeOutcome::NoneAvailable => {
            format!("installed {installed}, newest {release}: none available")
        }
        UpgradeOutcome::Upgraded => format!(
            "upgraded from {installed} to {release}: {binary}, the previous binary kept as \
             {binary}.previous"
        ),
        UpgradeOutcome::AlreadyCurrent => format!("{installed} is already current: {binary}"),
    }
}

/// Answers a refusal: one line naming its `UpgradeRefusal`, and exit 1.
fn refused(answer: &UpgradeRefused) -> Result<ExitCode> {
    bail!(
        "{COMMAND}: refused: {}: release {}, target {}: {}",
        refusal_name(answer.refusal),
        answer.release,
        answer.target,
        answer.detail
    )
}

/// The refusal as the command prints it. Exhaustive, so a refusal the specification adds fails
/// to compile here rather than rendering under a guessed name.
const fn refusal_name(refusal: UpgradeRefusal) -> &'static str {
    match refusal {
        UpgradeRefusal::ReleaseUnread => "release unread",
        UpgradeRefusal::NoArchiveForTarget => "no archive for this target",
        UpgradeRefusal::ChecksumMismatch => "checksum mismatch",
        UpgradeRefusal::NewBinaryDoesNotRun => "new binary does not run",
        UpgradeRefusal::BinaryNotWritable => "binary not writable",
    }
}

/// A directory of its own under `TMPDIR` for the downloads, removed when dropped.
struct WorkDir {
    path: PathBuf,
}

impl WorkDir {
    fn create() -> Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let path =
            std::env::temp_dir().join(format!("conductor-upgrade-{}-{nanos}", std::process::id()));
        if let Err(error) = fs::create_dir(&path) {
            bail!("{COMMAND}: {}: {error}", path.display());
        }
        Ok(Self { path })
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
