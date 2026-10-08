//! Just enough of the shell's grammar to see what a Bash call runs: its simple commands, each as
//! its words after quote removal, in order.
//!
//! The guard reads a command line the session wrote, not one it runs, so where the grammar is
//! ambiguous this module errs towards finding a command rather than missing one:
//! - `;`, `&`, `|`, `&&`, `||`, a newline, `(` and `)` end a simple command;
//! - a command substitution, `$(…)` or `` `…` ``, unquoted or inside double quotes, is read as
//!   commands of its own, ahead of the command that holds it;
//! - the script of `sh -c`, `bash -c` (any shell, any flag cluster ending in `c`) and the
//!   arguments of `eval` are read as commands of their own, after the command that holds them;
//! - a here-document's body and a comment are not commands, and are dropped.
//!
//! Redirection operators are dropped and their targets kept as words. Words are not expanded:
//! `~`, `$HOME` and globs reach the caller as written.
//!
//! A redirection is read only outside quotes, so a `>` inside a quoted program (`awk 'NR>=4'`) or
//! string is none. The files a command's redirections write ([`written`]) are the targets of `>`,
//! `>>`, `>|`, `<>`, `&>`, `&>>` and `>&<word>`; a descriptor duplication or close (`2>&1`, `>&2`,
//! `<&-`) writes no file, and neither do [`NOT_FILES`] (`/dev/null`). An output operator with no
//! word after it is read as a write to an unknown file, the empty string.

use std::collections::VecDeque;
use std::mem;

/// How deep substitutions, `sh -c` scripts and `eval` arguments are followed.
const DEPTH: usize = 8;

/// The shells whose `-c` script is read as commands.
const SHELLS: [&str; 6] = ["sh", "bash", "zsh", "dash", "ksh", "fish"];

/// Redirection targets that are no file: writing them changes nothing on disk.
const NOT_FILES: [&str; 3] = ["/dev/null", "/dev/stdout", "/dev/stderr"];

/// One simple command: its words after quote removal, and the targets its output redirections
/// write, as words.
struct Simple {
    words: Vec<String>,
    writes: Vec<String>,
}

/// The simple commands of `script`, in order, each as its words after quote removal.
pub(super) fn commands(script: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    collect(script, DEPTH, &mut out);
    out.into_iter().map(|simple| simple.words).collect()
}

/// The files the simple commands of `script` ([`commands`]) write by redirection, in order, each
/// as its target word after quote removal: descriptor duplications and [`NOT_FILES`] left out.
pub(super) fn written(script: &str) -> Vec<String> {
    let mut out = Vec::new();
    collect(script, DEPTH, &mut out);
    out.into_iter()
        .flat_map(|simple| simple.writes)
        .filter(|target| !NOT_FILES.contains(&target.as_str()))
        .collect()
}

/// The last path component of `word`: the program a command word names.
pub(super) fn program(word: &str) -> &str {
    word.rsplit('/').next().unwrap_or(word)
}

fn collect(script: &str, depth: usize, out: &mut Vec<Simple>) {
    for (nested, simple) in Lexer::new(script).run() {
        if depth > 0 {
            for inner in &nested {
                collect(inner, depth - 1, out);
            }
        }
        let scripts = if depth > 0 {
            argument_scripts(&simple.words)
        } else {
            Vec::new()
        };
        out.push(simple);
        for inner in scripts {
            collect(&inner, depth - 1, out);
        }
    }
}

/// The scripts `words` hands to a shell: the argument of `-c` after a shell, and the arguments of
/// `eval`.
fn argument_scripts(words: &[String]) -> Vec<String> {
    let mut scripts = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let name = program(word);
        if SHELLS.contains(&name) {
            let flags = words[at + 1..].iter().position(|flag| {
                flag.starts_with('-') && !flag.starts_with("--") && flag.ends_with('c')
            });
            if let Some(flag) = flags
                && let Some(script) = words.get(at + 1 + flag + 1)
            {
                scripts.push(script.clone());
            }
        } else if name == "eval" && at + 1 < words.len() {
            scripts.push(words[at + 1..].join(" "));
        }
    }
    scripts
}

