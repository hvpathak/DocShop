//! HarshPad Studio: tabs of plain text and rich documents in one window, with one
//! toolbar layout, one status bar, one find bar and one command catalog for both.

use std::cmp::Reverse;
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align, Align2, Color32, KeyboardShortcut, Layout, Modifiers, RichText, vec2,
};
use harshpad_core::commands::{self, CATALOG, Command, Key, Menu, Shortcut, TabKind};
use harshpad_core::document::{Document, write_atomically};
use harshpad_core::modes::{self, Chapter, Mode};
use harshpad_core::session::FindOptions;
use harshpad_rich::{Flag, RichDoc, RichFormat, markdown};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

use crate::chrome;
use crate::doc_tab::DocTab;
use crate::fonts;
use crate::recovery::{Recovery, Saved};
use crate::ribbon::{self, Act};
use crate::settings::Settings;
use crate::text_tab::{Step, TextTab};
use crate::writer_tab::WriterTab;

pub const APP_NAME: &str = "HarshPad";
const TEXT_SIZE: f32 = 15.0;
const ZOOM_STEP: f32 = 1.1;
/// Documents opened from disk larger than this are refused politely.
/// Files the WordCraft editor opens (Writer tabs).
const WRITER_FILES: [&str; 10] = [
    "docx", "docm", "dotx", "dotm", "doc", "dot", "rtf", "odt", "html", "md",
];
const MAX_OPEN_BYTES: u64 = 64 * 1024 * 1024;
const ACCENT: Color32 = Color32::from_rgb(30, 110, 210);

enum Content {
    Text(Box<TextTab>),
    Doc(Box<DocTab>),
    /// A document in the WordCraft editor.
    Writer(Box<WriterTab>),
}

