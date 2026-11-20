//! The text edit of one change of the config file (story:config-put-touches-only-its-path): the
//! bytes of the named path, and nothing else, changed in the style they stand in, so the file's
//! comments, key order, quoting and blank lines stay as they were.
//!
//! A small locator in safe Rust over the file's lines finds the span of the level's mapping (the
//! root's `globals`, the `instances` item of a name, or that item's `roles` item of a role) and
//! the span of the path's keys under it. It reads the shapes conductor's own files use: block
//! mappings, block sequences whose items are block mappings (the first key on the dash line) or
//! one-line flow mappings, one-line flow mappings and sequences as values, one-line plain and
//! quoted scalars, block scalars off the path, full-line and trailing comments, and blank lines.
//! Any other shape on the path (a document marker, an anchor, alias, merge key or tag, an explicit
//! `?` key, a flow collection or a scalar over more than one line) is a [`Why`]: the caller writes
//! the file whole. [`checked`] reads every edited text back and compares it, as a value tree, with
//! the tree the change expects; a text that differs is written whole too.

use std::ops::Range;

use serde_yaml::{Mapping, Value as Yaml};

/// Why the text could not be edited in place: the file is written whole.
pub(super) type Why = String;

/// The level whose mapping a change is written under.
#[derive(Debug, Clone, Copy)]
pub(super) enum Scope<'a> {
    /// The root's `globals`.
    Global,
    /// The `instances` item of this name.
    Instance(&'a str),
    /// The `roles` item of the role (second) in the instance (first).
    Role(&'a str, &'a str),
}

/// One change of a whole instance.
#[derive(Debug, Clone, Copy)]
pub(super) enum Whole<'a> {
    /// The mapping appended to `instances`.
    Add(&'a Mapping),
    /// The item of the name replaced by the mapping.
    Put(&'a str, &'a Mapping),
    /// The item of the name removed.
    Remove(&'a str),
}

/// The text `text` with the dotted `path` under the mapping of `scope` set to `value`, or with
/// no `value` removed; a [`Why`] when the locator cannot edit it.
pub(super) fn put(
    text: &str,
    scope: Scope<'_>,
    path: &str,
    value: Option<&Yaml>,
) -> Result<String, Why> {
    let lines = Text::new(text)?;
    let steps: Vec<&str> = path.split('.').collect();
    let splice = match scope {
        Scope::Global => {
            let mut all = vec!["globals"];
            all.extend(steps);
            walk(&lines, lines.root(), &all, value)?
        }
        Scope::Instance(name) => walk(&lines, instance(&lines, name)?.node, &steps, value)?,
        Scope::Role(name, role) => {
            let item = instance(&lines, name)?;
            let roles = items(&lines, &item.node, "roles")?;
            let found = find(&lines, roles, "role", role)?
                .ok_or_else(|| format!("the instance's roles hold no role {role:?}"))?;
            walk(&lines, found.node, &steps, value)?
        }
    };
    Ok(apply(text, splice))
}

/// The text `text` with one whole instance added, replaced or removed, every other line kept; a
/// [`Why`] when the locator cannot edit it.
pub(super) fn whole(text: &str, change: Whole<'_>) -> Result<String, Why> {
    let lines = Text::new(text)?;
    let root = entries(&lines, &root_block(&lines))?;
    let Some(list) = root.iter().find(|entry| entry.key == "instances") else {
        return match change {
            Whole::Add(mapping) => {
                let at = root.last().map_or(0, |last| lines.after(last.last));
                let mut with = format!("instances:\n{}", item_text(mapping, 2)?);
                if at == text.len() && !text.is_empty() && !text.ends_with('\n') {
                    with.insert(0, '\n');
                }
                Ok(apply(
                    text,
                    Some(Splice {
                        range: at..at,
                        with,
                    }),
                ))
            }
            Whole::Put(..) | Whole::Remove(_) => Err("the file holds no instances".to_owned()),
        };
    };
    if list.inline.is_some() || list.last == list.line {
        return Err("instances is not a block list".to_owned());
    }
    let all = block_items(&lines, list)?;
    let dash = |item: &Item| {
        item.dash
            .ok_or_else(|| "an instance is no block item".to_owned())
    };
    let splice = match change {
        Whole::Add(mapping) => {
            let last = all.last().ok_or("instances holds no item")?;
            let at = lines.after(last.last);
            let column = indent(lines.line(dash(last)?));
            let mut with = item_text(mapping, column)?;
            if at == text.len() && !text.ends_with('\n') {
                with.insert(0, '\n');
            }
            Splice {
                range: at..at,
                with,
            }
        }
        Whole::Put(name, mapping) => {
            let item = named(&lines, &all, name)?;
            let line = dash(item)?;
            Splice {
                range: lines.start(line)..lines.after(item.last),
                with: item_text(mapping, indent(lines.line(line)))?,
            }
        }
        Whole::Remove(name) => {
            let item = named(&lines, &all, name)?;
            let line = dash(item)?;
            if all.len() == 1 {
                emptied(&lines, list.line, list.after, line, item.last, "[]")
            } else {
                Splice {
                    range: lines.start(line)..lines.after(item.last),
                    with: String::new(),
                }
            }
        }
    };
    Ok(apply(text, Some(splice)))
}

/// The text to write for an edit `edited` of a change whose value tree is `expected`, and why it
/// is written whole when it is: the edited text where it reads back as `expected`, else the tree
/// as YAML writes it.
///
/// # Errors
///
/// `expected` cannot be written as YAML.
pub(super) fn checked(
    edited: Result<String, Why>,
    expected: &Yaml,
) -> Result<(String, Option<Why>), serde_yaml::Error> {
    let why = match edited {
        Ok(text) => match serde_yaml::from_str::<Yaml>(&text) {
            Ok(back) if back == *expected => return Ok((text, None)),
            Ok(_) => "the edited text does not read back as the change".to_owned(),
            Err(_) => "the edited text does not read as YAML".to_owned(),
        },
        Err(why) => why,
    };
    serde_yaml::to_string(expected).map(|text| (text, Some(why)))
}

/// One change of the text: the bytes `range` replaced by `with`.
#[derive(Debug)]
struct Splice {
    range: Range<usize>,
    with: String,
}

fn apply(text: &str, splice: Option<Splice>) -> String {
    let mut out = text.to_owned();
    if let Some(splice) = splice {
        out.replace_range(splice.range, &splice.with);
    }
    out
}

/// The file's lines: each one's byte range, without its line break, and whether it is a line of
/// a block scalar's content.
struct Text<'t> {
    text: &'t str,
    lines: Vec<Range<usize>>,
    scalar: Vec<bool>,
}

