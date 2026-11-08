//! The page header's instance selector (`story:dashboard-instance-switch`): `/api/instances`
//! answers every instance of the config file this process read, with its dashboard's port and
//! page, and which one this server is: the instance whose dashboard port ([`config::legacy`]) is
//! the port it listens on.
//!
//! Each instance keeps its own dashboard on its own port; the selector only opens another one's
//! page, so no answer of another dashboard is read here. The host registry that listed the
//! instances of every config file is gone in v1 (design architecture-next § 11: one daemon reads
//! one file and serves the one dashboard). When no instance holds this server's port (`--port`
//! naming another), this server is listed first, under the name of the instance this process
//! runs, in place of that instance's row. Nothing is read that can fail, so `errors` is empty.

use conductor_model::config::InstanceName;
use serde_json::{Value, json};

use crate::config;

/// The selector's data.
pub const INSTANCES_PATH: &str = "/api/instances";

/// One instance of the selector: its name, its dashboard's port and page, and whether this
/// server is its dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The instance.
    pub name: InstanceName,
    /// Its dashboard's port.
    pub dashboard_port: i64,
    /// Its dashboard's page.
    pub url: String,
    /// Whether this server listens on that port.
    pub current: bool,
}

/// The page of the dashboard on `port`.
#[must_use]
pub fn url(port: i64) -> String {
    format!("http://127.0.0.1:{port}/")
}

/// The instances of the config file this process read, the one on `port` marked current; `name`
/// is this process's instance, listed when none holds `port`.
#[must_use]
pub fn list(port: u16, name: &str) -> Vec<Row> {
    let port = i64::from(port);
    let active = config::active();
    let mut rows: Vec<Row> = Vec::new();
    if active.from_file {
        for instance in active.all() {
            let dashboard_port = config::legacy(instance).dashboard_port;
            if rows.iter().any(|row| row.dashboard_port == dashboard_port) {
                continue;
            }
            rows.push(Row {
                current: dashboard_port == port,
                url: url(dashboard_port),
                dashboard_port,
                name: instance.name.clone(),
            });
        }
    }
    if !rows.iter().any(|row| row.current) {
        // This server, on a port its instance does not write: its instance's row is this one.
        rows.retain(|row| row.name.0 != name);
        rows.insert(
            0,
            Row {
                name: InstanceName(name.to_owned()),
                dashboard_port: port,
                url: url(port),
                current: true,
            },
        );
    }
    rows
}

/// The answer of [`INSTANCES_PATH`]: `instances`, one object per row, and `errors`, empty.
#[must_use]
pub fn answer(port: u16, name: &str) -> Value {
    let rows = list(port, name);
    json!({
        "instances": rows.iter().map(|row| json!({
            "name": row.name.0,
            "dashboard_port": row.dashboard_port,
            "url": row.url,
            "current": row.current,
        })).collect::<Vec<_>>(),
        "errors": Vec::<String>::new(),
    })
}
