//! A rich document tab: a white page with styled text. The text is edited by egui's
//! TextEdit; a layouter paints it with the document's styles, and every edit is
//! fed back through `RichDoc::apply_edit` so styles follow the text.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Stroke, pos2,
    text::{CCursor, CCursorRange, LayoutJob, TextFormat},
    vec2,
};
use harshpad_core::commands::Command;
use harshpad_core::find;
use harshpad_core::session::FindOptions;
use harshpad_core::text_pos;
use harshpad_core::undo::{EditKind, UndoStack};
use harshpad_rich::{Align, CharStyle, Flag, ParaStyle, Paragraph, Rgb, RichDoc, RichFormat};

use crate::fonts::BOLD;
use crate::text_tab::Step;

/// A4 at 96 dpi, and a one-inch margin.
const PAGE_W: f32 = 794.0;
const PAGE_H: f32 = 1123.0;
const MARGIN: f32 = 72.0;

#[derive(Clone)]
struct Snapshot {
    doc: RichDoc,
    caret: usize,
}

pub struct DocTab {
    pub doc: RichDoc,
    /// The text the editor widget edits; equal to `doc.text()` between frames.
    buffer: String,
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
}

impl DocTab {
    pub fn new(
        doc: RichDoc,
        path: Option<PathBuf>,
        format: Option<RichFormat>,
        tab_id: u64,
    ) -> Self {
        Self {
            buffer: doc.text().to_owned(),
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
        self.buffer = self.doc.text().to_owned();
        let at = snap.caret.min(self.buffer.len());
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
            let mut text = self.buffer.clone();
            text.insert_str(self.caret.min(text.len()), &url);
            let style = CharStyle {
                link: Some(url.clone()),
                ..self.char_style()
            };
            self.doc.apply_edit(&text, Some(&style));
            self.buffer = self.doc.text().to_owned();
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
            self.buffer = self.doc.text().to_owned();
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
        self.buffer = self.doc.text().to_owned();
        self.record(before, EditKind::Other);
        matches.len()
    }

    pub fn search_changed(&mut self) {
        self.current_match = None;
    }

    // ---- Drawing --------------------------------------------------------------------

    pub fn ui(&mut self, ui: &mut egui::Ui, zoom: f32) {
        let ctx = ui.ctx().clone();
        if let Some(r) = self.pending.take() {
            let start = text_pos::char_index(&self.buffer, r.start);
            let end = text_pos::char_index(&self.buffer, r.end);
            let mut state = egui::TextEdit::load_state(&ctx, self.editor_id).unwrap_or_default();
            state.cursor.set_char_range(Some(CCursorRange::two(
                CCursor::new(start),
                CCursor::new(end),
            )));
            state.store(&ctx, self.editor_id);
            ctx.memory_mut(|m| m.request_focus(self.editor_id));
        }

        let avail = ui.available_width();
        let page_w = (PAGE_W * zoom).min(avail - 32.0).max(260.0);
        let margin = (MARGIN * zoom).min(page_w * 0.12);
        let inner_w = page_w - 2.0 * margin;
        let inner_h = PAGE_H * zoom - 2.0 * margin;

        let mut changed = false;
        let mut cursor: Option<(usize, usize)> = None;
        let DocTab {
            doc,
            buffer,
            editor_id,
            ..
        } = self;
        let doc_ref: &RichDoc = doc;
        egui::ScrollArea::vertical()
            .id_salt(*editor_id)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(18.0);
                ui.vertical_centered(|ui| {
                    egui::Frame::NONE
                        .fill(Color32::WHITE)
                        .inner_margin(margin)
                        .corner_radius(3.0)
                        .shadow(egui::epaint::Shadow {
                            offset: [0, 4],
                            blur: 16,
                            spread: 0,
                            color: Color32::from_black_alpha(70),
                        })
                        .show(ui, |ui| {
                            ui.set_width(inner_w);
                            let v = ui.visuals_mut();
                            v.text_cursor.stroke.color = Color32::from_rgb(20, 20, 20);
                            v.selection.bg_fill = Color32::from_rgb(180, 214, 255);
                            let mut layouter = |ui: &egui::Ui, text: &str, wrap: f32| {
                                let job = layout_job(doc_ref, text, wrap, zoom, ui);
                                ui.fonts(|f| f.layout_job(job))
                            };
                            let output = egui::TextEdit::multiline(buffer)
                                .id(*editor_id)
                                .frame(false)
                                .lock_focus(true)
                                .desired_width(inner_w)
                                .min_size(vec2(inner_w, inner_h))
                                .layouter(&mut layouter)
                                .show(ui);
                            paint_list_markers(ui, doc_ref, &output, zoom);
                            changed = output.response.changed();
                            cursor = output
                                .cursor_range
                                .map(|r| (r.primary.ccursor.index, r.secondary.ccursor.index));
                        });
                });
                ui.add_space(28.0);
            });

        if changed {
            let before = self.snapshot();
            let outcome = self.doc.apply_edit(&self.buffer, self.typing.as_ref());
            self.record(before, EditKind::Typing);
            if outcome.adjusted {
                self.buffer = self.doc.text().to_owned();
                self.pending = Some(outcome.cursor..outcome.cursor);
            }
        }
        if let Some((primary, secondary)) = cursor {
            let p = text_pos::byte_index(&self.buffer, primary);
            let s = text_pos::byte_index(&self.buffer, secondary);
            if p != self.caret && !changed {
                self.typing = None;
                self.undo.break_group();
                if self.current_match.as_ref().is_some_and(|m| m.end != p) {
                    self.current_match = None;
                }
            }
            self.caret = p;
            self.sel = p.min(s)..p.max(s);
        }
    }
}

fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

