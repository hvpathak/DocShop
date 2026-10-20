//! The command catalog: every action in HarshPad Studio, with its label, menu,
//! shortcut and the kinds of tab it works in. The menu bar, keyboard shortcuts,
//! command palette, CLI and tests all read this one list, so the same key always
//! does the same thing in every tab, and no key means two things.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    // File
    NewTab,
    ReopenClosedTab,
    NewText,
    NewDocument,
    Open,
    Save,
    SaveAs,
    ConvertToDocument,
    ConvertToText,
    CloseTab,
    Exit,
    // Edit
    Undo,
    Redo,
    Find,
    FindNext,
    FindPrevious,
    Replace,
    ReplaceNext,
    ReplaceAll,
    CommandPalette,
    // Format: documents
    StyleNormal,
    StyleTitle,
    StyleHeading1,
    StyleHeading2,
    StyleHeading3,
    StyleQuote,
    StyleCode,
    BulletList,
    NumberedList,
    Bold,
    Italic,
    Underline,
    Strike,
    InlineCode,
    BiggerText,
    SmallerText,
    ClearFormatting,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    InsertLink,
    // Format: plain text
    WordWrap,
    // View
    NextTab,
    PreviousTab,
    Tab1,
    Tab2,
    Tab3,
    Tab4,
    Tab5,
    Tab6,
    Tab7,
    Tab8,
    LastTab,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    StatusBar,
    ModeStudio,
    ModeNotepad,
    ModeWriter,
    ModePdf,
    ParentLock,
    StartPath,
    UnlockNextChapter,
    LeavePath,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menu {
    File,
    Edit,
    Format,
    View,
    Help,
}

impl Menu {
    pub const ALL: [Menu; 5] = [Menu::File, Menu::Edit, Menu::Format, Menu::View, Menu::Help];

    pub fn label(self) -> &'static str {
        match self {
            Menu::File => "File",
            Menu::Edit => "Edit",
            Menu::Format => "Format",
            Menu::View => "View",
            Menu::Help => "Help",
        }
    }
}

/// The kinds of tab a command works in. Elsewhere it is greyed out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kinds {
    pub text: bool,
    pub doc: bool,
}

impl Kinds {
    pub const ALL: Kinds = Kinds {
        text: true,
        doc: true,
    };
    pub const TEXT: Kinds = Kinds {
        text: true,
        doc: false,
    };
    pub const DOC: Kinds = Kinds {
        text: false,
        doc: true,
    };
}

/// The kind of the active tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabKind {
    Text,
    Doc,
    /// A PDF Studio tab (PdfCraft).
    Pdf,
}

/// Keys used by the catalog. The UI maps these to its toolkit's keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    B,
    E,
    F,
    H,
    I,
    J,
    K,
    L,
    N,
    O,
    P,
    R,
    S,
    T,
    U,
    W,
    X,
    Y,
    Z,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    F3,
    Plus,
    Minus,
    OpenBracket,
    CloseBracket,
    Space,
    Tab,
}

impl Key {
    fn label(self) -> &'static str {
        match self {
            Key::B => "B",
            Key::E => "E",
            Key::F => "F",
            Key::H => "H",
            Key::I => "I",
            Key::J => "J",
            Key::K => "K",
            Key::L => "L",
            Key::N => "N",
            Key::O => "O",
            Key::P => "P",
            Key::R => "R",
            Key::S => "S",
            Key::T => "T",
            Key::U => "U",
            Key::W => "W",
            Key::X => "X",
            Key::Y => "Y",
            Key::Z => "Z",
            Key::Num0 => "0",
            Key::Num1 => "1",
            Key::Num2 => "2",
            Key::Num3 => "3",
            Key::Num4 => "4",
            Key::Num5 => "5",
            Key::Num6 => "6",
            Key::Num7 => "7",
            Key::Num8 => "8",
            Key::Num9 => "9",
            Key::F3 => "F3",
            Key::Plus => "Plus",
            Key::Minus => "Minus",
            Key::OpenBracket => "[",
            Key::CloseBracket => "]",
            Key::Space => "Space",
            Key::Tab => "Tab",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub key: Key,
}

impl Shortcut {
    const fn key(key: Key) -> Self {
        Self {
            ctrl: false,
            shift: false,
            alt: false,
            key,
        }
    }
    const fn ctrl(key: Key) -> Self {
        Self {
            ctrl: true,
            shift: false,
            alt: false,
            key,
        }
    }
    const fn ctrl_shift(key: Key) -> Self {
        Self {
            ctrl: true,
            shift: true,
            alt: false,
            key,
        }
    }
    const fn ctrl_alt(key: Key) -> Self {
        Self {
            ctrl: true,
            shift: false,
            alt: true,
            key,
        }
    }
    const fn shift(key: Key) -> Self {
        Self {
            ctrl: false,
            shift: true,
            alt: false,
            key,
        }
    }