impl<'t> Text<'t> {
    fn new(text: &'t str) -> Result<Self, Why> {
        let mut lines = Vec::new();
        let mut start = 0;
        for line in text.split('\n') {
            let end = start + line.len();
            lines.push(start..end);
            start = end + 1;
        }
        let scalar = vec![false; lines.len()];
        let mut this = Self {
            text,
            lines,
            scalar,
        };
        for at in 0..this.lines.len() {
            let line = this.line(at);
            let word = line.split([' ', '#']).next().unwrap_or_default();
            if word == "---" || word == "..." || line.starts_with('%') {
                return Err("a document marker or directive".to_owned());
            }
            if line.trim_start_matches(' ').starts_with('\t') {
                return Err("a tab in the indentation".to_owned());
            }
        }
        this.scalar = scalar_lines(&this);
        Ok(this)
    }

    /// Whether the line `at` holds YAML content: a block scalar's line (one that starts with `#`
    /// included), or a line that is not blank and not a comment.
    fn content(&self, at: usize) -> bool {
        self.scalar[at] || content(self.line(at))
    }

    fn line(&self, at: usize) -> &'t str {
        let range = self.lines[at].clone();
        self.text[range].trim_end_matches('\r')
    }

    fn start(&self, at: usize) -> usize {
        self.lines[at].start
    }

    /// The end of the line `at`'s content.
    fn end(&self, at: usize) -> usize {
        self.lines[at].start + self.line(at).len()
    }

    /// The first byte after the line `at` and its line break.
    fn after(&self, at: usize) -> usize {
        (self.lines[at].end + 1).min(self.text.len())
    }

    fn root(&self) -> Node {
        Node::Block(root_block(self))
    }
}

fn root_block(lines: &Text<'_>) -> Block {
    Block {
        first: 0,
        end: lines.lines.len(),
        col: 0,
        dash: None,
        owner: None,
    }
}

fn indent(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

/// Whether the line, read alone, holds YAML content: not blank, not a comment.
fn content(line: &str) -> bool {
    let rest = line.trim();
    !rest.is_empty() && !rest.starts_with('#')
}

/// The lines of the text's block scalars: every line that is not blank below a key or a dash whose
/// value opens with `|` or `>`, while it is indented deeper than that key or dash. Such a line is
/// the scalar's content even where it starts with `#`.
fn scalar_lines(lines: &Text<'_>) -> Vec<bool> {
    let mut out = vec![false; lines.lines.len()];
    let mut open: Option<usize> = None;
    for (at, slot) in out.iter_mut().enumerate() {
        let line = lines.line(at);
        if let Some(col) = open {
            if line.trim().is_empty() {
                continue;
            }
            if indent(line) > col {
                *slot = true;
                continue;
            }
        }
        open = opens_scalar(line);
    }
    out
}

/// The column of the key or dash on `line` whose value opens a block scalar, if one does.
fn opens_scalar(line: &str) -> Option<usize> {
    if !content(line) {
        return None;
    }
    let mut col = indent(line);
    while is_dash(&line[col..]) {
        let next = col + 1 + indent(&line[col + 1..]);
        if line[next..].starts_with(['|', '>']) {
            return Some(col);
        }
        if next >= line.len() {
            return None;
        }
        col = next;
    }
    let entry = entry_at(line, col, 0).ok()?;
    let found = entry.inline?;
    (found.kind == Kind::Open && matches!(line.as_bytes()[found.start], b'|' | b'>')).then_some(col)
}

fn is_dash(rest: &str) -> bool {
    rest == "-" || rest.starts_with("- ")
}

/// A mapping the locator found.
enum Node {
    Block(Block),
    /// A one-line flow mapping: its line and the byte of its `{` in the line.
    Flow {
        line: usize,
        open: usize,
    },
}

/// A block mapping: the lines `first..end` hold its entries, each key at the column `col`; the
/// first key of a list item sits on its dash line `dash`; `owner` is the key line whose value it
/// is, and the byte after that key's `:`.
struct Block {
    first: usize,
    end: usize,
    col: usize,
    dash: Option<usize>,
    owner: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Plain,
    Quoted,
    Flow,
    /// An anchor, an alias or a tag.
    Special,
    /// A block scalar, or a flow collection or quoted scalar that does not end on its line.
    Open,
}

/// A value on its key's line: its bytes in the line.
#[derive(Debug, Clone, Copy)]
struct Inline {
    start: usize,
    end: usize,
    kind: Kind,
}

/// One entry of a block mapping.
#[derive(Debug)]
struct Entry {
    key: String,
    merge: bool,
    line: usize,
    /// The entry's last line of content.
    last: usize,
    /// The byte after the key's `:` in the line.
    after: usize,
    inline: Option<Inline>,
}

/// A key's text as the tree names it.
fn decode(text: &str) -> String {
    match serde_yaml::from_str::<Yaml>(text) {
        Ok(Yaml::String(key)) => key,
        _ => text.to_owned(),
    }
}

/// The byte after the quoted scalar that opens at `start`, or none when it does not end on the
/// line.
fn quoted_end(line: &str, start: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let quote = bytes[start];
    let mut at = start + 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' if quote == b'"' => at += 2,
            b'\'' if quote == b'\'' && bytes.get(at + 1) == Some(&b'\'') => at += 2,
            byte if byte == quote => return Some(at + 1),
            _ => at += 1,
        }
    }
    None
}

/// Whether the byte `at` of `line` opens a comment: a `#` after a space or a tab.
fn comment_at(line: &[u8], at: usize) -> bool {
    line[at] == b'#' && at > 0 && matches!(line[at - 1], b' ' | b'\t')
}

/// The end of a plain scalar starting at `start`: before a comment or the line's end, its
/// trailing spaces and tabs trimmed.
fn plain_end(line: &str, start: usize) -> usize {
    let bytes = line.as_bytes();
    let cut = (start + 1..bytes.len())
        .find(|&at| comment_at(bytes, at))
        .unwrap_or(line.len());
    start + line[start..cut].trim_end().len()
}