/// One pass over a script: each simple command with the substitutions found inside it.
struct Lexer {
    chars: Vec<char>,
    at: usize,
    word: String,
    in_word: bool,
    words: Vec<String>,
    nested: Vec<String>,
    /// The targets the current command's output redirections write.
    writes: Vec<String>,
    /// After a redirection operator: whether the word that follows is a target it writes.
    target: Option<bool>,
    commands: Vec<(Vec<String>, Simple)>,
    /// Here-documents opened on the current line: delimiter, and whether leading tabs are
    /// stripped (`<<-`).
    heredocs: VecDeque<(String, bool)>,
}

impl Lexer {
    fn new(script: &str) -> Self {
        Self {
            chars: script.chars().collect(),
            at: 0,
            word: String::new(),
            in_word: false,
            words: Vec::new(),
            nested: Vec::new(),
            writes: Vec::new(),
            target: None,
            commands: Vec::new(),
            heredocs: VecDeque::new(),
        }
    }

    fn peek(&self, ahead: usize) -> Option<char> {
        self.chars.get(self.at + ahead).copied()
    }

    fn push(&mut self, c: char) {
        self.word.push(c);
        self.in_word = true;
    }

    fn end_word(&mut self) {
        if self.in_word {
            let word = mem::take(&mut self.word);
            if self.target.take() == Some(true) {
                self.writes.push(word.clone());
            }
            self.words.push(word);
            self.in_word = false;
        }
    }

    fn end_command(&mut self) {
        self.end_word();
        // An output operator with no word after it still writes, somewhere.
        if self.target.take() == Some(true) {
            self.writes.push(String::new());
        }
        if !self.words.is_empty() || !self.nested.is_empty() || !self.writes.is_empty() {
            let nested = mem::take(&mut self.nested);
            let words = mem::take(&mut self.words);
            let writes = mem::take(&mut self.writes);
            self.commands.push((nested, Simple { words, writes }));
        }
    }

    /// Each simple command as `(substitutions inside it, the command)`.
    fn run(mut self) -> Vec<(Vec<String>, Simple)> {
        while let Some(c) = self.peek(0) {
            match c {
                '\\' => {
                    match self.peek(1) {
                        Some('\n') => {}
                        Some(next) => self.push(next),
                        None => {}
                    }
                    self.at += 2;
                }
                '\'' => {
                    self.at += 1;
                    self.in_word = true;
                    while let Some(c) = self.peek(0) {
                        self.at += 1;
                        if c == '\'' {
                            break;
                        }
                        self.word.push(c);
                    }
                }
                '"' => {
                    self.at += 1;
                    self.in_word = true;
                    self.double_quoted();
                }
                '$' if self.peek(1) == Some('\'') => {
                    self.at += 2;
                    self.in_word = true;
                    while let Some(c) = self.peek(0) {
                        self.at += 1;
                        match c {
                            '\\' => {
                                if let Some(next) = self.peek(0) {
                                    self.word.push(next);
                                    self.at += 1;
                                }
                            }
                            '\'' => break,
                            _ => self.word.push(c),
                        }
                    }
                }
                '$' if self.peek(1) == Some('(') => self.substitution(),
                '`' => self.backquoted(),
                '#' if !self.in_word => {
                    while let Some(c) = self.peek(0) {
                        if c == '\n' {
                            break;
                        }
                        self.at += 1;
                    }
                }
                ' ' | '\t' | '\r' => {
                    self.end_word();
                    self.at += 1;
                }
                '\n' => {
                    self.end_command();
                    self.at += 1;
                    self.heredoc_bodies();
                }
                '&' if self.peek(1) == Some('>') => {
                    self.end_word();
                    self.at += 2;
                    if self.peek(0) == Some('>') {
                        self.at += 1;
                    }
                    self.target = Some(true);
                }
                ';' | '&' | '|' | '(' | ')' => {
                    self.end_command();
                    self.at += 1;
                }
                '<' if self.peek(1) == Some('<') && self.peek(2) != Some('<') => {
                    self.end_word();
                    self.heredoc_operator();
                }
                '<' | '>' => {
                    // A file descriptor number written right before the operator is not a word.
                    if self.in_word && self.word.chars().all(|c| c.is_ascii_digit()) {
                        self.word.clear();
                        self.in_word = false;
                    }
                    self.end_word();
                    self.at += 1;
                    let mut output = c == '>';
                    let mut duplicates = false;
                    while let Some(next) = self.peek(0) {
                        if matches!(next, '<' | '>' | '|') {
                            output |= next == '>';
                            self.at += 1;
                        } else if next == '&' {
                            // `>&2`, `<&-`: a descriptor, not a separator, and no file; `>&file`
                            // writes the file.
                            self.at += 1;
                            while let Some(d) = self.peek(0) {
                                if d.is_ascii_digit() || d == '-' {
                                    duplicates = true;
                                    self.at += 1;
                                } else {
                                    break;
                                }
                            }
                            break;
                        } else {
                            break;
                        }
                    }
                    self.target = (!duplicates).then_some(output);
                }
                _ => {
                    self.push(c);
                    self.at += 1;
                }
            }
        }
        self.end_command();
        self.commands
    }

