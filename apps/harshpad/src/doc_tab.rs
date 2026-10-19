//! A rich document tab: real pages laid out by `harshpad-layout` and an editor of our
//! own (caret, selection, keyboard, mouse, clipboard, IME). Every edit goes through
//! `RichDoc::apply_edit` so styles follow the text.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui::{
    self, Align2, Color32, CursorIcon, Event, EventFilter, FontId, ImeEvent, Key, Modifiers,
    PointerButton, Pos2, Sense, Stroke, pos2, vec2,
};
use harshpad_core::commands::Command;
use harshpad_core::find;
use harshpad_core::session::FindOptions;
use harshpad_core::undo::{EditKind, UndoStack};
use harshpad_layout::{Layout, PageSetup};
use harshpad_rich::{Align, CharStyle, Flag, ParaStyle, Paragraph, Rgb, RichDoc, RichFormat};

use crate::page::{self, AppMetrics, PX_PER_PT, Pages};
use crate::text_tab::Step;

#[derive(Clone)]
struct Snapshot {
    doc: RichDoc,
    caret: usize,
}

pub struct DocTab {
    pub doc: RichDoc,
    pub path: Option<PathBuf>,
    pub format: Option<RichFormat>,
    dirty: bool,
    undo: UndoStack<Snapshot>,
    /// Style for the next typed characters (Ctrl+B with nothing selected, …).
    typing: Option<CharStyle>,
    /// Selection as a byte range, and the caret (byte offset).
    sel: Range<usize>,
    caret: usize,
    current_match: Option<Range<usize>>,
    /// Selection to apply on the next frame (bytes).
    pending: Option<Range<usize>>,
    editor_id: egui::Id,
    /// Name shown until the tab is saved (e.g. "notes.txt (copy)").
    pub name_hint: Option<String>,
    /// Restrict Editing: the text can be read and copied but not changed.
    pub read_only: bool,
    /// The other end of the selection from the caret.
    anchor: usize,
    /// Column kept while moving up and down (points).
    preferred_x: Option<f32>,
    layout: Option<Layout>,
    /// Bumped on every change, so the layout knows when to redo itself.
    rev: u64,
    laid_rev: u64,
    scroll_to_caret: bool,
    /// Text the input method is composing (not yet committed).
    preedit: String,
}

impl DocTab {
    pub fn new(
        doc: RichDoc,
        path: Option<PathBuf>,
        format: Option<RichFormat>,
        tab_id: u64,
    ) -> Self {
        Self {
            doc,
            path,
            format,
            dirty: false,
            undo: UndoStack::default(),
            typing: None,
            sel: 0..0,
            caret: 0,
            current_match: None,
            pending: None,
            editor_id: egui::Id::new(("doc-editor", tab_id)),
            name_hint: None,
            read_only: false,
            anchor: 0,
            preferred_x: None,
            layout: None,
            rev: 1,
            laid_rev: 0,
            scroll_to_caret: false,
            preedit: String::new(),
        }
    }

    pub fn title(&self) -> String {
        match self.path.as_deref().and_then(Path::file_name) {
            Some(n) => n.to_string_lossy().into_owned(),
            None => self
                .name_hint
                .clone()
                .unwrap_or_else(|| "Untitled document".to_owned()),
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_pristine(&self) -> bool {
        !self.dirty && self.path.is_none() && self.doc.text().is_empty()
    }

    pub fn set_saved(&mut self, path: PathBuf, format: RichFormat) {
        self.path = Some(path);
        self.format = Some(format);
        self.dirty = false;
    }

    pub fn word_count(&self) -> usize {
        self.doc.word_count()
    }

    pub fn line_and_column(&self) -> (usize, usize) {
        let para = self.doc.para_index_at(self.caret);
        let start = self.doc.paragraphs().nth(para).map_or(0, |(_, r)| r.start);
        let col = self
            .doc
            .text()
            .get(start..self.caret.max(start))
            .map_or(0, |s| s.chars().count());
        (para + 1, col + 1)
    }

    pub fn format_label(&self) -> String {
        self.format.map_or_else(
            || "Document (not saved yet)".to_owned(),
            |f| f.label().to_owned(),
        )
    }

    pub fn focus(&self, ctx: &egui::Context) {
        ctx.memory_mut(|m| m.request_focus(self.editor_id));
    }

    // ---- Toolbar state ---------------------------------------------------------------

    pub fn paragraph(&self) -> Paragraph {
        self.doc.para_at(self.sel.start)
    }

    pub fn char_style(&self) -> CharStyle {
        self.typing
            .clone()
            .unwrap_or_else(|| self.doc.style_at(self.sel.start))
    }

    pub fn has(&self, flag: Flag) -> bool {
        if self.sel.is_empty() {
            let s = self.char_style();
            match flag {
                Flag::Bold => s.bold,
                Flag::Italic => s.italic,
                Flag::Underline => s.underline,
                Flag::Strike => s.strike,
                Flag::Code => s.code,
            }
        } else {
            self.doc.all_have(self.sel.clone(), flag)
        }
    }

    // ---- Undo --------------------------------------------------------------------------

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            doc: self.doc.clone(),
            caret: self.caret,
        }
    }