/// The value on a key's line from the byte `from`, if any.
fn inline(line: &str, from: usize) -> Option<Inline> {
    let start = from + indent(&line[from..]);
    if start >= line.len() || line[start..].starts_with('#') {
        return None;
    }
    let (end, kind) = match line.as_bytes()[start] {
        b'{' | b'[' => match flow(line, start) {
            Ok(found) => (found.close + 1, Kind::Flow),
            Err(_) => (line.len(), Kind::Open),
        },
        b'"' | b'\'' => match quoted_end(line, start) {
            Some(end) => (end, Kind::Quoted),
            None => (line.len(), Kind::Open),
        },
        b'&' | b'*' | b'!' => (plain_end(line, start), Kind::Special),
        b'|' | b'>' => (line.len(), Kind::Open),
        _ => (plain_end(line, start), Kind::Plain),
    };
    Some(Inline { start, end, kind })
}

/// The entry whose key starts at the column `col` of the line `at`.
fn entry_at(line: &str, col: usize, at: usize) -> Result<Entry, Why> {
    let rest = &line[col..];
    if rest == "?" || rest.starts_with("? ") {
        return Err("an explicit `?` key".to_owned());
    }
    if rest.starts_with(['&', '*', '!']) {
        return Err("an anchor, alias or tag on a key".to_owned());
    }
    let colon = if rest.starts_with(['"', '\'']) {
        let end = quoted_end(rest, 0).ok_or("a quoted key over more than one line")?;
        if !rest[end..].starts_with(':') {
            return Err(format!("no `:` after the key on line {}", at + 1));
        }
        end
    } else {
        let bytes = rest.as_bytes();
        (0..bytes.len())
            .find(|&i| bytes[i] == b':' && matches!(bytes.get(i + 1), None | Some(b' ')))
            .ok_or_else(|| format!("no key on line {}", at + 1))?
    };
    let text = rest[..colon].trim_end();
    if text.contains(" #") || text.contains("\t#") {
        return Err(format!("no key on line {}", at + 1));
    }
    let after = col + colon + 1;
    Ok(Entry {
        key: decode(text),
        merge: text == "<<",
        line: at,
        last: at,
        after,
        inline: inline(line, after),
    })
}

/// The entries of a block mapping.
fn entries(lines: &Text<'_>, block: &Block) -> Result<Vec<Entry>, Why> {
    let mut out: Vec<Entry> = Vec::new();
    for at in block.first..block.end {
        let line = lines.line(at);
        if !lines.content(at) {
            continue;
        }
        let column = indent(line);
        if Some(at) == block.dash || (column == block.col && !is_dash(&line[column..])) {
            out.push(entry_at(line, block.col, at)?);
        } else if column < block.col {
            return Err(format!("line {} is outside its mapping", at + 1));
        } else {
            let Some(entry) = out.last_mut() else {
                return Err(format!("line {} has no key", at + 1));
            };
            entry.last = at;
        }
    }
    Ok(out)
}

/// The first line of content in `range`.
fn first_content(lines: &Text<'_>, range: Range<usize>) -> Option<usize> {
    range.into_iter().find(|&at| lines.content(at))
}

/// One item of a flow collection: its key (in a mapping), the byte its text starts at, and its
/// value's bytes.
#[derive(Debug)]
struct FlowItem {
    key: Option<String>,
    start: usize,
    value: Range<usize>,
}

/// A one-line flow collection: the byte of its closing bracket and its items.
#[derive(Debug)]
struct Flow {
    close: usize,
    map: bool,
    items: Vec<FlowItem>,
}

fn skip(bytes: &[u8], mut at: usize) -> usize {
    while bytes.get(at) == Some(&b' ') {
        at += 1;
    }
    at
}

/// The byte after the flow token (a scalar or a collection) that starts at `start`.
fn token(line: &str, start: usize) -> Result<usize, Why> {
    let bytes = line.as_bytes();
    match bytes[start] {
        b'{' | b'[' => flow(line, start).map(|found| found.close + 1),
        b'"' | b'\'' => {
            quoted_end(line, start).ok_or_else(|| "a quoted scalar over more than one line".into())
        }
        b'&' | b'*' | b'!' => Err("an anchor, alias or tag in a flow collection".to_owned()),
        b'?' => Err("an explicit `?` key".to_owned()),
        _ => {
            let mut at = start;
            while at < bytes.len() {
                let stop = match bytes[at] {
                    b',' | b'{' | b'}' | b'[' | b']' => true,
                    b':' => matches!(bytes.get(at + 1), None | Some(b' ' | b',' | b'}' | b']')),
                    b'#' => at > start && comment_at(bytes, at),
                    _ => false,
                };
                if stop {
                    break;
                }
                at += 1;
            }
            Ok(start + line[start..at].trim_end().len())
        }
    }
}

/// The flow collection whose bracket is the byte `open` of `line`; a [`Why`] when it does not
/// close on the line or holds a shape the locator does not read.
fn flow(line: &str, open: usize) -> Result<Flow, Why> {
    let bytes = line.as_bytes();
    let map = bytes[open] == b'{';
    let closer = if map { b'}' } else { b']' };
    let over = || "a flow collection over more than one line".to_owned();
    let mut items = Vec::new();
    let mut at = skip(bytes, open + 1);
    if bytes.get(at) == Some(&closer) {
        return Ok(Flow {
            close: at,
            map,
            items,
        });
    }
    loop {
        if at >= bytes.len() || bytes[at] == b'#' {
            return Err(over());
        }
        let start = at;
        let end = token(line, at)?;
        at = skip(bytes, end);
        let (key, value) = if map {
            if bytes.get(at) != Some(&b':') {
                return Err("a flow key without a value".to_owned());
            }
            at = skip(bytes, at + 1);
            let value = if at < bytes.len() && (bytes[at] == b',' || bytes[at] == closer) {
                at..at
            } else if at >= bytes.len() {
                return Err(over());
            } else {
                let value_end = token(line, at)?;
                let value = at..value_end;
                at = skip(bytes, value_end);
                value
            };
            (Some(decode(&line[start..end])), value)
        } else {
            (None, start..end)
        };
        items.push(FlowItem { key, start, value });
        match bytes.get(at) {
            Some(b',') => at = skip(bytes, at + 1),
            Some(&byte) if byte == closer => {
                return Ok(Flow {
                    close: at,
                    map,
                    items,
                });
            }
            None => return Err(over()),
            Some(_) => return Err("a flow collection the locator does not read".to_owned()),
        }
    }
}

