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
//! of rows; `--brief` one line per item, `<n>. <title>` ([`brief`]), so a session reads a
//! section's titles and asks for an item whole with `--item` (`story:conductor-context-budget`).
//! `--changed-since <revision>` keeps, of the items selected, those whose text differs from the
//! item of that number in the revision's `rules.md` of the records' git, or that it does not
//! hold, and names on one line of standard error the numbers the revision held there that the
//! file no longer holds ([`changed`]). A number no item holds, a heading the file does not hold,
//! or a revision the records' git does not resolve, is refused on one line of
//! standard error naming it, and nothing is printed; a heading's refusal names, on that line, every
//! section heading the file has (`story:rules-section-names`). For one release a role's name
//! and its old name read each other's heading, `director` a section headed `conductor`, with a
//! note on standard error naming both (`story:role-name-aliases`, [`role_headings`]).

use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Context as _, Result, bail};
use conductor_model::config::RoleName;
use conductor_model::direction::{Rules, RulesRefusal, RulesRefused, RulesRow};
use serde_json::{Value, json};

use crate::cli::RulesShowArgs;
use crate::config;
use crate::records::files::{ROLE_FILES, RULES_DIR, opens_rule};

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
            title: title(&self.text.split_whitespace().collect::<Vec<_>>().join(" ")),
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

/// Every section heading of `text`: the headings of level 2 and deeper, in file order, each once
/// (a heading that differs from an earlier one only in case is that one, as `--section` reads it).
#[must_use]
pub fn sections(text: &str) -> Vec<String> {
    let mut sections: Vec<String> = Vec::new();
    for line in text.lines() {
        if let Some((level, heading)) = heading(line)
            && level > 1
            && !sections
                .iter()
                .any(|seen| seen.to_lowercase() == heading.to_lowercase())
        {
            sections.push(heading);
        }
    }
    sections
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

/// The item's number and title on one line, `<n>. <title>` ([`title`]): its bold lead without
/// the asterisks, or else its first sentence, without a final full stop
/// (`story:conductor-context-budget`).
#[must_use]
pub fn brief(item: &Item) -> String {
    let title = title(&item.text.split_whitespace().collect::<Vec<_>>().join(" "));
    format!("{}. {title}", item.number)
}

/// Of `chosen`, the items whose text differs from the item of the same number in `before`, or
/// that `before` does not hold; and, in order, the numbers of `before`'s items that `still`
/// (the file's every item) no longer holds and that `was_chosen` keeps.
#[must_use]
pub fn changed<'a>(
    chosen: Vec<&'a Item>,
    before: &[Item],
    still: &[Item],
    was_chosen: impl Fn(&Item) -> bool,
) -> (Vec<&'a Item>, Vec<i64>) {
    let kept = chosen
        .into_iter()
        .filter(|item| {
            !before
                .iter()
                .any(|old| old.number == item.number && old.text == item.text)
        })
        .collect();
    let mut gone: Vec<i64> = before
        .iter()
        .filter(|old| was_chosen(old) && !still.iter().any(|item| item.number == old.number))
        .map(|old| old.number)
        .collect();
    gone.dedup();
    (kept, gone)
}

/// The text of `rules.md` at `revision` of the git of `records`; empty when that revision holds
/// no `rules.md`.
///
/// # Errors
///
/// [`RulesRefusal::RevisionNotFound`] when the records' git does not resolve `revision` to a
/// commit, or git does not run.
fn rules_at(records: &Path, revision: &str) -> Result<String> {
    file_at(records, revision, RULES_FILE)
}

