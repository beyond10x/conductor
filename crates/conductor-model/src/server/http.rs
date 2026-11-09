// generated from conductor v1
// contract digest 251aac8e3d7844fa8bf5d80ba2d224b99dac46d204effce0fa0168d87512e308
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
    /// The body; empty, with an empty `content_type`, for an answer that has none.
    pub body: String,
    /// Every header the binding declares for this answer, in the order it is written.
    pub headers: Vec<(String, String)>,
}

impl Response {
    /// An answer carrying a body.
    pub fn new(status: u16, content_type: &'static str, body: impl Into<String>) -> Self {
        Self {
            status,
            content_type,
            body: body.into(),
            headers: Vec::new(),
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
    // RFC 9110 §8.6 and §15: a `1xx`, `204` or `304` has no content and no `Content-Length`.
    let bodiless = answer.status < 200 || answer.status == 204 || answer.status == 304;
    let mut head = format!("HTTP/1.1 {} {}\r\n", answer.status, reason(answer.status));
    if !bodiless && !answer.content_type.is_empty() {
        head.push_str(&format!("Content-Type: {}\r\n", answer.content_type));
    }
    if !bodiless {
        head.push_str(&format!("Content-Length: {}\r\n", answer.body.len()));
    }
    for (name, value) in &answer.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    if !bodiless {
        stream.write_all(answer.body.as_bytes())?;
    }
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
        201 => "Created",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        410 => "Gone",
        412 => "Precondition Failed",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        421 => "Misdirected Request",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        511 => "Network Authentication Required",
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

// ---- an `ess-http/1` binding, served -------------------------------------------------------------

/// How one flat value — a path segment, a `form` or `query` value — is read as the JSON value its
/// field's decoder reads. The same table `ess_gen::http::Flat` states, so the Rust and Go servers
/// refuse one request alike.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flat {
    /// `String`, or a newtype of it: any text, the empty text included.
    String,
    /// Any other scalar a decoder reads from JSON text. Empty text is malformed.
    Text,
    /// `Integer`: a whole number's spelling is a JSON number, anything else stays text for the
    /// decoder to refuse by name. Empty text is malformed.
    Integer,
    /// `Boolean`: `true` or `false`, anything else stays text. Empty text is malformed.
    Boolean,
}

/// How many values one `form` or `query` key carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// Exactly one; given twice, the request is malformed (RFC 6749 §3.1).
    One,
    /// At most one; absent, the field is `None`.
    Optional,
    /// Any number: the key repeats once per element.
    List,
}

/// The media type of a `form` request body.
pub const FORM: &str = "application/x-www-form-urlencoded";

impl Response {
    /// An answer without a body: no `Content-Type`, and `Content-Length: 0` except on a `1xx`,
    /// `204` or `304`, which [`write`] gives none.
    pub fn empty(status: u16) -> Self {
        Self::new(status, "", "")
    }
}

/// A path's segments after its leading `/`; none for the root, and `None` for a path without one.
fn split(path: &str) -> Option<Vec<&str>> {
    let rest = path.strip_prefix('/')?;
    Some(if rest.is_empty() {
        Vec::new()
    } else {
        rest.split('/').collect()
    })
}

/// The raw text of each variable segment of `path`, in order, where `path` matches `template`:
/// as many segments, each literal equal, and each variable holding a segment that is not empty.
/// `None` where it does not match.
pub fn matched(template: &str, path: &str) -> Option<Vec<String>> {
    let template = split(template)?;
    let given = split(path)?;
    if template.len() != given.len() {
        return None;
    }
    let mut values = Vec::new();
    for (expected, text) in template.iter().zip(given) {
        if expected.starts_with('{') && expected.ends_with('}') {
            if text.is_empty() {
                return None;
            }
            values.push(text.to_owned());
        } else if *expected != literal_spelling(text) {
            return None;
        }
    }
    Some(values)
}

/// `text` with each `%XX` escape of an unreserved character decoded (RFC 3986 §6.2.2.2), the
/// spelling a segment is compared with a literal in: `%7Earchived` is `~archived`. Every other
/// escape stays as written, and a variable's text is decoded where it is read.
fn literal_spelling(text: &str) -> String {
    let bytes = text.as_bytes();
    let digit = |at: usize| bytes.get(at).and_then(|byte| char::from(*byte).to_digit(16));
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let escaped = match (bytes[index], digit(index + 1), digit(index + 2)) {
            (b'%', Some(high), Some(low)) => u8::try_from(high * 16 + low)
                .ok()
                .filter(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(byte)),
            _ => None,
        };
        if let Some(byte) = escaped {
            out.push(byte);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_owned())
}

/// `text` with every `%XX` escape decoded; `None` where an escape is not two hexadecimal digits or
/// the bytes are not UTF-8. A `+` stays a `+`: in a path it is itself.
fn percent_decoded(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = char::from(*bytes.get(index + 1)?).to_digit(16)?;
            let low = char::from(*bytes.get(index + 2)?).to_digit(16)?;
            out.push(u8::try_from(high * 16 + low).ok()?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// One flat text as the JSON value its field's decoder reads.
///
/// # Errors
///
/// A refusal naming `at` when the text is empty and the field is not a `String`.
pub fn flat_value(text: String, flat: Flat, at: &str) -> Result<crate::server::json::Value, String> {
    use crate::server::json::Value;
    Ok(match flat {
        Flat::String => Value::Text(text),
        _ if text.is_empty() => {
            return Err(format!("{at}: expected a value, found the empty text"));
        }
        Flat::Integer if whole(&text) => Value::Number(text),
        Flat::Boolean if text == "true" => Value::Bool(true),
        Flat::Boolean if text == "false" => Value::Bool(false),
        _ => Value::Text(text),
    })
}

/// One path variable's raw segment, percent-decoded, as the JSON value its field's decoder reads.
///
/// # Errors
///
/// A refusal naming `at` when the segment is not percent-encoded UTF-8, or is empty text for a
/// field that is not a `String`.
pub fn path_value(raw: &str, flat: Flat, at: &str) -> Result<crate::server::json::Value, String> {
    let text = percent_decoded(raw)
        .ok_or_else(|| format!("{at}: expected percent-encoded UTF-8 text, found `{raw}`"))?;
    flat_value(text, flat, at)
}

/// A `form` body or a query string as the object the input's decoder reads: one member per
/// declared `(wire name, flat, shape)` the text names, keyed by the wire name, and a `List` as an
/// array, empty when its key is absent.
///
/// Keys and values are form-decoded (`+` and `%XX`, UTF-8). A key the path carries, no field
/// declares, or that does not decode (every declared name is UTF-8), is unknown: ignored where
/// `ignore`, and malformed otherwise.
///
/// # Errors
///
/// A refusal naming the parameter under `at` for an unknown key under `refuse` (one that does not
/// decode included), a scalar key given twice, a known key's value that is not percent-encoded
/// UTF-8, or an empty value for a field that is not a `String`.
pub fn flat_object(
    text: &str,
    fields: &[(&str, Flat, Shape)],
    carried: &[&str],
    ignore: bool,
    at: &str,
) -> Result<crate::server::json::Value, String> {
    let mut found: Vec<Vec<String>> = vec![Vec::new(); fields.len()];
    for pair in text.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        // Every declared name is UTF-8, so a key that does not decode names no field: unknown.
        let Some(key) = form_decoded(key) else {
            if ignore {
                continue;
            }
            return Err(format!("{at}: a key is not percent-encoded UTF-8: `{key}`"));
        };
        let place = crate::server::json::nested(at, &key);
        let index = fields.iter().position(|(wire, _, _)| *wire == key);
        let Some(index) = index.filter(|_| !carried.contains(&key.as_str())) else {
            if ignore {
                continue;
            }
            return Err(if carried.contains(&key.as_str()) {
                format!("{place}: the path carries this field, so it is not a parameter here")
            } else {
                format!("{place}: no field of this request is named so")
            });
        };
        let value = form_decoded(value).ok_or_else(|| {
            format!("{place}: expected percent-encoded UTF-8 text, found `{value}`")
        })?;
        found[index].push(value);
    }
    let mut members = Vec::new();
    for ((wire, flat, shape), values) in fields.iter().zip(found) {
        let place = crate::server::json::nested(at, wire);
        match shape {
            Shape::List => {
                let items = values
                    .into_iter()
                    .map(|value| flat_value(value, *flat, &place))
                    .collect::<Result<Vec<_>, String>>()?;
                members.push(((*wire).to_owned(), crate::server::json::Value::Array(items)));
            }
            Shape::One | Shape::Optional => match values.len() {
                0 => {}
                1 => {
                    let value = values.into_iter().next().unwrap_or_default();
                    members.push(((*wire).to_owned(), flat_value(value, *flat, &place)?));
                }
                several => {
                    return Err(format!("{place}: expected one value, found {several}"));
                }
            },
        }
    }
    Ok(crate::server::json::Value::Object(members))
}

/// Whether the request's `Content-Type` is `expected`, its parameters ignored and its case too.
pub fn content_type_is(request: &Request, expected: &str) -> bool {
    request
        .headers
        .iter()
        .find(|(name, _)| name == "content-type")
        .is_some_and(|(_, value)| {
            value
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .eq_ignore_ascii_case(expected)
        })
}

/// The body as text.
///
/// # Errors
///
/// A refusal when it is not UTF-8.
pub fn body_text(body: &[u8]) -> Result<&str, String> {
    std::str::from_utf8(body).map_err(|error| format!("the body is not UTF-8: {error}"))
}

/// A `json` body as the value the input's decoder reads. Where the body is not `required`, no
/// body, or one of only whitespace, is the input `{}`.
///
/// # Errors
///
/// A refusal when the body is not UTF-8 or not JSON.
pub fn json_body(body: &[u8], required: bool) -> Result<crate::server::json::Value, String> {
    let text = body_text(body)?;
    if !required && text.trim().is_empty() {
        return Ok(crate::server::json::Value::Object(Vec::new()));
    }
    crate::server::json::parse(text).map_err(|error| format!("the body is not JSON: {error}"))
}

/// `value` with the members the path carries added. A path variable's value wins over the request
/// for its field, and the request naming that field is malformed: the path carries it.
///
/// # Errors
///
/// A refusal naming the member under `at` when `value` already holds one the path carries.
pub fn with_path(
    value: crate::server::json::Value,
    path: Vec<(String, crate::server::json::Value)>,
    at: &str,
) -> Result<crate::server::json::Value, String> {
    match value {
        crate::server::json::Value::Object(mut members) if !path.is_empty() => {
            for (wire, _) in &path {
                if members.iter().any(|(key, _)| key == wire) {
                    return Err(format!(
                        "{}: the path carries this field, so the request does not",
                        crate::server::json::nested(at, wire)
                    ));
                }
            }
            members.extend(path);
            Ok(crate::server::json::Value::Object(members))
        }
        // Not an object: the decoder refuses it by name.
        other => Ok(other),
    }
}

/// A scalar of an answer as text: a string as itself, a number as its spelling, a boolean as
/// `true` or `false`; `None` for an absent optional.
pub fn scalar_text(value: Option<&crate::server::json::Value>) -> Option<String> {
    match value? {
        crate::server::json::Value::Text(text) | crate::server::json::Value::Number(text) => Some(text.clone()),
        crate::server::json::Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

/// Adds the header `name` with `value` to `answer`; an absent value omits it.
///
/// # Errors
///
/// A refusal when the value is not a field value (RFC 9110 §5.5): it holds a control character
/// other than a horizontal tab, or starts or ends with a space or a tab. Such a value is never
/// written, truncated or re-encoded: a header value from input must not inject a header.
pub fn push_header(answer: &mut Response, name: &str, value: Option<String>) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    if let Some(found) = value.chars().find(|c| c.is_control() && *c != '\t') {
        return Err(format!(
            "header `{name}` would hold {found:?}, which is not field-value text"
        ));
    }
    if value.starts_with([' ', '\t']) || value.ends_with([' ', '\t']) {
        return Err(format!(
            "header `{name}` would start or end with a space or a tab, which a reader strips"
        ));
    }
    answer.headers.push((name.to_owned(), value));
    Ok(())
}

/// One RFC 9110 §11.6.1 challenge: `scheme`, then each param present as `name="value"`, in the
/// order given, separated by `, `. Each value is a quoted string whose `"` and `\` are escaped, so
/// no value can leave it; an absent param is omitted.
///
/// # Errors
///
/// A refusal when a value holds a control character other than a horizontal tab, which a quoted
/// string cannot carry.
pub fn challenge(scheme: &str, params: &[(&str, Option<String>)]) -> Result<String, String> {
    let mut out = scheme.to_owned();
    let mut first = true;
    for (name, value) in params {
        let Some(value) = value else {
            continue;
        };
        if let Some(found) = value.chars().find(|c| c.is_control() && *c != '\t') {
            return Err(format!(
                "challenge param `{name}` would hold {found:?}, which a quoted string cannot carry"
            ));
        }
        out.push_str(if first { " " } else { ", " });
        first = false;
        out.push_str(name);
        out.push_str("=\"");
        for c in value.chars() {
            if c == '"' || c == '\\' {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
    }
    Ok(out)
}

/// `text` as `application/x-www-form-urlencoded` writes it: ASCII letters, digits and `*-._` as
/// themselves, a space as `+`, every other byte as `%XX`.
pub fn form_encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(char::from(byte));
            }
            b' ' => out.push('+'),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Where a redirect sends the caller: `location`, with each parameter present appended to its
/// query (keeping a query it already has) or written as its fragment, form-encoded, in the order
/// given. An absent parameter is omitted.
///
/// # Errors
///
/// A refusal when `location` is absent or is not an absolute URI without a fragment: a scheme
/// (an ASCII letter, then letters, digits, `+`, `-` and `.`), a `:`, and visible ASCII only.
pub fn redirect(
    location: Option<String>,
    fragment: bool,
    parameters: &[(&str, Option<String>)],
) -> Result<String, String> {
    let location = location.ok_or_else(|| "the redirect has no location".to_owned())?;
    let absolute = location.split_once(':').is_some_and(|(scheme, _)| {
        let mut characters = scheme.chars();
        characters.next().is_some_and(|c| c.is_ascii_alphabetic())
            && characters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    });
    if !absolute || location.contains('#') || !location.chars().all(|c| c.is_ascii_graphic()) {
        return Err(format!(
            "the redirect location {location:?} is not an absolute URI without a fragment"
        ));
    }
    let written: Vec<String> = parameters
        .iter()
        .filter_map(|(name, value)| {
            value
                .as_ref()
                .map(|value| format!("{}={}", form_encoded(name), form_encoded(value)))
        })
        .collect();
    if written.is_empty() {
        return Ok(location);
    }
    let joined = written.join("&");
    Ok(if fragment {
        format!("{location}#{joined}")
    } else {
        match location.split_once('?') {
            Some((_, query)) if query.is_empty() || query.ends_with('&') => {
                format!("{location}{joined}")
            }
            Some(_) => format!("{location}&{joined}"),
            None => format!("{location}?{joined}"),
        }
    })
}

/// Appends a JSON value exactly as it was read.
pub fn push_json(out: &mut String, value: &crate::server::json::Value) {
    use crate::server::json::Value;
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(spelling) => out.push_str(spelling),
        Value::Text(text) => crate::server::json::push_text(out, text),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_json(out, item);
            }
            out.push(']');
        }
        Value::Object(members) => {
            out.push('{');
            for (index, (name, item)) in members.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                crate::server::json::push_text(out, name);
                out.push(':');
                push_json(out, item);
            }
            out.push('}');
        }
    }
}

