//! `conductor tui` (`story:tui-readonly`) as a process: it takes `--daemon <url>` and refuses
//! one that is no `http://` or `https://` origin by name; with no terminal on its standard input
//! and output it refuses to start, naming that, rather than drawing into a pipe. What it draws and
//! what it reads from the global API are the unit tests of `src/tui`.

mod common;

fn scratch() -> std::path::PathBuf {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "conductor-tui-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    std::fs::create_dir_all(&dir).expect("create the scratch home");
    dir
}

#[test]
fn tui_refuses_a_daemon_that_is_no_http_origin() {
    let home = scratch();
    let output = common::conductor(&home)
        .args(["tui", "--daemon", "ftp://daemon.example.org"])
        .output()
        .expect("run conductor tui");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(
        stderr.contains("--daemon ftp://daemon.example.org"),
        "stderr: {stderr}"
    );
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn tui_refuses_to_start_without_a_terminal() {
    let home = scratch();
    let output = common::conductor(&home)
        .args(["tui", "--daemon", "http://127.0.0.1:9"])
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run conductor tui");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(stderr.contains("needs a terminal"), "stderr: {stderr}");
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn bare_conductor_without_a_terminal_opens_no_tui() {
    let home = scratch();
    let output = common::conductor(&home)
        .stdin(std::process::Stdio::null())
        .output()
        .expect("run conductor");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "stderr: {stderr}");
    assert!(!stderr.contains("needs a terminal"), "stderr: {stderr}");
    let _ = std::fs::remove_dir_all(home);
}
