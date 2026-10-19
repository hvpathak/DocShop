//! Draws a laid-out document as pages, and measures text for the layout engine
//! with the app's real fonts. Layout works in points; the screen shows
//! `points × 4/3 × zoom` pixels (96 dpi at 100%).

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;

use eframe::egui::{
    self, Color32, FontFamily, FontId, Pos2, Rect, Stroke, pos2,
    text::{LayoutJob, TextFormat},
    vec2,
};
use harshpad_layout::{Face, Layout, Look, Metrics, PageSetup};
use harshpad_rich::{ParaStyle, Rgb};

use crate::fonts::BOLD;

/// Screen pixels per point at 100% zoom.
pub const PX_PER_PT: f32 = 4.0 / 3.0;
/// Gap between pages on screen.
pub const PAGE_GAP: f32 = 24.0;

pub fn family(face: &Face) -> FontFamily {
    if face.mono {
        FontFamily::Monospace
    } else if face.bold {
        FontFamily::Name(BOLD.into())
    } else {
        FontFamily::Proportional
    }
}

/// Measures with egui's fonts (at 100% zoom) and converts to points.
pub struct AppMetrics<'a> {
    ctx: &'a egui::Context,
    vertical: RefCell<HashMap<(FontFamily, u32), (f32, f32)>>,
    widths: RefCell<HashMap<(FontFamily, u32, char), f32>>,
}

impl<'a> AppMetrics<'a> {
    pub fn new(ctx: &'a egui::Context) -> Self {
        Self {
            ctx,
            vertical: RefCell::default(),
            widths: RefCell::default(),
        }
    }
}

fn font_id(face: &Face, size_pt: f32) -> FontId {
    FontId::new(size_pt * PX_PER_PT, family(face))
}

impl Metrics for AppMetrics<'_> {
    fn advance(&self, face: &Face, size: f32, ch: char) -> f32 {
        let key = (family(face), size.to_bits(), ch);
        if let Some(w) = self.widths.borrow().get(&key) {
            return *w;
        }
        let id = font_id(face, size);
        let w = self.ctx.fonts(|f| f.glyph_width(&id, ch)) / PX_PER_PT;
        self.widths.borrow_mut().insert(key, w);
        w
    }

    fn vertical(&self, face: &Face, size: f32) -> (f32, f32) {
        let key = (family(face), size.to_bits());
        if let Some(v) = self.vertical.borrow().get(&key) {
            return *v;
        }
        let id = font_id(face, size);
        let galley = self
            .ctx
            .fonts(|f| f.layout_no_wrap("Hg".to_owned(), id.clone(), Color32::BLACK));
        let (ascent, height) = galley
            .rows
            .first()
            .and_then(|r| r.glyphs.first())
            .map_or((size * PX_PER_PT * 0.8, size * PX_PER_PT * 1.2), |g| {
                (g.font_ascent, g.font_height)
            });
        let v = (ascent / PX_PER_PT, (height - ascent).max(0.0) / PX_PER_PT);
        self.vertical.borrow_mut().insert(key, v);
        v
    }
}

fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

pub fn text_colour(look: &Look) -> Color32 {
    if let Some(c) = look.style.color {
        return rgb(c);
    }
    if look.style.link.is_some() {
        return Color32::from_rgb(6, 95, 212);
    }
    match look.para {
        ParaStyle::Title => Color32::from_rgb(17, 17, 17),
        ParaStyle::Heading1 | ParaStyle::Heading2 | ParaStyle::Heading3 => {
            Color32::from_rgb(24, 72, 140)
        }
        ParaStyle::Quote => Color32::from_rgb(95, 95, 95),
        _ => Color32::from_rgb(32, 32, 32),
    }
}

/// Where each page sits on screen.
pub struct Pages {
    pub origin: Pos2,
    pub scale: f32,
    pub page: Vec2Size,
}

#[derive(Clone, Copy)]
pub struct Vec2Size {
    pub w: f32,
    pub h: f32,
}

