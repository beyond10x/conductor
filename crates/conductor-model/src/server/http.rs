// generated from conductor v1
// contract digest 8d3aee133e936ba053cf7cd2ad3b3aadd389f7fad772cddd3c670bb87f21cfef
// do not edit: regenerate with `ess synthesize --layout crate`

//! HTTP/1.1, as much of it as a synthesised surface needs and no more.
//!
//! Not a framework and not a deployment. One connection at a time, in accept order: read the
//! request line, read the headers, read exactly `Content-Length` bytes, answer, close. There is no
//! keep-alive, no pipelining, no compression, no TLS and no thread pool, and every one of those is
//! a decision a deployment gets to make rather than one a generator makes for it. What this file
//! *does* guarantee is the part the specification determines: the status codes and the bodies.
//!
//! Written here rather than taken from a crate for the reason the JSON reader beside it is: the
//! emitted tree builds with zero third-party crates, inside a gate that reaches no network.

use std::io::{BufRead, Read, Write};

/// The largest body this surface reads, in bytes.
///
/// A caller can claim any length, and a server that allocated whatever it was told to is a server
/// anyone can stop by saying a large number. A megabyte is far past any command input this model
/// can describe.
pub const MAX_BODY: usize = 1_048_576;

/// The most headers this surface keeps from one request.
///
/// Every header is kept for the caller ([`Request::headers`]), so a request that sent headers
/// without end would be memory without end. A hundred is far past what a client and a proxy add
/// together. The Go server keeps the same count and answers the same `431` beyond it.
pub const MAX_HEADERS: usize = 100;

/// The most bytes the request line and headers may take together: what Go's `net/http` reads by
/// default (`DefaultMaxHeaderBytes`, one MiB, and the 4096 bytes it allows beyond it).
///
/// The same bound, and above it the same answer ([`head_too_large`]), so the two servers synthesised
/// from one specification answer an oversized request alike. Without one, a caller could hold a
/// request line of any length in memory.
pub const MAX_HEAD: usize = 1_048_576 + 4096;

/// How long a connection may send nothing before this surface drops it.
///
/// The surface answers one connection at a time, so a caller that connects and goes quiet would
/// otherwise hold every other caller. Go's server answers each connection on its own goroutine
/// and sets no such bound; one connection at a time cannot, and a second without a byte is far
/// past the gap between two segments of a request in flight. It bounds each wait, not the whole
/// request: a caller that sends a byte every half second is still read.
pub const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

/// How long writing an answer may stall before this surface gives the connection up, for the same
/// reason: a caller that stops reading must not hold the others.
pub const WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// The media type every answer derived from the model carries.
pub const JSON: &str = "application/json";

/// The media type the prose answer carries.
///
/// The bytes served are the committed Markdown, unrendered: rendering it to HTML here would be a
/// second rendering of the documentation, and the two would differ the first time either moved.
pub const MARKDOWN: &str = "text/markdown; charset=utf-8";

/// One request, as much of it as this surface reads.
///
/// `Default` is the empty request, so a caller that builds one names only what it sets:
/// `Request { method: "GET".to_owned(), path: "/openapi.json".to_owned(), ..Default::default() }`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    /// The method, verbatim.
    pub method: String,
    /// The target, with any query string removed: what routing matches.
    pub path: String,
    /// The target's query string, after the `?` and still percent-encoded; empty when there is
    /// none.
    ///
    /// Only a view that declares parameters reads it, each by its wire name; a key no view
    /// declares names nothing on this surface, and is ignored rather than refused, because a
    /// caller that appends one has not made a different request.
    pub query: String,
    /// Every header, in the order it arrived: the name lower-cased, the value trimmed.
    ///
    /// Kept for the caller rather than read here: the model declares no header, so routing never
    /// looks at one, and a shell that authenticates the caller before a surface's `dispatch`
    /// reads `authorization` from this list. A name that arrives twice is kept twice.
    pub headers: Vec<(String, String)>,
    /// The body: exactly the `Content-Length` bytes the caller announced.
    pub body: Vec<u8>,
}

/// One answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The status code.
    pub status: u16,
    /// The media type of the body.
    pub content_type: &'static str,
    /// The body.
    pub body: String,
}

impl Response {
    /// An answer carrying a body.
    pub fn new(status: u16, content_type: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
        }
    }

    /// A refusal this surface makes rather than the specification.
    ///
    /// A malformed request, a path nothing declares, a method a path does not answer, an
    /// obligation nothing has satisfied. None of these is a declared outcome, and none is
    /// published in the contract, because each is a fact about a transport rather than about a
    /// command. The body is JSON with one member: a caller that has just failed to satisfy a
    /// contract should not have to parse a second format to read why.
    pub fn refusal(status: u16, detail: &str) -> Self {
        let mut body = String::from("{");
        crate::server::json::member(&mut body, "refused");
        crate::server::json::push_text(&mut body, detail);
        body.push('}');
        Self::new(status, JSON, body)
    }

    /// The `501` the contract declares: the realization is unfinished.
    ///
    /// Its body is [`Response::refusal`]'s with one more member, `committed`: `true` when the
    /// command's effect and events were committed and delivering what it published failed, and
    /// `false` when an unmet obligation stopped it before anything was written.
    pub fn unfinished(detail: &str, committed: bool) -> Self {
        let mut body = String::from("{");
        crate::server::json::member(&mut body, "refused");
        crate::server::json::push_text(&mut body, detail);
        crate::server::json::member(&mut body, "committed");
        body.push_str(if committed { "true" } else { "false" });
        body.push('}');
        Self::new(501, JSON, body)
    }
}

