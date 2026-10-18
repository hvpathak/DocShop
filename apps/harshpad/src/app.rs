//! The egui shell. It draws the session and turns menu clicks and key presses into
//! catalog commands; the editing logic itself lives in `harshpad-core`.

use std::cmp::Reverse;
use std::io;
use std::ops::Range;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, KeyboardShortcut, Layout, Modifiers,
    text::{CCursor, CCursorRange},
};
use harshpad_core::commands::{self, CATALOG, Command, Key, Menu, Shortcut};
use harshpad_core::document::Document;
use harshpad_core::session::Session;
use harshpad_core::text_pos;
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

pub const APP_NAME: &str = "HarshPad";

const EDITOR_ID: &str = "editor";
const DEFAULT_FONT_SIZE: f32 = 15.0;
const MIN_FONT_SIZE: f32 = 6.0;
const MAX_FONT_SIZE: f32 = 72.0;
const ZOOM_STEP: f32 = 1.1;

#[derive(Default)]
struct FindBar {
    open: bool,
    show_replace: bool,
    /// Move keyboard focus into the search box on the next frame.
    focus_needle: bool,
}

pub struct NotepadApp {
    session: Session,
    word_wrap: bool,
    font_size: f32,
    show_status_bar: bool,
    find_bar: FindBar,
    /// Caret position as a character index (what egui reports).
    caret: usize,
    /// Set once the user agreed to close, so the next close request goes through.
    allow_close: bool,
    status_message: Option<String>,
    window_title: String,
}

impl NotepadApp {
    pub fn new(cc: &eframe::CreationContext<'_>, initial_file: Option<PathBuf>) -> Self {
        // Ctrl +/- zooms the text (Notepad behaviour), not the whole interface.
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        install_system_fonts(&cc.egui_ctx);

        let mut app = Self {
            session: Session::default(),
            word_wrap: true,
            font_size: DEFAULT_FONT_SIZE,
            show_status_bar: true,
            find_bar: FindBar::default(),
            caret: 0,
            allow_close: false,
            status_message: None,
            window_title: String::new(),
        };
        if let Some(path) = initial_file {
            app.load(&path);
        }
        app
    }

    // ---- Command dispatch ---------------------------------------------------

