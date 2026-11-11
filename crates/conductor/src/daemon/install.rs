//! `conductor daemon serve --install` and `--uninstall` (`story:daemon-reconcile`, design
//! architecture-next § 4): conductord as the user unit [`UNIT`]. `--install` writes the unit file
//! under the user's systemd directory ([`unit_file`]) running this binary's `daemon serve` with
//! the config file and the state directory it was given, then enables and starts it; it starts
//! again after a failure and at the next login. `--uninstall` disables and stops it and removes
//! the file, and says `not installed` when there is no file. On a box with no local operator
//! (`daemon.loopback: false`, `story:daemon-remote`) the unit is the system unit under
//! [`SYSTEM_DIR`] ([`system_unit_text`]), after [`remote_ready`].

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};

use super::units::Units;

/// The daemon's user unit.
pub const UNIT: &str = "conductord.service";

/// Where the unit file goes: `$XDG_CONFIG_HOME/systemd/user/`, or `~/.config/systemd/user/`
/// when that variable names no absolute directory.
#[must_use]
pub fn unit_file(home: &Path, xdg_config_home: Option<&Path>) -> PathBuf {
    xdg_config_home
        .filter(|dir| dir.is_absolute())
        .map_or_else(|| home.join(".config"), Path::to_owned)
        .join("systemd/user")
        .join(UNIT)
}

/// conductord's directory on this host as its installed user unit ([`unit_file`] of `home` and
/// `xdg_config_home`) names it: the `--state-dir` of the unit's `ExecStart`, as [`unit_text`]
/// writes it, else `home`/[`super::DIR`], the one conductord runs on without the flag. A daemon
/// started by hand on a directory of its own is not known here.
#[must_use]
pub fn daemon_dir(home: &Path, xdg_config_home: Option<&Path>) -> PathBuf {
    fs::read_to_string(unit_file(home, xdg_config_home))
        .ok()
        .and_then(|text| {
            let exec = text
                .lines()
                .find_map(|line| line.trim().strip_prefix("ExecStart="))?;
            let words = words(exec);
            let at = words.iter().position(|word| word == "--state-dir")?;
            words.get(at + 1).map(PathBuf::from)
        })
        .filter(|dir| dir.is_absolute())
        .unwrap_or_else(|| home.join(super::DIR))
}

/// The words of an `ExecStart` value: each in double quotes as [`quoted`] writes it, or bare.
fn words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut chars = line.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        let Some(first) = chars.next() else {
            return words;
        };
        let mut word = String::new();
        if first == '"' {
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => word.extend(chars.next()),
                    '%' => {
                        chars.next_if_eq(&'%');
                        word.push('%');
                    }
                    c => word.push(c),
                }
            }
        } else {
            word.push(first);
            while let Some(c) = chars.next_if(|c| !c.is_whitespace()) {
                word.push(c);
            }
        }
        words.push(word);
    }
}

/// One word of `ExecStart`, quoted as systemd reads it: in double quotes, with `\`, `"` written
/// escaped and `%` doubled.
fn quoted(word: &str) -> String {
    let escaped = word
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%");
    format!("\"{escaped}\"")
}

/// The unit file's text: `program [--config <file>] [--state-dir <dir>] daemon serve`, with
/// `path` as its `PATH`, restarted 10 s after a failure, wanted by the user's default target.
#[must_use]
pub fn unit_text(
    program: &Path,
    config: Option<&Path>,
    state: Option<&Path>,
    path: Option<&str>,
) -> String {
    let mut words = vec![program.display().to_string()];
    if let Some(config) = config {
        words.extend(["--config".to_owned(), config.display().to_string()]);
    }
    if let Some(state) = state {
        words.extend(["--state-dir".to_owned(), state.display().to_string()]);
    }
    words.extend(["daemon".to_owned(), "serve".to_owned()]);
    let exec: Vec<String> = words.iter().map(|word| quoted(word)).collect();
    let environment = path
        .map(|path| format!("Environment={}\n", quoted(&format!("PATH={path}"))))
        .unwrap_or_default();
    format!(
        "[Unit]\n\
         Description=conductord: reconciles the instances of the conductor config\n\
         \n\
         [Service]\n\
         ExecStart={}\n\
         {environment}\
         Restart=on-failure\n\
         RestartSec=10\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exec.join(" ")
    )
}

/// Writes `text` to `file` and enables and starts [`UNIT`]; answers the line to print.
///
/// # Errors
///
/// The file cannot be written, or the service manager refuses.
pub fn install(units: &mut dyn Units, file: &Path, text: &str) -> Result<String> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(file, text).with_context(|| format!("write {}", file.display()))?;
    units.enable(UNIT)?;
    Ok(format!("installed and started {UNIT}: {}", file.display()))
}

/// Disables and stops [`UNIT`] and removes `file`; answers the line to print, `not installed`
/// when there is no file.
///
/// # Errors
///
/// The service manager refuses, or the file cannot be removed.
pub fn uninstall(units: &mut dyn Units, file: &Path) -> Result<String> {
    if !file.exists() {
        return Ok(format!("{UNIT}: not installed"));
    }
    units.disable(UNIT)?;
    fs::remove_file(file).with_context(|| format!("remove {}", file.display()))?;
    Ok(format!("stopped and removed {UNIT}"))
}