/// The value tree `value` under the keys `rest`, the outermost first.
fn wrap(rest: &[&str], value: &Yaml) -> Yaml {
    rest.iter().rev().fold(value.clone(), |inner, key| {
        let mut map = Mapping::new();
        map.insert(Yaml::from(*key), inner);
        Yaml::Mapping(map)
    })
}

/// Whether `value` is written as nested lines: a mapping or a sequence that is not empty.
fn nested(value: &Yaml) -> bool {
    match value {
        Yaml::Mapping(map) => !map.is_empty(),
        Yaml::Sequence(list) => !list.is_empty(),
        _ => false,
    }
}

/// A scalar as YAML writes it alone, on one line.
fn scalar_text(value: &Yaml) -> Result<String, Why> {
    let text = serde_yaml::to_string(value).map_err(|error| error.to_string())?;
    let text = text.trim_end();
    if text.contains('\n') {
        return Err("a value with a line break".to_owned());
    }
    Ok(text.to_owned())
}

/// A value as a flow collection holds it: a scalar as YAML writes it, double-quoted where its
/// text holds a flow indicator or `#`; a collection as one line of JSON.
fn flow_text(value: &Yaml) -> Result<String, Why> {
    if nested(value) {
        return serde_json::to_string(value).map_err(|error| error.to_string());
    }
    let text = scalar_text(value)?;
    match value {
        Yaml::String(string) if text.contains([',', '[', ']', '{', '}', '#']) => {
            serde_json::to_string(string).map_err(|error| error.to_string())
        }
        _ => Ok(text),
    }
}

/// `value` as YAML's block lines, each indented by `column` spaces and ended by a line break.
fn block_lines(value: &Yaml, column: usize) -> Result<String, Why> {
    let text = serde_yaml::to_string(value).map_err(|error| error.to_string())?;
    let pad = " ".repeat(column);
    Ok(text
        .lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_owned()
            } else {
                format!("{pad}{line}\n")
            }
        })
        .collect())
}

/// One block entry `key: value` at the column `column`, ended by a line break.
fn entry_text(column: usize, key: &str, value: &Yaml) -> Result<String, Why> {
    let key = scalar_text(&Yaml::from(key))?;
    let pad = " ".repeat(column);
    if nested(value) {
        Ok(format!("{pad}{key}:\n{}", block_lines(value, column + 2)?))
    } else {
        Ok(format!("{pad}{key}: {}\n", scalar_text(value)?))
    }
}

/// One instance's mapping as an item of a block list whose dash is at `column`.
fn item_text(mapping: &Mapping, column: usize) -> Result<String, Why> {
    block_lines(
        &Yaml::Sequence(vec![Yaml::Mapping(mapping.clone())]),
        column,
    )
}

/// The splice that empties a mapping or list whose key is on the line `owner` (its `:` ends
/// before the byte `after`), whose only entry spans the lines `first..=last`: `marker` on the
/// key's line, its comment and the lines before the entry kept.
fn emptied(
    lines: &Text<'_>,
    owner: usize,
    after: usize,
    first: usize,
    last: usize,
    marker: &str,
) -> Splice {
    let start = lines.start(owner) + after;
    let comment = lines.text[start..lines.end(owner)].trim();
    let mut with = format!(" {marker}");
    if !comment.is_empty() {
        with.push(' ');
        with.push_str(comment);
    }
    let between = lines.text[lines.end(owner)..lines.start(first)].trim_end_matches(['\n', '\r']);
    with.push_str(between);
    Splice {
        range: start..lines.end(last),
        with,
    }
}

/// The change of `steps` under `node`: `value` set there, or with none the key removed. `None`
/// when the text holds what is asked already (a removal of a key it does not hold).
fn walk(
    lines: &Text<'_>,
    node: Node,
    steps: &[&str],
    value: Option<&Yaml>,
) -> Result<Option<Splice>, Why> {
    let Some((&step, rest)) = steps.split_first() else {
        return Err("an empty path".to_owned());
    };
    match node {
        Node::Block(block) => walk_block(lines, &block, step, rest, value),
        Node::Flow { line, open } => walk_flow(lines, line, open, step, rest, value),
    }
}

fn walk_block(
    lines: &Text<'_>,
    block: &Block,
    step: &str,
    rest: &[&str],
    value: Option<&Yaml>,
) -> Result<Option<Splice>, Why> {
    let all = entries(lines, block)?;
    if all.iter().any(|entry| entry.merge) {
        return Err("a merge key `<<` on the path".to_owned());
    }
    let Some(index) = all.iter().position(|entry| entry.key == step) else {
        let Some(value) = value else {
            return Ok(None);
        };
        let last = all.last().ok_or("an empty mapping on the path")?;
        let at = lines.after(last.last);
        let mut with = entry_text(block.col, step, &wrap(rest, value))?;
        if at == lines.text.len() && !lines.text.ends_with('\n') {
            with.insert(0, '\n');
        }
        return Ok(Some(Splice {
            range: at..at,
            with,
        }));
    };
    let entry = &all[index];
    if !rest.is_empty() {
        return walk(lines, below(lines, entry)?, rest, value);
    }
    let Some(value) = value else {
        return remove_block(lines, block, &all, index).map(Some);
    };
    let base = lines.start(entry.line);
    let line = lines.line(entry.line);
    match entry.inline {
        Some(found) if found.kind == Kind::Special => {
            Err("an anchor, alias or tag at the path".to_owned())
        }
        Some(found) if found.kind == Kind::Open || entry.last != entry.line => {
            Err("a value over more than one line at the path".to_owned())
        }
        Some(found) if !nested(value) || found.kind == Kind::Flow => {
            let with = if found.kind == Kind::Flow && nested(value) {
                flow_text(value)?
            } else {
                scalar_text(value)?
            };
            Ok(Some(Splice {
                range: base + found.start..base + found.end,
                with,
            }))
        }
        _ => {
            // The value's own text, from after the key's `:` through its last line, the key
            // line's comment kept.
            let tail = match entry.inline {
                Some(found) => &line[found.end..],
                None => &line[entry.after..],
            };
            let comment = tail.trim();
            let comment = if comment.is_empty() {
                String::new()
            } else {
                format!(" {comment}")
            };
            let with = if nested(value) {
                let below = block_lines(value, block.col + 2)?;
                format!("{comment}\n{}", below.trim_end_matches('\n'))
            } else {
                format!(" {}{comment}", scalar_text(value)?)
            };
            Ok(Some(Splice {
                range: base + entry.after..lines.end(entry.last),
                with,
            }))
        }
    }
}

