//! The terminal UI against a fake daemon on loopback: what it reads from the global API, what it
//! draws into a test backend's buffer, its keys, and the attach route a remote daemon answers.

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use serde_json::{Value, json};

use super::client::Client;
use super::*;

/// A daemon on loopback answering each path with `(status, body)` from `answer`; the paths it
/// was asked are kept.
fn fake_daemon(answer: fn(&str) -> (u16, Value)) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let asked = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&asked);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut line = String::new();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            reader.read_line(&mut line).unwrap();
            loop {
                let mut header = String::new();
                if reader.read_line(&mut header).unwrap() == 0 || header == "\r\n" {
                    break;
                }
            }
            let path = line.split(' ').nth(1).unwrap_or_default().to_owned();
            kept.lock().unwrap().push(path.clone());
            let (status, body) = answer(&path);
            let body = body.to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (format!("http://127.0.0.1:{port}"), asked)
}

fn daemon(path: &str) -> (u16, Value) {
    match path {
        "/v1/instances" => (
            200,
            json!([
                {"name": "alpha", "state": "Running"},
                {"name": "beta", "state": "Running"},
            ]),
        ),
        "/v1/decisions" => (
            200,
            json!({"rows": [
                {"instance": "alpha", "id": "d1"},
                {"instance": "alpha", "id": "d2"},
                {"instance": "beta", "state": "unreachable", "last_seen": null, "error": "x"},
            ]}),
        ),
        "/v1/kpis" => (
            200,
            json!({"rows": [
                {"instance": "alpha", "id": "k1", "value": 5.0, "target": 2.0, "direction": "lower"},
                {"instance": "alpha", "id": "k2", "value": 1.0, "target": 2.0, "direction": "lower"},
                {"instance": "alpha", "id": "k3", "measured": false, "value": null, "target": 1.0},
            ]}),
        ),
        "/v1/sessions" => (
            200,
            json!({"rows": [
                {"instance": "alpha", "name": "alpha-conductor", "role": "conductor",
                 "state": "busy", "status": "busy", "repository": null, "dispatch": null},
                {"instance": "alpha", "name": "alpha-repo1", "role": "controller",
                 "state": "idle", "repository": "repo1", "context": "38k",
                 "dispatch": {"goal": "story:x"}},
                {"instance": "beta", "state": "unreachable", "last_seen": null, "error": "x"},
            ]}),
        ),
        "/v1/instances/alpha/settings" => (
            200,
            json!({"fields": [
                {"path": "cadence.cycle", "value": "2h", "takes_effect": "Cycle"},
                {"path": "thresholds.disk_low", "value": "200G", "takes_effect": "WatchTick"},
            ]}),
        ),
        "/v1/instances/alpha/agents/alpha-repo1/attach" => (
            501,
            json!({"error": "/v1/agents/alpha-repo1/attach: a PTY stream over HTTP is not served yet; attach on the instance's host with conductor agent attach"}),
        ),
        _ => (404, json!({"error": format!("no route {path}")})),
    }
}

fn loaded() -> (Client, Screen, Arc<Mutex<Vec<String>>>) {
    let (url, asked) = fake_daemon(daemon);
    let client = Client::parse(&url, Duration::from_secs(5)).unwrap();
    let mut screen = Screen::new(client.origin());
    screen.load(&client);
    (client, screen, asked)
}

/// What `screen` draws on a 160 by 16 test backend, one string per row.
fn drawn(screen: &Screen) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(160, 16)).unwrap();
    terminal
        .draw(|frame| view::draw(frame, screen))
        .expect("draw");
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn reads_the_three_panes_from_the_global_api_only() {
    let (_, screen, asked) = loaded();
    assert_eq!(
        screen.instances,
        vec![
            InstanceRow {
                name: "alpha".into(),
                state: "running".into(),
                decisions: Some(2),
                kpis: "K1 !".into(),
            },
            InstanceRow {
                name: "beta".into(),
                state: "unreachable".into(),
                decisions: None,
                kpis: "K ?".into(),
            },
        ]
    );
    let sessions = screen.sessions();
    assert_eq!(sessions.len(), 2);
    assert_eq!(
        sessions[1],
        SessionRow {
            name: "alpha-repo1".into(),
            role: "controller".into(),
            state: "idle".into(),
            context: "38k".into(),
            work: "story:x".into(),
        }
    );
    assert_eq!(sessions[0].context, "-");
    assert_eq!(
        screen.settings,
        vec![
            ("cadence.cycle".to_owned(), "2h".to_owned()),
            ("thresholds.disk_low".to_owned(), "200G".to_owned()),
        ]
    );
    assert!(screen.status.is_empty(), "{}", screen.status);
    let mut asked = asked.lock().unwrap().clone();
    asked.sort();
    assert_eq!(
        asked,
        [
            "/v1/decisions",
            "/v1/instances",
            "/v1/instances/alpha/settings",
            "/v1/kpis",
            "/v1/sessions",
        ]
    );
}