/// A toggle switch, like the AutoSave switch of a word processor.
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = vec2(32.0, 16.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(response.id, *on);
    let fill = if *on {
        ACCENT
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    let radius = rect.height() / 2.0;
    ui.painter().rect(
        rect,
        radius,
        fill,
        egui::Stroke::new(
            1.0_f32,
            ui.visuals()
                .widgets
                .inactive
                .fg_stroke
                .color
                .gamma_multiply(0.5),
        ),
        egui::StrokeKind::Inside,
    );
    let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
    ui.painter()
        .circle_filled(egui::pos2(x, rect.center().y), radius - 3.0, Color32::WHITE);
    response
}

struct Tab {
    id: u64,
    content: Content,
}

impl Tab {
    fn kind(&self) -> TabKind {
        match self.content {
            Content::Text(_) => TabKind::Text,
            Content::Doc(_) | Content::Writer(_) => TabKind::Doc,
        }
    }

    fn title(&self) -> String {
        match &self.content {
            Content::Text(t) => t.title(),
            Content::Doc(d) => d.title(),
            Content::Writer(w) => w.title(),
        }
    }

    fn path(&self) -> Option<&Path> {
        match &self.content {
            Content::Text(t) => t.path(),
            Content::Doc(d) => d.path.as_deref(),
            Content::Writer(w) => w.path(),
        }
    }

    fn is_dirty(&self) -> bool {
        match &self.content {
            Content::Text(t) => t.is_dirty(),
            Content::Doc(d) => d.is_dirty(),
            Content::Writer(w) => w.is_dirty(),
        }
    }

    fn is_pristine(&self) -> bool {
        match &self.content {
            Content::Text(t) => t.is_pristine(),
            Content::Doc(d) => d.is_pristine(),
            Content::Writer(w) => w.is_pristine(),
        }
    }

    fn focus(&self, ctx: &egui::Context) {
        match &self.content {
            Content::Text(t) => t.focus(ctx),
            Content::Doc(d) => d.focus(ctx),
            // WordCraft keeps its own focus.
            Content::Writer(_) => {}
        }
    }

    fn recovery_copy(&self) -> Option<Saved> {
        let (kind, content) = match &self.content {
            Content::Text(t) => ("text", t.text().to_owned()),
            Content::Doc(d) => ("doc", String::from_utf8(d.doc.to_json().ok()?).ok()?),
            // WordCraft keeps its own recovery copies.
            Content::Writer(_) => return None,
        };
        Some(Saved {
            kind: kind.into(),
            title: self.title(),
            path: self.path().map(Path::to_path_buf),
            content,
        })
    }
}

#[derive(Default)]
struct FindBar {
    open: bool,
    show_replace: bool,
    focus_needle: bool,
    options: FindOptions,
}

enum Dialog {
    WordCount {
        rows: Vec<(&'static str, String)>,
    },
    Shortcuts,
    Palette {
        query: String,
        focus: bool,
    },
    Link {
        url: String,
        focus: bool,
    },
    /// Asks for the parent PIN before doing `then`.
    Unlock {
        then: Gate,
        pin: String,
        wrong: bool,
        focus: bool,
    },
    SetLock {
        pin: String,
        confirm: String,
        error: Option<&'static str>,
        focus: bool,
        /// Start the growing path once the PIN is set.
        start_path: bool,
    },
    /// The card shown when a chapter of the growing path opens.
    Chapter {
        chapter: Chapter,
        dont_show: bool,
    },
    /// The start card: the three apps, and where to switch between them.
    Welcome {
        dont_show: bool,
    },
}

/// What the parent PIN unlocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gate {
    Mode(Mode),
    RemoveLock,
    NextChapter,
    LeavePath,
}

impl Gate {
    fn ask(self) -> String {
        match self {
            Gate::Mode(m) => format!("Enter the parent PIN to switch to {}.", m.label()),
            Gate::RemoveLock => "Enter the parent PIN to remove the lock.".into(),
            Gate::NextChapter => "Enter the parent PIN to unlock the next chapter.".into(),
            Gate::LeavePath => "Enter the parent PIN to leave the growing path.".into(),
        }
    }
}

pub struct StudioApp {
    tabs: Vec<Tab>,
    active: usize,
    next_id: u64,
    zoom: f32,
    show_status_bar: bool,
    find: FindBar,
    dialog: Option<Dialog>,
    status: Option<String>,
    allow_close: bool,
    window_title: String,
    recovery: Recovery,
    settings: Settings,
    /// Which ribbon tab is showing (index into `ribbon::TABS`).
    ribbon_tab: usize,
    ribbon_collapsed: bool,
    /// Open the new-tab choices on the next frame (Ctrl+T).
    open_picker: bool,
    tab_search: String,
    command_search: String,
    /// Recently closed tabs (title, content), newest last, for Ctrl+Shift+T.
    closed: Vec<(String, Content)>,
    /// Clipboard events for the editor, sent once it has focus.
    pending_input: Vec<egui::Event>,
    last_autosave: Instant,
    /// For Writer tabs, which need it to set up their dialogs and theme.
    ctx: egui::Context,
    /// Files Finder asks us to open (macOS).
    #[cfg(target_os = "macos")]
    finder: Option<fmv_macos_events::Inbox>,
}

impl StudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>) -> Self {
        // Ctrl +/- zooms the page, not the whole interface.
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        fonts::install(&cc.egui_ctx);
        let mut app = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
            zoom: 1.0,
            show_status_bar: true,
            find: FindBar::default(),
            dialog: None,
            status: None,
            allow_close: false,
            window_title: String::new(),
            recovery: Recovery::new(),
            settings: Settings::load(),
            ribbon_tab: 1,
            ribbon_collapsed: false,
            open_picker: false,
            tab_search: String::new(),
            command_search: String::new(),
            closed: Vec::new(),
            pending_input: Vec::new(),
            last_autosave: Instant::now(),
            ctx: cc.egui_ctx.clone(),
            #[cfg(target_os = "macos")]
            finder: None,
        };
        if !app.settings.mode.is_available() {
            app.settings.mode = Mode::Studio;
        }
        let recovered = app.recovery.load();
        if !recovered.is_empty() {
            let n = recovered.len();
            for saved in recovered {
                app.restore(saved);
            }
            app.status = Some(format!("Recovered {n} unsaved tab(s) from last time"));
        }
        if let Some(path) = initial_file {
            app.open_path(&path);
        }
        app.enforce_mode();
        // On the growing path, the chapter card says hello at every start.
        // Until "Don't show this again" is ticked, a card says hello at every start:
        // the chapter card on the growing path, otherwise the welcome card.
        if !app.settings.hide_welcome {
            app.dialog = Some(match app.settings.path {
                Some(chapter) => Dialog::Chapter {
                    chapter,
                    dont_show: false,
                },
                None => Dialog::Welcome { dont_show: false },
            });
        }
        app
    }

    /// Receives Finder's open-document events (macOS).
    #[cfg(target_os = "macos")]
    pub fn connect_finder(&mut self, inbox: fmv_macos_events::Inbox) {
        self.finder = Some(inbox);
    }

    // ---- Modes -------------------------------------------------------------------------

    /// The mode in effect: on the growing path the chapter decides.
    fn mode(&self) -> Mode {
        self.settings.path.map_or(self.settings.mode, Chapter::mode)
    }

    /// Whether the new-tab choices show PDF (as "coming soon" until phase 2).
    fn pdf_offered(&self) -> bool {
        match self.settings.path {
            Some(chapter) => chapter.offers_pdf(),
            None => matches!(self.mode(), Mode::Studio | Mode::Pdf),
        }
    }

    fn ask_pin(&mut self, then: Gate) {
        self.dialog = Some(Dialog::Unlock {
            then,
            pin: String::new(),
            wrong: false,
            focus: true,
        });
    }

    /// Starts the growing path at chapter 1. A parent PIN must exist first, so
    /// only a parent can unlock what comes next.
    fn start_path(&mut self) {
        if self.settings.path.is_some() {
            self.status = Some("The growing path is already on".into());
        } else if !self.settings.locked() {
            self.dialog = Some(Dialog::SetLock {
                pin: String::new(),
                confirm: String::new(),
                error: None,
                focus: true,
                start_path: true,
            });
        } else {
            self.enter_chapter(Chapter::Notepad);
        }
    }

    fn enter_chapter(&mut self, chapter: Chapter) {
        self.settings.path = Some(chapter);
        self.settings.save();
        self.enforce_mode();
        self.dialog = Some(Dialog::Chapter {
            chapter,
            dont_show: false,
        });
        self.status = Some(format!(
            "Chapter {}: {} — {}",
            chapter.number(),
            chapter.app(),
            chapter.title()
        ));
    }

    /// The kind "New" makes: plain text in Studio and Notepad, documents in Writer.
    fn home_kind(&self) -> TabKind {
        self.mode().home_kind()
    }

    /// Whether the current mode offers a command at all.
    fn offered(&self, spec: &commands::CommandSpec) -> bool {
        self.mode().allows_command(spec)
    }

    fn request_mode(&mut self, target: Mode) {
        if let Some(chapter) = self.settings.path {
            self.status = Some(format!(
                "On the growing path (chapter {}). A parent can unlock the next chapter or leave the path.",
                chapter.number()
            ));
        } else if !target.is_available() {
            self.status = Some("PDF tools arrive in the next update".into());
        } else if target == self.mode() {
        } else if self.settings.locked() {
            self.ask_pin(Gate::Mode(target));
        } else {
            self.set_mode(target);
        }
    }

    fn set_mode(&mut self, target: Mode) {
        self.settings.mode = target;
        self.settings.save();
        self.enforce_mode();
        self.status = Some(format!("Mode: {}", target.label()));
    }

    /// Turns tabs of kinds the mode doesn't allow into the mode's kind (as new,
    /// unsaved tabs, so nothing on disk is overwritten), and keeps one tab open.
    fn enforce_mode(&mut self) {
        let mode = self.mode();
        for i in 0..self.tabs.len() {
            let Some(tab) = self.tabs.get(i) else {
                continue;
            };
            if mode.allows(tab.kind()) {
                continue;
            }
            let id = self.id();
            if let Some(tab) = self.tabs.get_mut(i) {
                let old = std::mem::replace(
                    &mut tab.content,
                    Content::Text(Box::new(TextTab::new(Document::new(), id))),
                );
                tab.content = convert(old, mode.home_kind(), id, true);
                tab.id = id;
            }
        }
        if self.tabs.is_empty() {
            self.new_tab(self.home_kind());
        }
        self.active = self.active.min(self.tabs.len().saturating_sub(1));
    }

    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn restore(&mut self, saved: Saved) {
        let id = self.id();
        let content = if saved.kind == "doc" {
            let Ok(doc) = RichDoc::from_json(saved.content.as_bytes()) else {
                return;
            };
            let format = saved.path.as_deref().and_then(RichFormat::from_path);
            let mut tab = DocTab::new(doc, saved.path, format, id);
            tab.mark_dirty();
            Content::Doc(Box::new(tab))
        } else {
            let mut doc = saved
                .path
                .as_deref()
                .map_or_else(Document::new, Document::new_at);
            doc.text = saved.content;
            doc.mark_changed();
            Content::Text(Box::new(TextTab::new(doc, id)))
        };
        self.tabs.push(Tab { id, content });
    }

    fn tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    fn kind(&self) -> TabKind {
        self.tab().map_or(TabKind::Text, Tab::kind)
    }

    fn text_tab(&mut self) -> Option<&mut TextTab> {
        match &mut self.tab_mut()?.content {
            Content::Text(t) => Some(t),
            Content::Doc(_) | Content::Writer(_) => None,
        }
    }

    fn doc_tab(&mut self) -> Option<&mut DocTab> {
        match &mut self.tab_mut()?.content {
            Content::Doc(d) => Some(d),
            Content::Text(_) | Content::Writer(_) => None,
        }
    }

    // ---- Tabs ------------------------------------------------------------------------

    fn add_tab(&mut self, content: Content) {
        let id = self.id();
        let tab = Tab { id, content };
        // A brand-new empty tab is replaced rather than kept beside the new one.
        if self.tab().is_some_and(Tab::is_pristine) {
            if let Some(slot) = self.tabs.get_mut(self.active) {
                *slot = tab;
            }
        } else {
            self.tabs.push(tab);
            self.active = self.tabs.len() - 1;
        }
    }

    fn new_tab(&mut self, kind: TabKind) {
        let id = self.next_id + 1;
        let content = match kind {
            TabKind::Text => Content::Text(Box::new(TextTab::new(Document::new(), id))),
            // New documents open in the WordCraft editor.
            TabKind::Doc => Content::Writer(Box::new(WriterTab::new(&self.ctx))),
        };
        let id = self.id();
        self.tabs.push(Tab { id, content });
        self.active = self.tabs.len() - 1;
    }

    fn close_tab(&mut self, index: usize) {
        if !self.confirm_discard(index) {
            return;
        }
        if index < self.tabs.len() {
            let tab = self.tabs.remove(index);
            // Keep it for Ctrl+Shift+T (empty new tabs aren't worth reopening).
            if !tab.is_pristine() {
                self.closed.push((tab.title(), tab.content));
                if self.closed.len() > 10 {
                    self.closed.remove(0);
                }
            }
            if index < self.active || self.active >= self.tabs.len() {
                self.active = self.active.saturating_sub(1);
            }
        }
        if self.tabs.is_empty() {
            self.new_tab(self.home_kind());
        }
        self.active = self.active.min(self.tabs.len() - 1);
    }

    fn reopen(&mut self, index: usize) {
        if index >= self.closed.len() {
            return;
        }
        let (_, content) = self.closed.remove(index);
        let id = self.id();
        self.tabs.push(Tab { id, content });
        self.active = self.tabs.len() - 1;
        self.enforce_mode();
    }

    /// Opens a new tab: in a single-app mode straight away, otherwise ask which kind.
    fn new_tab_flow(&mut self) {
        if self.mode() == Mode::Studio || self.settings.path.is_some() {
            self.open_picker = true;
        } else {
            self.new_tab(self.home_kind());
        }
    }

    fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        self.active = to;
    }

    // ---- Commands ----------------------------------------------------------------------

    /// Last-resort guard: a bug in a command shows a message and keeps every tab.
    fn dispatch(&mut self, command: Command, ctx: &egui::Context) {
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| self.run(command, ctx)));
        if outcome.is_err() {
            let id = commands::spec_of(command).map_or("unknown", |s| s.id);
            show_error(&format!(
                "Something went wrong while running \"{id}\". Your text is still here; please save it."
            ));
        }
    }

    fn run(&mut self, command: Command, ctx: &egui::Context) {
        let kind = self.kind();
        if !commands::spec_of(command).is_some_and(|s| s.works_in(kind) && self.offered(s)) {
            return;
        }
        self.status = None;
        match command {
            Command::NewTab => self.new_tab_flow(),
            Command::ReopenClosedTab => match self.closed.len().checked_sub(1) {
                Some(last) => self.reopen(last),
                None => self.status = Some("No recently closed tabs".into()),
            },
            Command::Tab1
            | Command::Tab2
            | Command::Tab3
            | Command::Tab4
            | Command::Tab5
            | Command::Tab6
            | Command::Tab7
            | Command::Tab8 => {
                let n = [
                    Command::Tab1,
                    Command::Tab2,
                    Command::Tab3,
                    Command::Tab4,
                    Command::Tab5,
                    Command::Tab6,
                    Command::Tab7,
                    Command::Tab8,
                ]
                .iter()
                .position(|c| *c == command)
                .unwrap_or(0);
                if n < self.tabs.len() {
                    self.active = n;
                }
            }
            Command::LastTab => self.active = self.tabs.len().saturating_sub(1),
            Command::NewText => self.new_tab(self.home_kind()),
            Command::NewDocument => self.new_tab(TabKind::Doc),
            Command::ModeStudio => self.request_mode(Mode::Studio),
            Command::ModeNotepad => self.request_mode(Mode::Notepad),
            Command::ModeWriter => self.request_mode(Mode::Writer),
            Command::ModePdf => self.request_mode(Mode::Pdf),
            Command::ParentLock => {
                if self.settings.path.is_some() {
                    self.status = Some(
                        "The growing path uses the parent PIN; leave the path to remove it".into(),
                    );
                } else if self.settings.locked() {
                    self.ask_pin(Gate::RemoveLock);
                } else {
                    self.dialog = Some(Dialog::SetLock {
                        pin: String::new(),
                        confirm: String::new(),
                        error: None,
                        focus: true,
                        start_path: false,
                    });
                }
            }
            Command::StartPath => self.start_path(),
            Command::UnlockNextChapter => match self.settings.path {
                None => self.status = Some("Start the growing path first (View → Path)".into()),
                Some(c) if c.next().is_none() => {
                    self.status = Some("Every chapter is already unlocked".into());
                }
                Some(_) => self.ask_pin(Gate::NextChapter),
            },
            Command::LeavePath => {
                if self.settings.path.is_some() {
                    self.ask_pin(Gate::LeavePath);
                }
            }
            Command::Open => self.open_dialog(),
            Command::Save => {
                self.save(self.active);
            }
            Command::SaveAs => {
                self.save_as(self.active);
            }
            Command::ConvertToDocument => self.convert_to_document(),
            Command::ConvertToText => self.convert_to_text(),
            Command::CloseTab => self.close_tab(self.active),
            Command::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Command::Undo | Command::Redo => {
                let undo = command == Command::Undo;
                let done = match self.tab_mut().map(|t| &mut t.content) {
                    Some(Content::Text(t)) => {
                        if undo {
                            t.undo()
                        } else {
                            t.redo()
                        }
                    }
                    Some(Content::Doc(d)) => {
                        if undo {
                            d.undo()
                        } else {
                            d.redo()
                        }
                    }
                    // WordCraft handles its own undo keys and buttons.
                    Some(Content::Writer(_)) | None => false,
                };
                if !done {
                    self.status = Some(
                        if undo {
                            "Nothing to undo"
                        } else {
                            "Nothing to redo"
                        }
                        .into(),
                    );
                }
            }
            Command::Find => self.open_find(false),
            Command::Replace => self.open_find(true),
            Command::FindNext => self.search(ctx, Step::Next),
            Command::FindPrevious => self.search(ctx, Step::Previous),
            Command::ReplaceNext => self.search(ctx, Step::ReplaceNext),
            Command::ReplaceAll => {
                let options = self.find.options.clone();
                let count = match self.tab_mut().map(|t| &mut t.content) {
                    Some(Content::Text(t)) => t.replace_all(&options),
                    Some(Content::Doc(d)) => d.replace_all(&options),
                    Some(Content::Writer(_)) | None => 0,
                };
                self.status = Some(format!("Replaced {count} occurrence(s)"));
            }
            Command::CommandPalette => {
                self.dialog = Some(Dialog::Palette {
                    query: String::new(),
                    focus: true,
                })
            }
            Command::InsertLink => {
                let url = self
                    .doc_tab()
                    .and_then(|d| d.selected_link())
                    .unwrap_or_default();
                self.dialog = Some(Dialog::Link { url, focus: true });
            }
            Command::WordWrap => {
                if let Some(t) = self.text_tab() {
                    t.word_wrap = !t.word_wrap;
                }
            }
            Command::NextTab => self.active = (self.active + 1) % self.tabs.len().max(1),
            Command::PreviousTab => {
                let n = self.tabs.len().max(1);
                self.active = (self.active + n - 1) % n;
            }
            Command::ZoomIn => self.zoom = (self.zoom * ZOOM_STEP).min(5.0),
            Command::ZoomOut => self.zoom = (self.zoom / ZOOM_STEP).max(0.3),
            Command::ZoomReset => self.zoom = 1.0,
            Command::StatusBar => self.show_status_bar = !self.show_status_bar,
            Command::About => show_about(),
            other => {
                if let Some(d) = self.doc_tab() {
                    d.apply(other);
                }
            }
        }
        if !matches!(
            command,
            Command::Find | Command::Replace | Command::CommandPalette | Command::InsertLink
        ) && let Some(t) = self.tab()
        {
            t.focus(ctx);
        }
    }

    fn checked(&self, command: Command) -> Option<bool> {
        let mode = self.mode();
        match command {
            Command::StatusBar => Some(self.show_status_bar),
            Command::ModeStudio => Some(mode == Mode::Studio),
            Command::ModeNotepad => Some(mode == Mode::Notepad),
            Command::ModeWriter => Some(mode == Mode::Writer),
            Command::ModePdf => Some(mode == Mode::Pdf),
            Command::ParentLock => Some(self.settings.locked()),
            Command::WordWrap => match self.tab().map(|t| &t.content) {
                Some(Content::Text(t)) => Some(t.word_wrap),
                _ => Some(false),
            },
            _ => None,
        }
    }

    // ---- Files -------------------------------------------------------------------------

    fn open_dialog(&mut self) {
        let picked = FileDialog::new()
            .add_filter(
                "Text and documents",
                &[
                    "txt", "md", "markdown", "docx", "hpdoc", "log", "csv", "json", "ini", "xml",
                ],
            )
            .add_filter("Documents", &["docx", "md", "hpdoc"])
            .add_filter("All files", &["*"])
            .pick_files();
        for path in picked.unwrap_or_default() {
            self.open_path(&path);
        }
    }

    fn open_path(&mut self, path: &Path) {
        if let Some(i) = self.tabs.iter().position(|t| t.path() == Some(path)) {
            self.active = i;
            return;
        }
        let id = self.next_id + 1;
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let content = if WRITER_FILES.contains(&ext.as_str()) {
            match WriterTab::open(&self.ctx, path) {
                Ok(w) => Content::Writer(Box::new(w)),
                Err(e) => return show_error(&format!("Could not open {}:\n{e}", path.display())),
            }
        } else {
            match RichFormat::from_path(path) {
                Some(format) => match read_limited(path)
                    .and_then(|b| harshpad_rich::decode(&b, format).map_err(io::Error::other))
                {
                    Ok(doc) => Content::Doc(Box::new(DocTab::new(
                        doc,
                        Some(path.to_path_buf()),
                        Some(format),
                        id,
                    ))),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => Content::Doc(Box::new(
                        DocTab::new(RichDoc::new(), Some(path.to_path_buf()), Some(format), id),
                    )),
                    Err(e) => {
                        return show_error(&format!("Could not open {}:\n{e}", path.display()));
                    }
                },
                None => match Document::open(path) {
                    Ok(doc) => Content::Text(Box::new(TextTab::new(doc, id))),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => {
                        Content::Text(Box::new(TextTab::new(Document::new_at(path), id)))
                    }
                    Err(e) => {
                        return show_error(&format!("Could not open {}:\n{e}", path.display()));
                    }
                },
            }
        };
        // In a single-app mode every file opens in that app (as a new, unsaved copy,
        // so the original file is never overwritten in another format).
        let mode = self.mode();
        let kind = match content {
            Content::Text(_) => TabKind::Text,
            Content::Doc(_) | Content::Writer(_) => TabKind::Doc,
        };
        let content = if mode.allows(kind) {
            content
        } else {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.status = Some(format!(
                "Opened {name} in {} mode as a new copy",
                mode.short()
            ));
            convert(content, mode.home_kind(), id, kind == TabKind::Text)
        };
        self.add_tab(content);
    }

    /// Returns true if the tab was saved.
    fn save(&mut self, index: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(index) else {
            return false;
        };
        let result = match &mut tab.content {
            Content::Text(t) if t.path().is_some() => t.session.doc.save(),
            Content::Doc(d) => match (d.path.clone(), d.format) {
                (Some(path), Some(format)) => write_doc(d, path, format),
                _ => return self.save_as(index),
            },
            Content::Writer(w) => w.run("file.save"),
            Content::Text(_) => return self.save_as(index),
        };
        self.report_save(result)
    }

    fn save_as(&mut self, index: usize) -> bool {
        let Some(tab) = self.tabs.get_mut(index) else {
            return false;
        };
        let result = match &mut tab.content {
            Content::Writer(w) => w.run("file.saveAs"),
            Content::Text(t) => {
                let name = if t.path().is_some() {
                    t.title()
                } else {
                    "Untitled.txt".into()
                };
                let Some(path) = FileDialog::new()
                    .add_filter("Text Documents (*.txt)", &["txt"])
                    .add_filter("All Files", &["*"])
                    .set_file_name(name)
                    .save_file()
                else {
                    return false;
                };
                t.session.doc.save_as(path)
            }
            Content::Doc(d) => {
                let stem = d.path.as_deref().and_then(Path::file_stem).map_or_else(
                    || "Untitled".to_owned(),
                    |s| s.to_string_lossy().into_owned(),
                );
                let Some(mut path) = FileDialog::new()
                    .add_filter("Word document (.docx)", &["docx"])
                    .add_filter("Markdown (.md)", &["md"])
                    .add_filter("HarshPad document (.hpdoc)", &["hpdoc"])
                    .set_file_name(format!("{stem}.docx"))
                    .save_file()
                else {
                    return false;
                };
                let format = RichFormat::from_path(&path).unwrap_or_else(|| {
                    path.set_extension("docx");
                    RichFormat::Docx
                });
                write_doc(d, path, format)
            }
        };
        self.report_save(result)
    }

    fn report_save(&mut self, result: io::Result<()>) -> bool {
        match result {
            Ok(()) => {
                let markdown = self.tab().is_some_and(|t| matches!(&t.content, Content::Doc(d) if d.format == Some(RichFormat::Markdown)));
                self.status = Some(if markdown {
                    "Saved as Markdown (colours, highlights and sizes aren't kept in Markdown)"
                        .into()
                } else {
                    "Saved".into()
                });
                self.write_recovery();
                true
            }
            Err(e) => {
                let name = self.tab().map(Tab::title).unwrap_or_default();
                show_error(&format!("Could not save {name}:\n{e}"));
                false
            }
        }
    }

    /// Asks to save a tab with unsaved changes. Returns false if the user cancelled.
    fn confirm_discard(&mut self, index: usize) -> bool {
        let Some(tab) = self.tabs.get(index) else {
            return true;
        };
        if !tab.is_dirty() {
            return true;
        }
        self.active = index;
        let answer = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title(APP_NAME)
            .set_description(format!("Do you want to save changes to {}?", tab.title()))
            .set_buttons(MessageButtons::YesNoCancel)
            .show();
        match answer {
            // A save that needs a file name finishes in the Writer's own dialog; the tab
            // stays open until the document is really saved.
            MessageDialogResult::Yes => {
                self.save(index) && !self.tabs.get(index).is_some_and(Tab::is_dirty)
            }
            MessageDialogResult::No => true,
            _ => false,
        }
    }

    // ---- Sync between kinds -------------------------------------------------------------

    fn convert_to_document(&mut self) {
        let Some(text) = self.text_tab().map(|t| t.text().to_owned()) else {
            return;
        };
        let doc = markdown::read(&text);
        let id = self.id();
        let mut tab = DocTab::new(doc, None, None, id);
        tab.mark_dirty();
        self.tabs.push(Tab {
            id,
            content: Content::Doc(Box::new(tab)),
        });
        self.active = self.tabs.len() - 1;
        self.status = Some(
            "Opened as a document: Markdown headings, lists and emphasis became formatting".into(),
        );
    }

    fn convert_to_text(&mut self) {
        let Some(md) = self.doc_tab().map(|d| markdown::write(&d.doc)) else {
            return;
        };
        let id = self.id();
        let mut doc = Document::new();
        doc.text = md;
        doc.mark_changed();
        self.tabs.push(Tab {
            id,
            content: Content::Text(Box::new(TextTab::new(doc, id))),
        });
        self.active = self.tabs.len() - 1;
        self.status = Some("Opened as plain text: formatting is written as Markdown".into());
    }

    // ---- Find ----------------------------------------------------------------------------

    fn open_find(&mut self, with_replace: bool) {
        self.find.open = true;
        self.find.show_replace |= with_replace;
        self.find.focus_needle = true;
    }

    fn search(&mut self, ctx: &egui::Context, step: Step) {
        if self.find.options.needle.is_empty() {
            self.open_find(false);
            return;
        }
        let options = self.find.options.clone();
        let found = match self.tab_mut().map(|t| &mut t.content) {
            Some(Content::Text(t)) => t.search(ctx, &options, step),
            Some(Content::Doc(d)) => d.search(&options, step),
            Some(Content::Writer(_)) | None => false,
        };
        if !found {
            self.status = Some(format!("Cannot find \"{}\"", options.needle));
        }
    }

    fn search_changed(&mut self) {
        match self.tab_mut().map(|t| &mut t.content) {
            Some(Content::Text(t)) => t.search_changed(),
            Some(Content::Doc(d)) => d.search_changed(),
            Some(Content::Writer(_)) | None => {}
        }
    }

    // ---- Recovery -------------------------------------------------------------------------

    fn dirty_copies(&self) -> Vec<(u64, Saved)> {
        self.tabs
            .iter()
            .filter(|t| t.is_dirty())
            .filter_map(|t| t.recovery_copy().map(|s| (t.id, s)))
            .collect()
    }

    fn write_recovery(&self) {
        self.recovery.write(self.dirty_copies());
    }

    // ---- Per-frame input -----------------------------------------------------------------

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.viewport().close_requested()) || self.allow_close {
            return;
        }
        for i in 0..self.tabs.len() {
            if !self.confirm_discard(i) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                return;
            }
        }
        self.allow_close = true;
        self.recovery.clear();
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        for path in dropped {
            self.open_path(&path);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let kind = self.kind();
        // Typing in the find box, palette or link dialog keeps undo, formatting
        // and editing keys for that box.
        let editing_elsewhere = ctx.egui_wants_keyboard_input() && self.dialog.is_some()
            || ctx.memory(|m| m.focused()).is_some_and(|f| {
                [
                    "find-needle",
                    "find-replacement",
                    "command-search",
                    "tab-search",
                ]
                .iter()
                .any(|name| f == egui::Id::new(name))
            });
        let mut bound: Vec<_> = CATALOG
            .iter()
            .filter(|s| s.works_in(kind) && self.offered(s))
            .filter_map(|s| s.shortcut.map(|k| (s.command, s.menu, k)))
            .collect();
        bound.sort_by_key(|(_, _, k)| Reverse(k.modifier_count()));
        // In a WordCraft tab only the tab-level keys are ours; WordCraft handles the rest.
        let writer = matches!(self.tab().map(|t| &t.content), Some(Content::Writer(_)));
        for (command, menu, shortcut) in bound {
            if writer && !tab_level(command) {
                continue;
            }
            let global = matches!(menu, Some(Menu::File | Menu::View))
                || matches!(
                    command,
                    Command::FindNext
                        | Command::FindPrevious
                        | Command::Find
                        | Command::Replace
                        | Command::CommandPalette
                );
            if editing_elsewhere && !global {
                continue;
            }
            if consume(ctx, shortcut) {
                self.dispatch(command, ctx);
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let Some(tab) = self.tab() else { return };
        let dirty = if tab.is_dirty() { "*" } else { "" };
        let title = format!("{dirty}{} - {APP_NAME}", tab.title());
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    // ---- Panels ---------------------------------------------------------------------------

    /// The classic menus, kept under ⋯ for people who prefer them.
    fn classic_menus(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let kind = self.kind();
        let mut clicked = None;
        for menu in Menu::ALL {
            ui.menu_button(menu.label(), |ui| {
                for spec in commands::in_menu(menu).filter(|s| self.offered(s)) {
                    if spec.separator_before {
                        ui.separator();
                    }
                    let shortcut = spec.shortcut.map(Shortcut::label).unwrap_or_default();
                    // In a single-app mode "New text file" is just "New".
                    let label = if spec.command == Command::NewText && self.mode() != Mode::Studio {
                        "New"
                    } else {
                        spec.label
                    };
                    let mut button = egui::Button::new(label).shortcut_text(shortcut);
                    if let Some(on) = self.checked(spec.command) {
                        button = button.selected(on);
                    }
                    if ui.add_enabled(spec.works_in(kind), button).clicked() {
                        clicked = Some(spec.command);
                        ui.close();
                    }
                }
            });
        }
        clicked
    }

    /// Browser-style tabs in the title bar.
    fn tab_bar(&mut self, ui: &mut egui::Ui) {
        let infos: Vec<chrome::TabInfo> = self
            .tabs
            .iter()
            .map(|t| chrome::TabInfo {
                title: t.title(),
                icon: match t.kind() {
                    TabKind::Doc => crate::icons::AppIcon::Writer,
                    TabKind::Text => crate::icons::AppIcon::Notepad,
                },
                dirty: t.is_dirty(),
                tooltip: t.path().map_or_else(
                    || format!("{} · not saved yet", t.title()),
                    |p| p.display().to_string(),
                ),
            })
            .collect();
        let mode = self.mode();
        let path = self.settings.path;
        let offer = chrome::Offer {
            ask: mode == Mode::Studio || path.is_some(),
            home: self.home_kind(),
            text: mode.allows(TabKind::Text),
            doc: mode.allows(TabKind::Doc),
            pdf: self.pdf_offered(),
            doc_locked: path
                .filter(|c| !c.allows(TabKind::Doc))
                .map(|_| Chapter::Writer.number()),
            pdf_locked: path
                .filter(|c| !c.offers_pdf())
                .map(|_| Chapter::Pdf.number()),
        };
        let closed: Vec<String> = self.closed.iter().map(|(t, _)| t.clone()).collect();
        let open_picker = std::mem::take(&mut self.open_picker);
        let events = chrome::title_bar(
            ui,
            &infos,
            self.active,
            &offer,
            open_picker,
            &mut self.tab_search,
            &closed,
        );
        for event in events {
            match event {
                chrome::Event::Select(i) => {
                    if i < self.tabs.len() {
                        self.active = i;
                    }
                }
                chrome::Event::Close(i) => self.close_tab(i),
                chrome::Event::Move { from, to } => self.move_tab(from, to),
                chrome::Event::New(kind) => self.new_tab(kind),
                chrome::Event::Open => self.open_dialog(),
                chrome::Event::PdfSoon => {
                    self.status = Some("PDF tabs arrive in the next update".into());
                }
                chrome::Event::Reopen(i) => self.reopen(i),
                chrome::Event::Locked(n) => {
                    self.status = Some(format!(
                        "That opens in Chapter {n}. Ask a parent to unlock it (top right → Path)."
                    ));
                }
            }
        }
    }

    /// AutoSave · Save · Undo · Redo · ⋯ menus | title | command search · mode.
    fn quick_bar(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let mut clicked = None;
        let tip = |c: Command| {
            commands::spec_of(c).map_or(String::new(), |s| match s.shortcut {
                Some(k) => format!("{} ({})", s.label, k.label()),
                None => s.label.to_owned(),
            })
        };
        let bar = ui.max_rect();
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let mut on = self.settings.autosave;
            if toggle_switch(ui, &mut on)
                .on_hover_text("AutoSave: files that have a name save themselves every few seconds")
                .changed()
            {
                self.settings.autosave = on;
                self.settings.save();
                self.status = Some(
                    if on {
                        "AutoSave on: files that have a name now save themselves"
                    } else {
                        "AutoSave off"
                    }
                    .into(),
                );
            }
            ui.label(RichText::new("AutoSave").size(12.0));
            ui.add_space(4.0);
            for (c, icon) in [
                (Command::Save, "💾"),
                (Command::Undo, "↶"),
                (Command::Redo, "↷"),
            ] {
                let button = egui::Button::new(RichText::new(icon).size(16.0)).frame(false);
                if ui.add(button).on_hover_text(tip(c)).clicked() {
                    clicked = Some(c);
                }
            }
            ui.menu_button(RichText::new("⋯").size(16.0), |ui| {
                if let Some(c) = self.classic_menus(ui) {
                    clicked = Some(c);
                }
            })
            .response
            .on_hover_text("All menus");

            // The document name and its state, centred like a title.
            if let Some(tab) = self.tab() {
                let state = if tab.is_dirty() {
                    if self.settings.autosave && tab.path().is_some() {
                        "Saving…"
                    } else {
                        "Edited"
                    }
                } else if tab.path().is_some() {
                    "Saved"
                } else {
                    "Not saved yet"
                };
                ui.painter().text(
                    bar.center(),
                    Align2::CENTER_CENTER,
                    format!("{} — {state}", tab.title()),
                    egui::FontId::proportional(13.0),
                    ui.visuals().text_color(),
                );
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // The mode switch: all together, or one app at a time.
                let mode = self.mode();
                let lock = if self.settings.locked() { " 🔒" } else { "" };
                let path = self.settings.path;
                let button = match path {
                    Some(c) => format!("📖 Chapter {}: {} ▾", c.number(), c.app()),
                    None => format!("◧ {}{lock} ▾", mode.short()),
                };
                ui.menu_button(button, |ui| {
                    // On the growing path the chapters replace the modes.
                    if let Some(current) = path {
                        ui.label(RichText::new("The growing path").strong());
                        for c in Chapter::ALL {
                            let mark = if c < current {
                                "✔"
                            } else if c == current {
                                "★"
                            } else {
                                "🔒"
                            };
                            let text =
                                format!("{mark}  {}. {} — {}", c.number(), c.app(), c.title());
                            ui.add_enabled(
                                c <= current,
                                egui::Button::new(text).selected(c == current),
                            );
                        }
                        ui.separator();
                        if let Some(next) = current.next()
                            && ui
                                .button(format!(
                                    "🔑 Unlock Chapter {}: {}…",
                                    next.number(),
                                    next.app()
                                ))
                                .clicked()
                        {
                            clicked = Some(Command::UnlockNextChapter);
                            ui.close();
                        }
                        if ui.button("🚪 Leave the growing path…").clicked() {
                            clicked = Some(Command::LeavePath);
                            ui.close();
                        }
                        return;
                    }
                    for (m, c) in [
                        (Mode::Studio, Command::ModeStudio),
                        (Mode::Notepad, Command::ModeNotepad),
                        (Mode::Writer, Command::ModeWriter),
                        (Mode::Pdf, Command::ModePdf),
                    ] {
                        let label = if m.is_available() {
                            m.label().to_owned()
                        } else {
                            format!("{} (coming soon)", m.label())
                        };
                        if ui
                            .add_enabled(
                                m.is_available(),
                                egui::Button::new(label).selected(m == mode),
                            )
                            .clicked()
                        {
                            clicked = Some(c);
                            ui.close();
                        }
                    }
                    ui.separator();
                    let lock_label = if self.settings.locked() {
                        "🔒 Remove parent lock…"
                    } else {
                        "🔓 Parent lock…"
                    };
                    if ui.button(lock_label).clicked() {
                        clicked = Some(Command::ParentLock);
                        ui.close();
                    }
                    if ui
                        .button("📖 Start the growing path…")
                        .on_hover_text("Notepad first; a parent unlocks the Writer, then PDF")
                        .clicked()
                    {
                        clicked = Some(Command::StartPath);
                        ui.close();
                    }
                })
                .response
                .on_hover_text("Use all apps together, just one, or the growing path for a child");

                // Search every command, like the search box of a word processor.
                let search = ui.add(
                    egui::TextEdit::singleline(&mut self.command_search)
                        .id(egui::Id::new("command-search"))
                        .hint_text("🔍  Search commands")
                        .desired_width(240.0),
                );
                let popup = egui::Id::new("command-search-results");
                if search.has_focus() && !self.command_search.is_empty() {
                    egui::Popup::open_id(ui.ctx(), popup);
                }
                let kind = self.kind();
                let found: Vec<_> = commands::search(&self.command_search)
                    .into_iter()
                    .filter(|s| s.works_in(kind) && mode.allows_command(s))
                    .take(12)
                    .collect();
                let mut picked = None;
                if search.lost_focus()
                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    && let Some(first) = found.first()
                {
                    picked = Some(first.command);
                }
                egui::Popup::from_response(&search)
                    .id(popup)
                    .open_memory(None)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.set_min_width(300.0);
                        if found.is_empty() {
                            ui.weak("No matching commands");
                        }
                        for spec in &found {
                            let key = spec.shortcut.map(Shortcut::label).unwrap_or_default();
                            let button = egui::Button::new(spec.label)
                                .shortcut_text(key)
                                .min_size(vec2(300.0, 0.0));
                            if ui.add(button).clicked() {
                                picked = Some(spec.command);
                            }
                        }
                    });
                if let Some(c) = picked {
                    clicked = Some(c);
                    self.command_search.clear();
                    egui::Popup::close_all(ui.ctx());
                }
            });
        });
        clicked
    }

    /// File · Home · Insert · … with the active one underlined.
    fn ribbon_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 16.0;
            if !ribbon::shows(self.ribbon_tab, self.mode()) {
                self.ribbon_tab = 1;
            }
            for (i, name) in ribbon::TABS.iter().enumerate() {
                if !ribbon::shows(i, self.mode()) {
                    continue;
                }
                let on = self.ribbon_tab == i && !self.ribbon_collapsed;
                let text = RichText::new(*name).size(13.0);
                let text = if on {
                    text.color(ACCENT).strong()
                } else {
                    text
                };
                let response = ui.add(
                    egui::Button::new(text)
                        .frame(false)
                        .min_size(vec2(0.0, 26.0)),
                );
                if on {
                    let r = response.rect;
                    ui.painter().line_segment(
                        [
                            egui::pos2(r.left() + 6.0, r.bottom()),
                            egui::pos2(r.right() - 6.0, r.bottom()),
                        ],
                        egui::Stroke::new(2.5_f32, ACCENT),
                    );
                }
                if response.double_clicked() {
                    self.ribbon_collapsed = !self.ribbon_collapsed;
                } else if response.clicked() {
                    self.ribbon_tab = i;
                    self.ribbon_collapsed = false;
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let arrow = if self.ribbon_collapsed { "⌄" } else { "⌃" };
                if ui
                    .add(egui::Button::new(arrow).frame(false))
                    .on_hover_text("Show or hide the ribbon (double-click a tab)")
                    .clicked()
                {
                    self.ribbon_collapsed = !self.ribbon_collapsed;
                }
                for (label, what) in [("↗ Share", "Sharing"), ("💬 Comments", "Comments")] {
                    if ui
                        .button(RichText::new(label).size(12.5))
                        .on_hover_text(format!("{what} (coming soon)"))
                        .clicked()
                    {
                        self.status = Some(format!("{what} is coming soon"));
                    }
                }
            });
        });
    }

    fn ribbon_body(&mut self, ui: &mut egui::Ui) -> Option<Act> {
        let tab = self.ribbon_tab;
        let mut state = ribbon::State {
            mode: self.mode(),
            ruler: self.settings.ruler,
            status_bar: self.show_status_bar,
            ..Default::default()
        };
        let doc = match self.tab_mut().map(|t| &mut t.content) {
            Some(Content::Doc(d)) => {
                state.doc = true;
                state.flags = [
                    Flag::Bold,
                    Flag::Italic,
                    Flag::Underline,
                    Flag::Strike,
                    Flag::Code,
                ]
                .map(|f| d.has(f));
                state.para = d.paragraph();
                state.read_only = d.read_only;
                Some(&mut **d)
            }
            Some(Content::Text(t)) => {
                state.word_wrap = t.word_wrap;
                None
            }
            Some(Content::Writer(_)) | None => None,
        };
        ribbon::body(ui, tab, &state, doc)
    }

    fn run_act(&mut self, act: Act, ctx: &egui::Context) {
        match act {
            Act::Cmd(c) => self.dispatch(c, ctx),
            Act::Soon(name) => self.status = Some(format!("{name} is coming soon")),
            Act::Paste => match arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                Ok(text) => self.send_to_editor(ctx, egui::Event::Paste(text)),
                Err(_) => self.status = Some("The clipboard has no text".into()),
            },
            Act::Cut => self.send_to_editor(ctx, egui::Event::Cut),
            Act::Copy => self.send_to_editor(ctx, egui::Event::Copy),
            Act::SelectAll => match self.tab_mut().map(|t| &mut t.content) {
                Some(Content::Text(t)) => t.select_all(ctx),
                Some(Content::Doc(d)) => d.select_all(),
                Some(Content::Writer(_)) | None => {}
            },
            Act::InsertText(text) => self.insert(ctx, &text),
            Act::InsertDate => {
                let now = chrono::Local::now().format("%d %B %Y, %H:%M").to_string();
                self.insert(ctx, &now);
            }
            Act::HorizontalLine => self.insert(ctx, &format!("\n{}\n", "─".repeat(40))),
            Act::WordCount => {
                let rows = match self.tab().map(|t| &t.content) {
                    Some(Content::Text(t)) => t.stats(),
                    Some(Content::Doc(d)) => d.stats(),
                    Some(Content::Writer(_)) | None => Vec::new(),
                };
                self.dialog = Some(Dialog::WordCount { rows });
            }
            Act::ToggleReadOnly => {
                if let Some(d) = self.doc_tab() {
                    d.read_only = !d.read_only;
                    let on = d.read_only;
                    self.status = Some(
                        if on {
                            "Editing restricted: you can read and copy, not change (Review → Restrict Editing to undo)"
                        } else {
                            "Editing allowed again"
                        }
                        .into(),
                    );
                }
            }
            Act::ToggleRuler => {
                self.settings.ruler = !self.settings.ruler;
                self.settings.save();
            }
            Act::Shortcuts => self.dialog = Some(Dialog::Shortcuts),
        }
    }

    /// Clipboard buttons act like the keyboard: focus the editor, then send the event.
    fn send_to_editor(&mut self, ctx: &egui::Context, event: egui::Event) {
        if let Some(t) = self.tab() {
            t.focus(ctx);
        }
        self.pending_input.push(event);
        ctx.request_repaint();
    }

    fn insert(&mut self, ctx: &egui::Context, text: &str) {
        match self.tab_mut().map(|t| &mut t.content) {
            Some(Content::Text(t)) => t.insert_text(ctx, text),
            Some(Content::Doc(d)) => {
                if d.read_only {
                    self.status =
                        Some("This document is read-only (Review → Restrict Editing)".into());
                } else {
                    d.insert_text(text);
                }
            }
            Some(Content::Writer(_)) | None => {}
        }
    }

    /// Saves named, changed tabs every 20 seconds when AutoSave is on.
    fn autosave(&mut self, ctx: &egui::Context) {
        if !self.settings.autosave {
            return;
        }
        ctx.request_repaint_after(Duration::from_secs(5));
        if self.last_autosave.elapsed() < Duration::from_secs(20) {
            return;
        }
        self.last_autosave = Instant::now();
        for tab in &mut self.tabs {
            if !tab.is_dirty() {
                continue;
            }
            // Only files that already have a name and a format; never ask.
            let _ = match &mut tab.content {
                Content::Text(t) if t.path().is_some() => t.session.doc.save(),
                Content::Doc(d) => match (d.path.clone(), d.format) {
                    (Some(path), Some(format)) => write_doc(d, path, format),
                    _ => continue,
                },
                // WordCraft saves through its own AutoSave.
                Content::Writer(_) | Content::Text(_) => continue,
            };
        }
        self.write_recovery();
    }

    fn find_bar(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let mut command = None;
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label("Find:");
            let needle = ui.add(
                egui::TextEdit::singleline(&mut self.find.options.needle)
                    .id(egui::Id::new("find-needle"))
                    .desired_width(220.0),
            );
            if std::mem::take(&mut self.find.focus_needle) {
                needle.request_focus();
            }
            changed |= needle.changed();
            if needle.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                command = Some(Command::FindNext);
            }
            if ui.button("Find Next").clicked() {
                command = Some(Command::FindNext);
            }
            if ui.button("Find Previous").clicked() {
                command = Some(Command::FindPrevious);
            }
            changed |= ui
                .checkbox(&mut self.find.options.match_case, "Match case")
                .changed();
            ui.toggle_value(&mut self.find.show_replace, "Replace…");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("✕").on_hover_text("Close (Esc)").clicked() {
                    self.find.open = false;
                }
            });
        });
        if self.find.show_replace {
            ui.horizontal(|ui| {
                ui.label("Replace:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.find.options.replacement)
                        .id(egui::Id::new("find-replacement"))
                        .desired_width(200.0),
                );
                if ui.button("Replace").clicked() {
                    command = Some(Command::ReplaceNext);
                }
                if ui.button("Replace All").clicked() {
                    command = Some(Command::ReplaceAll);
                }
            });
        }
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.find.open = false;
        }
        if changed {
            self.search_changed();
        }
        command
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        let Some(tab) = self.tab() else { return };
        let (line, column, words, format) = match &tab.content {
            Content::Text(t) => {
                let (l, c) = t.line_and_column();
                (l, c, t.word_count(), t.format_label())
            }
            Content::Doc(d) => {
                let (l, c) = d.line_and_column();
                (l, c, d.word_count(), d.format_label())
            }
            // WordCraft draws its own status bar.
            Content::Writer(_) => return,
        };
        ui.horizontal(|ui| {
            if let Some(message) = &self.status {
                ui.label(RichText::new(message).color(ACCENT));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(format);
                ui.separator();
                ui.label(format!("{:.0}%", self.zoom * 100.0));
                ui.separator();
                ui.label(format!("{words} words"));
                ui.separator();
                let place = if tab.kind() == TabKind::Doc {
                    "Para"
                } else {
                    "Ln"
                };
                ui.label(format!("{place} {line}, Col {column}"));
            });
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) -> Option<Command> {
        let kind = self.kind();
        let mode = self.mode();
        let pin_hash = self.settings.pin;
        let mut run = None;
        let mut close = false;
        let mut link: Option<Option<String>> = None;
        let mut unlocked: Option<Gate> = None;
        let mut path_after_pin = false;
        let mut new_pin: Option<u64> = None;
        match self.dialog.as_mut() {
            None => return None,
            Some(Dialog::WordCount { rows }) => {
                egui::Window::new("Word Count")
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(RichText::new("Statistics").strong());
                        egui::Grid::new("word-count")
                            .num_columns(2)
                            .spacing([40.0, 6.0])
                            .show(ui, |ui| {
                                for (name, value) in rows.iter() {
                                    ui.label(*name);
                                    ui.label(RichText::new(value).strong());
                                    ui.end_row();
                                }
                            });
                        ui.add_space(6.0);
                        if ui.button("Close").clicked()
                            || ui.input(|i| {
                                i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)
                            })
                        {
                            close = true;
                        }
                    });
            }
            Some(Dialog::Shortcuts) => {
                egui::Window::new("Keyboard shortcuts")
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(440.0)
                            .show(ui, |ui| {
                                egui::Grid::new("shortcuts")
                                    .num_columns(2)
                                    .striped(true)
                                    .spacing([40.0, 4.0])
                                    .show(ui, |ui| {
                                        for spec in
                                            CATALOG.iter().filter(|s| mode.allows_command(s))
                                        {
                                            if let Some(key) = spec.shortcut {
                                                ui.label(spec.label);
                                                ui.label(RichText::new(key.label()).monospace());
                                                ui.end_row();
                                            }
                                        }
                                    });
                            });
                        ui.add_space(6.0);
                        if ui.button("Close").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Escape))
                        {
                            close = true;
                        }
                    });
            }
            Some(Dialog::Unlock {
                then,
                pin,
                wrong,
                focus,
            }) => {
                egui::Window::new("Parent lock")
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label(then.ask());
                        let edit = ui.add(
                            egui::TextEdit::singleline(pin)
                                .password(true)
                                .hint_text("PIN")
                                .desired_width(160.0),
                        );
                        if std::mem::take(focus) {
                            edit.request_focus();
                        }
                        if *wrong {
                            ui.colored_label(
                                Color32::from_rgb(220, 60, 60),
                                "That PIN isn't right.",
                            );
                        }
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                if Some(modes::hash_pin(pin)) == pin_hash {
                                    unlocked = Some(*then);
                                } else {
                                    *wrong = true;
                                    pin.clear();
                                    *focus = true;
                                }
                            }
                            if ui.button("Cancel").clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Escape))
                            {
                                close = true;
                            }
                        });
                    });
            }
            Some(Dialog::SetLock {
                pin,
                confirm,
                error,
                focus,
                start_path,
            }) => {
                let start_path = *start_path;
                egui::Window::new("Parent lock")
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        if start_path {
                            ui.label(
                                "The growing path starts with the Notepad.\nChoose a parent PIN: only it can unlock the next chapter.",
                            );
                        } else {
                            ui.label(format!(
                                "Lock HarshPad in its current mode ({}).\nChanging the mode will then need this PIN.",
                                mode.label()
                            ));
                        }
                        ui.small("The lock keeps the app in one mode; it isn't a password for your files.");
                        egui::Grid::new("pin-grid").num_columns(2).show(ui, |ui| {
                            ui.label("New PIN (4–8 digits)");
                            let edit = ui.add(egui::TextEdit::singleline(pin).password(true).desired_width(140.0));
                            if std::mem::take(focus) {
                                edit.request_focus();
                            }
                            ui.end_row();
                            ui.label("Type it again");
                            ui.add(egui::TextEdit::singleline(confirm).password(true).desired_width(140.0));
                            ui.end_row();
                        });
                        if let Some(e) = error {
                            ui.colored_label(Color32::from_rgb(220, 60, 60), *e);
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Lock").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if !modes::valid_pin(pin) {
                                    *error = Some("Use 4 to 8 digits.");
                                } else if pin.trim() != confirm.trim() {
                                    *error = Some("The two PINs don't match.");
                                } else {
                                    new_pin = Some(modes::hash_pin(pin));
                                    path_after_pin = start_path;
                                }
                            }
                            if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                close = true;
                            }
                        });
                    });
            }
            Some(Dialog::Chapter { chapter, dont_show }) => {
                let chapter = *chapter;
                egui::Window::new("The growing path")
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .title_bar(false)
                    .show(ctx, |ui| {
                        ui.set_width(420.0);
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            let icon = match chapter {
                                Chapter::Notepad => crate::icons::AppIcon::Notepad,
                                Chapter::Writer => crate::icons::AppIcon::Writer,
                                Chapter::Pdf => crate::icons::AppIcon::Pdf,
                            };
                            let (r, _) =
                                ui.allocate_exact_size(vec2(52.0, 52.0), egui::Sense::hover());
                            crate::icons::paint(ui.painter(), r, icon, false);
                            ui.label(
                                RichText::new(format!("Chapter {}", chapter.number()))
                                    .size(13.0)
                                    .color(ACCENT),
                            );
                            ui.label(RichText::new(chapter.title()).size(24.0).strong());
                            ui.label(
                                RichText::new(format!("The {} is yours", chapter.app()))
                                    .size(14.0)
                                    .weak(),
                            );
                            ui.add_space(10.0);
                            ui.label(RichText::new(chapter.story()).size(14.0));
                            ui.add_space(14.0);
                            // The path so far: done, here, still locked.
                            ui.horizontal(|ui| {
                                let w = 3.0 * 120.0;
                                ui.add_space((ui.available_width() - w).max(0.0) / 2.0);
                                for c in Chapter::ALL {
                                    let (mark, colour) = if c < chapter {
                                        ("✔", Color32::from_rgb(60, 170, 90))
                                    } else if c == chapter {
                                        ("★", ACCENT)
                                    } else {
                                        ("🔒", ui.visuals().weak_text_color())
                                    };
                                    ui.allocate_ui(vec2(120.0, 40.0), |ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.label(RichText::new(mark).size(18.0).color(colour));
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}. {}",
                                                    c.number(),
                                                    c.app()
                                                ))
                                                .size(12.0)
                                                .color(colour),
                                            );
                                        });
                                    });
                                }
                            });
                            ui.add_space(8.0);
                            ui.checkbox(dont_show, "Don't show this again");
                            ui.add_space(8.0);
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Let's go").size(15.0))
                                        .min_size(vec2(140.0, 32.0)),
                                )
                                .clicked()
                                || ui.input(|i| {
                                    i.key_pressed(egui::Key::Enter)
                                        || i.key_pressed(egui::Key::Escape)
                                })
                            {
                                close = true;
                            }
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("HarshPad is dedicated to Harshal Pathak")
                                    .size(11.0)
                                    .weak(),
                            );
                        });
                    });
            }
            Some(Dialog::Welcome { dont_show }) => {
                egui::Window::new("Welcome")
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .title_bar(false)
                    .show(ctx, |ui| {
                        ui.set_width(460.0);
                        ui.vertical_centered(|ui| {
                            ui.add_space(8.0);
                            ui.label(RichText::new("Welcome to HarshPad").size(24.0).strong());
                            ui.label(RichText::new("Three apps in one window, opened one by one like chapters").weak());
                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                let w = 3.0 * 145.0;
                                ui.add_space((ui.available_width() - w).max(0.0) / 2.0);
                                for (n, icon, name, what, kind) in [
                                    (1, crate::icons::AppIcon::Notepad, "Notepad", "Plain text, fast", Some(TabKind::Text)),
                                    (2, crate::icons::AppIcon::Writer, "Writer Studio", "Documents with style", Some(TabKind::Doc)),
                                    (3, crate::icons::AppIcon::Pdf, "PDF Studio", "Coming soon", None),
                                ] {
                                    let on = kind.is_some_and(|k| mode.allows(k));
                                    let colour = if on { ACCENT } else { ui.visuals().weak_text_color() };
                                    ui.allocate_ui(vec2(145.0, 96.0), |ui| {
                                        ui.vertical_centered(|ui| {
                                            let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), egui::Sense::hover());
                                            crate::icons::paint(ui.painter(), r, icon, !on && kind.is_some());
                                            ui.label(RichText::new(format!("Chapter {n}")).size(11.0).color(colour));
                                            ui.label(RichText::new(name).size(15.0).strong());
                                            ui.label(RichText::new(what).size(11.5).weak());
                                            let state = match (on, kind) {
                                                (true, _) => "✔ open now",
                                                (false, Some(_)) => "off in this mode",
                                                (false, None) => "on its way",
                                            };
                                            ui.label(RichText::new(state).size(11.0).color(colour));
                                        });
                                    });
                                }
                            });
                            ui.add_space(12.0);
                            ui.label(format!("You're in {} mode.", mode.label()));
                            ui.label(
                                RichText::new(
                                    "Switch apps from ◧ at the top right. Notepad or Writer missing? Choose All together (Studio).",
                                )
                                .size(12.5)
                                .weak(),
                            );
                            ui.add_space(10.0);
                            ui.checkbox(dont_show, "Don't show this again");
                            ui.add_space(8.0);
                            if ui
                                .add(egui::Button::new(RichText::new("Let's go").size(15.0)).min_size(vec2(140.0, 32.0)))
                                .clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape))
                            {
                                close = true;
                            }
                            ui.add_space(6.0);
                            ui.label(RichText::new("HarshPad is dedicated to Harshal Pathak").size(11.0).weak());
                        });
                    });
            }
            Some(Dialog::Palette { query, focus }) => {
                egui::Window::new("Command palette")
                    .anchor(Align2::CENTER_TOP, vec2(0.0, 80.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        let edit = ui.add(
                            egui::TextEdit::singleline(query)
                                .hint_text("Type a command…")
                                .desired_width(380.0),
                        );
                        if std::mem::take(focus) {
                            edit.request_focus();
                        }
                        let found: Vec<_> = commands::search(query)
                            .into_iter()
                            .filter(|s| s.works_in(kind) && mode.allows_command(s))
                            .collect();
                        if ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && let Some(first) = found.first()
                        {
                            run = Some(first.command);
                        }
                        egui::ScrollArea::vertical()
                            .max_height(320.0)
                            .show(ui, |ui| {
                                for spec in found.iter().take(40) {
                                    let key =
                                        spec.shortcut.map(Shortcut::label).unwrap_or_default();
                                    if ui
                                        .add(
                                            egui::Button::new(spec.label)
                                                .shortcut_text(key)
                                                .min_size(vec2(380.0, 0.0)),
                                        )
                                        .clicked()
                                    {
                                        run = Some(spec.command);
                                    }
                                }
                            });
                        close |= ui.input(|i| i.key_pressed(egui::Key::Escape));
                    });
                close |= run.is_some();
            }
            Some(Dialog::Link { url, focus }) => {
                egui::Window::new("Link")
                    .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                    .collapsible(false)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Address (web page or e-mail):");
                        let edit = ui.add(
                            egui::TextEdit::singleline(url)
                                .hint_text("https://…")
                                .desired_width(360.0),
                        );
                        if std::mem::take(focus) {
                            edit.request_focus();
                        }
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                link = Some(Some(url.trim().to_owned()));
                            }
                            if ui.button("Remove link").clicked() {
                                link = Some(None);
                            }
                            if ui.button("Cancel").clicked()
                                || ui.input(|i| i.key_pressed(egui::Key::Escape))
                            {
                                close = true;
                            }
                        });
                    });
            }
        }
        if unlocked.is_some() || new_pin.is_some() || link.is_some() {
            close = true;
        }
        if close
            && matches!(
                self.dialog,
                Some(
                    Dialog::Chapter {
                        dont_show: true,
                        ..
                    } | Dialog::Welcome { dont_show: true }
                )
            )
        {
            self.settings.hide_welcome = true;
            self.settings.save();
        }
        if close {
            self.dialog = None;
            if let Some(t) = self.tab() {
                t.focus(ctx);
            }
        }
        // Actions run after closing, since some open the next dialog (a chapter card).
        match unlocked {
            Some(Gate::Mode(m)) => self.set_mode(m),
            Some(Gate::RemoveLock) => {
                self.settings.pin = None;
                self.settings.save();
                self.status = Some("Parent lock removed".into());
            }
            Some(Gate::NextChapter) => {
                if let Some(next) = self.settings.path.and_then(Chapter::next) {
                    self.enter_chapter(next);
                }
            }
            Some(Gate::LeavePath) => {
                self.settings.path = None;
                self.settings.save();
                self.enforce_mode();
                self.status = Some("Left the growing path; every mode is available again".into());
            }
            None => {}
        }
        if let Some(hash) = new_pin {
            self.settings.pin = Some(hash);
            self.settings.save();
            if path_after_pin {
                self.enter_chapter(Chapter::Notepad);
            } else {
                self.status = Some(format!("Locked in {} mode", mode.short()));
            }
        }
        if let Some(url) = link
            && let Some(d) = self.doc_tab()
        {
            d.set_link(url.filter(|u| !u.is_empty()));
        }
        run
    }
}