/// The answer for what a construct's shared path produced: the declared outcome at the status
/// the contract declares for its branch, or the refusal at the status this surface gives it.
///
/// The one place a [`crate::server::entry::Refused`] becomes a status, so a route and `handle` refuse
/// with the same words.
pub fn answer(result: Result<(u16, String), crate::server::entry::Refused>) -> Response {
    match result {
        Ok((status, body)) => Response::new(status, JSON, body),
        Err(refused) => Response::from(&refused),
    }
}

impl From<&crate::server::entry::Refused> for Response {
    /// The refusal as served: at [`crate::server::entry::Refused::status`], and for a `501` with the
    /// `committed` member the contract declares.
    fn from(refused: &crate::server::entry::Refused) -> Self {
        match refused.status() {
            501 => Self::unfinished(&refused.to_string(), refused.committed()),
            status => Self::refusal(status, &refused.to_string()),
        }
    }
}

/// The answer for a path this surface holds under a different method.
pub fn method_not_allowed(allowed: &str) -> Response {
    Response::refusal(
        405,
        &format!("this path answers `{allowed}`, and the contract declares no other method for it"),
    )
}

/// Reads one request, or the refusal that says why it could not be read.
///
/// # Errors
///
/// Never as an `Err` of the outer kind: everything that can go wrong with a request is an answer
/// the caller should receive, so the failure arm is the [`Response`] to send back.
pub fn read(reader: &mut std::io::BufReader<std::net::TcpStream>) -> Result<Request, Response> {
    let mut budget = MAX_HEAD;
    let mut line = String::new();
    match head_line(reader, &mut line, &mut budget) {
        Ok(Some(0)) => {
            return Err(Response::refusal(
                400,
                "the connection closed before a request line arrived",
            ))
        }
        Ok(None) => return Err(head_too_large()),
        Ok(Some(_)) => {}
        Err(error) => {
            return Err(Response::refusal(
                400,
                &format!("the request line could not be read: {error}"),
            ))
        }
    }
    let mut parts = line.trim_end().split(' ');
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default().to_owned();
    let version = parts.next().unwrap_or_default().to_owned();
    if method.is_empty() || target.is_empty() || !version.starts_with("HTTP/1.") {
        return Err(Response::refusal(
            400,
            "the request line is not `METHOD TARGET HTTP/1.1`",
        ));
    }
    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path.to_owned(), query.to_owned()),
        None => (target.clone(), String::new()),
    };

    let mut length = 0_usize;
    let mut chunked = false;
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        match head_line(reader, &mut header, &mut budget) {
            Ok(Some(0)) => {
                return Err(Response::refusal(
                    400,
                    "the connection closed inside the headers",
                ))
            }
            Ok(None) => return Err(head_too_large()),
            Ok(Some(_)) => {}
            Err(error) => {
                return Err(Response::refusal(
                    400,
                    &format!("a header could not be read: {error}"),
                ))
            }
        }
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let Some((name, value)) = header.split_once(':') else {
            return Err(Response::refusal(400, "a header line has no `:`"));
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        if name == "content-length" {
            match value.parse::<usize>() {
                Ok(parsed) => length = parsed,
                Err(_) => {
                    return Err(Response::refusal(
                        400,
                        "`Content-Length` is not a number of bytes",
                    ))
                }
            }
        } else if name == "transfer-encoding" && value.eq_ignore_ascii_case("chunked") {
            chunked = true;
        }
        if headers.len() == MAX_HEADERS {
            return Err(Response::refusal(
                431,
                &format!(
                    "the request carries more than {MAX_HEADERS} headers, which is all this \
                     surface keeps"
                ),
            ));
        }
        headers.push((name, value.to_owned()));
    }
    if chunked {
        return Err(Response::refusal(
            411,
            "this surface reads a body announced by `Content-Length`; chunked transfer is not read",
        ));
    }
    if length > MAX_BODY {
        return Err(Response::refusal(
            413,
            &format!("the body is {length} bytes and this surface reads at most {MAX_BODY}"),
        ));
    }
    let mut body = vec![0_u8; length];
    if let Err(error) = reader.read_exact(&mut body) {
        return Err(Response::refusal(
            400,
            &format!("the body was shorter than `Content-Length` announced: {error}"),
        ));
    }
    Ok(Request {
        method,
        path,
        query,
        headers,
        body,
    })
}

