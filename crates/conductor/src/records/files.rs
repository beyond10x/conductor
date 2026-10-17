//! What `records check` reads in files rather than log lines (`story:rules-into-code`): the
//! charters, for a raw number a threshold key covers (`RawThreshold`), and the role rule files,
//! for the cap on their rules (`RoleFileRules`).

use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use conductor_model::config::Thresholds;
use conductor_model::dispatch::{RecordsFinding, RecordsFindingKind};

/// The directory of the charters under the records directory.
const CHARTERS: &str = "charters";

/// The directory of the role rule files under the records directory.
pub const RULES_DIR: &str = "rules";

/// The roles that have a rule file, `<records>/rules/<role>.md`.
pub const ROLE_FILES: [&str; 5] = ["all", "conductor", "controller", "worker", "maintainer"];

/// The most rules a role file holds.
pub const RULE_CAP: usize = 20;

/// Every finding of the charters and role rule files under `records`, files in name order.
///
/// # Errors
///
/// A file that exists cannot be read.
pub fn findings(records: &Path, thresholds: &Thresholds) -> Result<Vec<RecordsFinding>> {
    let mut found = Vec::new();
    let mut charters: Vec<_> = fs::read_dir(records.join(CHARTERS))
        .into_iter()
        .flatten()
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .collect();
    charters.sort();
    for path in charters {
        let text = fs::read_to_string(&path)
            .with_context(|| format!("records check: read {}", path.display()))?;
        let name = format!("{CHARTERS}/{}", file_name(&path));
        found.extend(raw_thresholds(&name, &text, thresholds));
    }
    for role in ROLE_FILES {
        let path = records.join(RULES_DIR).join(format!("{role}.md"));
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        found.extend(role_file(&format!("{RULES_DIR}/{role}.md"), &text));
    }
    Ok(found)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn number(line: usize) -> i64 {
    i64::try_from(line).unwrap_or(i64::MAX)
}

/// The thresholds a charter must not write a raw number for, with their values in gibibytes.
fn covered(thresholds: &Thresholds) -> [(&'static str, i64); 5] {
    [
        ("disk_admit", thresholds.disk_admit.0),
        ("gate_stop", thresholds.gate_stop.0),
        ("build_slot", thresholds.build_slot.0),
        ("disk_low", thresholds.disk_low.0),
        ("disk_clear", thresholds.disk_clear.0),
    ]
}

/// The sizes in gibibytes `line` writes, as `20G`, `20GB`, `20 GiB`, each with its text.
fn sizes(line: &str) -> Vec<(i64, String)> {
    let bytes = line.as_bytes();
    let mut sizes = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        if !bytes[at].is_ascii_digit()
            || (at > 0 && (bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'.'))
        {
            at += 1;
            continue;
        }
        let start = at;
        while at < bytes.len() && bytes[at].is_ascii_digit() {
            at += 1;
        }
        let digits = &line[start..at];
        let mut unit = at;
        while unit < bytes.len() && bytes[unit] == b' ' {
            unit += 1;
        }
        let rest = &line[unit..];
        let length = ["GiB", "GB", "G"]
            .iter()
            .find(|name| rest.starts_with(**name))
            .map(|name| name.len());
        let Some(length) = length else {
            continue;
        };
        let after = rest[length..].chars().next();
        if after.is_some_and(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        if let Ok(value) = digits.parse::<i64>() {
            sizes.push((value, line[start..unit + length].to_owned()));
        }
        at = unit + length;
    }
    sizes
}

/// A finding per charter line that writes a size equal to a threshold's value.
fn raw_thresholds(file: &str, text: &str, thresholds: &Thresholds) -> Vec<RecordsFinding> {
    let keys = covered(thresholds);
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        for (value, written) in sizes(line) {
            let names: Vec<&str> = keys
                .iter()
                .filter(|(_, held)| *held == value)
                .map(|(key, _)| *key)
                .collect();
            if names.is_empty() {
                continue;
            }
            found.push(RecordsFinding {
                file: file.to_owned(),
                line: number(index + 1),
                finding: RecordsFindingKind::RawThreshold,
                detail: format!(
                    "`{written}` is the value of thresholds.{}: write the key, not the number",
                    names.join(" or thresholds.")
                ),
            });
        }
    }
    found
}

/// Whether `line` opens a rule: `- ` or a number and `. ` at the margin.
#[must_use]
pub fn opens_rule(line: &str) -> bool {
    line.starts_with("- ") || {
        let digits = line.bytes().take_while(u8::is_ascii_digit).count();
        digits > 0 && line[digits..].starts_with(". ")
    }
}

/// A finding at the 21st rule of a role file and at the second line of each rule that has one.
fn role_file(file: &str, text: &str) -> Vec<RecordsFinding> {
    let mut found = Vec::new();
    let mut rules = 0;
    let mut open = false;
    for (index, line) in text.lines().enumerate() {
        if opens_rule(line) {
            rules += 1;
            open = true;
            if rules == RULE_CAP + 1 {
                found.push(RecordsFinding {
                    file: file.to_owned(),
                    line: number(index + 1),
                    finding: RecordsFindingKind::RoleFileRules,
                    detail: format!(
                        "a rule file holds at most {RULE_CAP} rules; this is the {rules}st"
                    ),
                });
            }
        } else if line.trim().is_empty() || line.starts_with('#') {
            open = false;
        } else if open {
            open = false;
            found.push(RecordsFinding {
                file: file.to_owned(),
                line: number(index + 1),
                finding: RecordsFindingKind::RoleFileRules,
                detail: "a rule is one line; this continues the rule above".to_owned(),
            });
        }
    }
    found
}
