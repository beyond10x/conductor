//! `conductor status` with no instance (`story:federated-views`, decision F5): with neither
//! `--state-dir` nor `CONDUCTOR_INSTANCE`, the status is asked of conductord on
//! `127.0.0.1:<daemon.port>` of the config file first, and when it answers within
//! `daemon.timeout` (plus that again for the instances it asks) its federated view is printed: one
//! row per instance with its state, the operator items it has unanswered and its open decision
//! requests, and an instance that does not answer is a row `unreachable` with the time it was last
//! seen, not an error. When the daemon does not answer, or the file does not load, the local read
//! runs as before.

use std::io::{Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use anyhow::{Result, bail};
use serde_json::{Value, json};

use crate::config::{self, Environment};

/// The federated view as `conductor status` prints it, or none when the daemon does not answer
/// (or the config file does not load); `json` prints JSON.
#[must_use]
pub fn view(config_flag: Option<&Path>, json: bool) -> Option<String> {
    let environment = Environment::from_process().ok()?;
    if environment.instance.is_some() {
        return None;
    }
    let home = environment.home().ok()?.to_owned();
    let (file, _) = config::locate(config_flag, &environment).ok()?;
    let text = std::fs::read_to_string(environment.cwd.join(file)).ok()?;
    let loaded = config::parse(&text, &home).ok()?;
    let port = u16::try_from(loaded.daemon.port).ok()?;
    let timeout = Duration::from_secs(
        config::seconds(&loaded.daemon.timeout).unwrap_or(config::FEDERATED_TIMEOUT),
    );
    let instances = get(port, "/v1/instances", timeout).ok()?;
    let unanswered = get(port, "/v1/unanswered", timeout).ok()?;
    let decisions = get(port, "/v1/decisions", timeout).ok()?;
    let rows = rows(&instances, &unanswered, &decisions);
    Some(if json {
        format!(
            "{}\n",
            json!({"daemon": format!("127.0.0.1:{port}"), "instances": rows})
        )
    } else {
        text_of(port, &rows)
    })
}

/// One row per instance the daemon lists: from `instances` its state, from the federated
/// `unanswered` its count or its unreachable row, from `decisions` the count of its open requests.
fn rows(instances: &Value, unanswered: &Value, decisions: &Value) -> Vec<Value> {
    let federated = |view: &Value, name: &str| -> Vec<Value> {
        view["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| row["instance"] == name)
            .cloned()
            .collect()
    };
    instances
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|instance| {
            let name = instance["name"].as_str()?;
            let asked = federated(unanswered, name);
            let unreachable = asked
                .iter()
                .find(|row| row["state"] == "unreachable")
                .cloned();
            Some(match unreachable {
                Some(row) => json!({
                    "instance": name,
                    "state": row["state"],
                    "last_seen": row["last_seen"],
                    "error": row["error"],
                }),
                None => json!({
                    "instance": name,
                    "state": instance["state"].as_str().map(str::to_ascii_lowercase),
                    "last_seen": instance["last_seen"],
                    "unanswered": asked.first().map_or(Value::Null, |row| row["count"].clone()),
                    "open_decisions": federated(decisions, name)
                        .iter()
                        .filter(|row| row["state"] != "unreachable")
                        .count(),
                }),
            })
        })
        .collect()
}

/// The rows as text: a header naming the daemon, then one line per instance.
fn text_of(port: u16, rows: &[Value]) -> String {
    let mut out = format!(
        "conductord on 127.0.0.1:{port}: {} instance(s)\n",
        rows.len()
    );
    for row in rows {
        let name = row["instance"].as_str().unwrap_or("?");
        let state = row["state"].as_str().unwrap_or("?");
        if state == "unreachable" {
            out.push_str(&format!(
                "{name}  unreachable  last seen {}\n",
                row["last_seen"].as_str().unwrap_or("never")
            ));
        } else {
            out.push_str(&format!(
                "{name}  {state}  unanswered {}  open decisions {}\n",
                row["unanswered"]
                    .as_u64()
                    .map_or_else(|| "?".to_owned(), |n| n.to_string()),
                row["open_decisions"]
            ));
        }
    }
    out
}

/// `GET target` of the daemon's loopback listener on `port`: connected within `timeout`, answered
/// within twice that and a second (the daemon asks each instance within `timeout`).
fn get(port: u16, target: &str, timeout: Duration) -> Result<Value> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout * 2 + Duration::from_secs(1)))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(
        format!(
            "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: application/json\r\n\
             Connection: close\r\n\r\n"
        )
        .as_bytes(),
    )?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    let (status, body) = crate::daemon::api::split(&bytes)?;
    if status != 200 {
        bail!("the daemon answered {status}");
    }
    Ok(serde_json::from_slice(&body)?)
}
