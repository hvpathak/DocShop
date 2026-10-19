//! App modes: everything together, or just one app. A parent can lock a mode with
//! a PIN so a child can only use, say, the Writer. The lock keeps the app in one
//! mode; it is not a password for files.

use crate::commands::{Command, CommandSpec, TabKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Mode {
    /// Notepad, Writer and PDF together.
    #[default]
    Studio,
    Notepad,
    Writer,
    Pdf,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Studio, Mode::Notepad, Mode::Writer, Mode::Pdf];

    pub fn label(self) -> &'static str {
        match self {
            Mode::Studio => "All together (Studio)",
            Mode::Notepad => "Notepad only",
            Mode::Writer => "Writer only",
            Mode::Pdf => "PDF only",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Mode::Studio => "Studio",
            Mode::Notepad => "Notepad",
            Mode::Writer => "Writer",
            Mode::Pdf => "PDF",
        }
    }

    /// PDF tabs arrive in the next phase; until then that mode can't be chosen.
    pub fn is_available(self) -> bool {
        self != Mode::Pdf
    }

    pub fn allows(self, kind: TabKind) -> bool {
        match self {
            Mode::Studio => true,
            Mode::Notepad => kind == TabKind::Text,
            Mode::Writer => kind == TabKind::Doc,
            Mode::Pdf => false,
        }
    }

    /// The kind new tabs and opened files become in this mode.
    pub fn home_kind(self) -> TabKind {
        match self {
            Mode::Writer => TabKind::Doc,
            _ => TabKind::Text,
        }
    }

    /// Whether a command is offered at all in this mode (menus, palette, keys).
    /// In a single-app mode, "New" (Ctrl+N) makes that app's kind of tab, and the
    /// commands that create or convert to another kind disappear.
    pub fn allows_command(self, spec: &CommandSpec) -> bool {
        match spec.command {
            Command::NewText => true,
            Command::NewDocument | Command::ConvertToDocument | Command::ConvertToText => {
                self == Mode::Studio
            }
            _ => {
                (spec.kinds.text && self.allows(TabKind::Text))
                    || (spec.kinds.doc && self.allows(TabKind::Doc))
            }
        }
    }
}

/// The growing path: the app grows up with a child. Chapter 1 is the Notepad; a
/// parent unlocks the Writer (chapter 2) and then PDF (chapter 3). Every chapter
/// keeps what came before. Written for Harshal.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Chapter {
    Notepad,
    Writer,
    Pdf,
}

impl Chapter {
    pub const ALL: [Chapter; 3] = [Chapter::Notepad, Chapter::Writer, Chapter::Pdf];

    pub fn number(self) -> u8 {
        match self {
            Chapter::Notepad => 1,
            Chapter::Writer => 2,
            Chapter::Pdf => 3,
        }
    }

    /// The app this chapter opens.
    pub fn app(self) -> &'static str {
        match self {
            Chapter::Notepad => "Notepad",
            Chapter::Writer => "Writer",
            Chapter::Pdf => "PDF",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Chapter::Notepad => "First words",
            Chapter::Writer => "Stories with style",
            Chapter::Pdf => "Real papers",
        }
    }

    /// What the chapter card says when it opens.
    pub fn story(self) -> &'static str {
        match self {
            Chapter::Notepad => {
                "Every writer starts with a blank page. Type anything: a list, a joke, a plan for tomorrow. Save it, open it again, find a word, change it. Plain words are where every story begins."
            }
            Chapter::Writer => {
                "Your words can now wear clothes. Make a title, a heading, a bold word, a colour, a list. Write a story, a school report, a letter. Your notepad is still here, right next to it."
            }
            Chapter::Pdf => {
                "The grown-up papers: forms, books, certificates. Read them, find things in them, put pages in order, sign your name. Everything you learned before comes with you."
            }
        }
    }

    /// Which mode the chapter behaves like.
    pub fn mode(self) -> Mode {
        match self {
            Chapter::Notepad => Mode::Notepad,
            Chapter::Writer | Chapter::Pdf => Mode::Studio,
        }
    }

    /// Kinds of tab this chapter has unlocked (always cumulative).
    pub fn allows(self, kind: TabKind) -> bool {
        match kind {
            TabKind::Text => true,
            TabKind::Doc => self >= Chapter::Writer,
        }
    }

    pub fn offers_pdf(self) -> bool {
        self >= Chapter::Pdf
    }

    pub fn next(self) -> Option<Chapter> {
        match self {
            Chapter::Notepad => Some(Chapter::Writer),
            Chapter::Writer => Some(Chapter::Pdf),
            Chapter::Pdf => None,
        }
    }
}

