//! `POST /api/answers`, the Decisions page's one write (`story:decisions-batch-answer`).
//!
//! The page sends the operator's answers to open escalated requests as one batch,
//! `{"answers": [{"request_id", "choice": [<letter>…], "words"?}…]}`, and each is recorded as his
//! decision through the path conductor's relay takes too ([`crate::decide::answer_as_operator`]):
//! `recorded_by` `dashboard`, his words or [`crate::decide::PAGE_WORDS`]. The answer is row by
//! row, `{"results": [{"request_id", "outcome": "recorded", "decision_id"} | {"request_id",
//! "outcome": "refused", "error", "reason"} | {"request_id", "outcome": "failed", "reason"}…]}`,
//! in the batch's order; a refused row records nothing.
//!
//! A batch is taken only when the request carries all of:
//! - the server's own `Host` (as every request must) and its own `Origin`, `http://127.0.0.1:<port>`
//!   or `http://localhost:<port>`, or no `Origin` with `Sec-Fetch-Site: same-origin`; else `403`;
//! - `X-Conductor-Token` equal to the token the page was served with ([`Token`]), compared in
//!   constant time; else `403`, naming neither the token given nor the right one;
//! - `Content-Type: application/json`; else `415`, so a cross-site form cannot send it without a
//!   preflight the server never answers;
//! - a `Content-Length` of at most [`MAX_BODY`] and no `Transfer-Encoding`; else `411`, or `413`
//!   before the body is read;
//! - at most [`MAX_ROWS`] rows; else `413`. A body that is no such batch is `400`.
//!
//! Every answer carries `Cache-Control: no-store`, `X-Content-Type-Options: nosniff` and the
//! page's `Content-Security-Policy`. The batches of one server are recorded one at a time, and
//! after one the answers kept in memory are dropped, so the page's next read of
//! `/api/decisions` shows what was recorded.
//!
//! The boundary is a speed bump against a session of conductor or a controller, not a wall: the
//! guard denies a Bash call that names this path or posts to the dashboard's port
//! ([`crate::guard`]), but a local process that reads the page, and so its token, can post as the
//! operator, and is recorded as him (`website/docs/security/overview.md`).

use std::fs::File;
use std::io::{self, Read as _};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::decide::{self, OperatorAnswer, OperatorAnswered};

/// The path the page posts its batch to.
pub const ANSWERS_PATH: &str = "/api/answers";

/// The session an answer from the page is recorded by.
pub const RECORDED_BY: &str = "dashboard";

/// The longest body taken, in bytes.
pub const MAX_BODY: usize = 64 * 1024;

/// The most rows one batch holds.
pub const MAX_ROWS: usize = 50;

/// The header the page sends its token in.
const TOKEN_HEADER: &str = "x-conductor-token";

/// The token one server process serves its page with and takes a batch with: 32 random bytes,
/// written as 64 hex digits, minted when the server starts. It is never logged, never echoed and
/// never set as a cookie; the page reads it from its `<meta name="conductor-token">`.
pub(super) struct Token(String);

impl Token {
    /// A new token from the system's random source.
    pub(super) fn mint() -> io::Result<Self> {
        let mut bytes = [0_u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut bytes)?;
        Ok(Self(
            bytes.iter().map(|byte| format!("{byte:02x}")).collect(),
        ))
    }

    /// Whether `given` is the token, compared in constant time over the token's length.
    fn admits(&self, given: &str) -> bool {
        let (token, given) = (self.0.as_bytes(), given.as_bytes());
        token.len() == given.len()
            && token
                .iter()
                .zip(given)
                .fold(0_u8, |differs, (a, b)| differs | (a ^ b))
                == 0
    }

    /// `index.html` with the token in a `<meta name="conductor-token">` before `</head>`.
    pub(super) fn page_with(&self, page: &[u8]) -> Vec<u8> {
        let meta = format!(r#"<meta name="conductor-token" content="{}">"#, self.0);
        let text = String::from_utf8_lossy(page);
        match text.find("</head>") {
            Some(at) => format!("{}{meta}{}", &text[..at], &text[at..]).into_bytes(),
            None => format!("{meta}{text}").into_bytes(),
        }
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

/// Whether the request's headers, `header` reading one by name, are those of the page served on
/// `port`, posting JSON with `token`; the length of its body when they are.
pub(super) fn admitted<'a>(
    header: impl Fn(&str) -> Option<&'a str>,
    port: u16,
    token: &Token,
) -> Result<usize, Refused> {
    let own = [
        format!("http://127.0.0.1:{port}"),
        format!("http://localhost:{port}"),
    ];
    let same_origin = match header("origin") {
        Some(origin) => own.iter().any(|own| own.eq_ignore_ascii_case(origin)),
        None => {
            header("sec-fetch-site").is_some_and(|site| site.eq_ignore_ascii_case("same-origin"))
        }
    };
    if !same_origin {
        return Err(refuse(
            403,
            "Forbidden",
            "answers are taken only from this server's own page\n",
        ));
    }
    if !header(TOKEN_HEADER).is_some_and(|given| token.admits(given)) {
        return Err(refuse(
            403,
            "Forbidden",
            "the request carries no token, or not the one this server's page was served with; \
             reload the page\n",
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
    if length > MAX_BODY {
        return Err(refuse(
            413,
            "Content Too Large",
            format!("a batch is at most {MAX_BODY} bytes\n"),
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
