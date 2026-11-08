//! The dashboard as a browser reaches it since `story:instance-api`: the instance API served in
//! this process on a Unix socket ([`dashboard::serve_unix_tuned`]), behind conductord's loopback
//! listener ([`api::serve`]) on the TCP listener a case bound, which serves the app at `/` with the
//! daemon's token and proxies `/v1/instances/example-org/<route>` to the instance with the
//! instance's token. The loopback operator is `operator`, holding every grant: the deployment
//! names no operators.

use std::net::TcpListener;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use conductor_cli::daemon::api::{self, Api, Route};
use conductor_cli::dashboard::{self, Sources, Tuning};
use serde_json::json;

/// The instance the daemon proxies to in these cases.
pub const INSTANCE: &str = "example-org";

/// A socket path of this test binary, short enough for a socket address.
fn socket() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("s");
    std::fs::create_dir_all(&dir).expect("create the sockets' directory");
    let path = dir.join(format!(
        "{}-{}.sock",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&path);
    path
}

/// Serves `sources` and the store under `state` as [`serve_tuned`] does, with the default tuning.
#[allow(dead_code)]
pub fn serve(listener: TcpListener, sources: Sources, state: Option<PathBuf>) {
    serve_tuned(listener, sources, state, Tuning::default());
}

/// Serves the instance API over `sources` and the store under `state` with `tuning` on a socket of
/// its own, and the daemon's loopback listener on `listener` in front of it; returns at once.
pub fn serve_tuned(
    listener: TcpListener,
    sources: Sources,
    state: Option<PathBuf>,
    tuning: Tuning,
) {
    // The instance API reads the active instance; every case runs on its own.
    super::active::isolate();
    let path = socket();
    let unix = UnixListener::bind(&path).expect("bind the instance's socket");
    let token = dashboard::mint().expect("mint the instance's token");
    let given = token.clone();
    std::thread::spawn(move || dashboard::serve_unix_tuned(unix, sources, state, tuning, given));
    let api = Arc::new(Api::new(
        dashboard::mint().expect("mint the daemon's token"),
        "operator".to_owned(),
        None,
    ));
    api.publish(
        vec![json!({"name": INSTANCE, "state": "Running"})],
        [(
            INSTANCE.to_owned(),
            Route {
                socket: path,
                token,
            },
        )]
        .into_iter()
        .collect(),
        Vec::new(),
    );
    std::thread::spawn(move || api::serve(&listener, &api));
}
