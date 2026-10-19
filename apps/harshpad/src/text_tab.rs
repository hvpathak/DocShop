//! A plain-text tab: the classic notepad editor.

use std::ops::Range;
use std::path::Path;
use std::time::Instant;

use eframe::egui::{
    self,
    text::{CCursor, CCursorRange},
};
use harshpad_core::document::Document;
use harshpad_core::session::{FindOptions, Session};
use harshpad_core::text_pos;
use harshpad_core::undo::{EditKind, UndoStack};

/// Which way a search goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Next,
    Previous,
    ReplaceNext,
}

pub struct TextTab {
    pub session: Session,
    pub word_wrap: bool,
    undo: UndoStack<String>,
    /// The text as of the last frame (the "before" of the next edit).
    snapshot: String,
    /// Caret as a character index.
    caret: usize,
    editor_id: egui::Id,
    /// Name shown until the tab is saved (e.g. "notes.txt (copy)").
    pub name_hint: Option<String>,
}

impl TextTab {
    pub fn new(doc: Document, tab_id: u64) -> Self {
        Self {
            snapshot: doc.text.clone(),
            session: Session::new(doc),
            word_wrap: true,
            undo: UndoStack::default(),
            caret: 0,
            editor_id: egui::Id::new(("text-editor", tab_id)),
            name_hint: None,
        }
    }

    pub fn title(&self) -> String {
        match (&self.name_hint, &self.session.doc.path) {
            (Some(hint), None) => hint.clone(),
            _ => self.session.doc.display_name(),
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.session.doc.path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.session.doc.is_dirty()
    }

    /// An untouched new tab, which opening a file may replace.
    pub fn is_pristine(&self) -> bool {
        !self.is_dirty() && self.session.doc.path.is_none() && self.session.doc.text.is_empty()
    }

    pub fn text(&self) -> &str {
        &self.session.doc.text
    }

    pub fn line_and_column(&self) -> (usize, usize) {
        text_pos::line_and_column(&self.session.doc.text, self.caret)
    }

    pub fn word_count(&self) -> usize {
        self.session.doc.text.split_whitespace().count()
    }

    pub fn format_label(&self) -> String {
        format!(
            "{} · {}",
            self.session.doc.encoding.label(),
            self.session.doc.line_ending.label()
        )
    }

    pub fn focus(&self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.request_focus(self.editor_id));
    }

    // ---- Undo ------------------------------------------------------------------------

    fn record(&mut self, kind: EditKind) {
        let before = std::mem::replace(&mut self.snapshot, self.session.doc.text.clone());
        let size = before.len();
        self.undo.record(before, size, kind, Instant::now());
    }

    pub fn undo(&mut self) -> bool {
        let current = self.session.doc.text.clone();
        let size = current.len();
        match self.undo.undo(current, size) {
            Some(prev) => {
                self.restore(prev);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        let current = self.session.doc.text.clone();
        let size = current.len();
        match self.undo.redo(current, size) {
            Some(next) => {
                self.restore(next);
                true
            }
            None => false,
        }
    }

    fn restore(&mut self, text: String) {
        self.snapshot = text.clone();
        self.session.doc.text = text;
        self.session.text_edited();
        self.caret = self.caret.min(self.session.doc.text.chars().count());
    }

    // ---- Find / replace ----------------------------------------------------------------

    pub fn search(&mut self, ctx: &egui::Context, options: &FindOptions, step: Step) -> bool {
        self.session.find = options.clone();
        let caret = text_pos::byte_index(&self.session.doc.text, self.caret);
        let found = match step {
            Step::Next => self.session.find_next(caret),
            Step::Previous => self.session.find_previous(caret),
            Step::ReplaceNext => {
                let before = self.session.doc.text.clone();
                let found = self.session.replace_next(caret);
                if before != self.session.doc.text {
                    self.record(EditKind::Other);
                }
                found
            }
        };
        match found {
            Some(range) => {
                self.select(ctx, &range);
                true
            }
            None => false,
        }
    }

    pub fn replace_all(&mut self, options: &FindOptions) -> usize {
        self.session.find = options.clone();
        let count = self.session.replace_all();
        if count > 0 {
            self.record(EditKind::Other);
        }
        count
    }

    pub fn search_changed(&mut self) {
        self.session.search_changed();
    }

    fn select(&mut self, ctx: &egui::Context, range: &Range<usize>) {
        let text = &self.session.doc.text;
        let (start, end) = (
            text_pos::char_index(text, range.start),
            text_pos::char_index(text, range.end),
        );
        let mut state = egui::TextEdit::load_state(ctx, self.editor_id).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::two(
            CCursor::new(start),
            CCursor::new(end),
        )));
        state.store(ctx, self.editor_id);
        self.focus(ctx);
        self.caret = end;
    }

    // ---- Ribbon actions ----------------------------------------------------------------

    /// Types `text` at the caret.
    pub fn insert_text(&mut self, ctx: &egui::Context, text: &str) {
        let at = text_pos::byte_index(&self.session.doc.text, self.caret);
        self.session.doc.text.insert_str(at, text);
        self.record(EditKind::Other);
        self.session.text_edited();
        let end = at + text.len();
        self.select(ctx, &(end..end));
    }

    pub fn select_all(&mut self, ctx: &egui::Context) {
        let len = self.session.doc.text.len();
        self.select(ctx, &(0..len));
    }

    /// Word Count window rows.
    pub fn stats(&self) -> Vec<(&'static str, String)> {
        let text = &self.session.doc.text;
        vec![
            ("Lines", self.session.doc.line_count().to_string()),
            ("Words", self.word_count().to_string()),
            (
                "Characters (no spaces)",
                text.chars()
                    .filter(|c| !c.is_whitespace())
                    .count()
                    .to_string(),
            ),
            (
                "Characters (with spaces)",
                text.chars()
                    .filter(|c| *c != '\n' && *c != '\r')
                    .count()
                    .to_string(),
            ),
        ]
    }

    // ---- Drawing ---------------------------------------------------------------------

    pub fn ui(&mut self, ui: &mut egui::Ui, font_size: f32) {
        let font_id = egui::FontId::monospace(font_size);
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
        scroll
            .id_salt(self.editor_id)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let output = egui::TextEdit::multiline(&mut self.session.doc.text)
                    .id(self.editor_id)
                    .frame(false)
                    .lock_focus(true)
                    .desired_width(visible.x)
                    .min_size(visible)
                    .layouter(&mut layouter)
                    .show(ui);
                if output.response.changed() {
                    self.record(EditKind::Typing);
                    self.session.text_edited();
                }
                if let Some(range) = output.cursor_range {
                    let caret = range.primary.ccursor.index;
                    if caret != self.caret {
                        if !output.response.changed() {
                            self.undo.break_group();
                        }
                        self.caret = caret;
                        let byte = text_pos::byte_index(&self.session.doc.text, caret);
                        self.session.caret_moved(byte);
                    }
                }
            });
    }
}
