//! `conductor host slices` (`story:slice-remove`): the systemd user slices of the `conductor`
//! family, over the `local` callable `host-slices` and its input `conductor.host.ListSlices`
//! (`spec/domains/host.yaml`, with the row `conductor.host.SliceRow`).
//!
//! A slice outlives the last session that ran in it, so an instance that was removed or renamed
//! keeps `conductor_<prefix>.slice` loaded. The command asks the user manager, with `systemctl
//! --user`, for the loaded slices named `conductor*.slice` and for every loaded unit's slice, and
//! prints one row per slice, by name: the slice, the number of units loaded in it, and the
//! instance of the config file whose `session_prefix` names it (`-` for none). `--gc` then stops,
//! with `systemctl --user stop <slice>`, each slice that holds no unit and that no instance of
//! the file names; a slice that holds a unit, and the slice of an instance, is never stopped. The
//! rows are text, a tab between the three columns and a fourth, `stopped`, for a slice `--gc`
//! stopped, or with `--format json` a list of rows. The command opens no store and writes no
//! record; the slices of other tools are not listed.

use std::collections::BTreeMap;
use std::io::{self, Write as _};
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, bail};
use conductor_model::host::{ListSlices, SliceRow};
use serde_json::{Value, json};

use crate::cli::HostSlicesArgs;
use crate::collect;
use crate::config;
use crate::confine::systemd::{SYSTEMCTL, slice_name};
use crate::confine::{BOUND, exited};

/// The command's words, as its messages name it.
const COMMAND: &str = "host slices";

/// `conductor host slices`: the command the module documentation describes.
///
/// # Errors
///
/// `--format` is neither text nor json, `systemctl` cannot be run or fails, a slice cannot be
/// stopped, or standard output cannot be written.
pub fn host_slices(args: &HostSlicesArgs) -> Result<ExitCode> {
    let input = ListSlices {
        format: args.format.clone(),
        gc: args.gc,
    };
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is neither text nor json"),
    };
    let mut rows = list()?;
    if input.gc == Some(true) {
        for row in &mut rows {
            if row.units == 0 && row.instance.is_none() {
                systemctl(&["--user", "stop", &row.slice])
                    .with_context(|| format!("{COMMAND}: stop {}", row.slice))?;
                row.stopped = true;
            }
        }
    }
    let mut out = io::stdout().lock();
    if json {
        let rows: Vec<Value> = rows
            .iter()
            .map(|row| {
                json!({
                    "slice": row.slice,
                    "instance": row.instance,
                    "units": row.units,
                    "stopped": row.stopped,
                })
            })
            .collect();
        writeln!(out, "{}", Value::Array(rows))
    } else {
        rows.iter().try_for_each(|row| {
            let stopped = if row.stopped { "\tstopped" } else { "" };
            writeln!(
                out,
                "{}\t{}\t{}{stopped}",
                row.slice,
                row.units,
                row.instance.as_deref().unwrap_or("-")
            )
        })
    }
    .context("write standard output")?;
    Ok(ExitCode::SUCCESS)
}

/// The loaded slices of the `conductor` family, by name, each with its unit count and instance.
fn list() -> Result<Vec<SliceRow>> {
    let listed = systemctl(&[
        "--user",
        "list-units",
        "--all",
        "--plain",
        "--no-legend",
        "--no-pager",
        "--type=slice",
        "conductor*.slice",
    ])?;
    let mut rows: BTreeMap<String, SliceRow> = BTreeMap::new();
    for line in listed.lines() {
        let Some(slice) = line.split_whitespace().next() else {
            continue;
        };
        if !is_family(slice) {
            continue;
        }
        let instance = config::active()
            .all()
            .into_iter()
            .find(|instance| {
                slice_name(
                    instance
                        .session_prefix
                        .as_ref()
                        .map(|prefix| prefix.0.as_str()),
                ) == slice
            })
            .map(|instance| instance.name.0.clone());
        rows.insert(
            slice.to_owned(),
            SliceRow {
                slice: slice.to_owned(),
                instance,
                units: 0,
                stopped: false,
            },
        );
    }
    if !rows.is_empty() {
        let shown = systemctl(&["--user", "show", "-p", "Id", "-p", "Slice", "*"])?;
        for block in shown.split("\n\n") {
            let slice = block
                .lines()
                .find_map(|line| line.strip_prefix("Slice="))
                .map(str::trim);
            if let Some(row) = slice.and_then(|slice| rows.get_mut(slice)) {
                row.units += 1;
            }
        }
    }
    Ok(rows.into_values().collect())
}

/// Whether `name` is the slice of an instance, `conductor.slice` or `conductor_<prefix>.slice`:
/// no other tool's.
fn is_family(name: &str) -> bool {
    name == "conductor.slice"
        || name
            .strip_prefix("conductor_")
            .is_some_and(|rest| rest.len() > ".slice".len() && rest.ends_with(".slice"))
}

/// Runs `systemctl <args>` within [`BOUND`] and answers its standard output; fails naming how it
/// ended when it did not succeed.
fn systemctl(args: &[&str]) -> Result<String> {
    let shown = format!("{SYSTEMCTL} {}", args.join(" "));
    let output = collect::run(Command::new(SYSTEMCTL).args(args), BOUND)
        .with_context(|| format!("{COMMAND}: run `{shown}`"))?;
    if !output.status.success() {
        bail!("{COMMAND}: `{shown}` {}", exited(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
