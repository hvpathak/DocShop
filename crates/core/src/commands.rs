//! The command catalog: every user-visible action, with the label, menu and default
//! shortcut it has in Windows Notepad. The menu bar, keyboard shortcuts, CLI and
//! tests all read this one list.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    New,
    Open,
    Save,
    SaveAs,
    Exit,
    Find,
    FindNext,
    FindPrevious,
    Replace,
    ReplaceNext,
    ReplaceAll,
    WordWrap,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    StatusBar,
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

/// Keys used by the catalog. The UI maps these to its toolkit's keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    N,
    O,
    S,
    F,
    H,
    F3,
    Plus,
    Minus,
    Num0,
}

impl Key {
    fn label(self) -> &'static str {
        match self {
            Key::N => "N",
            Key::O => "O",
            Key::S => "S",
            Key::F => "F",
            Key::H => "H",
            Key::F3 => "F3",
            Key::Plus => "Plus",
            Key::Minus => "Minus",
            Key::Num0 => "0",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub ctrl: bool,
    pub shift: bool,
    pub key: Key,
}

impl Shortcut {
    const fn key(key: Key) -> Self {
        Self {
            ctrl: false,
            shift: false,
            key,
        }
    }
    const fn ctrl(key: Key) -> Self {
        Self {
            ctrl: true,
            shift: false,
            key,
        }
    }
    const fn ctrl_shift(key: Key) -> Self {
        Self {
            ctrl: true,
            shift: true,
            key,
        }
    }
    const fn shift(key: Key) -> Self {
        Self {
            ctrl: false,
            shift: true,
            key,
        }
    }

    /// More modifiers first, so Ctrl+S never swallows Ctrl+Shift+S.
    pub fn modifier_count(self) -> u8 {
        u8::from(self.ctrl) + u8::from(self.shift)
    }

    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
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
    /// `None` for commands reached only from the Find bar or a shortcut.
    pub menu: Option<Menu>,
    pub shortcut: Option<Shortcut>,
    /// Shown with a check mark (on/off setting).
    pub toggle: bool,
    pub separator_before: bool,
}

const fn spec(
    command: Command,
    id: &'static str,
    label: &'static str,
    menu: Option<Menu>,
    shortcut: Option<Shortcut>,
) -> CommandSpec {
    CommandSpec {
        command,
        id,
        label,
        menu,
        shortcut,
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
use Menu as M;

/// In menu order. Labels and shortcuts follow Windows Notepad (observed behaviour).
pub const CATALOG: &[CommandSpec] = &[
    spec(
        C::New,
        "file.new",
        "New",
        Some(M::File),
        Some(Shortcut::ctrl(K::N)),
    ),
    spec(
        C::Open,
        "file.open",
        "Open…",
        Some(M::File),
        Some(Shortcut::ctrl(K::O)),
    ),
    spec(
        C::Save,
        "file.save",
        "Save",
        Some(M::File),
        Some(Shortcut::ctrl(K::S)),
    ),
    spec(
        C::SaveAs,
        "file.saveAs",
        "Save As…",
        Some(M::File),
        Some(Shortcut::ctrl_shift(K::S)),
    ),
    separated(spec(C::Exit, "file.exit", "Exit", Some(M::File), None)),
    spec(
        C::Find,
        "edit.find",
        "Find…",
        Some(M::Edit),
        Some(Shortcut::ctrl(K::F)),
    ),
    spec(
        C::FindNext,
        "edit.findNext",
        "Find Next",
        Some(M::Edit),
        Some(Shortcut::key(K::F3)),
    ),
    spec(
        C::FindPrevious,
        "edit.findPrevious",
        "Find Previous",
        Some(M::Edit),
        Some(Shortcut::shift(K::F3)),
    ),
    spec(
        C::Replace,
        "edit.replace",
        "Replace…",
        Some(M::Edit),
        Some(Shortcut::ctrl(K::H)),
    ),
    spec(C::ReplaceNext, "edit.replaceNext", "Replace", None, None),
    spec(C::ReplaceAll, "edit.replaceAll", "Replace All", None, None),
    toggle(spec(
        C::WordWrap,
        "format.wordWrap",
        "Word Wrap",
        Some(M::Format),
        None,
    )),
    spec(
        C::ZoomIn,
        "view.zoomIn",
        "Zoom In",
        Some(M::View),
        Some(Shortcut::ctrl(K::Plus)),
    ),
    spec(
        C::ZoomOut,
        "view.zoomOut",
        "Zoom Out",
        Some(M::View),
        Some(Shortcut::ctrl(K::Minus)),
    ),
    spec(
        C::ZoomReset,
        "view.zoomReset",
        "Restore Default Zoom",
        Some(M::View),
        Some(Shortcut::ctrl(K::Num0)),
    ),
    separated(toggle(spec(
        C::StatusBar,
        "view.statusBar",
        "Status Bar",
        Some(M::View),
        None,
    ))),
    spec(
        C::About,
        "help.about",
        "About HarshPad",
        Some(M::Help),
        None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const ALL: [Command; 17] = [
        C::New,
        C::Open,
        C::Save,
        C::SaveAs,
        C::Exit,
        C::Find,
        C::FindNext,
        C::FindPrevious,
        C::Replace,
        C::ReplaceNext,
        C::ReplaceAll,
        C::WordWrap,
        C::ZoomIn,
        C::ZoomOut,
        C::ZoomReset,
        C::StatusBar,
        C::About,
    ];

    #[test]
    fn every_command_is_in_the_catalog_once() {
        for command in ALL {
            let count = CATALOG.iter().filter(|s| s.command == command).count();
            assert_eq!(count, 1, "{command:?}");
        }
        assert_eq!(CATALOG.len(), ALL.len());
    }

    #[test]
    fn ids_and_shortcuts_are_unique() {
        let ids: HashSet<_> = CATALOG.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), CATALOG.len());
        let shortcuts: Vec<_> = CATALOG.iter().filter_map(|s| s.shortcut).collect();
        let unique: HashSet<_> = shortcuts.iter().collect();
        assert_eq!(unique.len(), shortcuts.len());
    }

    #[test]
    fn lookups_work() {
        assert_eq!(by_id("file.saveAs").map(|s| s.command), Some(C::SaveAs));
        assert_eq!(spec_of(C::Find).map(|s| s.id), Some("edit.find"));
        assert!(by_id("nope").is_none());
        assert_eq!(in_menu(M::Format).count(), 1);
    }

    #[test]
    fn shortcut_labels_read_like_notepad() {
        assert_eq!(Shortcut::ctrl_shift(K::S).label(), "Ctrl+Shift+S");
        assert_eq!(Shortcut::key(K::F3).label(), "F3");
        assert_eq!(Shortcut::shift(K::F3).modifier_count(), 1);
    }
}