    /// Last-resort guard: a bug in a command shows an error and keeps the text.
    fn dispatch(&mut self, command: Command, ctx: &egui::Context) {
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| self.run(command, ctx)));
        if outcome.is_err() {
            let id = commands::spec_of(command).map_or("unknown", |s| s.id);
            show_error(&format!(
                "Something went wrong while running \"{id}\". Your text is still here; \
                 please save it."
            ));
        }
    }

    fn run(&mut self, command: Command, ctx: &egui::Context) {
        match command {
            Command::New => self.new_file(),
            Command::Open => self.open_file(),
            Command::Save => {
                self.save();
            }
            Command::SaveAs => {
                self.save_as();
            }
            Command::Exit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Command::Find => self.open_find_bar(false),
            Command::Replace => self.open_find_bar(true),
            Command::FindNext => self.search(ctx, Session::find_next),
            Command::FindPrevious => self.search(ctx, Session::find_previous),
            Command::ReplaceNext => self.search(ctx, Session::replace_next),
            Command::ReplaceAll => {
                let count = self.session.replace_all();
                self.status_message = Some(format!("Replaced {count} occurrence(s)"));
            }
            Command::WordWrap => self.word_wrap = !self.word_wrap,
            Command::StatusBar => self.show_status_bar = !self.show_status_bar,
            Command::ZoomIn => self.zoom(ZOOM_STEP),
            Command::ZoomOut => self.zoom(1.0 / ZOOM_STEP),
            Command::ZoomReset => self.font_size = DEFAULT_FONT_SIZE,
            Command::About => show_about(),
        }
    }

    fn toggle_state(&self, command: Command) -> Option<bool> {
        match command {
            Command::WordWrap => Some(self.word_wrap),
            Command::StatusBar => Some(self.show_status_bar),
            _ => None,
        }
    }

    // ---- File actions -------------------------------------------------------

    fn new_file(&mut self) {
        if self.confirm_discard() {
            self.replace_document(Document::new());
        }
    }

    fn open_file(&mut self) {
        if !self.confirm_discard() {
            return;
        }
        if let Some(path) = text_file_dialog().pick_file() {
            self.load(&path);
        }
    }

    fn load(&mut self, path: &Path) {
        match Document::open(path) {
            Ok(doc) => self.replace_document(doc),
            // Like Notepad, a missing file starts empty and saves to that path.
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                self.replace_document(Document::new_at(path));
            }
            Err(e) => show_error(&format!("Could not open {}:\n{e}", path.display())),
        }
    }

    /// Returns true if the document was saved.
    fn save(&mut self) -> bool {
        if self.session.doc.path.is_none() {
            return self.save_as();
        }
        let result = self.session.doc.save();
        self.report_save(result)
    }

    /// Returns true if the document was saved.
    fn save_as(&mut self) -> bool {
        let suggested = match &self.session.doc.path {
            Some(_) => self.session.doc.display_name(),
            None => "Untitled.txt".to_owned(),
        };
        let Some(path) = text_file_dialog().set_file_name(suggested).save_file() else {
            return false;
        };
        let result = self.session.doc.save_as(path);
        self.report_save(result)
    }

    fn report_save(&mut self, result: io::Result<()>) -> bool {
        match result {
            Ok(()) => {
                self.status_message = Some("Saved".to_owned());
                true
            }
            Err(e) => {
                show_error(&format!(
                    "Could not save {}:\n{e}",
                    self.session.doc.display_name()
                ));
                false
            }
        }
    }

    /// Asks to save unsaved changes. Returns false if the user cancelled.
    fn confirm_discard(&mut self) -> bool {
        if !self.session.doc.is_dirty() {
            return true;
        }
        let answer = MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title(APP_NAME)
            .set_description(format!(
                "Do you want to save changes to {}?",
                self.session.doc.display_name()
            ))
            .set_buttons(MessageButtons::YesNoCancel)
            .show();
        match answer {
            MessageDialogResult::Yes => self.save(),
            MessageDialogResult::No => true,
            _ => false,
        }
    }

    fn replace_document(&mut self, doc: Document) {
        self.session.set_document(doc);
        self.caret = 0;
        self.status_message = None;
    }

    // ---- Find / replace -----------------------------------------------------

    fn open_find_bar(&mut self, with_replace: bool) {
        self.find_bar.open = true;
        self.find_bar.show_replace |= with_replace;
        self.find_bar.focus_needle = true;
    }

    fn search(
        &mut self,
        ctx: &egui::Context,
        step: fn(&mut Session, usize) -> Option<Range<usize>>,
    ) {
        if self.session.find.needle.is_empty() {
            self.open_find_bar(false);
            return;
        }
        let caret = text_pos::byte_index(&self.session.doc.text, self.caret);
        match step(&mut self.session, caret) {
            Some(range) => {
                self.select_in_editor(ctx, &range);
                self.status_message = None;
            }
            None => {
                self.status_message = Some(format!("Cannot find \"{}\"", self.session.find.needle));
            }
        }
    }

    fn select_in_editor(&mut self, ctx: &egui::Context, range: &Range<usize>) {
        let text = &self.session.doc.text;
        let start = text_pos::char_index(text, range.start);
        let end = text_pos::char_index(text, range.end);

        let id = egui::Id::new(EDITOR_ID);
        let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(start),
            CCursor::new(end),
        )));
        state.store(ctx, id);
        ctx.memory_mut(|m| m.request_focus(id));
        self.caret = end;
    }

    // ---- View ---------------------------------------------------------------

    fn zoom(&mut self, factor: f32) {
        self.font_size = (self.font_size * factor).clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
    }

    fn zoom_percent(&self) -> u32 {
        (self.font_size / DEFAULT_FONT_SIZE * 100.0).round() as u32
    }

    // ---- Per-frame input ----------------------------------------------------

    fn handle_close_request(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.confirm_discard() {
                self.allow_close = true;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
    }

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()));
        if let Some(path) = dropped
            && self.confirm_discard()
        {
            self.load(&path);
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let mut bound: Vec<_> = CATALOG
            .iter()
            .filter_map(|s| s.shortcut.map(|k| (s.command, k)))
            .collect();
        bound.sort_by_key(|(_, k)| Reverse(k.modifier_count()));
        for (command, shortcut) in bound {
            if consume(ctx, shortcut) {
                self.dispatch(command, ctx);
            }
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let dirty = if self.session.doc.is_dirty() { "*" } else { "" };
        let title = format!("{dirty}{} - {APP_NAME}", self.session.doc.display_name());
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    // ---- Panels -------------------------------------------------------------

    fn menu_bar(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let mut clicked = None;
        egui::menu::bar(ui, |ui| {
            for menu in Menu::ALL {
                ui.menu_button(menu.label(), |ui| {
                    for spec in commands::in_menu(menu) {
                        if spec.separator_before {
                            ui.separator();
                        }
                        let shortcut = spec.shortcut.map(Shortcut::label).unwrap_or_default();
                        let mut button = egui::Button::new(spec.label).shortcut_text(shortcut);
                        if let Some(on) = self.toggle_state(spec.command) {
                            button = button.selected(on);
                        }
                        if ui.add(button).clicked() {
                            clicked = Some(spec.command);
                            ui.close_menu();
                        }
                    }
                });
            }
        });
        clicked
    }

    fn find_bar(&mut self, ui: &mut egui::Ui) -> Option<Command> {
        let mut command = None;
        ui.horizontal(|ui| {
            ui.label("Find:");
            let needle = ui.add(
                egui::TextEdit::singleline(&mut self.session.find.needle).desired_width(220.0),
            );
            if std::mem::take(&mut self.find_bar.focus_needle) {
                needle.request_focus();
            }
            if needle.changed() {
                self.session.search_changed();
            }
            if needle.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                command = Some(Command::FindNext);
            }
            if ui.button("Find Next").clicked() {
                command = Some(Command::FindNext);
            }
            if ui.button("Find Previous").clicked() {
                command = Some(Command::FindPrevious);
            }
            if ui
                .checkbox(&mut self.session.find.match_case, "Match case")
                .changed()
            {
                self.session.search_changed();
            }
            ui.toggle_value(&mut self.find_bar.show_replace, "Replace…");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui.button("✕").on_hover_text("Close (Esc)").clicked() {
                    self.find_bar.open = false;
                }
            });
        });
        if self.find_bar.show_replace {
            ui.horizontal(|ui| {
                ui.label("Replace:");
                ui.add(
                    egui::TextEdit::singleline(&mut self.session.find.replacement)
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
            self.find_bar.open = false;
        }
        command
    }

    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(message) = &self.status_message {
                ui.label(message);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let doc = &self.session.doc;
                ui.label(doc.encoding.label());
                ui.separator();
                ui.label(doc.line_ending.label());
                ui.separator();
                ui.label(format!("{}%", self.zoom_percent()));
                ui.separator();
                let (line, column) = text_pos::line_and_column(&doc.text, self.caret);
                ui.label(format!("Ln {line}, Col {column}"));
            });
        });
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let font_id = egui::FontId::monospace(self.font_size);
        let word_wrap = self.word_wrap;
        let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
            let width = if word_wrap { wrap_width } else { f32::INFINITY };
            let job = egui::text::LayoutJob::simple(
                text.to_owned(),
                font_id.clone(),
                ui.visuals().text_color(),
                width,
            );
            ui.fonts(|f| f.layout_job(job))
        };

        // Measure before entering the scroll area, whose inner width can be unbounded.
        let visible = ui.available_size();
        let scroll = if word_wrap {
            egui::ScrollArea::vertical()
        } else {
            egui::ScrollArea::both()
        };
        scroll.auto_shrink([false, false]).show(ui, |ui| {
            let output = egui::TextEdit::multiline(&mut self.session.doc.text)
                .id(egui::Id::new(EDITOR_ID))
                .frame(false)
                .lock_focus(true)
                .desired_width(visible.x)
                .min_size(visible)
                .layouter(&mut layouter)
                .show(ui);

            if output.response.changed() {
                self.session.text_edited();
            }
            if let Some(range) = output.cursor_range {
                let caret = range.primary.ccursor.index;
                if caret != self.caret {
                    self.caret = caret;
                    let byte = text_pos::byte_index(&self.session.doc.text, caret);
                    self.session.caret_moved(byte);
                }
            }
        });
    }
}