#[test]
fn draws_instances_sessions_settings_and_the_keys() {
    let (_, screen, _) = loaded();
    let rows = drawn(&screen);
    let all = rows.join("\n");
    for wanted in [
        "instances on http://127.0.0.1:",
        "sessions of alpha",
        "settings of alpha (read-only)",
        "> alpha      running      2 dec  K1 !",
        "  beta       unreachable  ? dec  K ?",
        "controller   idle          38k  story:x  alpha-repo1",
        "cadence.cycle",
        "200G",
        "Enter attach  w watch  l log  ctrl-] d back",
    ] {
        assert!(all.contains(wanted), "{wanted:?} is not drawn:\n{all}");
    }
}

#[test]
fn keys_move_the_selection_and_ask_for_attach_watch_and_log() {
    let (client, mut screen, _) = loaded();
    let none = KeyModifiers::NONE;
    assert_eq!(screen.key(KeyCode::Down, none), Action::Selected);
    assert_eq!(screen.instance(), Some("beta"));
    assert_eq!(screen.key(KeyCode::Enter, none), Action::Nothing);
    assert_eq!(screen.status, "attach: no session is selected");
    assert_eq!(screen.key(KeyCode::Char('k'), none), Action::Selected);
    screen.load_settings(&client);
    assert_eq!(screen.key(KeyCode::Tab, none), Action::Nothing);
    assert_eq!(screen.focus, Pane::Sessions);
    assert_eq!(screen.key(KeyCode::Char('j'), none), Action::Nothing);
    assert_eq!(screen.selected_session().unwrap().name, "alpha-repo1");
    assert_eq!(
        screen.key(KeyCode::Enter, none),
        Action::Enter(Mode::Attach)
    );
    assert_eq!(
        screen.key(KeyCode::Char('w'), none),
        Action::Enter(Mode::Watch)
    );
    assert_eq!(
        screen.key(KeyCode::Char('l'), none),
        Action::Enter(Mode::Log)
    );
    assert_eq!(screen.key(KeyCode::Char('r'), none), Action::Refresh);
    assert_eq!(
        screen.key(KeyCode::Char('c'), KeyModifiers::CONTROL),
        Action::Quit
    );
    assert_eq!(screen.key(KeyCode::Char('q'), none), Action::Quit);
}

#[test]
fn a_local_daemon_hands_the_terminal_to_the_session_here() {
    let client = Client::parse("http://127.0.0.1:7300", Duration::from_secs(1)).unwrap();
    assert_eq!(
        handoff(&client, "alpha", "alpha-repo1", Mode::Watch),
        Handoff::Local {
            instance: "alpha".into(),
            name: "alpha-repo1".into(),
            mode: Mode::Watch,
        }
    );
}

#[test]
fn a_remote_daemon_is_asked_the_attach_route_and_its_refusal_shown() {
    let (client, _, asked) = loaded();
    let line = remote_attach(&client, "alpha", "alpha-repo1", Mode::Attach);
    assert!(
        line.starts_with("attach alpha-repo1: the daemon at http://127.0.0.1:"),
        "{line}"
    );
    assert!(line.contains("answered 501: "), "{line}");
    assert!(line.contains("conductor agent attach"), "{line}");
    assert!(
        asked
            .lock()
            .unwrap()
            .contains(&"/v1/instances/alpha/agents/alpha-repo1/attach".to_owned())
    );
}

#[test]
fn a_failing_route_is_named_on_the_status_line() {
    let (url, _) = fake_daemon(|path| match path {
        "/v1/instances" => (200, json!([{"name": "alpha", "state": "Running"}])),
        _ => (502, json!({"error": "instance alpha does not answer"})),
    });
    let client = Client::parse(&url, Duration::from_secs(5)).unwrap();
    let mut screen = Screen::new(client.origin());
    screen.load(&client);
    assert_eq!(screen.instances.len(), 1);
    assert!(
        screen
            .status
            .contains("GET /v1/sessions: 502: instance alpha does not answer"),
        "{}",
        screen.status
    );
}

#[test]
fn the_daemon_is_the_flag_else_the_variable_else_the_config_port() {
    assert_eq!(
        daemon_url(Some("https://d.example.org"), Some("http://x:1"), Some(1)),
        "https://d.example.org"
    );
    assert_eq!(daemon_url(None, Some("http://x:1"), Some(1)), "http://x:1");
    assert_eq!(daemon_url(None, None, Some(7400)), "http://127.0.0.1:7400");
    assert_eq!(daemon_url(None, Some(" "), None), "http://127.0.0.1:7300");
}
