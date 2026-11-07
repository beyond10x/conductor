//! The page header's instance selector (`story:dashboard-instance-switch`): `/api/instances`
//! answers every instance of the host registry ([`crate::registry`], `instances.json` under the
//! home directory's host directory) as a `conductor.host.DashboardInstance`, with its dashboard's
//! port and page, and which one this server is: the instance whose `dashboard_port` is the port it
//! listens on.
//!
//! Each instance keeps its own dashboard on its own port; the selector only opens another one's
//! page, so no answer of another dashboard is read here. The registry is read at each request and
//! nothing is written. When no registered instance holds this server's port (a registry not yet
//! written, or `--port` naming another), this server is listed first, under the name of the
//! instance this process runs. A registry that cannot be read is named in `errors`, and this
//! server is still listed.

use std::path::Path;

use conductor_model::config::InstanceName;
use conductor_model::host::DashboardInstance;
use serde_json::{Value, json};

use crate::registry;

/// The selector's data.
pub const INSTANCES_PATH: &str = "/api/instances";

/// The page of the dashboard on `port`.
#[must_use]
pub fn url(port: i64) -> String {
    format!("http://127.0.0.1:{port}/")
}

/// The instances of the host registry under `home`, the one on `port` marked current, and the
/// problems reading it; `name` is this process's instance, listed when none holds `port`.
#[must_use]
pub fn list(home: &Path, port: u16, name: &str) -> (Vec<DashboardInstance>, Vec<String>) {
    let mut errors = Vec::new();
    let registered = match registry::read(home) {
        Ok(registry) => registry.files,
        Err(error) => {
            errors.push(format!("{error:#}"));
            Vec::new()
        }
    };
    let port = i64::from(port);
    let mut rows: Vec<DashboardInstance> = Vec::new();
    for instance in registered.into_iter().flat_map(|file| file.instances) {
        if rows
            .iter()
            .any(|row| row.dashboard_port == instance.dashboard_port)
        {
            continue;
        }
        rows.push(DashboardInstance {
            current: instance.dashboard_port == port,
            url: url(instance.dashboard_port),
            dashboard_port: instance.dashboard_port,
            name: instance.name,
        });
    }
    if !rows.iter().any(|row| row.current) {
        rows.insert(
            0,
            DashboardInstance {
                name: InstanceName(name.to_owned()),
                dashboard_port: port,
                url: url(port),
                current: true,
            },
        );
    }
    (rows, errors)
}

/// The answer of [`INSTANCES_PATH`]: `instances`, one object per row, and `errors`.
#[must_use]
pub fn answer(home: &Path, port: u16, name: &str) -> Value {
    let (rows, errors) = list(home, port, name);
    json!({
        "instances": rows.iter().map(|row| json!({
            "name": row.name.0,
            "dashboard_port": row.dashboard_port,
            "url": row.url,
            "current": row.current,
        })).collect::<Vec<_>>(),
        "errors": errors,
    })
}
