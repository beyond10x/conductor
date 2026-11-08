//! conductord's global API on the loopback listener (`story:daemon-reconcile`, design
//! architecture-next § 5.2): `GET /v1/instances` answers the rows the reconciler last published
//! ([`super::reconcile::Daemon::rows`]) as one JSON list. Any other path is 404 and any other
//! method on it 405, each with a JSON body naming why.

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

/// The rows the reconciler publishes and the API serves.
pub type Rows = Arc<Mutex<Vec<Value>>>;

/// The route of the instances list.
pub const INSTANCES: &str = "/v1/instances";

/// How long a client may take to send its request.
const READ_WAIT: Duration = Duration::from_secs(5);

/// Serves `listener` until it fails, one connection at a time.
pub fn serve(listener: &TcpListener, rows: &Rows) {
    for stream in listener.incoming().flatten() {
        // A client that goes away mid-request costs only its own answer.
        let _ = answer(stream, rows);
    }
}

fn answer(stream: TcpStream, rows: &Rows) -> std::io::Result<()> {
    stream.set_read_timeout(Some(READ_WAIT))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut request = String::new();
    reader.read_line(&mut request)?;
    // The headers are read and dropped: the route needs none.
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line.trim_end().is_empty() {
            break;
        }
    }
    let mut words = request.split_whitespace();
    let method = words.next().unwrap_or_default();
    let target = words.next().unwrap_or_default();
    let path = target.split('?').next().unwrap_or_default();
    let (status, body) = match (method, path) {
        ("GET", INSTANCES) => {
            let rows = rows.lock().map(|rows| rows.clone()).unwrap_or_default();
            ("200 OK", Value::Array(rows))
        }
        (_, INSTANCES) => (
            "405 Method Not Allowed",
            json!({"error": format!("{INSTANCES} answers GET only")}),
        ),
        _ => (
            "404 Not Found",
            json!({"error": format!("no route {path}")}),
        ),
    };
    let body = body.to_string();
    let mut stream = stream;
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()
}