impl Pages {
    pub fn new(origin: Pos2, setup: &PageSetup, zoom: f32) -> Self {
        let scale = PX_PER_PT * zoom;
        Self {
            origin,
            scale,
            page: Vec2Size {
                w: setup.width * scale,
                h: setup.height * scale,
            },
        }
    }

    pub fn rect(&self, page: usize) -> Rect {
        let top = self.origin.y + page as f32 * (self.page.h + PAGE_GAP);
        Rect::from_min_size(pos2(self.origin.x, top), vec2(self.page.w, self.page.h))
    }

    /// Screen position of a point on a page.
    pub fn to_screen(&self, page: usize, x: f32, y: f32) -> Pos2 {
        let r = self.rect(page);
        pos2(r.left() + x * self.scale, r.top() + y * self.scale)
    }

    /// The page and the point on it under a screen position.
    pub fn page_point(&self, p: Pos2, pages: usize) -> (usize, f32, f32) {
        let stride = self.page.h + PAGE_GAP;
        let raw = ((p.y - self.origin.y) / stride).floor();
        let page = (raw.max(0.0) as usize).min(pages.saturating_sub(1));
        let r = self.rect(page);
        (
            page,
            (p.x - r.left()) / self.scale,
            (p.y - r.top()) / self.scale,
        )
    }

    pub fn total_height(&self, pages: usize) -> f32 {
        pages as f32 * (self.page.h + PAGE_GAP)
    }
}

