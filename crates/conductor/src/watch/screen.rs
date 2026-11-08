//! What a session's typescript says of its pane (`story:watch-unblocks-merged-requests`,
//! acceptance 4): whether it stands at the prompt, and since when.
//!
//! The typescript, `<state>/sessions/<session name>.typescript`, is the copy of everything the
//! pane printed ([`crate::confine::systemd`]). Its last [`TAIL`] bytes are read with the terminal's
//! escape sequences taken out. The harness redraws its input box, whose line starts with the
//! prompt [`PROMPT`], below everything else whenever a turn ends; while a turn runs it redraws its
//! status above the box in place, which writes a mark of [`RUNNING`] after the box. So the pane is
//! at the prompt when the text after the last [`PROMPT`] holds no mark of [`RUNNING`], and the turn
//! ended when the typescript was last written, its modification time.

use std::fs::File;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::Path;
use std::time::SystemTime;

/// The bytes of a typescript read, from its end.
pub const TAIL: u64 = 16 * 1024;

/// The prompt of the harness's input box.
pub const PROMPT: char = '\u{276f}';

/// The marks of a turn that runs: the ellipsis of its status, its interrupt hint, and the
/// spinner's glyphs.
pub const RUNNING: [&str; 7] = [
    "\u{2026}",
    "to interrupt",
    "\u{2722}",
    "\u{2733}",
    "\u{2736}",
    "\u{273b}",
    "\u{273d}",
];

/// What a typescript says of its pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// Whether it stands at the prompt.
    pub at_prompt: bool,
    /// When the typescript was last written.
    pub written: SystemTime,
}

/// What the typescript `path` says; `None` when it cannot be read.
#[must_use]
pub fn read(path: &Path) -> Option<Screen> {
    let mut file = File::open(path).ok()?;
    let meta = file.metadata().ok()?;
    let written = meta.modified().ok()?;
    let start = meta.len().saturating_sub(TAIL);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    let text = plain(&String::from_utf8_lossy(&bytes)).to_lowercase();
    let at_prompt = text
        .rfind(PROMPT)
        .is_some_and(|at| !RUNNING.iter().any(|mark| text[at..].contains(mark)));
    Some(Screen { at_prompt, written })
}

/// `text` without its escape sequences: CSI (`ESC [` … a final byte), OSC (`ESC ]` … BEL or
/// `ESC \`), a character set's (`ESC (` and one character), and every other `ESC` with the
/// character after it.
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for c in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            }
            Some(']') => {
                while let Some(c) = chars.next() {
                    if c == '\u{7}' {
                        break;
                    }
                    if c == '\u{1b}' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            Some('(' | ')' | '*' | '+') => {
                chars.next();
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::plain;

    #[test]
    fn escape_sequences_are_taken_out() {
        assert_eq!(
            plain("\u{1b}[2K\u{1b}[38;5;174ma\u{1b}[39m b\u{1b}]0;title\u{7}c\u{1b}(Bd"),
            "a bcd"
        );
    }
}