    /// More modifiers first, so Ctrl+S never swallows Ctrl+Shift+S.
    pub fn modifier_count(self) -> u8 {
        u8::from(self.ctrl) + u8::from(self.shift) + u8::from(self.alt)
    }

    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(self.key.label());
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CommandSpec {
    pub command: Command,
    /// Stable id for the CLI, logs and agents (`area.action`).
    pub id: &'static str,
    pub label: &'static str,
    /// `None` for commands reached only from a toolbar, the Find bar or a shortcut.
    pub menu: Option<Menu>,
    pub shortcut: Option<Shortcut>,
    pub kinds: Kinds,
    /// Shown with a check mark (on/off setting).
    pub toggle: bool,
    pub separator_before: bool,
}

impl CommandSpec {
    pub fn works_in(&self, kind: TabKind) -> bool {
        match kind {
            TabKind::Text => self.kinds.text,
            TabKind::Doc => self.kinds.doc,
            // PDF Studio handles its own editing; only commands that work everywhere apply.
            TabKind::Pdf => self.kinds.text && self.kinds.doc,
        }
    }
}

const fn spec(
    command: Command,
    id: &'static str,
    label: &'static str,
    menu: Option<Menu>,
    shortcut: Option<Shortcut>,
    kinds: Kinds,
) -> CommandSpec {
    CommandSpec {
        command,
        id,
        label,
        menu,
        shortcut,
        kinds,
        toggle: false,
        separator_before: false,
    }
}

const fn toggle(mut s: CommandSpec) -> CommandSpec {
    s.toggle = true;
    s
}

const fn separated(mut s: CommandSpec) -> CommandSpec {
    s.separator_before = true;
    s
}

use Command as C;
use Key as K;
use Kinds as KS;
use Menu as M;
use Shortcut as S;

/// In menu order.
pub const CATALOG: &[CommandSpec] = &[
    spec(
        C::NewTab,
        "file.newTab",
        "New tab…",
        Some(M::File),
        Some(S::ctrl(K::T)),
        KS::ALL,
    ),
    spec(
        C::ReopenClosedTab,
        "file.reopenClosedTab",
        "Reopen closed tab",
        Some(M::File),
        Some(S::ctrl_shift(K::T)),
        KS::ALL,
    ),
    spec(
        C::NewText,
        "file.newText",
        "New text file",
        Some(M::File),
        Some(S::ctrl(K::N)),
        KS::ALL,
    ),
    spec(
        C::NewDocument,
        "file.newDocument",
        "New document",
        Some(M::File),
        Some(S::ctrl_shift(K::N)),
        KS::ALL,
    ),
    spec(
        C::Open,
        "file.open",
        "Open…",
        Some(M::File),
        Some(S::ctrl(K::O)),
        KS::ALL,
    ),
    spec(
        C::Save,
        "file.save",
        "Save",
        Some(M::File),
        Some(S::ctrl(K::S)),
        KS::ALL,
    ),
    spec(
        C::SaveAs,
        "file.saveAs",
        "Save As…",
        Some(M::File),
        Some(S::ctrl_shift(K::S)),
        KS::ALL,
    ),
    separated(spec(
        C::ConvertToDocument,
        "file.convertToDocument",
        "Open as document",
        Some(M::File),
        None,
        KS::TEXT,
    )),
    spec(
        C::ConvertToText,
        "file.convertToText",
        "Open as plain text (Markdown)",
        Some(M::File),
        None,
        KS::DOC,
    ),
    separated(spec(
        C::CloseTab,
        "file.closeTab",
        "Close tab",
        Some(M::File),
        Some(S::ctrl(K::W)),
        KS::ALL,
    )),
    spec(C::Exit, "file.exit", "Exit", Some(M::File), None, KS::ALL),
    spec(
        C::Undo,
        "edit.undo",
        "Undo",
        Some(M::Edit),
        Some(S::ctrl(K::Z)),
        KS::ALL,
    ),
    spec(
        C::Redo,
        "edit.redo",
        "Redo",
        Some(M::Edit),
        Some(S::ctrl(K::Y)),
        KS::ALL,
    ),
    separated(spec(
        C::Find,
        "edit.find",
        "Find…",
        Some(M::Edit),
        Some(S::ctrl(K::F)),
        KS::ALL,
    )),
    spec(
        C::FindNext,
        "edit.findNext",
        "Find Next",
        Some(M::Edit),
        Some(S::key(K::F3)),
        KS::ALL,
    ),
    spec(
        C::FindPrevious,
        "edit.findPrevious",
        "Find Previous",
        Some(M::Edit),
        Some(S::shift(K::F3)),
        KS::ALL,
    ),
    spec(
        C::Replace,
        "edit.replace",
        "Replace…",
        Some(M::Edit),
        Some(S::ctrl(K::H)),
        KS::ALL,
    ),
    spec(
        C::ReplaceNext,
        "edit.replaceNext",
        "Replace",
        None,
        None,
        KS::ALL,
    ),
    spec(
        C::ReplaceAll,
        "edit.replaceAll",
        "Replace All",
        None,
        None,
        KS::ALL,
    ),
    separated(spec(
        C::CommandPalette,
        "edit.commandPalette",
        "Command palette…",
        Some(M::Edit),
        Some(S::ctrl_shift(K::P)),
        KS::ALL,
    )),
    spec(
        C::StyleNormal,
        "format.normal",
        "Normal text",
        Some(M::Format),
        Some(S::ctrl_alt(K::Num0)),
        KS::DOC,
    ),
    spec(
        C::StyleTitle,
        "format.title",
        "Title",
        Some(M::Format),
        None,
        KS::DOC,
    ),
    spec(
        C::StyleHeading1,
        "format.heading1",
        "Heading 1",
        Some(M::Format),
        Some(S::ctrl_alt(K::Num1)),
        KS::DOC,
    ),
    spec(
        C::StyleHeading2,
        "format.heading2",
        "Heading 2",
        Some(M::Format),
        Some(S::ctrl_alt(K::Num2)),
        KS::DOC,
    ),
    spec(
        C::StyleHeading3,
        "format.heading3",
        "Heading 3",
        Some(M::Format),
        Some(S::ctrl_alt(K::Num3)),
        KS::DOC,
    ),
    spec(
        C::StyleQuote,
        "format.quote",
        "Quote",
        Some(M::Format),
        None,
        KS::DOC,
    ),
    spec(
        C::StyleCode,
        "format.codeBlock",
        "Code block",
        Some(M::Format),
        None,
        KS::DOC,
    ),
    separated(spec(
        C::BulletList,
        "format.bulletList",
        "Bulleted list",
        Some(M::Format),
        Some(S::ctrl_shift(K::L)),
        KS::DOC,
    )),
    spec(
        C::NumberedList,
        "format.numberedList",
        "Numbered list",
        Some(M::Format),
        None,
        KS::DOC,
    ),
    separated(spec(
        C::Bold,
        "format.bold",
        "Bold",
        Some(M::Format),
        Some(S::ctrl(K::B)),
        KS::DOC,
    )),
    spec(
        C::Italic,
        "format.italic",
        "Italic",
        Some(M::Format),
        Some(S::ctrl(K::I)),
        KS::DOC,
    ),
    spec(
        C::Underline,
        "format.underline",
        "Underline",
        Some(M::Format),
        Some(S::ctrl(K::U)),
        KS::DOC,
    ),
    spec(
        C::Strike,
        "format.strike",
        "Strikethrough",
        Some(M::Format),
        Some(S::ctrl_shift(K::X)),
        KS::DOC,
    ),
    spec(
        C::InlineCode,
        "format.inlineCode",
        "Inline code",
        Some(M::Format),
        None,
        KS::DOC,
    ),
    spec(
        C::BiggerText,
        "format.bigger",
        "Bigger text",
        Some(M::Format),
        Some(S::ctrl(K::CloseBracket)),
        KS::DOC,
    ),
    spec(
        C::SmallerText,
        "format.smaller",
        "Smaller text",
        Some(M::Format),
        Some(S::ctrl(K::OpenBracket)),
        KS::DOC,
    ),
    spec(
        C::ClearFormatting,
        "format.clear",
        "Clear formatting",
        Some(M::Format),
        Some(S::ctrl(K::Space)),
        KS::DOC,
    ),
    separated(spec(
        C::AlignLeft,
        "format.alignLeft",
        "Align left",
        Some(M::Format),
        Some(S::ctrl(K::L)),
        KS::DOC,
    )),
    spec(
        C::AlignCenter,
        "format.alignCenter",
        "Center",
        Some(M::Format),
        Some(S::ctrl(K::E)),
        KS::DOC,
    ),
    spec(
        C::AlignRight,
        "format.alignRight",
        "Align right",
        Some(M::Format),
        Some(S::ctrl(K::R)),
        KS::DOC,
    ),
    spec(
        C::AlignJustify,
        "format.justify",
        "Justify",
        Some(M::Format),
        Some(S::ctrl(K::J)),
        KS::DOC,
    ),
    separated(spec(
        C::InsertLink,
        "format.link",
        "Link…",
        Some(M::Format),
        Some(S::ctrl(K::K)),
        KS::DOC,
    )),
    separated(toggle(spec(
        C::WordWrap,
        "format.wordWrap",
        "Word Wrap",
        Some(M::Format),
        None,
        KS::TEXT,
    ))),
    spec(
        C::NextTab,
        "view.nextTab",
        "Next tab",
        Some(M::View),
        Some(S::ctrl(K::Tab)),
        KS::ALL,
    ),
    spec(
        C::PreviousTab,
        "view.previousTab",
        "Previous tab",
        Some(M::View),
        Some(S::ctrl_shift(K::Tab)),
        KS::ALL,
    ),
    spec(
        C::Tab1,
        "view.tab1",
        "Go to tab 1",
        None,
        Some(S::ctrl(K::Num1)),
        KS::ALL,
    ),
    spec(
        C::Tab2,
        "view.tab2",
        "Go to tab 2",
        None,
        Some(S::ctrl(K::Num2)),
        KS::ALL,
    ),
    spec(
        C::Tab3,
        "view.tab3",
        "Go to tab 3",
        None,
        Some(S::ctrl(K::Num3)),
        KS::ALL,
    ),
    spec(
        C::Tab4,
        "view.tab4",
        "Go to tab 4",
        None,
        Some(S::ctrl(K::Num4)),
        KS::ALL,
    ),
    spec(
        C::Tab5,
        "view.tab5",
        "Go to tab 5",
        None,
        Some(S::ctrl(K::Num5)),
        KS::ALL,
    ),
    spec(
        C::Tab6,
        "view.tab6",
        "Go to tab 6",
        None,
        Some(S::ctrl(K::Num6)),
        KS::ALL,
    ),
    spec(
        C::Tab7,
        "view.tab7",
        "Go to tab 7",
        None,
        Some(S::ctrl(K::Num7)),
        KS::ALL,
    ),
    spec(
        C::Tab8,
        "view.tab8",
        "Go to tab 8",
        None,
        Some(S::ctrl(K::Num8)),
        KS::ALL,
    ),
    spec(
        C::LastTab,
        "view.lastTab",
        "Go to last tab",
        None,
        Some(S::ctrl(K::Num9)),
        KS::ALL,
    ),
    separated(spec(
        C::ZoomIn,
        "view.zoomIn",
        "Zoom In",
        Some(M::View),
        Some(S::ctrl(K::Plus)),
        KS::ALL,
    )),
    spec(
        C::ZoomOut,
        "view.zoomOut",
        "Zoom Out",
        Some(M::View),
        Some(S::ctrl(K::Minus)),
        KS::ALL,
    ),
    spec(
        C::ZoomReset,
        "view.zoomReset",
        "Restore Default Zoom",
        Some(M::View),
        Some(S::ctrl(K::Num0)),
        KS::ALL,
    ),
    separated(toggle(spec(
        C::StatusBar,
        "view.statusBar",
        "Status Bar",
        Some(M::View),
        None,
        KS::ALL,
    ))),
    separated(toggle(spec(
        C::ModeStudio,
        "view.modeStudio",
        "Mode: All together (DocShop)",
        Some(M::View),
        None,
        KS::ALL,
    ))),
    toggle(spec(
        C::ModeNotepad,
        "view.modeNotepad",
        "Mode: NoteShop only",
        Some(M::View),
        None,
        KS::ALL,
    )),
    toggle(spec(
        C::ModeWriter,
        "view.modeWriter",
        "Mode: WordShop only",
        Some(M::View),
        None,
        KS::ALL,
    )),
    toggle(spec(
        C::ModePdf,
        "view.modePdf",
        "Mode: PDFShop only",
        Some(M::View),
        None,
        KS::ALL,
    )),
    spec(
        C::ParentLock,
        "view.parentLock",
        "Parent lock…",
        Some(M::View),
        None,
        KS::ALL,
    ),
    spec(
        C::StartPath,
        "view.startPath",
        "Start the growing path…",
        Some(M::View),
        None,
        KS::ALL,
    ),
    spec(
        C::UnlockNextChapter,
        "view.unlockNextChapter",
        "Unlock the next chapter…",
        Some(M::View),
        None,
        KS::ALL,
    ),
    spec(
        C::LeavePath,
        "view.leavePath",
        "Leave the growing path…",
        Some(M::View),
        None,
        KS::ALL,
    ),
    spec(
        C::About,
        "help.about",
        "About DocShop",
        Some(M::Help),
        None,
        KS::ALL,
    ),
];

pub fn spec_of(command: Command) -> Option<&'static CommandSpec> {
    CATALOG.iter().find(|s| s.command == command)
}

pub fn by_id(id: &str) -> Option<&'static CommandSpec> {
    CATALOG.iter().find(|s| s.id == id)
}