/// The text of `file` of the records at `revision`; empty when that revision holds none.
///
/// # Errors
///
/// As [`rules_at`].
fn file_at(records: &Path, revision: &str, file: &str) -> Result<String> {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(records)
            .args(args)
            .stdin(Stdio::null())
            .output()
    };
    let not_found = |why: &str| {
        refuse(&RulesRefused {
            refusal: RulesRefusal::RevisionNotFound,
            detail: format!(
                "the git of {} resolves no commit {revision:?}{why}",
                records.display()
            ),
        })
    };
    let commit = format!("{revision}^{{commit}}");
    match git(&[
        "rev-parse",
        "--verify",
        "--quiet",
        "--end-of-options",
        &commit,
    ]) {
        Ok(output) if output.status.success() => {}
        Ok(_) => {
            not_found("")?;
        }
        Err(error) => {
            not_found(&format!(" (git does not run: {error})"))?;
        }
    }
    let shown = git(&["show", &format!("{revision}:{file}")])
        .with_context(|| format!("{COMMAND}: run git show in {}", records.display()))?;
    if shown.status.success() {
        String::from_utf8(shown.stdout)
            .with_context(|| format!("{COMMAND}: {file} at {revision} is not UTF-8"))
    } else {
        Ok(String::new())
    }
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

/// The headings, in lower case, besides the role's name and its old name, that a role's section
/// of a `rules.md` written before the new names carries: the lead's, `conductor init` wrote it
/// `Repository controller`.
const OLD_HEADINGS: [(&str, RoleName); 2] = [
    ("repository controller", RoleName::Lead),
    ("repo-controller", RoleName::Lead),
];

/// The role `heading` names, in lower case: by the role's name, its old name, or a heading of
/// [`OLD_HEADINGS`].
fn heading_role(heading: &str) -> Option<RoleName> {
    config::role_name(heading).or_else(|| {
        OLD_HEADINGS
            .iter()
            .find(|(old, _)| *old == heading)
            .map(|(_, role)| *role)
    })
}

/// Whether the heading `heading` answers `--section wanted`, both in lower case: it reads
/// `wanted`, or, for this release, both name one role (`story:role-name-aliases`).
fn role_headings(wanted: &str, heading: &str) -> bool {
    heading == wanted
        || heading_role(wanted).is_some_and(|role| heading_role(heading) == Some(role))
}