/// Paints the pages that are visible, with selection, find match and caret.
pub fn paint(
    ui: &egui::Ui,
    layout: &Layout,
    at: &Pages,
    selection: Range<usize>,
    caret: Option<usize>,
) {
    let painter = ui.painter();
    let clip = painter.clip_rect();
    let s = at.scale;
    let selection_fill = Color32::from_rgb(180, 214, 255);
    for page in 0..layout.pages {
        let rect = at.rect(page);
        if !rect.expand(20.0).intersects(clip) {
            continue;
        }
        painter.add(
            egui::epaint::Shadow {
                offset: [0, 4],
                blur: 16,
                spread: 0,
                color: Color32::from_black_alpha(70),
            }
            .as_shape(rect, 3.0),
        );
        painter.rect_filled(rect, 3.0, Color32::WHITE);
    }
    for line in &layout.lines {
        let page_rect = at.rect(line.page);
        if !page_rect.intersects(clip) {
            continue;
        }
        let top = at.to_screen(line.page, 0.0, line.top).y;
        let bottom = top + line.height * s;
        if bottom < clip.top() || top > clip.bottom() {
            continue;
        }
        let baseline = top + line.baseline * s;
        // Code blocks: a grey band across the text column.
        let code_block = line
            .glyphs
            .first()
            .and_then(|g| layout.looks.get(g.look))
            .is_some_and(|l| l.para == ParaStyle::Code);
        if code_block && let Some(setup) = layout.setup {
            let left = at.to_screen(line.page, setup.left - 4.0, 0.0).x;
            let right = at
                .to_screen(line.page, setup.width - setup.right + 4.0, 0.0)
                .x;
            painter.rect_filled(
                Rect::from_min_max(pos2(left, top), pos2(right, bottom)),
                0.0,
                Color32::from_rgb(244, 245, 247),
            );
        }
        // Highlights and inline code backgrounds, then the selection.
        for g in &line.glyphs {
            let Some(look) = layout.looks.get(g.look) else {
                continue;
            };
            let bg = look.style.highlight.map(rgb).or_else(|| {
                (look.style.code && look.para != ParaStyle::Code)
                    .then_some(Color32::from_rgb(238, 240, 243))
            });
            if let Some(bg) = bg {
                let x = at.to_screen(line.page, g.x, 0.0).x;
                painter.rect_filled(
                    Rect::from_min_max(pos2(x, top), pos2(x + g.width * s, bottom)),
                    0.0,
                    bg,
                );
            }
        }
        let sel_start = selection.start.max(line.range.start);
        let sel_end = selection.end.min(line.range.end);
        let to_next_line = selection.end > line.range.end && selection.start <= line.range.end;
        if sel_start < sel_end || (to_next_line && selection.start != selection.end) {
            let x_of = |byte: usize| {
                line.glyphs
                    .iter()
                    .find(|g| g.byte >= byte)
                    .map_or(line.end_x, |g| g.x)
            };
            let x0 = x_of(sel_start);
            let mut x1 = x_of(sel_end);
            if to_next_line {
                x1 = line.end_x + 4.0; // the paragraph mark
            }
            let a = at.to_screen(line.page, x0, 0.0).x;
            let b = at.to_screen(line.page, x1, 0.0).x;
            painter.rect_filled(
                Rect::from_min_max(pos2(a, top), pos2(b.max(a + 2.0), bottom)),
                0.0,
                selection_fill,
            );
        }
        // List marker.
        if let Some(m) = &line.marker
            && let Some(look) = layout.looks.get(m.look)
        {
            let x = at.to_screen(line.page, m.x, 0.0).x;
            draw_text(
                ui,
                &m.text,
                look,
                pos2(x, baseline),
                s,
                Color32::from_rgb(32, 32, 32),
            );
        }
        // The text, one glyph at a time at its laid-out position.
        for g in &line.glyphs {
            if g.ch == ' ' {
                continue;
            }
            let Some(look) = layout.looks.get(g.look) else {
                continue;
            };
            let x = at.to_screen(line.page, g.x, 0.0).x;
            let colour = text_colour(look);
            let y = baseline - look.rise * s;
            draw_text(
                ui,
                g.ch.encode_utf8(&mut [0; 4]),
                look,
                pos2(x, y),
                s,
                colour,
            );
        }
        // Underline, strikethrough, double strikethrough.
        for g in &line.glyphs {
            let Some(look) = layout.looks.get(g.look) else {
                continue;
            };
            let x = at.to_screen(line.page, g.x, 0.0).x;
            let w = g.width * s;
            let colour = text_colour(look);
            let stroke = Stroke::new((look.size * s / 14.0).max(1.0), colour);
            if look.style.underline || look.style.link.is_some() {
                let y = baseline + look.size * s * 0.12;
                painter.line_segment([pos2(x, y), pos2(x + w, y)], stroke);
            }
            let mid = baseline - look.rise * s - look.size * s * 0.3;
            if look.style.strike {
                painter.line_segment([pos2(x, mid), pos2(x + w, mid)], stroke);
            }
            if look.style.double_strike {
                let d = look.size * s * 0.07;
                painter.line_segment([pos2(x, mid - d), pos2(x + w, mid - d)], stroke);
                painter.line_segment([pos2(x, mid + d), pos2(x + w, mid + d)], stroke);
            }
        }
    }
    if let Some(byte) = caret
        && let Some((page, x, top, height)) = layout.caret(byte)
    {
        let a = at.to_screen(page, x, top);
        let b = at.to_screen(page, x, top + height);
        painter.line_segment([a, b], Stroke::new(1.5_f32, Color32::from_rgb(20, 20, 20)));
    }
}

/// Draws text so that its baseline sits at `baseline.y`.
fn draw_text(ui: &egui::Ui, text: &str, look: &Look, baseline: Pos2, scale: f32, colour: Color32) {
    let format = TextFormat {
        font_id: FontId::new(look.size * scale, family(&look.face)),
        color: colour,
        italics: look.face.italic,
        ..Default::default()
    };
    let galley = ui.fonts(|f| f.layout_job(LayoutJob::single_section(text.to_owned(), format)));
    let ascent = galley
        .rows
        .first()
        .and_then(|r| r.glyphs.first())
        .map_or(0.0, |g| g.pos.y);
    ui.painter()
        .galley(pos2(baseline.x, baseline.y - ascent), galley, colour);
}

/// The caret's rectangle on screen (for scrolling it into view and placing the IME).
pub fn caret_rect(layout: &Layout, at: &Pages, byte: usize) -> Option<Rect> {
    let (page, x, top, height) = layout.caret(byte)?;
    let a = at.to_screen(page, x, top);
    Some(Rect::from_min_size(a, vec2(2.0, height * at.scale)))
}
