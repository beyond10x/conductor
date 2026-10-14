//! `conductor rules show` (`story:rules-show`): the `Rules` view over the instance's `rules.md`
//! in its records directory ([`config::active`]), declared in `spec/domains/direction.yaml` as
//! `conductor.direction.Rules` and `conductor.direction.RulesRow`.
//!
//! It reads `rules.md` only: it writes nothing and opens no store. An item is a line that opens
//! with its number and `. ` at the margin, and every line after it up to the next such line, the
//! next heading or the next line that starts at the margin; blank lines at its end are not part
//! of it. It is printed byte for byte as the file holds it, so a worker brief carries an item as
//! the file says it, wherever it sits, instead of an excerpt cut by line numbers.
//!
//! `--item 43,49,51` prints those items in the order given; `--section <heading>` every item under
//! each heading of that text (any case), its subsections' included, in file order; neither, every
//! item. Text output is the items verbatim with one blank line between two; `--format json` a list
//! of rows. A number no item holds, or a heading the file does not hold, is refused on one line of
//! standard error naming it, and nothing is printed.

use std::fs;
use std::io::{self, Write as _};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result, bail};
use conductor_model::direction::{Rules, RulesRefusal, RulesRefused, RulesRow};
use serde_json::{Value, json};

use crate::cli::RulesShowArgs;
use crate::config;

/// The command's words, as its messages name it.
const COMMAND: &str = "rules show";

/// The file under the records directory the view reads.
pub const RULES_FILE: &str = "rules.md";

/// One item of `rules.md` as [`parse`] finds it, with the headings above it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// Its number.
    pub number: i64,
    /// The headings above it, outermost first.
    pub headings: Vec<String>,
    /// Its lines, byte for byte, each ending in a newline.
    pub text: String,
}

impl Item {
    /// The view's row of this item.
    #[must_use]
    pub fn row(&self) -> RulesRow {
        RulesRow {
            item: self.number,
            title: title(&self.text),
            section: self.headings.join(" / "),
            source: source(&self.text),
            text: self.text.clone(),
        }
    }
}

/// An item being read: its number, the headings above it and its lines so far.
type Open<'a> = (i64, Vec<String>, Vec<&'a str>);

/// Every item of `text`, in file order. The file's title, a `#` heading, is no section: the
/// headings above an item are those of level 2 and deeper.
#[must_use]
pub fn parse(text: &str) -> Vec<Item> {
    let mut items = Vec::new();
    // Each heading above the current line, with its level.
    let mut headings: Vec<(usize, String)> = Vec::new();
    let mut open: Option<Open<'_>> = None;
    for line in text.split_inclusive('\n') {
        let line = line.strip_suffix('\n').unwrap_or(line);
        if let Some((level, heading)) = heading(line) {
            close(&mut open, &mut items);
            headings.retain(|(above, _)| *above < level);
            if level > 1 {
                headings.push((level, heading));
            }
            continue;
        }
        if let Some(number) = item_number(line) {
            close(&mut open, &mut items);
            let path = headings.iter().map(|(_, text)| text.clone()).collect();
            open = Some((number, path, vec![line]));
            continue;
        }
        let at_margin = !line.is_empty() && !line.starts_with([' ', '\t']);
        if at_margin {
            close(&mut open, &mut items);
        } else if let Some((_, _, lines)) = open.as_mut() {
            lines.push(line);
        }
    }
    close(&mut open, &mut items);
    items
}

/// Ends the item being read, if any, without the blank lines at its end.
fn close(open: &mut Option<Open<'_>>, items: &mut Vec<Item>) {
    let Some((number, headings, mut lines)) = open.take() else {
        return;
    };
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let mut text = String::new();
    for line in lines {
        text.push_str(line);
        text.push('\n');
    }
    items.push(Item {
        number,
        headings,
        text,
    });
}

/// The level and text of a Markdown heading line (`## Conductor`).
fn heading(line: &str) -> Option<(usize, String)> {
    let level = line.bytes().take_while(|&byte| byte == b'#').count();
    if level == 0 {
        return None;
    }
    let rest = &line[level..];
    if !(rest.is_empty() || rest.starts_with([' ', '\t'])) {
        return None;
    }
    Some((level, rest.trim().trim_end_matches('#').trim().to_owned()))
}

/// The number of a line that opens an item: digits at the margin, then `. `.
fn item_number(line: &str) -> Option<i64> {
    let digits = line.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    if rest == "." || rest.starts_with(". ") {
        line[..digits].parse().ok()
    } else {
        None
    }
}