/// One line of the request head, read within what is left of [`MAX_HEAD`]: `Some` with the bytes
/// it took (`0` at the end of the connection), and `None` where the line runs past the bound.
fn head_line(
    reader: &mut std::io::BufReader<std::net::TcpStream>,
    line: &mut String,
    budget: &mut usize,
) -> std::io::Result<Option<usize>> {
    let allowed = u64::try_from(*budget).unwrap_or(u64::MAX);
    let read = reader.by_ref().take(allowed).read_line(line)?;
    if read == *budget && !line.ends_with('\n') {
        return Ok(None);
    }
    *budget -= read;
    Ok(Some(read))
}

/// The answer to a request head past [`MAX_HEAD`]: byte for byte what Go's `net/http` answers, the
/// one refusal on this surface that is not JSON, because the Go server writes it before any code
/// of its own runs and the two servers must answer one request alike.
pub fn head_too_large() -> Response {
    Response::new(
        431,
        "text/plain; charset=utf-8",
        "431 Request Header Fields Too Large",
    )
}

/// After answering a request it refused while reading it: stop writing, then read and drop what
/// the caller is still sending — at most 64 reads of 64 KiB, each waiting at most half a second.
///
/// Closing with unread bytes waiting makes the kernel reset the connection, and a reset can
/// destroy the answer before the caller reads it. Go's `net/http` lingers the same way. Bounded
/// by reads rather than by a clock, so this surface reads no clock.
pub fn linger(stream: &mut std::net::TcpStream) {
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
    let mut sink = vec![0_u8; 65_536];
    for _ in 0..64 {
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

/// Writes one answer, and lets the connection close behind it.
///
/// # Errors
///
/// Whatever the socket refuses.
pub fn write(stream: &mut std::net::TcpStream, answer: &Response) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        answer.status,
        reason(answer.status),
        answer.content_type,
        answer.body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(answer.body.as_bytes())?;
    stream.flush()
}

/// The reason phrase for every status this surface can answer with.
///
/// Every one of them is either a status the contract declares for a branch, or one of the four this
/// surface answers about the request itself. A status not in this list is one nothing emits.
pub fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Content Too Large",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        _ => "Unknown",
    }
}

/// How one declared parameter's query value is written as JSON before its decoder reads it.
///
/// A query value is text; the parameter's declared type says which JSON scalar the wire writes
/// it as, and the generated decoder then reads it exactly as it reads a command's input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scalar {
    /// A JSON string: text, a decimal, an instant, a duration, a UUID, base64 bytes, an enum's
    /// wire spelling.
    Text,
    /// A JSON number where the value spells a whole number, and a string otherwise, so the
    /// decoder names what arrived rather than a number that never did.
    Integer,
    /// A JSON boolean where the value is `true` or `false`, and a string otherwise.
    Boolean,
}

/// The parameters a query string carries, as the object their decoder reads: one member per
/// declared `(wire name, scalar)` the query names, keyed by that wire name.
///
/// Keys and values are form-decoded (`%XX` and `+`). A key nothing declares is ignored, and a
/// declared key the query omits is absent from the object, for the decoder to refuse or to read
/// as `None`.
///
/// # Errors
///
/// A refusal naming the parameter under `at` when a declared key arrives more than once, or its
/// value is not percent-encoded UTF-8.
pub fn query_object(
    query: &str,
    declared: &[(&str, Scalar)],
    at: &str,
) -> Result<crate::server::json::Value, String> {
    let mut members = Vec::new();
    for (wire, scalar) in declared {
        let found: Vec<&str> = query
            .split('&')
            .filter(|pair| !pair.is_empty())
            .map(|pair| pair.split_once('=').unwrap_or((pair, "")))
            .filter(|(key, _)| form_decoded(key).as_deref() == Some(*wire))
            .map(|(_, value)| value)
            .collect();
        let place = crate::server::json::nested(at, wire);
        let text = match found.as_slice() {
            [] => continue,
            [value] => form_decoded(value).ok_or_else(|| {
                format!("{place}: expected percent-encoded UTF-8 text, found `{value}`")
            })?,
            several => {
                return Err(format!(
                    "{place}: expected one value, found {}",
                    several.len()
                ))
            }
        };
        let value = match scalar {
            Scalar::Integer if whole(&text) => crate::server::json::Value::Number(text),
            Scalar::Boolean if text == "true" => crate::server::json::Value::Bool(true),
            Scalar::Boolean if text == "false" => crate::server::json::Value::Bool(false),
            _ => crate::server::json::Value::Text(text),
        };
        members.push(((*wire).to_owned(), value));
    }
    Ok(crate::server::json::Value::Object(members))
}

/// One form-encoded key or value, decoded: `+` is a space and `%XX` a byte. `None` where an
/// escape is not two hexadecimal digits or the bytes are not UTF-8.
fn form_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => out.push(b' '),
            b'%' => {
                let high = char::from(*bytes.get(index + 1)?).to_digit(16)?;
                let low = char::from(*bytes.get(index + 2)?).to_digit(16)?;
                out.push(u8::try_from(high * 16 + low).ok()?);
                index += 2;
            }
            byte => out.push(byte),
        }
        index += 1;
    }
    String::from_utf8(out).ok()
}

/// `true` where `text` spells a whole number: an optional `-`, then one or more digits.
fn whole(text: &str) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}
