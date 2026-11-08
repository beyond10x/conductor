//! `conductor daemon serve --install` and `--uninstall` (`story:daemon-reconcile`, design
//! architecture-next § 4): conductord as the user unit [`UNIT`]. `--install` writes the unit file
//! under the user's systemd directory ([`unit_file`]) running this binary's `daemon serve` with
//! the config file and the state directory it was given, then enables and starts it; it starts
//! again after a failure and at the next login. `--uninstall` disables and stops it and removes
//! the file, and says `not installed` when there is no file.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};

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