    /// The rest of a double-quoted string, after its opening quote.
    fn double_quoted(&mut self) {
        while let Some(c) = self.peek(0) {
            match c {
                '"' => {
                    self.at += 1;
                    return;
                }
                '\\' => {
                    match self.peek(1) {
                        Some(next @ ('$' | '`' | '"' | '\\')) => self.word.push(next),
                        Some('\n') => {}
                        Some(next) => {
                            self.word.push('\\');
                            self.word.push(next);
                        }
                        None => {}
                    }
                    self.at += 2;
                }
                '$' if self.peek(1) == Some('(') => self.substitution(),
                '`' => self.backquoted(),
                _ => {
                    self.word.push(c);
                    self.at += 1;
                }
            }
        }
    }

    /// `$(…)` or `$((…))` at the cursor. A command substitution's script is kept for reading as
    /// commands; both are kept in the word as written.
    fn substitution(&mut self) {
        let arithmetic = self.peek(2) == Some('(');
        let start = self.at + 2;
        let mut depth = 0usize;
        let mut end = self.chars.len();
        let mut quote: Option<char> = None;
        for (offset, &c) in self.chars[start..].iter().enumerate() {
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '\'' | '"') => quote = Some(c),
                (None, '(') => depth += 1,
                (None, ')') if depth == 0 => {
                    end = start + offset;
                    break;
                }
                (None, ')') => depth -= 1,
                _ => {}
            }
        }
        let inner: String = self.chars[start..end].iter().collect();
        if !arithmetic {
            self.nested.push(inner.clone());
        }
        self.word.push_str("$(");
        self.word.push_str(&inner);
        self.word.push(')');
        self.in_word = true;
        self.at = (end + 1).min(self.chars.len());
    }

    /// `` `…` `` at the cursor: its script is kept for reading as commands.
    fn backquoted(&mut self) {
        let start = self.at + 1;
        let mut end = self.chars.len();
        let mut escaped = false;
        for (offset, &c) in self.chars[start..].iter().enumerate() {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '`' {
                end = start + offset;
                break;
            }
        }
        let inner: String = self.chars[start..end].iter().collect();
        self.nested.push(inner.clone());
        self.word.push('`');
        self.word.push_str(&inner);
        self.word.push('`');
        self.in_word = true;
        self.at = (end + 1).min(self.chars.len());
    }

    /// `<<` or `<<-` at the cursor, then its delimiter word: the body after this line is not
    /// commands.
    fn heredoc_operator(&mut self) {
        self.at += 2;
        let strip_tabs = self.peek(0) == Some('-');
        if strip_tabs {
            self.at += 1;
        }
        while matches!(self.peek(0), Some(' ' | '\t')) {
            self.at += 1;
        }
        let mut delimiter = String::new();
        let mut quote: Option<char> = None;
        while let Some(c) = self.peek(0) {
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), c) => delimiter.push(c),
                (None, '\'' | '"') => quote = Some(c),
                (None, '\\') => {}
                (None, ' ' | '\t' | '\n' | ';' | '&' | '|' | '<' | '>' | '(' | ')') => break,
                (None, c) => delimiter.push(c),
            }
            self.at += 1;
        }
        if !delimiter.is_empty() {
            self.heredocs.push_back((delimiter, strip_tabs));
        }
    }

    /// Skips the bodies of the here-documents opened on the line that just ended.
    fn heredoc_bodies(&mut self) {
        while let Some((delimiter, strip_tabs)) = self.heredocs.pop_front() {
            loop {
                if self.at >= self.chars.len() {
                    self.heredocs.clear();
                    return;
                }
                let end = self.chars[self.at..]
                    .iter()
                    .position(|&c| c == '\n')
                    .map_or(self.chars.len(), |offset| self.at + offset);
                let line: String = self.chars[self.at..end].iter().collect();
                self.at = (end + 1).min(self.chars.len());
                let line = if strip_tabs {
                    line.trim_start_matches('\t')
                } else {
                    line.as_str()
                };
                if line == delimiter {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{commands, written};

    #[test]
    fn a_redirection_writes_a_file_only_outside_quotes() {
        assert_eq!(
            written(
                "ls x 2>&1 >&2 1>&2 <&- >/dev/null 2>/dev/null &>/dev/null < in <<< w; \
                 awk 'NR>=4' f; echo \"a > b\" '>>' >/dev/stderr"
            ),
            Vec::<String>::new()
        );
        assert_eq!(
            written(
                "echo a > f1; echo b >> \"f 2\" 2>&1; echo c &>f3; echo d >|f4; echo e >&f5; \
                 echo \"$(echo g 2>f6)\"; bash -c 'echo h <> f7'; cat <<EOF >f8\nx > y\nEOF\n\
                 echo i &>> f9; echo j >"
            ),
            ["f1", "f 2", "f3", "f4", "f5", "f6", "f7", "f8", "f9", ""]
        );
    }

    fn words(script: &str) -> Vec<Vec<String>> {
        commands(script)
    }

    #[test]
    fn separators_end_commands_and_quotes_are_removed() {
        assert_eq!(
            words("cd 'a b' && git -C \"c d\" status; ls|wc -l"),
            [
                vec!["cd", "a b"],
                vec!["git", "-C", "c d", "status"],
                vec!["ls"],
                vec!["wc", "-l"],
            ]
        );
    }

    #[test]
    fn substitutions_and_shell_scripts_are_read_as_commands() {
        assert_eq!(
            words("echo \"$(cd x; pwd)\" `git -C y log`"),
            [
                vec!["cd", "x"],
                vec!["pwd"],
                vec!["git", "-C", "y", "log"],
                vec!["echo", "$(cd x; pwd)", "`git -C y log`"],
            ]
        );
        assert_eq!(
            words("bash -lc 'gh pr create' && eval cd z"),
            [
                vec!["bash", "-lc", "gh pr create"],
                vec!["gh", "pr", "create"],
                vec!["eval", "cd", "z"],
                vec!["cd", "z"],
            ]
        );
    }

    #[test]
    fn heredoc_bodies_comments_and_redirections_are_not_commands() {
        assert_eq!(
            words("git commit -F - <<'MSG'\ngh pr create\nMSG\ngit log 2>&1 >out # gh pr merge"),
            [vec!["git", "commit", "-F", "-"], vec!["git", "log", "out"],]
        );
        assert_eq!(
            words("cat <<-EOF >x\n\tcd y\n\tEOF\nls"),
            [vec!["cat", "x"], vec!["ls"]]
        );
    }
}