impl eframe::App for NotepadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_close_request(ctx);
        self.handle_dropped_files(ctx);
        self.handle_shortcuts(ctx);
        self.update_title(ctx);

        let mut clicked = None;
        egui::TopBottomPanel::top("menu").show(ctx, |ui| clicked = self.menu_bar(ui));
        if self.find_bar.open {
            egui::TopBottomPanel::top("find").show(ctx, |ui| {
                clicked = clicked.or(self.find_bar(ui));
            });
        }
        if self.show_status_bar {
            egui::TopBottomPanel::bottom("status").show(ctx, |ui| self.status_bar(ui));
        }
        let editor_frame =
            egui::Frame::central_panel(&ctx.style()).fill(ctx.style().visuals.extreme_bg_color);
        egui::CentralPanel::default()
            .frame(editor_frame)
            .show(ctx, |ui| self.editor(ui));

        // Run after drawing so dialogs never open from inside a menu.
        if let Some(command) = clicked {
            self.dispatch(command, ctx);
        }
    }
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
    egui_keys(shortcut.key)
        .iter()
        .any(|key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(modifiers, *key))))
}

fn egui_keys(key: Key) -> &'static [egui::Key] {
    match key {
        Key::N => &[egui::Key::N],
        Key::O => &[egui::Key::O],
        Key::S => &[egui::Key::S],
        Key::F => &[egui::Key::F],
        Key::H => &[egui::Key::H],
        Key::F3 => &[egui::Key::F3],
        // "=" is the unshifted "+" key on most layouts.
        Key::Plus => &[egui::Key::Plus, egui::Key::Equals],
        Key::Minus => &[egui::Key::Minus],
        Key::Num0 => &[egui::Key::Num0],
    }
}

