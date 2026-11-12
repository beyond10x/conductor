//! `POST /v1/answers`, the Decisions page's one write (`story:decisions-batch-answer`), on the
//! instance API (`story:instance-api`).
//!
//! The page sends the operator's answers to open escalated requests as one batch,
//! `{"answers": [{"request_id", "choice": [<letter>…], "words"?}…]}`, and each is recorded as his
//! decision through the path conductor's relay takes too ([`crate::decide::answer_as_operator`]):
//! `recorded_by` `dashboard`, his words or [`crate::decide::PAGE_WORDS`]. The answer is row by
//! row, `{"results": [{"request_id", "outcome": "recorded", "decision_id"} | {"request_id",
//! "outcome": "refused", "error", "reason"} | {"request_id", "outcome": "failed", "reason"}…]}`,
//! in the batch's order; a refused row records nothing.
//!
//! The browser's page, its host and its origin are the daemon's to admit
//! ([`crate::daemon::api`]); the instance takes a batch only when the request carries all of:
//! - `X-Conductor-Token` equal to the token the daemon minted for the instance ([`Token`]),
//!   compared in constant time; else `403`, naming neither the token given nor the right one;
//! - `Content-Type: application/json`; else `415`;
//! - a `Content-Length` of at most [`MAX_BODY`] and no `Transfer-Encoding`; else `411`, or `413`
//!   before the body is read;
//! - at most [`MAX_ROWS`] rows; else `413`. A body that is no such batch is `400`.
//!
//! The operator the daemon admitted is named in [`OPERATOR_HEADER`], and the batch is recorded as
//! that operator's ([`crate::actor::as_operator`]).
//!
//! Every answer carries `Cache-Control: no-store`, `X-Content-Type-Options: nosniff` and the
//! page's `Content-Security-Policy`. The batches of one server are recorded one at a time, and
//! after one the answers kept in memory are dropped, so the page's next read of
//! `/v1/decisions` shows what was recorded.
//!
//! The boundary is a speed bump against a session of conductor or a controller, not a wall: the
//! guard denies a Bash call that names this path or posts to the daemon's port
//! ([`crate::guard`]), but a local process that reads the daemon's page, and so its token, can post
//! as the operator, and is recorded as him (`website/docs/security/overview.md`).

use std::fs::File;
use std::io::{self, Read as _};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::decide::{self, OperatorAnswer, OperatorAnswered};

/// The path the page posts its batch to.
pub const ANSWERS_PATH: &str = "/v1/answers";

/// The session an answer from the page is recorded by.
pub const RECORDED_BY: &str = "dashboard";

/// The longest body taken, in bytes.
pub const MAX_BODY: usize = 64 * 1024;

/// The most rows one batch holds.
pub const MAX_ROWS: usize = 50;

/// The header a write carries its token in.
pub const TOKEN_HEADER: &str = "x-conductor-token";

/// The header the daemon names the operator of a write in (`story:instance-api`).
pub const OPERATOR_HEADER: &str = "x-conductor-operator";

/// 32 random bytes from the system's random source, written as 64 hex digits: a token the daemon
/// mints for itself and for each instance it starts.
///
/// # Errors
///
/// The random source does not read.
pub fn mint() -> io::Result<String> {
    let mut bytes = [0_u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Whether `given` is `token`, compared in constant time over the token's length.
#[must_use]
pub fn same(token: &str, given: &str) -> bool {
    let (token, given) = (token.as_bytes(), given.as_bytes());
    token.len() == given.len()
        && token
            .iter()
            .zip(given)
            .fold(0_u8, |differs, (a, b)| differs | (a ^ b))
            == 0
}

/// `page` (the app's `index.html`) with `token` in a `<meta name="conductor-token">` before
/// `</head>`.
#[must_use]
pub fn page_with(page: &[u8], token: &str) -> Vec<u8> {
    let meta = format!(r#"<meta name="conductor-token" content="{token}">"#);
    let text = String::from_utf8_lossy(page);
    match text.find("</head>") {
        Some(at) => format!("{}{meta}{}", &text[..at], &text[at..]).into_bytes(),
        None => format!("{meta}{text}").into_bytes(),
    }
}

/// The token the instance takes a write with: the one the daemon minted for it at its start. It is
/// never logged and never echoed.
pub(super) struct Token(String);

impl Token {
    /// The token `token`.
    pub(super) const fn given(token: String) -> Self {
        Self(token)
    }

    /// Whether `given` is the token.
    pub(super) fn admits(&self, given: &str) -> bool {
        same(&self.0, given)
    }
}

/// A batch as the page sends it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Batch {
    answers: Vec<Row>,
}

/// One answer of a batch.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    request_id: String,
    choice: Vec<String>,
    #[serde(default)]
    words: Option<String>,
}

/// A refusal of the whole request: its status, its reason phrase and why.
pub(super) struct Refused {
    pub(super) status: u16,
    pub(super) reason: &'static str,
    pub(super) why: String,
}

fn refuse(status: u16, reason: &'static str, why: impl Into<String>) -> Refused {
    Refused {
        status,
        reason,
        why: why.into(),
    }
}

/// Whether the request's headers, `header` reading one by name, come from the page served by the
/// loopback listener bound to `bound`: its own `Origin` (`http://` on `127.0.0.1`, `localhost` or
/// the address it is bound to, an IPv6 one in brackets, with its port), or no `Origin` with
/// `Sec-Fetch-Site: same-origin`. The daemon's loopback listener asks it of every write it
/// forwards (`story:instance-api`).
pub fn same_origin<'a>(
    header: impl Fn(&str) -> Option<&'a str>,
    bound: std::net::SocketAddr,
) -> bool {
    let port = bound.port();
    let own = [
        format!("http://127.0.0.1:{port}"),
        format!("http://localhost:{port}"),
        format!("http://{bound}"),
    ];
    match header("origin") {
        Some(origin) => own.iter().any(|own| own.eq_ignore_ascii_case(origin)),
        None => {
            header("sec-fetch-site").is_some_and(|site| site.eq_ignore_ascii_case("same-origin"))
        }
    }
}