/// The mapping that is the value of `entry`.
fn below(lines: &Text<'_>, entry: &Entry) -> Result<Node, Why> {
    match entry.inline {
        Some(found) if found.kind == Kind::Flow && entry.last == entry.line => {
            if lines.line(entry.line).as_bytes()[found.start] == b'{' {
                Ok(Node::Flow {
                    line: entry.line,
                    open: found.start,
                })
            } else {
                Err("a list on the path".to_owned())
            }
        }
        Some(found) if found.kind == Kind::Special => {
            Err("an anchor, alias or tag on the path".to_owned())
        }
        Some(found) if found.kind == Kind::Open || entry.last != entry.line => {
            Err("a value over more than one line on the path".to_owned())
        }
        Some(_) => Err("a scalar on the path".to_owned()),
        None => {
            let first = first_content(lines, entry.line + 1..entry.last + 1)
                .ok_or("an empty value on the path")?;
            let line = lines.line(first);
            let col = indent(line);
            if is_dash(&line[col..]) {
                return Err("a list on the path".to_owned());
            }
            Ok(Node::Block(Block {
                first: entry.line + 1,
                end: entry.last + 1,
                col,
                dash: None,
                owner: Some((entry.line, entry.after)),
            }))
        }
    }
}

fn remove_block(
    lines: &Text<'_>,
    block: &Block,
    all: &[Entry],
    index: usize,
) -> Result<Splice, Why> {
    let entry = &all[index];
    if Some(entry.line) == block.dash {
        // The first key of a list item: the next key moves up onto the dash line, and the
        // comment and blank lines between the two follow it, below the dash line.
        return Ok(match all.get(index + 1) {
            Some(next) => {
                let between = &lines.text[lines.after(entry.last)..lines.start(next.line)];
                match between.strip_suffix('\n') {
                    None => Splice {
                        range: lines.start(entry.line) + block.col
                            ..lines.start(next.line) + block.col,
                        with: String::new(),
                    },
                    Some(between) => Splice {
                        range: lines.start(entry.line) + block.col..lines.end(next.line),
                        with: format!("{}\n{between}", &lines.line(next.line)[block.col..]),
                    },
                }
            }
            None => Splice {
                range: lines.start(entry.line) + block.col..lines.end(entry.last),
                with: "{}".to_owned(),
            },
        });
    }
    if all.len() == 1 {
        let (owner, after) = block.owner.ok_or("a removal that empties a list item")?;
        return Ok(emptied(lines, owner, after, entry.line, entry.last, "{}"));
    }
    Ok(Splice {
        range: lines.start(entry.line)..lines.after(entry.last),
        with: String::new(),
    })
}

fn walk_flow(
    lines: &Text<'_>,
    at: usize,
    open: usize,
    step: &str,
    rest: &[&str],
    value: Option<&Yaml>,
) -> Result<Option<Splice>, Why> {
    let line = lines.line(at);
    let found = flow(line, open)?;
    if !found.map {
        return Err("a list on the path".to_owned());
    }
    let base = lines.start(at);
    let Some(index) = found
        .items
        .iter()
        .position(|item| item.key.as_deref() == Some(step))
    else {
        let Some(value) = value else {
            return Ok(None);
        };
        let key = scalar_text(&Yaml::from(step))?;
        let text = format!("{key}: {}", flow_text(&wrap(rest, value))?);
        return Ok(Some(match found.items.last() {
            Some(last) => {
                let at = base + last.value.end;
                Splice {
                    range: at..at,
                    with: format!(", {text}"),
                }
            }
            None => {
                let at = base + found.close;
                Splice {
                    range: at..at,
                    with: text,
                }
            }
        }));
    };
    let item = &found.items[index];
    if !rest.is_empty() {
        return if line.as_bytes().get(item.value.start) == Some(&b'{') {
            walk(
                lines,
                Node::Flow {
                    line: at,
                    open: item.value.start,
                },
                rest,
                value,
            )
        } else {
            Err("not a mapping on the path".to_owned())
        };
    }
    let Some(value) = value else {
        let range = match (index.checked_sub(1), found.items.get(index + 1)) {
            (_, Some(next)) => item.start..next.start,
            (Some(before), None) => found.items[before].value.end..item.value.end,
            (None, None) => item.start..item.value.end,
        };
        return Ok(Some(Splice {
            range: base + range.start..base + range.end,
            with: String::new(),
        }));
    };
    Ok(Some(Splice {
        range: base + item.value.start..base + item.value.end,
        with: flow_text(value)?,
    }))
}

/// One item of a list: its mapping, its dash line (none in a flow list) and its last line.
struct Item {
    node: Node,
    dash: Option<usize>,
    last: usize,
}

/// The items of the block list that is the value of `entry`, each a mapping.
fn block_items(lines: &Text<'_>, entry: &Entry) -> Result<Vec<Item>, Why> {
    let first = first_content(lines, entry.line + 1..entry.last + 1).ok_or("an empty list")?;
    let column = indent(lines.line(first));
    let mut spans: Vec<(usize, usize)> = Vec::new();
    for at in first..=entry.last {
        let line = lines.line(at);
        if !lines.content(at) {
            continue;
        }
        let here = indent(line);
        if here == column && is_dash(&line[here..]) {
            spans.push((at, at));
        } else if here > column {
            let Some(span) = spans.last_mut() else {
                return Err("a list the locator does not read".to_owned());
            };
            span.1 = at;
        } else {
            return Err("a list the locator does not read".to_owned());
        }
    }
    spans
        .into_iter()
        .map(|(dash, last)| {
            let line = lines.line(dash);
            let after = column + 1 + indent(&line[column + 1..]);
            let rest = &line[after..];
            let node = if rest.is_empty() || rest.starts_with('#') {
                let first = first_content(lines, dash + 1..last + 1).ok_or("an empty list item")?;
                Node::Block(Block {
                    first: dash + 1,
                    end: last + 1,
                    col: indent(lines.line(first)),
                    dash: None,
                    owner: None,
                })
            } else if rest.starts_with('{') {
                if last != dash {
                    return Err("a list item over more than one line".to_owned());
                }
                Node::Flow {
                    line: dash,
                    open: after,
                }
            } else if rest.starts_with(['&', '*', '!']) {
                return Err("an anchor, alias or tag on a list item".to_owned());
            } else if is_dash(rest) || rest.starts_with('[') {
                return Err("a list in a list".to_owned());
            } else {
                Node::Block(Block {
                    first: dash,
                    end: last + 1,
                    col: after,
                    dash: Some(dash),
                    owner: None,
                })
            };
            Ok(Item {
                node,
                dash: Some(dash),
                last,
            })
        })
        .collect()
}