    fn record(&mut self, before: Snapshot, kind: EditKind) {
        let size = before.doc.text().len() * 2;
        self.undo.record(before, size, kind, Instant::now());
        self.rev += 1;
        self.dirty = true;
        self.current_match = None;
    }

    pub fn undo(&mut self) -> bool {
        let current = self.snapshot();
        let size = current.doc.text().len() * 2;
        match self.undo.undo(current, size) {
            Some(prev) => {
                self.restore(prev);
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        let current = self.snapshot();
        let size = current.doc.text().len() * 2;
        match self.undo.redo(current, size) {
            Some(next) => {
                self.restore(next);
                true
            }
            None => false,
        }
    }

    fn restore(&mut self, snap: Snapshot) {
        self.doc = snap.doc;
        self.rev += 1;
        let at = snap.caret.min(self.doc.text().len());
        self.pending = Some(at..at);
        self.dirty = true;
        self.typing = None;
        self.current_match = None;
    }

    // ---- Formatting --------------------------------------------------------------------

    /// Runs a Format command. Returns false for commands that aren't formatting.
    pub fn apply(&mut self, command: Command) -> bool {
        let r = self.sel.clone();
        let before = self.snapshot();
        let changed = match command {
            Command::Bold => self.toggle(Flag::Bold),
            Command::Italic => self.toggle(Flag::Italic),
            Command::Underline => self.toggle(Flag::Underline),
            Command::Strike => self.toggle(Flag::Strike),
            Command::InlineCode => self.toggle(Flag::Code),
            Command::StyleNormal => self.para(ParaStyle::Normal),
            Command::StyleTitle => self.para(ParaStyle::Title),
            Command::StyleHeading1 => self.para(ParaStyle::Heading1),
            Command::StyleHeading2 => self.para(ParaStyle::Heading2),
            Command::StyleHeading3 => self.para(ParaStyle::Heading3),
            Command::StyleQuote => self.para(ParaStyle::Quote),
            Command::StyleCode => self.para(ParaStyle::Code),
            Command::BulletList => {
                self.doc.toggle_list(r, ParaStyle::Bullet);
                true
            }
            Command::NumberedList => {
                self.doc.toggle_list(r, ParaStyle::Numbered);
                true
            }
            Command::AlignLeft => self.align(Align::Left),
            Command::AlignCenter => self.align(Align::Center),
            Command::AlignRight => self.align(Align::Right),
            Command::AlignJustify => self.align(Align::Justify),
            Command::BiggerText => self.resize(2),
            Command::SmallerText => self.resize(-2),
            Command::ClearFormatting => {
                self.typing = None;
                self.doc.clear_formatting(r);
                true
            }
            _ => return false,
        };
        if changed {
            self.record(before, EditKind::Other);
        }
        true
    }

    fn toggle(&mut self, flag: Flag) -> bool {
        if self.sel.is_empty() {
            let mut s = self.char_style();
            let on = !self.has(flag);
            flag.set(&mut s, on);
            self.typing = Some(s);
            false
        } else {
            self.doc.toggle(self.sel.clone(), flag);
            true
        }
    }

    fn para(&mut self, style: ParaStyle) -> bool {
        self.doc.set_para_style(self.sel.clone(), style);
        true
    }

    fn align(&mut self, align: Align) -> bool {
        self.doc.set_align(self.sel.clone(), align);
        true
    }

    fn resize(&mut self, step: i32) -> bool {
        let base = i32::from(
            self.char_style()
                .size
                .unwrap_or_else(|| self.paragraph().style.size()),
        );
        let size = (base + step).clamp(6, 96) as u16;
        self.char_change(|s| s.size = Some(size))
    }

    /// Applies a character change to the selection, or to what is typed next.
    fn char_change(&mut self, f: impl Fn(&mut CharStyle)) -> bool {
        if self.sel.is_empty() {
            let mut s = self.char_style();
            f(&mut s);
            self.typing = Some(s);
            false
        } else {
            self.doc.set_char_style(self.sel.clone(), f);
            true
        }
    }

    pub fn set_color(&mut self, color: Option<Rgb>) {
        let before = self.snapshot();
        if self.char_change(|s| s.color = color) {
            self.record(before, EditKind::Other);
        }
    }

    pub fn set_highlight(&mut self, color: Option<Rgb>) {
        let before = self.snapshot();
        if self.char_change(|s| s.highlight = color) {
            self.record(before, EditKind::Other);
        }
    }

    /// Links the selection to `url`; with nothing selected, inserts the address as a link.
    pub fn set_link(&mut self, url: Option<String>) {
        let before = self.snapshot();
        if self.sel.is_empty() {
            let Some(url) = url.filter(|u| !u.trim().is_empty()) else {
                return;
            };
            let mut text = self.doc.text().to_owned();
            text.insert_str(self.caret.min(text.len()), &url);
            let style = CharStyle {
                link: Some(url.clone()),
                ..self.char_style()
            };
            self.doc.apply_edit(&text, Some(&style));
            let end = self.caret + url.len();
            self.pending = Some(end..end);
        } else {
            self.doc
                .set_char_style(self.sel.clone(), |s| s.link = url.clone());
        }
        self.record(before, EditKind::Other);
    }

    /// The link under the caret or at the start of the selection.
    pub fn selected_link(&self) -> Option<String> {
        let at = if self.sel.is_empty() {
            self.caret
        } else {
            self.sel.start + 1
        };
        self.doc.style_at(at).link
    }

    // ---- Find / replace ----------------------------------------------------------------

    pub fn search(&mut self, options: &FindOptions, step: Step) -> bool {
        let text = self.doc.text().to_owned();
        if step == Step::ReplaceNext
            && let Some(m) = self.current_match.clone()
            && find::is_match(&text, &m, &options.needle, options.match_case)
        {
            let before = self.snapshot();
            let mut new = text.clone();
            new.replace_range(m.clone(), &options.replacement);
            let style = self.doc.style_at(m.start + 1);
            self.doc.apply_edit(&new, Some(&style));
            self.record(before, EditKind::Other);
            let end = m.start + options.replacement.len();
            self.current_match = Some(end..end);
        }
        let text = self.doc.text();
        let found = match step {
            Step::Previous => {
                let before = self.current_match.as_ref().map_or(self.caret, |m| m.start);
                find::find_previous(text, &options.needle, before, options.match_case)
            }
            _ => {
                let from = self.current_match.as_ref().map_or(self.caret, |m| m.end);
                find::find_next(text, &options.needle, from, options.match_case)
            }
        };
        self.current_match = found.clone();
        match found {
            Some(r) => {
                self.pending = Some(r);
                true
            }
            None => false,
        }
    }

    /// Replaces every match one at a time (last first), so each keeps its own style.
    pub fn replace_all(&mut self, options: &FindOptions) -> usize {
        let matches: Vec<Range<usize>> =
            find::matches(self.doc.text(), &options.needle, options.match_case).collect();
        if matches.is_empty() {
            return 0;
        }
        let before = self.snapshot();
        for m in matches.iter().rev() {
            let mut new = self.doc.text().to_owned();
            if new.get(m.clone()).is_none() {
                continue;
            }
            new.replace_range(m.clone(), &options.replacement);
            let style = self.doc.style_at(m.start + 1);
            self.doc.apply_edit(&new, Some(&style));
        }
        self.record(before, EditKind::Other);
        matches.len()
    }

    pub fn search_changed(&mut self) {
        self.current_match = None;
    }

    // ---- Ribbon actions ----------------------------------------------------------------

    /// Types `text` at the cursor, replacing any selection.
    pub fn insert_text(&mut self, text: &str) {
        let range = self.sel.clone();
        let style = self.char_style();
        self.replace(range, text, EditKind::Other, Some(style));
    }

    pub fn select_all(&mut self) {
        self.pending = Some(0..self.doc.text().len());
    }

    /// Word Count window rows.
    pub fn stats(&self) -> Vec<(&'static str, String)> {
        let text = self.doc.text();
        let words = self.doc.word_count();
        let pages = words.div_ceil(450).max(1);
        vec![
            ("Pages (estimated)", pages.to_string()),
            ("Words", words.to_string()),
            (
                "Characters (no spaces)",
                text.chars()
                    .filter(|c| !c.is_whitespace())
                    .count()
                    .to_string(),
            ),
            (
                "Characters (with spaces)",
                self.doc.char_count().to_string(),
            ),
            (
                "Paragraphs",
                self.doc
                    .paragraphs()
                    .filter(|(_, r)| !r.is_empty())
                    .count()
                    .to_string(),
            ),
        ]
    }

    // ---- Editing primitives -------------------------------------------------------------

    fn update_sel(&mut self) {
        self.sel = self.anchor.min(self.caret)..self.anchor.max(self.caret);
    }

    /// Moves the caret; `extend` keeps the anchor (Shift).
    fn move_to(&mut self, byte: usize, extend: bool) {
        let byte = byte.min(self.doc.text().len());
        if byte != self.caret {
            self.typing = None;
            self.undo.break_group();
            if self.current_match.as_ref().is_some_and(|m| m.end != byte) {
                self.current_match = None;
            }
        }
        self.caret = byte;
        if !extend {
            self.anchor = byte;
        }
        self.update_sel();
        self.scroll_to_caret = true;
    }

    /// Replaces `range` with `text`. Styles follow the text through `apply_edit`.
    fn replace(
        &mut self,
        range: Range<usize>,
        text: &str,
        kind: EditKind,
        style: Option<CharStyle>,
    ) {
        if self.read_only {
            return;
        }
        let len = self.doc.text().len();
        let start = range.start.min(len);
        let end = range.end.clamp(start, len);
        let mut new = self.doc.text().to_owned();
        if new.get(start..end).is_none() {
            return;
        }
        let before = self.snapshot();
        new.replace_range(start..end, text);
        let style = style.or_else(|| self.typing.clone());
        let outcome = self.doc.apply_edit(&new, style.as_ref());
        self.record(before, kind);
        let at = if outcome.adjusted {
            outcome.cursor
        } else {
            start + text.len()
        };
        self.caret = at.min(self.doc.text().len());
        self.anchor = self.caret;
        self.update_sel();
        self.preferred_x = None;
        self.scroll_to_caret = true;
    }

    fn type_text(&mut self, text: &str) {
        let range = self.sel.clone();
        self.replace(range, text, EditKind::Typing, None);
    }

    fn selected_text(&self) -> String {
        self.doc
            .text()
            .get(self.sel.clone())
            .unwrap_or_default()
            .to_owned()
    }

    fn delete(&mut self, forward: bool, word: bool) {
        if !self.sel.is_empty() {
            let range = self.sel.clone();
            self.replace(range, "", EditKind::Other, None);
            return;
        }
        let text = self.doc.text();
        let at = self.caret;
        let other = match (forward, word) {
            (false, false) => prev_char(text, at),
            (false, true) => prev_word(text, at),
            (true, false) => next_char(text, at),
            (true, true) => next_word(text, at),
        };
        let range = at.min(other)..at.max(other);
        if !range.is_empty() {
            self.replace(range, "", EditKind::Typing, None);
        }
    }

    /// Selects the word (or run of spaces) around `byte`.
    fn select_word(&mut self, byte: usize) {
        let text = self.doc.text();
        let start = word_start(text, byte);
        let end = word_end(text, byte);
        self.anchor = start;
        self.caret = end;
        self.update_sel();
    }

    fn select_paragraph(&mut self, byte: usize) {
        let i = self.doc.para_index_at(byte);
        let range = self.doc.paragraphs().nth(i).map(|(_, r)| r);
        if let Some(r) = range {
            self.anchor = r.start;
            self.caret = r.end;
            self.update_sel();
        }
    }

    // ---- Keyboard and mouse ----------------------------------------------------------

    fn handle_events(&mut self, ctx: &egui::Context, layout: &Layout) {
        let events = ctx.input(|i| i.events.clone());
        for event in events {
            match event {
                Event::Text(t) => self.type_text(&t),
                Event::Ime(ImeEvent::Commit(t)) => {
                    self.preedit.clear();
                    self.type_text(&t);
                }
                Event::Ime(ImeEvent::Preedit(t)) => self.preedit = t,
                Event::Ime(ImeEvent::Disabled) => self.preedit.clear(),
                Event::Copy => {
                    if !self.sel.is_empty() {
                        ctx.copy_text(self.selected_text());
                    }
                }
                Event::Cut => {
                    if !self.sel.is_empty() {
                        ctx.copy_text(self.selected_text());
                        let range = self.sel.clone();
                        self.replace(range, "", EditKind::Other, None);
                    }
                }
                Event::Paste(t) => {
                    let t = t.replace("\r\n", "\n").replace('\r', "\n");
                    let range = self.sel.clone();
                    let style = self.char_style();
                    self.replace(range, &t, EditKind::Other, Some(style));
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => self.key(key, modifiers, layout),
                _ => {}
            }
        }
    }

    fn key(&mut self, key: Key, m: Modifiers, layout: &Layout) {
        let mac = cfg!(target_os = "macos");
        let word = if mac { m.alt } else { m.ctrl };
        let line_edge = mac && m.mac_cmd;
        let extend = m.shift;
        let text_len = self.doc.text().len();
        let caret_x = || layout.caret(self.caret).map_or(0.0, |c| c.1);
        match key {
            Key::ArrowLeft | Key::ArrowRight => {
                let right = key == Key::ArrowRight;
                self.preferred_x = None;
                if !extend && !self.sel.is_empty() && !word && !line_edge {
                    let to = if right { self.sel.end } else { self.sel.start };
                    self.move_to(to, false);
                    return;
                }
                let text = self.doc.text();
                let to = match (right, word, line_edge) {
                    (_, _, true) => {
                        let b = layout.line_bounds(self.caret);
                        if right {
                            visual_end(layout, self.caret, text)
                        } else {
                            b.start
                        }
                    }
                    (true, true, _) => next_word(text, self.caret),
                    (false, true, _) => prev_word(text, self.caret),
                    (true, false, _) => next_char(text, self.caret),
                    (false, false, _) => prev_char(text, self.caret),
                };
                self.move_to(to, extend);
            }
            Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown => {
                let down = matches!(key, Key::ArrowDown | Key::PageDown);
                let steps = if matches!(key, Key::PageUp | Key::PageDown) {
                    25
                } else {
                    1
                };
                let x = self.preferred_x.unwrap_or_else(caret_x);
                let mut at = self.caret;
                for _ in 0..steps {
                    match layout.vertical_move(at, x, down) {
                        Some(next) => at = next,
                        None => {
                            at = if down { text_len } else { 0 };
                            break;
                        }
                    }
                }
                self.move_to(at, extend);
                self.preferred_x = Some(x);
            }
            Key::Home => {
                self.preferred_x = None;
                let to = if m.command {
                    0
                } else {
                    layout.line_bounds(self.caret).start
                };
                self.move_to(to, extend);
            }
            Key::End => {
                self.preferred_x = None;
                let to = if m.command {
                    text_len
                } else {
                    visual_end(layout, self.caret, self.doc.text())
                };
                self.move_to(to, extend);
            }
            Key::Backspace => self.delete(false, word),
            Key::Delete => self.delete(true, word),
            Key::Enter => {
                let range = self.sel.clone();
                self.replace(range, "\n", EditKind::Other, None);
            }
            Key::Tab if !m.command => {
                let range = self.sel.clone();
                self.replace(range, "\t", EditKind::Typing, None);
            }
            Key::A if m.command => {
                self.anchor = 0;
                self.caret = text_len;
                self.update_sel();
            }
            _ => {}
        }
    }

    // ---- Drawing --------------------------------------------------------------------

    pub fn ui(&mut self, ui: &mut egui::Ui, zoom: f32, show_ruler: bool) {
        let ctx = ui.ctx().clone();
        let setup = PageSetup::default();
        if let Some(r) = self.pending.take() {
            let len = self.doc.text().len();
            self.anchor = r.start.min(len);
            self.caret = r.end.min(len);
            self.update_sel();
            self.scroll_to_caret = true;
            ctx.memory_mut(|m| m.request_focus(self.editor_id));
        }
        self.relayout(&ctx, setup);
        let scale = PX_PER_PT * zoom;
        if show_ruler {
            Self::ruler(ui, setup.width * scale, setup.left * scale, zoom);
        }

        egui::ScrollArea::both()
            .id_salt(self.editor_id)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let Some(layout) = self.layout.take() else {
                    return;
                };
                let page_w = setup.width * scale;
                let width = (page_w + 64.0).max(ui.available_width());
                let probe = Pages::new(Pos2::ZERO, &setup, zoom);
                let height = probe.total_height(layout.pages) + 36.0;
                let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
                let at = Pages::new(
                    pos2(rect.center().x - page_w / 2.0, rect.top() + 18.0),
                    &setup,
                    zoom,
                );
                let response = ui
                    .interact(rect, self.editor_id, Sense::click_and_drag())
                    .on_hover_cursor(CursorIcon::Text);

                // Mouse: click places the caret, Shift+click extends, drag selects,
                // double-click selects a word, triple-click a paragraph.
                let pointer = ctx.input(|i| i.pointer.interact_pos());
                let hit = |p: Pos2| {
                    let (page, x, y) = at.page_point(p, layout.pages);
                    layout.hit(page, x, y)
                };
                if response.is_pointer_button_down_on()
                    && ctx.input(|i| i.pointer.primary_pressed())
                    && let Some(p) = pointer
                {
                    response.request_focus();
                    let byte = hit(p);
                    let shift = ctx.input(|i| i.modifiers.shift);
                    self.preferred_x = None;
                    self.move_to(byte, shift);
                }
                if response.dragged_by(PointerButton::Primary)
                    && let Some(p) = pointer
                {
                    self.move_to(hit(p), true);
                }
                if response.triple_clicked()
                    && let Some(p) = pointer
                {
                    self.select_paragraph(hit(p));
                } else if response.double_clicked()
                    && let Some(p) = pointer
                {
                    self.select_word(hit(p));
                }

                let focused = ctx.memory(|m| m.has_focus(self.editor_id));
                if focused {
                    ctx.memory_mut(|m| {
                        m.set_focus_lock_filter(
                            self.editor_id,
                            EventFilter {
                                tab: true,
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: false,
                            },
                        )
                    });
                    self.handle_events(&ctx, &layout);
                }
                // Edits change the layout; lay out again before drawing.
                let layout = if self.laid_rev == self.rev {
                    layout
                } else {
                    self.layout = None;
                    self.relayout(&ctx, setup);
                    match self.layout.take() {
                        Some(l) => l,
                        None => return,
                    }
                };
                let caret = focused.then_some(self.caret);
                page::paint(ui, &layout, &at, self.sel.clone(), caret);
                if let Some(r) = page::caret_rect(&layout, &at, self.caret) {
                    if focused {
                        ctx.output_mut(|o| {
                            o.ime = Some(egui::output::IMEOutput {
                                rect,
                                cursor_rect: r,
                            });
                        });
                    }
                    if std::mem::take(&mut self.scroll_to_caret) {
                        ui.scroll_to_rect(r.expand(48.0), None);
                    }
                }
                self.layout = Some(layout);
            });
    }

    fn relayout(&mut self, ctx: &egui::Context, setup: PageSetup) {
        if self.layout.is_none() || self.laid_rev != self.rev {
            let metrics = AppMetrics::new(ctx);
            self.layout = Some(Layout::new(&self.doc, setup, &metrics));
            self.laid_rev = self.rev;
        }
    }

    /// A ruler in inches above the page, numbered from the left margin, with
    /// markers at both margins.
    fn ruler(ui: &mut egui::Ui, page_w: f32, margin: f32, zoom: f32) {
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 22.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let page_left = rect.center().x - page_w / 2.0 - 5.0;
        let inch = 96.0 * zoom;
        let strip = egui::Rect::from_min_max(
            pos2(page_left, rect.top() + 3.0),
            pos2(page_left + page_w, rect.bottom() - 3.0),
        );
        painter.rect_filled(strip, 2.0, Color32::from_gray(205));
        let writing = egui::Rect::from_min_max(
            pos2(page_left + margin, strip.top()),
            pos2(page_left + page_w - margin, strip.bottom()),
        );
        painter.rect_filled(writing, 0.0, Color32::from_gray(250));
        let ink = Color32::from_gray(90);
        let zero = page_left + margin;
        let first = -((margin / (inch / 8.0)).floor() as i32);
        let last = ((page_w - margin) / (inch / 8.0)).ceil() as i32;
        for k in first..=last {
            let x = zero + k as f32 * inch / 8.0;
            if x < strip.left() || x > strip.right() {
                continue;
            }
            let mid = strip.center().y;
            if k % 8 == 0 && k != 0 {
                painter.text(
                    pos2(x, mid),
                    Align2::CENTER_CENTER,
                    (k / 8).abs().to_string(),
                    FontId::proportional(10.0),
                    ink,
                );
            } else if k % 4 == 0 {
                painter.line_segment(
                    [pos2(x, mid - 3.0), pos2(x, mid + 3.0)],
                    Stroke::new(1.0_f32, ink),
                );
            } else if k % 8 != 0 {
                painter.line_segment(
                    [pos2(x, mid - 1.0), pos2(x, mid + 1.0)],
                    Stroke::new(1.0_f32, ink.gamma_multiply(0.7)),
                );
            }
        }
        for x in [writing.left(), writing.right()] {
            let tri = vec![
                pos2(x - 5.0, strip.bottom()),
                pos2(x + 5.0, strip.bottom()),
                pos2(x, strip.bottom() - 6.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                tri,
                Color32::from_gray(120),
                Stroke::NONE,
            ));
        }
    }
}

fn prev_char(text: &str, at: usize) -> usize {
    text.get(..at.min(text.len()))
        .and_then(|s| s.char_indices().next_back())
        .map_or(0, |(i, _)| i)
}

fn next_char(text: &str, at: usize) -> usize {
    text.get(at.min(text.len())..)
        .and_then(|s| s.chars().next())
        .map_or(text.len(), |c| at + c.len_utf8())
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\''
}

/// Start of the previous word (Ctrl+Left).
fn prev_word(text: &str, at: usize) -> usize {
    let head = text.get(..at.min(text.len())).unwrap_or_default();
    let mut chars = head.char_indices().rev().peekable();
    // Skip spaces, then the word (or the run of punctuation).
    while chars
        .peek()
        .is_some_and(|(_, c)| c.is_whitespace() && *c != '\n')
    {
        chars.next();
    }
    let Some(&(first_i, first)) = chars.peek() else {
        return 0;
    };
    if first == '\n' {
        return first_i;
    }
    let word = is_word(first);
    let mut start = first_i;
    for (i, c) in chars {
        if c.is_whitespace() || is_word(c) != word {
            break;
        }
        start = i;
    }
    start
}

/// End of the next word, past the spaces after it (Ctrl+Right).
fn next_word(text: &str, at: usize) -> usize {
    let at = at.min(text.len());
    let tail = text.get(at..).unwrap_or_default();
    let mut chars = tail.char_indices().peekable();
    let Some(&(_, first)) = chars.peek() else {
        return text.len();
    };
    if first == '\n' {
        return at + 1;
    }
    let mut end = 0;
    if !first.is_whitespace() {
        let word = is_word(first);
        end = tail.len();
        for (i, c) in chars.by_ref() {
            if c.is_whitespace() || is_word(c) != word {
                end = i;
                break;
            }
        }
    }
    let rest = tail.get(end..).unwrap_or_default();
    let spaces: usize = rest
        .chars()
        .take_while(|c| c.is_whitespace() && *c != '\n')
        .map(char::len_utf8)
        .sum();
    at + end + spaces
}

fn word_start(text: &str, at: usize) -> usize {
    let head = text.get(..at.min(text.len())).unwrap_or_default();
    let Some(kind) = text
        .get(at.min(text.len())..)
        .and_then(|t| t.chars().next())
        .or_else(|| head.chars().next_back())
    else {
        return at;
    };
    let word = is_word(kind);
    let mut start = at.min(text.len());
    for (i, c) in head.char_indices().rev() {
        if c == '\n' || is_word(c) != word || (!word && !c.is_whitespace() != !kind.is_whitespace())
        {
            break;
        }
        start = i;
    }
    start
}

fn word_end(text: &str, at: usize) -> usize {
    let at = at.min(text.len());
    let tail = text.get(at..).unwrap_or_default();
    let Some(kind) = tail.chars().next() else {
        return at;
    };
    if kind == '\n' {
        return at;
    }
    let word = is_word(kind);
    let mut end = at;
    for (i, c) in tail.char_indices() {
        if c == '\n' || is_word(c) != word || (!word && c.is_whitespace() != kind.is_whitespace()) {
            break;
        }
        end = at + i + c.len_utf8();
    }
    end
}

/// End of the visual line: before the trailing space of a wrapped line, so the
/// caret stays on that line.
fn visual_end(layout: &Layout, at: usize, text: &str) -> usize {
    let i = layout.line_at(at);
    let Some(line) = layout.lines.get(i) else {
        return at;
    };
    let wraps = layout.lines.get(i + 1).is_some_and(|n| n.para == line.para);
    if !wraps {
        return line.range.end;
    }
    let s = text.get(line.range.clone()).unwrap_or_default();
    line.range.start + s.trim_end().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use harshpad_layout::{Face, Metrics};

    /// Every character 5.5 pt wide at 11 pt; lines 11 pt tall.
    struct Fixed;
    impl Metrics for Fixed {
        fn advance(&self, _: &Face, size: f32, _: char) -> f32 {
            size / 2.0
        }
        fn vertical(&self, _: &Face, size: f32) -> (f32, f32) {
            (size * 0.8, size * 0.2)
        }
    }

    fn tab(text: &str) -> DocTab {
        DocTab::new(RichDoc::from_plain(text), None, None, 1)
    }

    fn laid(t: &DocTab) -> Layout {
        Layout::new(&t.doc, PageSetup::default(), &Fixed)
    }

    /// What a real Ctrl press reports (Cmd on macOS, where word jumps use Alt).
    const CTRL: Modifiers = Modifiers {
        alt: cfg!(target_os = "macos"),
        ctrl: true,
        shift: false,
        mac_cmd: false,
        command: true,
    };

    fn press(t: &mut DocTab, key: Key, m: Modifiers) {
        let l = laid(t);
        t.key(key, m, &l);
    }

    #[test]
    fn typing_deleting_and_enter_edit_the_document() {
        let mut t = tab("Hello");
        t.move_to(5, false);
        t.type_text(" world");
        assert_eq!(t.doc.text(), "Hello world");
        press(&mut t, Key::Backspace, Modifiers::NONE);
        assert_eq!(t.doc.text(), "Hello worl");
        press(&mut t, Key::Backspace, CTRL);
        assert_eq!(t.doc.text(), "Hello ");
        press(&mut t, Key::Enter, Modifiers::NONE);
        t.type_text("Next");
        assert_eq!(t.doc.text(), "Hello \nNext");
        assert_eq!(t.doc.paras().len(), 2);
        press(&mut t, Key::Home, Modifiers::NONE);
        press(&mut t, Key::Delete, Modifiers::NONE);
        assert_eq!(t.doc.text(), "Hello \next");
        assert!(t.undo());
        assert_eq!(t.doc.text(), "Hello \nNext");
    }

    #[test]
    fn arrows_home_end_and_selection() {
        let mut t = tab("one two three\nfour");
        press(&mut t, Key::ArrowRight, CTRL);
        assert_eq!(t.caret, 4);
        press(&mut t, Key::ArrowRight, Modifiers::SHIFT);
        assert_eq!(t.sel, 4..5);
        press(&mut t, Key::ArrowLeft, Modifiers::NONE);
        assert_eq!((t.caret, t.sel.clone()), (4, 4..4));
        press(&mut t, Key::End, Modifiers::NONE);
        assert_eq!(t.caret, 13);
        press(&mut t, Key::ArrowDown, Modifiers::NONE);
        assert_eq!(t.caret, 18); // "four" is shorter: the end of it
        press(&mut t, Key::Home, CTRL);
        assert_eq!(t.caret, 0);
        press(&mut t, Key::A, CTRL);
        assert_eq!(t.sel, 0..18);
        t.type_text("x");
        assert_eq!(t.doc.text(), "x");
    }

    #[test]
    fn enter_continues_lists_and_read_only_blocks_edits() {
        let mut t = tab("item");
        t.doc.set_para_style(0..0, ParaStyle::Bullet);
        t.move_to(4, false);
        press(&mut t, Key::Enter, Modifiers::NONE);
        t.type_text("second");
        assert_eq!(
            t.doc.paras().get(1).map(|p| p.style),
            Some(ParaStyle::Bullet)
        );
        t.read_only = true;
        t.type_text("nope");
        press(&mut t, Key::Backspace, Modifiers::NONE);
        assert_eq!(t.doc.text(), "item\nsecond");
    }

    #[test]
    fn paste_and_word_selection() {
        let mut t = tab("alpha beta");
        t.select_word(7);
        assert_eq!(t.sel, 6..10);
        t.replace(
            t.sel.clone(),
            "gamma\r\nline".replace("\r\n", "\n").as_str(),
            EditKind::Other,
            None,
        );
        assert_eq!(t.doc.text(), "alpha gamma\nline");
        t.select_paragraph(13);
        assert_eq!(t.selected_text(), "line");
    }

    #[test]
    fn word_movement_follows_words_and_spaces() {
        let t = "Hello, big world\nNext";
        assert_eq!(next_word(t, 0), 5); // "Hello"
        assert_eq!(next_word(t, 5), 7); // "," then the space
        assert_eq!(next_word(t, 7), 11); // "big "
        assert_eq!(prev_word(t, 11), 7);
        assert_eq!(prev_word(t, 7), 5);
        assert_eq!(next_word(t, 16), 17); // over the paragraph mark
        assert_eq!(prev_char(t, 0), 0);
        assert_eq!(next_char("é", 0), 2);
        assert_eq!(word_start(t, 9), 7);
        assert_eq!(word_end(t, 9), 10);
    }
}
