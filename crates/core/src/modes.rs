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
    fn pins_hash_stably_and_are_validated() {
        assert_eq!(hash_pin("1234"), hash_pin(" 1234 "));
        assert_ne!(hash_pin("1234"), hash_pin("1235"));
        assert!(valid_pin("0042") && valid_pin("12345678"));
        assert!(!valid_pin("123") && !valid_pin("12a4") && !valid_pin("123456789"));
    }
}