/// The items `input` asks for, in the order it gives them.
///
/// # Errors
///
/// A word of `item` that is no number, both `item` and `section`, or the refusal the
/// specification names: [`RulesRefusal::RuleItemNotFound`] for a number no item holds,
/// [`RulesRefusal::RuleSectionNotFound`] for a heading the file does not hold, naming `sections`,
/// the file's section headings ([`sections`]).
pub fn select<'a>(items: &'a [Item], sections: &[String], input: &Rules) -> Result<Vec<&'a Item>> {
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
                        .any(|heading| role_headings(&wanted, &heading.to_lowercase()))
                })
                .collect();
            // A heading read under the role's other name is noted, once each.
            let mut noted: Vec<&str> = Vec::new();
            for heading in chosen.iter().flat_map(|item| &item.headings) {
                let lower = heading.to_lowercase();
                if lower != wanted
                    && role_headings(&wanted, &lower)
                    && !noted.contains(&heading.as_str())
                {
                    noted.push(heading);
                    let role = heading_role(&wanted).map_or("", config::role_word);
                    eprintln!(
                        "conductor: {COMMAND}: note: --section {} read the heading {heading:?}, \
                         a name of the role {role}; the old names are read for this release",
                        section.trim()
                    );
                }
            }
            if chosen.is_empty() {
                refuse(&RulesRefused {
                    refusal: RulesRefusal::RuleSectionNotFound,
                    detail: format!(
                        "no heading of {RULES_FILE} with an item under it reads {section:?}; \
                         sections: {}",
                        if sections.is_empty() {
                            "none".to_owned()
                        } else {
                            sections.join(", ")
                        }
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
        RulesRefusal::RevisionNotFound => "RevisionNotFound",
        RulesRefusal::RuleRoleNotFound => "RuleRoleNotFound",
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
        brief: args.brief,
        changed_since: args.changed_since.clone(),
        role: args.role.clone(),
    };
    if let Some(role) = input.role.as_deref() {
        return role_show(&input, role);
    }
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is not one of text, json"),
    };
    let brief_lines = input.brief == Some(true);
    if json && brief_lines {
        bail!("{COMMAND}: give --brief or --format json, not both: a json row carries its title");
    }
    let records = PathBuf::from(&config::active().instance.records);
    let file = records.join(RULES_FILE);
    let text =
        fs::read_to_string(&file).with_context(|| format!("{COMMAND}: read {}", file.display()))?;
    let items = parse(&text);
    let mut chosen = select(&items, &sections(&text), &input)?;
    if let Some(revision) = input.changed_since.as_deref() {
        let before = parse(&rules_at(&records, revision)?);
        let wanted = input
            .section
            .as_deref()
            .map(|section| section.trim().to_lowercase());
        let numbers: Option<Vec<i64>> = input.item.as_deref().map(|list| {
            list.split(',')
                .filter_map(|word| word.trim().parse().ok())
                .collect()
        });
        let was_chosen = |old: &Item| match (&numbers, &wanted) {
            (Some(numbers), _) => numbers.contains(&old.number),
            (None, Some(wanted)) => old
                .headings
                .iter()
                .any(|heading| role_headings(wanted, &heading.to_lowercase())),
            (None, None) => true,
        };
        let (kept, gone) = changed(chosen, &before, &items, was_chosen);
        chosen = kept;
        if !gone.is_empty() {
            let numbers: Vec<String> = gone.iter().map(i64::to_string).collect();
            eprintln!(
                "conductor: {COMMAND}: note: {RULES_FILE} no longer holds, since {revision}: {}",
                numbers.join(", ")
            );
        }
    }

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
    } else if brief_lines {
        chosen
            .iter()
            .map(|item| format!("{}\n", brief(item)))
            .collect()
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

/// `conductor rules show --role <role>` (`story:rules-into-code`): the rules of the role's file,
/// `rules/<role>.md` of the records, one line each; with `--changed-since`, those that file at
/// that revision does not hold.
///
/// # Errors
///
/// `role` is no role with a rule file, it is given with `--item`, `--section`, `--brief` or
/// `--format json`, its file is not there ([`RulesRefusal::RuleRoleNotFound`]), the revision does
/// not resolve ([`RulesRefusal::RevisionNotFound`]), or standard output cannot be written.
fn role_show(input: &Rules, role: &str) -> Result<ExitCode> {
    if !ROLE_FILES.contains(&role) {
        bail!(
            "{COMMAND}: --role {role:?} is no role with a rule file: {}",
            ROLE_FILES.join(", ")
        );
    }
    if input.item.is_some()
        || input.section.is_some()
        || input.brief.is_some()
        || input
            .format
            .as_deref()
            .is_some_and(|format| format != "text")
    {
        bail!(
            "{COMMAND}: --role reads one role file: give no --item, --section, --brief or --format"
        );
    }
    let records = PathBuf::from(&config::active().instance.records);
    let name = format!("{RULES_DIR}/{role}.md");
    let file = records.join(&name);
    let Ok(text) = fs::read_to_string(&file) else {
        return refuse(&RulesRefused {
            refusal: RulesRefusal::RuleRoleNotFound,
            detail: format!("the records hold no {name} ({})", file.display()),
        })
        .map(|()| ExitCode::FAILURE);
    };
    let held: Vec<&str> = text.lines().filter(|line| opens_rule(line)).collect();
    let before = match input.changed_since.as_deref() {
        Some(revision) => Some(file_at(&records, revision, &name)?),
        None => None,
    };
    let printed: String = held
        .iter()
        .filter(|rule| {
            before
                .as_deref()
                .is_none_or(|old| !old.lines().any(|line| line == **rule))
        })
        .map(|rule| format!("{rule}\n"))
        .collect();
    io::stdout()
        .lock()
        .write_all(printed.as_bytes())
        .with_context(|| format!("{COMMAND}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}