/// Uses the fonts Windows already has (Notepad's Consolas, Segoe UI, and symbol
/// fonts for characters like ✓) instead of bundling font files. Any font that is
/// missing is skipped, and egui's built-in fonts remain as the last fallback.
fn install_system_fonts(ctx: &egui::Context) {
    let Some(windows_dir) = std::env::var_os("WINDIR") else {
        return;
    };
    let font_dir = PathBuf::from(windows_dir).join("Fonts");
    let mut fonts = egui::FontDefinitions::default();
    let mut add = |name: &str, file: &str, families: &[egui::FontFamily], first: bool| {
        let Ok(bytes) = std::fs::read(font_dir.join(file)) else {
            return;
        };
        fonts.font_data.insert(
            name.to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(bytes)),
        );
        for family in families {
            let list = fonts.families.entry(family.clone()).or_default();
            if first {
                list.insert(0, name.to_owned());
            } else {
                list.push(name.to_owned());
            }
        }
    };
    use egui::FontFamily::{Monospace, Proportional};
    add("consolas", "consola.ttf", &[Monospace], true);
    add("segoe-ui", "segoeui.ttf", &[Proportional], true);
    add(
        "segoe-symbol",
        "seguisym.ttf",
        &[Monospace, Proportional],
        false,
    );
    add(
        "segoe-emoji",
        "seguiemj.ttf",
        &[Monospace, Proportional],
        false,
    );
    ctx.set_fonts(fonts);
}

fn text_file_dialog() -> FileDialog {
    FileDialog::new()
        .add_filter("Text Documents (*.txt)", &["txt"])
        .add_filter("All Files", &["*"])
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
            "{APP_NAME} {}\n\nA clean-room plain-text editor written in Rust.\n\
             Undo: Ctrl+Z   Redo: Ctrl+Y   Select all: Ctrl+A",
            env!("CARGO_PKG_VERSION")
        ))
        .show();
}