/// The items of the list under `key` of the mapping `node`.
fn items(lines: &Text<'_>, node: &Node, key: &str) -> Result<Vec<Item>, Why> {
    let (at, open) = match node {
        Node::Block(block) => {
            let all = entries(lines, block)?;
            let entry = all
                .iter()
                .find(|entry| entry.key == key)
                .ok_or_else(|| format!("no {key}"))?;
            match entry.inline {
                None => return block_items(lines, entry),
                Some(found) if found.kind == Kind::Flow => (entry.line, found.start),
                Some(_) => return Err(format!("{key} is not a list the locator reads")),
            }
        }
        Node::Flow { line, open } => {
            let text = lines.line(*line);
            let found = flow(text, *open)?;
            let item = found
                .items
                .iter()
                .find(|item| item.key.as_deref() == Some(key))
                .ok_or_else(|| format!("no {key}"))?;
            (*line, item.value.start)
        }
    };
    let text = lines.line(at);
    if text.as_bytes().get(open) != Some(&b'[') {
        return Err(format!("{key} is not a list"));
    }
    flow(text, open)?
        .items
        .into_iter()
        .map(|item| {
            if text.as_bytes().get(item.value.start) == Some(&b'{') {
                Ok(Item {
                    node: Node::Flow {
                        line: at,
                        open: item.value.start,
                    },
                    dash: None,
                    last: at,
                })
            } else {
                Err(format!("an item of {key} is not a mapping"))
            }
        })
        .collect()
}

/// The text of the scalar under `key` of the mapping `node`, if it holds one.
fn scalar_at(lines: &Text<'_>, node: &Node, key: &str) -> Result<Option<String>, Why> {
    let decoded = |text: &str| match serde_yaml::from_str::<Yaml>(text) {
        Ok(Yaml::String(value)) => Some(value),
        _ => None,
    };
    match node {
        Node::Block(block) => {
            let all = entries(lines, block)?;
            Ok(all
                .iter()
                .find(|entry| entry.key == key)
                .and_then(|entry| {
                    let found = entry.inline?;
                    matches!(found.kind, Kind::Plain | Kind::Quoted)
                        .then(|| &lines.line(entry.line)[found.start..found.end])
                })
                .and_then(decoded))
        }
        Node::Flow { line, open } => {
            let text = lines.line(*line);
            let found = flow(text, *open)?;
            Ok(found
                .items
                .iter()
                .find(|item| item.key.as_deref() == Some(key))
                .and_then(|item| decoded(&text[item.value.clone()])))
        }
    }
}

/// The item of `all` whose `key` is `value`.
fn find(lines: &Text<'_>, all: Vec<Item>, key: &str, value: &str) -> Result<Option<Item>, Why> {
    for item in all {
        if scalar_at(lines, &item.node, key)?.as_deref() == Some(value) {
            return Ok(Some(item));
        }
    }
    Ok(None)
}

fn named<'a>(lines: &Text<'_>, all: &'a [Item], name: &str) -> Result<&'a Item, Why> {
    for item in all {
        if scalar_at(lines, &item.node, "name")?.as_deref() == Some(name) {
            return Ok(item);
        }
    }
    Err(format!("instances hold no {name:?}"))
}