/// Whether the request's headers, `header` reading one by name, post JSON with `token`; the
/// length of its body when they do.
pub(super) fn admitted<'a>(
    header: impl Fn(&str) -> Option<&'a str>,
    token: &Token,
) -> Result<usize, Refused> {
    admitted_within(header, token, MAX_BODY)
}

/// [`admitted`] for a body of at most `most` bytes, refused `413` from its `Content-Length`
/// before it is read.
pub(super) fn admitted_within<'a>(
    header: impl Fn(&str) -> Option<&'a str>,
    token: &Token,
    most: usize,
) -> Result<usize, Refused> {
    if !header(TOKEN_HEADER).is_some_and(|given| token.admits(given)) {
        return Err(refuse(
            403,
            "Forbidden",
            "the request carries no token, or not the one the daemon minted for this instance\n",
        ));
    }
    let media = header("content-type")
        .and_then(|value| value.split(';').next())
        .map(str::trim);
    if !media.is_some_and(|media| media.eq_ignore_ascii_case("application/json")) {
        return Err(refuse(
            415,
            "Unsupported Media Type",
            "answers are sent as application/json\n",
        ));
    }
    if header("transfer-encoding").is_some() {
        return Err(refuse(
            411,
            "Length Required",
            "send the answers with a Content-Length, not chunked\n",
        ));
    }
    let Some(length) = header("content-length").and_then(|value| value.parse::<usize>().ok())
    else {
        return Err(refuse(
            411,
            "Length Required",
            "send the answers with a Content-Length\n",
        ));
    };
    if length > most {
        return Err(refuse(
            413,
            "Content Too Large",
            format!("a body is at most {most} bytes\n"),
        ));
    }
    Ok(length)
}

/// The rows of the batch `body`, or why it is none.
fn batch(body: &[u8]) -> Result<Vec<Row>, Refused> {
    let batch: Batch = serde_json::from_slice(body).map_err(|error| {
        refuse(
            400,
            "Bad Request",
            format!(
                "the body is no batch {{\"answers\": [{{\"request_id\", \"choice\", \"words\"}}]}}: \
                 {error}\n"
            ),
        )
    })?;
    if batch.answers.len() > MAX_ROWS {
        return Err(refuse(
            413,
            "Content Too Large",
            format!("a batch holds at most {MAX_ROWS} answers\n"),
        ));
    }
    Ok(batch.answers)
}

/// Records each answer of the batch `body` in the store under `state`, in order; the answer row
/// by row, as the module describes it.
pub(super) fn record(state: Option<&std::path::Path>, body: &[u8]) -> Result<Value, Refused> {
    let rows = batch(body)?;
    let Some(state) = state else {
        return Err(refuse(
            500,
            "Internal Server Error",
            "this server reads no store, so it records no answer\n",
        ));
    };
    let results: Vec<Value> = rows
        .into_iter()
        .map(|row| {
            let answer = OperatorAnswer {
                request_id: row.request_id.clone(),
                choice: row.choice,
                operator_words: row.words,
                recorded_by: RECORDED_BY.to_owned(),
                ..OperatorAnswer::default()
            };
            match decide::answer_as_operator(Some(state), &answer) {
                Ok(OperatorAnswered::Recorded(decision_id)) => json!({
                    "request_id": row.request_id,
                    "outcome": "recorded",
                    "decision_id": decision_id,
                }),
                Ok(OperatorAnswered::Refused { error, reason }) => json!({
                    "request_id": row.request_id,
                    "outcome": "refused",
                    "error": error,
                    "reason": reason,
                }),
                Err(error) => json!({
                    "request_id": row.request_id,
                    "outcome": "failed",
                    "reason": format!("{error:#}"),
                }),
            }
        })
        .collect();
    Ok(json!({"results": results}))
}
