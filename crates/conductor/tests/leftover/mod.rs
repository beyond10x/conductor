//! What a test case leaves running (`story:convoy-test-leaks-no-process`): a process whose
//! working directory lies under the case's directory outlives the case unless something ends it.
//! A case that starts a waiting command (`while [ ! -e stop ]; …`) ends it by writing `stop`;
//! these helpers check, after the case, that nothing is left, and end what is.
//!
//! The processes are found through `/proc/*/cwd`, on Linux only: elsewhere [`under`] answers none
//! and the check passes. A process another user owns, whose `cwd` cannot be read, is not counted.
//! The test process itself is never counted, and a process that has ended but is not reaped yet
//! (a zombie) has no `cwd` to read.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

/// How long the processes of a case that told them to end may take to end.
pub const END_WITHIN: Duration = Duration::from_secs(10);

/// Every process whose working directory lies under `dir`, as its pid and command line.
#[must_use]
pub fn under(dir: &Path) -> Vec<(i32, String)> {
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_owned());
    let Ok(entries) = fs::read_dir("/proc") else {
        return Vec::new();
    };
    let own = std::process::id();
    let mut found: Vec<(i32, String)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let pid: i32 = entry.file_name().to_str()?.parse().ok()?;
            if u32::try_from(pid).ok() == Some(own) {
                return None;
            }
            let cwd = fs::read_link(entry.path().join("cwd")).ok()?;
            let cwd = cwd.to_string_lossy();
            let cwd = PathBuf::from(cwd.strip_suffix(" (deleted)").unwrap_or(&cwd));
            cwd.starts_with(&dir).then(|| (pid, command_line(pid)))
        })
        .collect();
    found.sort_unstable();
    found
}

/// The command line of `pid`, its words joined by spaces; empty when it cannot be read.
fn command_line(pid: i32) -> String {
    fs::read(format!("/proc/{pid}/cmdline"))
        .map(|bytes| {
            String::from_utf8_lossy(&bytes)
                .split('\0')
                .filter(|word| !word.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// Sends SIGKILL to each process [`under`] `dir` names, by its pid; answers what it named.
pub fn kill_under(dir: &Path) -> Vec<(i32, String)> {
    let found = under(dir);
    for (pid, _) in &found {
        let _ = kill(Pid::from_raw(*pid), Signal::SIGKILL);
    }
    found
}

/// Asserts that no process has its working directory under `dir` once [`END_WITHIN`] has
/// passed: a case's commands, told to end, may take a moment. A process still there then is
/// killed by its pid before the assertion fails, so a failing check leaves nothing behind either.
///
/// # Panics
///
/// A process under `dir` outlived [`END_WITHIN`]; the message names its pid and command line.
pub fn assert_none_under(dir: &Path) {
    let deadline = Instant::now() + END_WITHIN;
    loop {
        if under(dir).is_empty() {
            return;
        }
        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let left = kill_under(dir);
    assert!(
        left.is_empty(),
        "{} process(es) outlived the case with their working directory under {}, killed now: {}",
        left.len(),
        dir.display(),
        left.iter()
            .map(|(pid, command)| format!("{pid} `{command}`"))
            .collect::<Vec<_>>()
            .join(", ")
    );
}