/// Turns a tab into another kind as a new, unsaved copy (no path, so nothing on disk
/// is overwritten in another format). Text becomes plain paragraphs when `plain`,
/// otherwise it is read as Markdown; documents become Markdown text.
fn convert(content: Content, kind: TabKind, id: u64, plain: bool) -> Content {
    match (content, kind) {
        (Content::Text(t), TabKind::Doc) => {
            let doc = if plain {
                RichDoc::from_plain(t.text())
            } else {
                markdown::read(t.text())
            };
            let mut d = DocTab::new(doc, None, None, id);
            d.name_hint = Some(format!("{} (copy)", t.title()));
            if t.is_dirty() {
                d.mark_dirty();
            }
            Content::Doc(Box::new(d))
        }
        (Content::Doc(d), TabKind::Text) => {
            let mut doc = Document::new();
            doc.text = markdown::write(&d.doc);
            if d.is_dirty() {
                doc.mark_changed();
            }
            let mut t = TextTab::new(doc, id);
            t.name_hint = Some(format!("{} (copy)", d.title()));
            Content::Text(Box::new(t))
        }
        (same, _) => same,
    }
}

fn write_doc(d: &mut DocTab, path: PathBuf, format: RichFormat) -> io::Result<()> {
    let bytes = harshpad_rich::encode(&d.doc, format).map_err(io::Error::other)?;
    write_atomically(&path, &bytes)?;
    d.set_saved(path, format);
    Ok(())
}