/// A bare body: the value as JSON, `null` where there is none.
pub fn bare(value: Option<&crate::server::json::Value>) -> String {
    let mut out = String::new();
    push_json(&mut out, value.unwrap_or(&crate::server::json::Value::Null));
    out
}

/// `{<member>: <wire name>, …the error's own fields}`: the error object a binding declares.
pub fn error_object(member: &str, wire: &str, payload: Option<&crate::server::json::Value>) -> String {
    let mut out = String::from("{");
    crate::server::json::member(&mut out, member);
    crate::server::json::push_text(&mut out, wire);
    if let Some(crate::server::json::Value::Object(fields)) = payload {
        for (name, value) in fields {
            crate::server::json::member(&mut out, name);
            push_json(&mut out, value);
        }
    }
    out.push('}');
    out
}

/// `{<member>: <text>}`: a standard refusal's literal body.
pub fn literal_object(member: &str, text: &str) -> String {
    let mut out = String::from("{");
    crate::server::json::member(&mut out, member);
    crate::server::json::push_text(&mut out, text);
    out.push('}');
    out
}

/// A `single` view route's answer, from the `{"rows": [...]}` its shared path renders: the one row,
/// bare, at `200`, or `empty` without a body when the view holds none.
pub fn single(rows: &str, empty: u16) -> Response {
    let read = crate::server::json::parse(rows).unwrap_or(crate::server::json::Value::Null);
    match read.member("rows") {
        Some(crate::server::json::Value::Array(items)) if !items.is_empty() => {
            Response::new(200, JSON, bare(items.first()))
        }
        _ => Response::empty(empty),
    }
}