/// A stable, salted FNV-1a hash of a PIN, so the PIN itself is never stored.
pub fn hash_pin(pin: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in "harshpad-mode-lock:".bytes().chain(pin.trim().bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// PINs are 4 to 8 digits.
pub fn valid_pin(pin: &str) -> bool {
    let pin = pin.trim();
    (4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{CATALOG, by_id};

    #[test]
    fn single_modes_only_allow_their_kind() {
        assert!(Mode::Studio.allows(TabKind::Text) && Mode::Studio.allows(TabKind::Doc));
        assert!(Mode::Notepad.allows(TabKind::Text) && !Mode::Notepad.allows(TabKind::Doc));
        assert!(Mode::Writer.allows(TabKind::Doc) && !Mode::Writer.allows(TabKind::Text));
        assert_eq!(Mode::Writer.home_kind(), TabKind::Doc);
        assert!(!Mode::Pdf.is_available());
    }

    #[test]
    fn commands_follow_the_mode() {
        let cmd = |id: &str| by_id(id).unwrap();
        // Writer: formatting yes, word wrap and plain-text tabs no.
        assert!(Mode::Writer.allows_command(cmd("format.bold")));
        assert!(!Mode::Writer.allows_command(cmd("format.wordWrap")));
        assert!(Mode::Writer.allows_command(cmd("file.newText"))); // "New" makes a document
        assert!(!Mode::Writer.allows_command(cmd("file.newDocument")));
        assert!(!Mode::Writer.allows_command(cmd("file.convertToText")));
        assert!(Mode::Writer.allows_command(cmd("view.modeStudio"))); // always able to switch back
        // Notepad: the reverse.
        assert!(!Mode::Notepad.allows_command(cmd("format.bold")));
        assert!(!Mode::Notepad.allows_command(cmd("file.newDocument")));
        // Shared commands stay everywhere, and Studio offers everything.
        for mode in [Mode::Notepad, Mode::Writer] {
            assert!(mode.allows_command(cmd("edit.find")));
            assert!(mode.allows_command(cmd("file.save")));
        }
        assert!(CATALOG.iter().all(|s| Mode::Studio.allows_command(s)));
    }

    #[test]
    fn chapters_unlock_one_by_one_and_keep_what_came_before() {
        let mut chapter = Chapter::Notepad;
        assert!(chapter.allows(TabKind::Text) && !chapter.allows(TabKind::Doc));
        assert!(!chapter.offers_pdf());
        assert_eq!(chapter.mode(), Mode::Notepad);
        let mut seen = vec![chapter.number()];
        while let Some(next) = chapter.next() {
            assert!(next > chapter);
            chapter = next;
            seen.push(chapter.number());
            // Nothing learned earlier is ever taken away.
            assert!(chapter.allows(TabKind::Text));
        }
        assert_eq!(seen, [1, 2, 3]);
        assert!(Chapter::Writer.allows(TabKind::Doc) && !Chapter::Writer.offers_pdf());
        assert!(Chapter::Pdf.allows(TabKind::Doc) && Chapter::Pdf.offers_pdf());
        assert!(
            Chapter::ALL
                .iter()
                .all(|c| !c.story().is_empty() && !c.title().is_empty())
        );
    }

    #[test]
    fn pins_hash_stably_and_are_validated() {
        assert_eq!(hash_pin("1234"), hash_pin(" 1234 "));
        assert_ne!(hash_pin("1234"), hash_pin("1235"));
        assert!(valid_pin("0042") && valid_pin("12345678"));
        assert!(!valid_pin("123") && !valid_pin("12a4") && !valid_pin("123456789"));
    }
}