fn read_limited(path: &Path) -> io::Result<Vec<u8>> {
    if std::fs::metadata(path)?.len() > MAX_OPEN_BYTES {
        return Err(io::Error::other("the file is larger than 64 MB"));
    }
    std::fs::read(path)
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx_owned = ui.ctx().clone();
        let ctx = &ctx_owned;
        self.handle_close_request(ctx);
        self.handle_dropped_files(ctx);
        #[cfg(target_os = "macos")]
        if let Some(inbox) = &self.finder {
            let (files, quit) = crate::wordcraft_host::apple_events::drain(inbox);
            for f in files {
                self.open_path(&f);
            }
            if quit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        if !self.pending_input.is_empty() {
            let events = std::mem::take(&mut self.pending_input);
            ctx.input_mut(|i| i.events.extend(events));
        }
        self.handle_shortcuts(ctx);
        self.update_title(ctx);
        let copies = || self.dirty_copies();
        let tabs = copies();
        self.recovery.tick(|| tabs);
        self.autosave(ctx);
        chrome::resize_edges(ctx);

        let mut clicked = None;
        let mut act = None;
        // A WordCraft tab brings its own title bar, ribbon and status bar.
        let writer = matches!(self.tab().map(|t| &t.content), Some(Content::Writer(_)));
        egui::Panel::top("titlebar")
            .exact_size(chrome::BAR_H)
            .frame(egui::Frame::NONE)
            .show_separator_line(false)
            .show(ui, |ui| self.tab_bar(ui));
        if !writer {
            egui::Panel::top("quickbar")
                .show_separator_line(false)
                .frame(
                    egui::Frame::side_top_panel(&ctx.global_style())
                        .inner_margin(egui::Margin::symmetric(10, 5)),
                )
                .show(ui, |ui| clicked = self.quick_bar(ui));
            egui::Panel::top("ribbon")
                .frame(
                    egui::Frame::side_top_panel(&ctx.global_style())
                        .inner_margin(egui::Margin::symmetric(8, 2)),
                )
                .show(ui, |ui| {
                    self.ribbon_tabs(ui);
                    if !self.ribbon_collapsed {
                        ui.add_space(4.0);
                        act = self.ribbon_body(ui);
                        ui.add_space(2.0);
                    }
                });
            if self.find.open {
                egui::Panel::top("find").show(ui, |ui| clicked = clicked.or(self.find_bar(ui)));
            }
            if self.show_status_bar {
                egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
            }
        }
        let kind = self.kind();
        let backdrop = if kind == TabKind::Doc {
            if ctx.global_style().visuals.dark_mode {
                Color32::from_gray(48)
            } else {
                Color32::from_gray(222)
            }
        } else {
            ctx.global_style().visuals.extreme_bg_color
        };
        let zoom = self.zoom;
        let ruler = self.settings.ruler;
        let panel_frame = if writer {
            egui::Frame::NONE
        } else {
            egui::Frame::central_panel(&ctx.global_style()).fill(backdrop)
        };
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ui, |ui| {
                match self.tabs.get_mut(self.active).map(|t| &mut t.content) {
                    Some(Content::Text(t)) => t.ui(ui, TEXT_SIZE * zoom),
                    Some(Content::Doc(d)) => d.ui(ui, zoom, ruler),
                    Some(Content::Writer(w)) => w.ui(ui, frame),
                    None => {}
                }
            });
        if let Some(c) = self.dialogs(ctx) {
            clicked = clicked.or(Some(c));
        }
        if let Some(command) = clicked {
            self.dispatch(command, ctx);
        }
        if let Some(act) = act {
            self.run_act(act, ctx);
        }
        chrome::border(ctx);
    }

    fn on_exit(&mut self) {
        for tab in &mut self.tabs {
            if let Content::Writer(w) = &mut tab.content {
                w.save_prefs(true);
            }
        }
        crate::wordcraft_host::print::finish_pending_jobs();
    }
}