fn indent(style: ParaStyle, zoom: f32) -> f32 {
    zoom * match style {
        ParaStyle::Bullet | ParaStyle::Numbered => 28.0,
        ParaStyle::Quote => 22.0,
        ParaStyle::Code => 12.0,
        _ => 0.0,
    }
}

fn char_format(para: ParaStyle, s: &CharStyle, zoom: f32) -> TextFormat {
    let points = s.size.map_or_else(|| f32::from(para.size()), f32::from);
    let px = points * 4.0 / 3.0 * zoom;
    let mono = s.code || para == ParaStyle::Code;
    let bold = s.bold
        || matches!(
            para,
            ParaStyle::Heading1 | ParaStyle::Heading2 | ParaStyle::Heading3
        );
    let family = if mono {
        FontFamily::Monospace
    } else if bold {
        FontFamily::Name(BOLD.into())
    } else {
        FontFamily::Proportional
    };
    let default_color = match para {
        ParaStyle::Title => Color32::from_rgb(17, 17, 17),
        ParaStyle::Heading1 | ParaStyle::Heading2 | ParaStyle::Heading3 => {
            Color32::from_rgb(24, 72, 140)
        }
        ParaStyle::Quote => Color32::from_rgb(95, 95, 95),
        _ => Color32::from_rgb(32, 32, 32),
    };
    let color = s.color.map(rgb).unwrap_or(if s.link.is_some() {
        Color32::from_rgb(6, 95, 212)
    } else {
        default_color
    });
    let line = Stroke::new((zoom).max(1.0), color);
    TextFormat {
        font_id: FontId::new(px, family),
        color,
        background: s.highlight.map(rgb).unwrap_or(if mono {
            Color32::from_rgb(242, 243, 245)
        } else {
            Color32::TRANSPARENT
        }),
        italics: s.italic || para == ParaStyle::Quote,
        underline: if s.underline || s.link.is_some() {
            line
        } else {
            Stroke::NONE
        },
        strikethrough: if s.strike { line } else { Stroke::NONE },
        // Headings get a taller line: breathing room above them, like a printed page.
        line_height: para.is_heading().then_some(px * 1.85),
        ..Default::default()
    }
}

/// Builds the styled layout for the editor. If the widget's text differs from the
/// document (the single frame between an edit and `apply_edit`), it is drawn plain.
fn layout_job(doc: &RichDoc, text: &str, wrap: f32, zoom: f32, ui: &egui::Ui) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    if text != doc.text() {
        job.append(
            text,
            0.0,
            char_format(ParaStyle::Normal, &CharStyle::default(), zoom),
        );
        return job;
    }
    let runs: Vec<(CharStyle, Range<usize>)> =
        doc.styled_ranges().map(|(s, r)| (s.clone(), r)).collect();
    let paras: Vec<(Paragraph, Range<usize>)> = doc.paragraphs().collect();
    let last = paras.len().saturating_sub(1);
    for (i, (para, range)) in paras.iter().enumerate() {
        let lead = indent(para.style, zoom);
        let pieces: Vec<(Range<usize>, TextFormat)> = runs
            .iter()
            .filter_map(|(s, r)| {
                let (start, end) = (r.start.max(range.start), r.end.min(range.end));
                (start < end).then(|| (start..end, char_format(para.style, s, zoom)))
            })
            .collect();
        let offset = align_offset(para, &pieces, text, wrap - lead, ui);
        let mut first = true;
        for (r, fmt) in pieces {
            if let Some(t) = text.get(r) {
                job.append(t, if first { lead + offset } else { 0.0 }, fmt);
                first = false;
            }
        }
        if i < last {
            let mut nl = char_format(para.style, &CharStyle::default(), zoom);
            nl.background = Color32::TRANSPARENT;
            job.append("\n", if first { lead } else { 0.0 }, nl);
        }
    }
    job
}

/// Centre or right alignment for paragraphs that fit on one line.
fn align_offset(
    para: &Paragraph,
    pieces: &[(Range<usize>, TextFormat)],
    text: &str,
    room: f32,
    ui: &egui::Ui,
) -> f32 {
    let factor = match para.align {
        Align::Center => 0.5,
        Align::Right => 1.0,
        _ => return 0.0,
    };
    if pieces.is_empty() || para.style == ParaStyle::Code {
        return 0.0;
    }
    let mut probe = LayoutJob::default();
    for (r, fmt) in pieces {
        if let Some(t) = text.get(r.clone()) {
            probe.append(t, 0.0, fmt.clone());
        }
    }
    let width = ui.fonts(|f| f.layout_job(probe)).size().x;
    if width < room {
        (room - width) * factor
    } else {
        0.0
    }
}

/// Draws bullets and numbers in the indent of list paragraphs.
fn paint_list_markers(
    ui: &egui::Ui,
    doc: &RichDoc,
    output: &egui::text_edit::TextEditOutput,
    zoom: f32,
) {
    let mut number = 0u32;
    for (para, range) in doc.paragraphs() {
        number = if para.style == ParaStyle::Numbered {
            number + 1
        } else {
            0
        };
        if !para.style.is_list() {
            continue;
        }
        let ci = text_pos::char_index(doc.text(), range.start);
        let rect = output.galley.pos_from_ccursor(CCursor::new(ci));
        let y = output.galley_pos.y + rect.center().y;
        let x = output.galley_pos.x + 6.0 * zoom;
        let label = if para.style == ParaStyle::Bullet {
            "•".to_owned()
        } else {
            format!("{number}.")
        };
        ui.painter().text(
            pos2(x, y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(f32::from(ParaStyle::Normal.size()) * 4.0 / 3.0 * zoom),
            Color32::from_rgb(32, 32, 32),
        );
    }
}