/// The system manager's directory of installed units (`story:daemon-remote`).
pub const SYSTEM_DIR: &str = "/etc/systemd/system";

/// Where the system unit goes on a box with no local operator: [`SYSTEM_DIR`]/[`UNIT`].
#[must_use]
pub fn system_unit_file() -> PathBuf {
    Path::new(SYSTEM_DIR).join(UNIT)
}

/// The system unit's text (`story:daemon-remote`): `program --config <config> daemon serve` as
/// `user` with `home` as its home and `path` as its `PATH`, after the network is up, restarted
/// 10 s after a failure, started at boot.
#[must_use]
pub fn system_unit_text(
    program: &Path,
    config: &Path,
    user: Option<&str>,
    home: &Path,
    path: Option<&str>,
) -> String {
    let exec: Vec<String> = [
        program.display().to_string(),
        "--config".to_owned(),
        config.display().to_string(),
        "daemon".to_owned(),
        "serve".to_owned(),
    ]
    .iter()
    .map(|word| quoted(word))
    .collect();
    let user = user
        .map(|user| format!("User={user}\n"))
        .unwrap_or_default();
    let mut environment = format!(
        "Environment={}\n",
        quoted(&format!("HOME={}", home.display()))
    );
    if let Some(path) = path {
        environment.push_str(&format!(
            "Environment={}\n",
            quoted(&format!("PATH={path}"))
        ));
    }
    format!(
        "[Unit]\n\
         Description=conductord: reconciles the instances of the conductor config, on a box with \
         no local operator\n\
         Wants=network-online.target\n\
         After=network-online.target\n\
         \n\
         [Service]\n\
         ExecStart={}\n\
         {user}{environment}\
         Restart=on-failure\n\
         RestartSec=10\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        exec.join(" ")
    )
}

/// Whether the daemon of `daemon` may be installed as a box's system unit with the config file
/// `file`: its secrets on the Secrets service, a TLS listener, and the file in a git checkout
/// that does not ignore it, so every `PutConfig` commits there.
///
/// # Errors
///
/// Each condition that does not hold, by its YAML path or the file.
pub fn remote_ready(file: &Path, daemon: &conductor_model::config::Daemon) -> Result<()> {
    let what = match daemon.secrets.source {
        conductor_model::secret::Source::Remote => None,
        conductor_model::secret::Source::Keychain => {
            Some("keychain, the OS keychain of a workstation")
        }
        conductor_model::secret::Source::Files => {
            Some("files, a directory of mounted files conductor never writes")
        }
    };
    if let Some(what) = what {
        bail!(
            "daemon.secrets.source: is {what}; a box with no local operator reads its secrets \
             from the Secrets service: write source: remote and its url"
        );
    }
    if daemon.tls.is_none() {
        bail!("daemon.tls: a box with no local operator serves its API on the TLS listener alone");
    }
    let dir = file.parent().unwrap_or_else(|| Path::new("."));
    let name = file.file_name().unwrap_or_default();
    let git = |words: &[&std::ffi::OsStr]| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(words)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    };
    let inside = git(&["rev-parse".as_ref(), "--is-inside-work-tree".as_ref()]);
    let ignored = inside && git(&["check-ignore".as_ref(), "-q".as_ref(), "--".as_ref(), name]);
    if !inside || ignored {
        bail!(
            "{}: is in no git checkout{}: clone the config repository on the box and name its \
             file, so every config change commits there",
            file.display(),
            if ignored { " that keeps it" } else { "" }
        );
    }
    Ok(())
}

/// Writes `text` to `file` and enables and starts [`UNIT`] on the system manager; answers the
/// line to print.
///
/// # Errors
///
/// The file cannot be written (the install runs as root), or the service manager refuses.
pub fn install_system(units: &mut dyn Units, file: &Path, text: &str) -> Result<String> {
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    }
    fs::write(file, text).with_context(|| format!("write {}", file.display()))?;
    units.enable_system(UNIT)?;
    Ok(format!(
        "installed and started the system unit {UNIT}: {}",
        file.display()
    ))
}

/// Disables and stops the system unit [`UNIT`] and removes `file`; answers the line to print.
///
/// # Errors
///
/// The service manager refuses, or the file cannot be removed.
pub fn uninstall_system(units: &mut dyn Units, file: &Path) -> Result<String> {
    if !file.exists() {
        return Ok(format!("{UNIT}: no system unit installed"));
    }
    units.disable_system(UNIT)?;
    fs::remove_file(file).with_context(|| format!("remove {}", file.display()))?;
    Ok(format!("stopped and removed the system unit {UNIT}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_directory_of_a_unit_is_read_back_as_it_was_written() {
        let dir = Path::new("/srv/a \"b\" 100%\\c d");
        let text = unit_text(
            Path::new("/opt/conductor/bin/conductor"),
            None,
            Some(dir),
            None,
        );
        let exec = text
            .lines()
            .find_map(|line| line.strip_prefix("ExecStart="))
            .expect("an ExecStart");
        let words = words(exec);
        let at = words.iter().position(|word| word == "--state-dir");
        assert_eq!(
            at.and_then(|at| words.get(at + 1))
                .map(PathBuf::from)
                .as_deref(),
            Some(dir)
        );
        assert_eq!(words[words.len() - 2..], ["daemon", "serve"]);
    }
}