/// The item's bold lead without the asterisks, or else its first sentence; without a final full
/// stop either way.
fn title(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default();
    let body = first
        .split_once(". ")
        .map_or("", |(_, body)| body)
        .trim_start();
    let lead = body
        .strip_prefix("**")
        .and_then(|bold| bold.split_once("**"))
        .map_or_else(
            || {
                body.split_once(". ")
                    .map_or(body, |(sentence, _)| sentence)
                    .trim()
            },
            |(bold, _)| bold.trim(),
        );
    lead.trim_end_matches(['.', ':']).to_owned()
}

/// What follows the item's `Source:` line, on one line.
fn source(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let at = lines
        .iter()
        .skip(1)
        .position(|line| line.trim_start().starts_with("Source:"))?
        + 1;
    let rest = lines[at].trim_start().trim_start_matches("Source:");
    let words: Vec<&str> = std::iter::once(rest)
        .chain(lines[at + 1..].iter().copied())
        .flat_map(str::split_whitespace)
        .collect();
    Some(words.join(" "))
}

/// The items `input` asks for, in the order it gives them.
///
/// # Errors
///
/// A word of `item` that is no number, both `item` and `section`, or the refusal the
/// specification names: [`RulesRefusal::RuleItemNotFound`] for a number no item holds,
/// [`RulesRefusal::RuleSectionNotFound`] for a heading the file does not hold.
pub fn select<'a>(items: &'a [Item], input: &Rules) -> Result<Vec<&'a Item>> {
    match (input.item.as_deref(), input.section.as_deref()) {
        (Some(_), Some(_)) => bail!("{COMMAND}: give --item or --section, not both"),
        (Some(list), None) => {
            let mut chosen = Vec::new();
            let mut missing = Vec::new();
            for word in list
                .split(',')
                .map(str::trim)
                .filter(|word| !word.is_empty())
            {
                let Ok(number) = word.parse::<i64>() else {
                    bail!("{COMMAND}: --item {word:?} is not an item number");
                };
                let found: Vec<&Item> = items.iter().filter(|item| item.number == number).collect();
                if found.is_empty() {
                    missing.push(number.to_string());
                }
                chosen.extend(found);
            }
            if !missing.is_empty() {
                refuse(&RulesRefused {
                    refusal: RulesRefusal::RuleItemNotFound,
                    detail: format!("no item of {RULES_FILE} is numbered {}", missing.join(", ")),
                })?;
            }
            Ok(chosen)
        }
        (None, Some(section)) => {
            let wanted = section.trim().to_lowercase();
            let chosen: Vec<&Item> = items
                .iter()
                .filter(|item| {
                    item.headings
                        .iter()
                        .any(|heading| heading.to_lowercase() == wanted)
                })
                .collect();
            if chosen.is_empty() {
                refuse(&RulesRefused {
                    refusal: RulesRefusal::RuleSectionNotFound,
                    detail: format!(
                        "no heading of {RULES_FILE} with an item under it reads {section:?}"
                    ),
                })?;
            }
            Ok(chosen)
        }
        (None, None) => Ok(items.iter().collect()),
    }
}

fn refuse(answer: &RulesRefused) -> Result<()> {
    bail!(
        "{COMMAND}: refused: {}: {}",
        refusal_name(answer.refusal),
        answer.detail
    )
}

/// The refusal's name as the specification spells it. Exhaustive, so a refusal the specification
/// adds fails to compile here rather than rendering under a guessed name.
const fn refusal_name(refusal: RulesRefusal) -> &'static str {
    match refusal {
        RulesRefusal::RuleItemNotFound => "RuleItemNotFound",
        RulesRefusal::RuleSectionNotFound => "RuleSectionNotFound",
    }
}

/// `conductor rules show`: the view the module documentation describes, over the binding's
/// `local` callable `rules-show` (input `conductor.direction.Rules`).
///
/// # Errors
///
/// `--format` is neither text nor json, `rules.md` cannot be read, the selection is refused
/// ([`select`]), or standard output cannot be written.
pub fn rules_show(args: &RulesShowArgs) -> Result<ExitCode> {
    let input = Rules {
        item: args.item.clone(),
        section: args.section.clone(),
        format: args.format.clone(),
    };
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is not one of text, json"),
    };
    let file = PathBuf::from(&config::active().instance.records).join(RULES_FILE);
    let text =
        fs::read_to_string(&file).with_context(|| format!("{COMMAND}: read {}", file.display()))?;
    let items = parse(&text);
    let chosen = select(&items, &input)?;

    let printed = if json {
        let rows: Vec<Value> = chosen
            .iter()
            .map(|item| {
                let row = item.row();
                json!({
                    "item": row.item,
                    "title": row.title,
                    "section": row.section,
                    "source": row.source,
                    "text": row.text,
                })
            })
            .collect();
        format!("{}\n", Value::Array(rows))
    } else {
        chosen
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    io::stdout()
        .lock()
        .write_all(printed.as_bytes())
        .with_context(|| format!("{COMMAND}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}