pub fn in_menu(menu: Menu) -> impl Iterator<Item = &'static CommandSpec> {
    CATALOG.iter().filter(move |s| s.menu == Some(menu))
}

/// Commands whose label or id contains every word of `query` (command palette).
pub fn search(query: &str) -> Vec<&'static CommandSpec> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    CATALOG
        .iter()
        .filter(|s| {
            let hay = format!("{} {}", s.label, s.id).to_lowercase();
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn ids_commands_and_shortcuts_are_unique() {
        let ids: HashSet<_> = CATALOG.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), CATALOG.len());
        let commands: HashSet<_> = CATALOG.iter().map(|s| s.command).collect();
        assert_eq!(commands.len(), CATALOG.len());
        let shortcuts: Vec<_> = CATALOG.iter().filter_map(|s| s.shortcut).collect();
        let unique: HashSet<_> = shortcuts.iter().collect();
        assert_eq!(unique.len(), shortcuts.len(), "a shortcut means two things");
    }

    #[test]
    fn every_command_works_somewhere() {
        assert!(CATALOG.iter().all(|s| s.kinds.text || s.kinds.doc));
        assert!(
            by_id("format.bold")
                .is_some_and(|s| s.works_in(TabKind::Doc) && !s.works_in(TabKind::Text))
        );
        assert!(
            by_id("edit.find")
                .is_some_and(|s| s.works_in(TabKind::Doc) && s.works_in(TabKind::Text))
        );
    }

    #[test]
    fn labels_and_search() {
        assert_eq!(Shortcut::ctrl_alt(K::Num1).label(), "Ctrl+Alt+1");
        assert_eq!(Shortcut::ctrl_shift(K::S).label(), "Ctrl+Shift+S");
        assert!(
            search("head 2")
                .iter()
                .any(|s| s.command == C::StyleHeading2)
        );
        assert_eq!(spec_of(C::Find).map(|s| s.id), Some("edit.find"));
        assert_eq!(in_menu(M::Help).count(), 1);
    }
}