/// The `instances` item named `name`.
fn instance(lines: &Text<'_>, name: &str) -> Result<Item, Why> {
    let all = items(lines, &lines.root(), "instances")?;
    find(lines, all, "name", name)?.ok_or_else(|| format!("instances hold no {name:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = "\
# the host's instances
version: conductor.config/2
daemon: {store: \"sqlite:~/daemon.sqlite\"}

globals:
  # caps for every instance
  cadence:
    cycle: 3h   # slow
    watch: 9m
  thresholds: {disk_low: 20G, silent_session: 30m}

instances:
  - name: one   # the first
    placement: systemd
    standing_rules: |
      keep: this
        name: not a key
    controllers: {max_working: 4, max_subagents: 2}
    roles:
      - {role: director, harness: claude, model: opus}
      - role: controller
        model: sonnet

  # the second
  - name: two
    placement: systemd
    cadence:
      watch: 7m
";

    fn value(text: &str) -> Yaml {
        serde_yaml::from_str(text).expect("a value")
    }

    /// The edit, held to the tree the change expects.
    fn edit(scope: Scope<'_>, path: &str, to: Option<&str>) -> String {
        let to = to.map(value);
        let text = put(FILE, scope, path, to.as_ref()).expect("an edit");
        let mut expected: Yaml = value(FILE);
        let map = match scope {
            Scope::Global => expected["globals"].as_mapping_mut(),
            Scope::Instance(name) => expected["instances"]
                .as_sequence_mut()
                .and_then(|list| list.iter_mut().find(|item| item["name"] == name))
                .and_then(Yaml::as_mapping_mut),
            Scope::Role(name, role) => expected["instances"]
                .as_sequence_mut()
                .and_then(|list| list.iter_mut().find(|item| item["name"] == name))
                .and_then(|item| item["roles"].as_sequence_mut())
                .and_then(|list| list.iter_mut().find(|item| item["role"] == role))
                .and_then(Yaml::as_mapping_mut),
        }
        .expect("the scope");
        let mut steps: Vec<&str> = path.split('.').collect();
        let last = steps.pop().expect("a key");
        let mut at = map;
        for step in steps {
            at = at
                .entry(Yaml::from(step))
                .or_insert_with(|| Yaml::Mapping(Mapping::new()))
                .as_mapping_mut()
                .expect("a mapping");
        }
        match to {
            Some(to) => {
                at.insert(Yaml::from(last), to);
            }
            None => {
                at.remove(last);
            }
        }
        let (written, why) = checked(Ok(text.clone()), &expected).expect("written");
        assert_eq!(why, None, "{text}");
        written
    }

    /// The lines of `new` that differ from `FILE`: removed, then added.
    fn diff(new: &str) -> (Vec<String>, Vec<String>) {
        let old: Vec<&str> = FILE.lines().collect();
        let new: Vec<&str> = new.lines().collect();
        let mut front = 0;
        while front < old.len() && front < new.len() && old[front] == new[front] {
            front += 1;
        }
        let mut back = 0;
        while back < old.len() - front
            && back < new.len() - front
            && old[old.len() - 1 - back] == new[new.len() - 1 - back]
        {
            back += 1;
        }
        let gone = old[front..old.len() - back]
            .iter()
            .map(|line| (*line).to_owned())
            .collect();
        let came = new[front..new.len() - back]
            .iter()
            .map(|line| (*line).to_owned())
            .collect();
        (gone, came)
    }

    fn one_line(new: &str, from: &str, to: &str) {
        assert_eq!(
            diff(new),
            (vec![from.to_owned()], vec![to.to_owned()]),
            "{new}"
        );
    }

    #[test]
    fn a_block_value_is_replaced_with_its_trailing_comment() {
        let new = edit(Scope::Global, "cadence.cycle", Some("2h"));
        one_line(&new, "    cycle: 3h   # slow", "    cycle: 2h   # slow");
    }

    #[test]
    fn a_flow_value_is_replaced_in_its_mapping() {
        let new = edit(Scope::Instance("one"), "controllers.max_working", Some("5"));
        one_line(
            &new,
            "    controllers: {max_working: 4, max_subagents: 2}",
            "    controllers: {max_working: 5, max_subagents: 2}",
        );
    }

    #[test]
    fn the_first_key_on_the_dash_line_is_found_and_replaced() {
        let new = edit(Scope::Instance("two"), "placement", Some("local"));
        one_line(&new, "    placement: systemd", "    placement: local");
        assert!(new.contains("  - name: one   # the first\n"), "{new}");
        let new = edit(Scope::Role("one", "controller"), "model", Some("opus"));
        one_line(&new, "        model: sonnet", "        model: opus");
    }

    #[test]
    fn a_flow_role_item_stays_one_line() {
        let new = edit(Scope::Role("one", "director"), "model", Some("sonnet"));
        one_line(
            &new,
            "      - {role: director, harness: claude, model: opus}",
            "      - {role: director, harness: claude, model: sonnet}",
        );
    }

    #[test]
    fn a_new_block_key_is_one_line_at_the_mappings_indentation() {
        let new = edit(Scope::Global, "cadence.sweep", Some("1h"));
        assert_eq!(
            diff(&new),
            (vec![], vec!["    sweep: 1h".to_owned()]),
            "{new}"
        );
        // The last instance's mapping ends before the end of the file.
        let new = edit(Scope::Instance("two"), "records", Some("~/records"));
        assert!(
            new.ends_with("      watch: 7m\n    records: ~/records\n"),
            "{new}"
        );
    }

    #[test]
    fn a_missing_mapping_is_written_in_its_parents_style() {
        let new = edit(Scope::Instance("two"), "checkouts.root", Some("~/two"));
        assert_eq!(
            diff(&new),
            (
                vec![],
                vec!["    checkouts:".to_owned(), "      root: ~/two".to_owned()]
            ),
            "{new}"
        );
        let new = edit(Scope::Instance("one"), "controllers.limits.cap", Some("3"));
        one_line(
            &new,
            "    controllers: {max_working: 4, max_subagents: 2}",
            "    controllers: {max_working: 4, max_subagents: 2, limits: {\"cap\":3}}",
        );
    }

    #[test]
    fn a_new_flow_key_goes_before_the_brace() {
        let new = edit(Scope::Global, "thresholds.build_slot", Some("2"));
        one_line(
            &new,
            "  thresholds: {disk_low: 20G, silent_session: 30m}",
            "  thresholds: {disk_low: 20G, silent_session: 30m, build_slot: 2}",
        );
    }

    #[test]
    fn a_flow_string_with_an_indicator_is_quoted() {
        let new = edit(Scope::Global, "thresholds.disk_low", Some("'a, b'"));
        one_line(
            &new,
            "  thresholds: {disk_low: 20G, silent_session: 30m}",
            "  thresholds: {disk_low: \"a, b\", silent_session: 30m}",
        );
    }

    #[test]
    fn a_removal_takes_one_entry_only() {
        let new = edit(Scope::Global, "cadence.watch", None);
        assert_eq!(
            diff(&new),
            (vec!["    watch: 9m".to_owned()], vec![]),
            "{new}"
        );
        let new = edit(Scope::Global, "thresholds.disk_low", None);
        one_line(
            &new,
            "  thresholds: {disk_low: 20G, silent_session: 30m}",
            "  thresholds: {silent_session: 30m}",
        );
        let new = edit(Scope::Global, "thresholds.silent_session", None);
        one_line(
            &new,
            "  thresholds: {disk_low: 20G, silent_session: 30m}",
            "  thresholds: {disk_low: 20G}",
        );
        let new = edit(Scope::Instance("two"), "cadence", None);
        assert!(new.ends_with("    placement: systemd\n"), "{new}");
        // A block mapping left empty is `{}`.
        let new = edit(Scope::Instance("two"), "cadence.watch", None);
        assert!(new.ends_with("    cadence: {}\n"), "{new}");
        // A key the file does not hold changes nothing.
        let new = edit(Scope::Instance("two"), "cadence.cycle", None);
        assert_eq!(new, FILE);
    }

    #[test]
    fn the_block_scalar_off_the_path_and_the_blank_lines_are_kept() {
        let new = edit(Scope::Instance("one"), "placement", Some("local"));
        one_line(&new, "    placement: systemd", "    placement: local");
        assert!(
            new.contains("      keep: this\n        name: not a key\n"),
            "{new}"
        );
        assert!(new.contains("\n\n  # the second\n"), "{new}");
    }

    #[test]
    fn a_collection_value_is_nested_block_lines() {
        let new = edit(
            Scope::Instance("two"),
            "cadence",
            Some("{watch: 5m, cycle: 2h}"),
        );
        assert_eq!(
            diff(&new),
            (
                vec!["      watch: 7m".to_owned()],
                vec!["      watch: 5m".to_owned(), "      cycle: 2h".to_owned()]
            ),
            "{new}"
        );
    }

    #[test]
    fn globals_the_file_lacks_are_added_at_the_root() {
        let file = "version: conductor.config/2\n# the end\ninstances:\n  - name: one\n";
        let new = put(file, Scope::Global, "cadence.watch", Some(&value("5m"))).expect("an edit");
        assert_eq!(
            new,
            "version: conductor.config/2\n# the end\ninstances:\n  - name: one\nglobals:\n  \
             cadence:\n    watch: 5m\n"
        );
    }

    #[test]
    fn a_whole_instance_is_replaced_added_and_removed_alone() {
        let mut map = Mapping::new();
        map.insert(Yaml::from("name"), Yaml::from("two"));
        map.insert(Yaml::from("placement"), Yaml::from("local"));
        let new = whole(FILE, Whole::Put("two", &map)).expect("an edit");
        assert!(
            new.ends_with("  # the second\n  - name: two\n    placement: local\n"),
            "{new}"
        );
        assert!(new.starts_with(&FILE[..FILE.find("  # the second").expect("x")]));
        map.insert(Yaml::from("name"), Yaml::from("three"));
        let new = whole(FILE, Whole::Add(&map)).expect("an edit");
        assert_eq!(
            new,
            format!("{FILE}  - name: three\n    placement: local\n")
        );
        let new = whole(FILE, Whole::Remove("one")).expect("an edit");
        let start = FILE.find("  - name: one").expect("x");
        let end = FILE.find("\n\n  # the second").expect("x") + 1;
        assert_eq!(new, format!("{}{}", &FILE[..start], &FILE[end..]));
        let new = whole(FILE, Whole::Remove("two")).expect("an edit");
        assert!(
            new.ends_with("        model: sonnet\n\n  # the second\n"),
            "{new}"
        );
    }

    #[test]
    fn each_shape_the_locator_does_not_read_is_named() {
        let cases = [
            (
                "---\nversion: x\nglobals:\n  a: 1\n",
                "a",
                "document marker",
            ),
            ("globals:\n  caps: &caps {a: 1}\n", "caps.a", "anchor"),
            ("globals:\n  caps: *caps\n", "caps.a", "anchor"),
            ("globals:\n  caps: !tag {a: 1}\n", "caps.a", "anchor"),
            ("globals:\n  <<: *caps\n  a: 1\n", "a", "merge key"),
            ("globals:\n  ? a\n  : 1\n", "a", "explicit"),
            (
                "globals:\n  caps: {a: 1,\n    b: 2}\n",
                "caps.a",
                "more than one line",
            ),
            ("globals:\n  a: one\n    two\n", "a", "more than one line"),
            (
                "globals:\n  a: \"one\n    two\"\n",
                "a",
                "more than one line",
            ),
            ("globals:\n  a: |\n    one\n", "a", "more than one line"),
        ];
        for (file, path, said) in cases {
            let why = put(file, Scope::Global, path, Some(&value("2"))).expect_err(file);
            assert!(why.contains(said), "{file}: {why}");
        }
        let why = put(
            "globals:\n  a: 1\n",
            Scope::Global,
            "a",
            Some(&Yaml::from("one\ntwo")),
        )
        .expect_err("a line break");
        assert!(why.contains("line break"), "{why}");
    }

    /// The text of `put` of `to` (none: a removal) at `path` under `scope` of `file`.
    fn put_text(file: &str, scope: Scope<'_>, path: &str, to: Option<&str>) -> String {
        let to = to.map(value);
        put(file, scope, path, to.as_ref()).expect("an edit")
    }

    #[test]
    fn a_comment_after_a_tab_is_kept() {
        let new = put_text(
            "globals:\n  a: b\t# keep me\n",
            Scope::Global,
            "a",
            Some("2"),
        );
        assert_eq!(new, "globals:\n  a: 2\t# keep me\n");
        let new = put_text(
            "globals:\n  t: {a: b, c: d}\t# keep me\n",
            Scope::Global,
            "t.a",
            Some("2"),
        );
        assert_eq!(new, "globals:\n  t: {a: 2, c: d}\t# keep me\n");
    }

    #[test]
    fn a_block_scalar_line_that_starts_with_a_hash_is_its_content() {
        // Removing the scalar takes its last line too.
        let new = put_text(
            "globals:\n  n: |\n    text\n    # last\n  a: 1\n",
            Scope::Global,
            "n",
            None,
        );
        assert_eq!(new, "globals:\n  a: 1\n");
        // A key added after it goes after its last line, not into it.
        let file = "globals:\n  n: |\n    text\n    # last\n";
        let new = put_text(file, Scope::Global, "a", Some("1"));
        assert_eq!(new, format!("{file}  a: 1\n"));
        // An instance whose last key is such a scalar is removed whole.
        let file = "version: x\ninstances:\n  - name: one\n    standing_rules: |\n      text\n      \
                    # last\n  - name: two\n";
        let new = whole(file, Whole::Remove("one")).expect("an edit");
        assert_eq!(new, "version: x\ninstances:\n  - name: two\n");
        // An instance added after it goes after its last line.
        let file = "instances:\n  - name: one\n    standing_rules: |\n      text\n      # last\n";
        let mut map = Mapping::new();
        map.insert(Yaml::from("name"), Yaml::from("two"));
        let new = whole(file, Whole::Add(&map)).expect("an edit");
        assert_eq!(new, format!("{file}  - name: two\n"));
        // A comment below the scalar, at its key's column, is a comment still.
        let file = "globals:\n  n: |\n    text\n  # about a\n  a: 1\n";
        let new = put_text(file, Scope::Global, "n", None);
        assert_eq!(new, "globals:\n  # about a\n  a: 1\n");
    }

    #[test]
    fn removing_the_first_key_on_a_dash_line_keeps_the_comments_under_it() {
        let file = "instances:\n  - name: one\n    roles:\n      - role: director\n        \
                    # pinned\n        model: opus\n";
        let new = put_text(file, Scope::Role("one", "director"), "role", None);
        assert_eq!(
            new,
            "instances:\n  - name: one\n    roles:\n      - model: opus\n        # pinned\n"
        );
        let file = "instances:\n  - name: one\n    # pinned\n\n    placement: systemd\n    \
                    cadence:\n      watch: 5m\n";
        let new = put_text(file, Scope::Instance("one"), "name", None);
        assert_eq!(
            new,
            "instances:\n  - placement: systemd\n    # pinned\n\n    cadence:\n      watch: 5m\n"
        );
        // With no comment between, the next key moves up as before.
        let file = "instances:\n  - name: one\n    placement: systemd\n";
        let new = put_text(file, Scope::Instance("one"), "name", None);
        assert_eq!(new, "instances:\n  - placement: systemd\n");
    }

    #[test]
    fn a_text_that_does_not_read_back_is_written_whole() {
        let expected = value("a: 2\n");
        let (text, why) = checked(Ok("a: 1\n".to_owned()), &expected).expect("written");
        assert_eq!(text, "a: 2\n");
        assert!(why.is_some_and(|why| why.contains("read back")));
    }
}