/// Commands that belong to the tab strip, so they work in every kind of tab.
fn tab_level(command: Command) -> bool {
    matches!(
        command,
        Command::NewTab
            | Command::NewText
            | Command::NewDocument
            | Command::CloseTab
            | Command::ReopenClosedTab
            | Command::NextTab
            | Command::PreviousTab
            | Command::Tab1
            | Command::Tab2
            | Command::Tab3
            | Command::Tab4
            | Command::Tab5
            | Command::Tab6
            | Command::Tab7
            | Command::Tab8
            | Command::LastTab
            | Command::CommandPalette
    )
}

/// Consumes the shortcut if it was pressed this frame.
fn consume(ctx: &egui::Context, shortcut: Shortcut) -> bool {
    let mut modifiers = Modifiers::NONE;
    if shortcut.ctrl {
        modifiers |= Modifiers::COMMAND;
    }
    if shortcut.shift {
        modifiers |= Modifiers::SHIFT;
    }
    if shortcut.alt {
        modifiers |= Modifiers::ALT;
    }
    egui_keys(shortcut.key)
        .iter()
        .any(|key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(modifiers, *key))))
}

fn egui_keys(key: Key) -> &'static [egui::Key] {
    use egui::Key as E;
    match key {
        Key::B => &[E::B],
        Key::E => &[E::E],
        Key::F => &[E::F],
        Key::H => &[E::H],
        Key::I => &[E::I],
        Key::J => &[E::J],
        Key::K => &[E::K],
        Key::L => &[E::L],
        Key::N => &[E::N],
        Key::O => &[E::O],
        Key::P => &[E::P],
        Key::R => &[E::R],
        Key::S => &[E::S],
        Key::T => &[E::T],
        Key::U => &[E::U],
        Key::W => &[E::W],
        Key::X => &[E::X],
        Key::Y => &[E::Y],
        Key::Z => &[E::Z],
        Key::Num0 => &[E::Num0],
        Key::Num1 => &[E::Num1],
        Key::Num2 => &[E::Num2],
        Key::Num3 => &[E::Num3],
        Key::Num4 => &[E::Num4],
        Key::Num5 => &[E::Num5],
        Key::Num6 => &[E::Num6],
        Key::Num7 => &[E::Num7],
        Key::Num8 => &[E::Num8],
        Key::Num9 => &[E::Num9],
        Key::F3 => &[E::F3],
        // "=" is the unshifted "+" key on most layouts.
        Key::Plus => &[E::Plus, E::Equals],
        Key::Minus => &[E::Minus],
        Key::OpenBracket => &[E::OpenBracket],
        Key::CloseBracket => &[E::CloseBracket],
        Key::Space => &[E::Space],
        Key::Tab => &[E::Tab],
    }
}

fn show_error(message: &str) {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(APP_NAME)
        .set_description(message)
        .show();
}

fn show_about() {
    MessageDialog::new()
        .set_title(APP_NAME)
        .set_description(format!(
            "{APP_NAME} {}\n\nDedicated to Harshal Pathak, who grows up with it:\nfirst the Notepad, then the Writer, then PDF.\n\nPlain text and rich documents in one fast app, written in Rust.\n\
             Tabs · .docx · Markdown · Find & Replace · crash recovery\n\n\
             Ctrl+Shift+P opens the command palette.",
            env!("CARGO_PKG_VERSION")
        ))
        .show();
}
