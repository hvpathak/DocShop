//! Page layout for HarshPad documents: paragraphs become lines, lines fill pages.
//! Everything is in points (1/72 inch), so zooming never re-lays out a document.
//! Text is measured through [`Metrics`], so the app measures with real fonts and the
//! tests use fixed widths. No UI code.

use std::ops::Range;

use harshpad_rich::{
    Align, Baseline, CharStyle, LineSpacing, ParaStyle, Paragraph, RichDoc, TWIPS_PER_POINT,
};

/// A typeface choice the app maps to a real font.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Face {
    /// The family name from the document, or `None` for the body font.
    pub family: Option<String>,
    pub mono: bool,
    pub bold: bool,
    pub italic: bool,
}

/// Font measurements in points for a face at a size in points.
pub trait Metrics {
    /// Advance width of one character.
    fn advance(&self, face: &Face, size: f32, ch: char) -> f32;
    /// Ascent and descent (both positive) of the face's line box.
    fn vertical(&self, face: &Face, size: f32) -> (f32, f32);
}

/// Paper size and margins, in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSetup {
    pub width: f32,
    pub height: f32,
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl Default for PageSetup {
    /// A4 with one-inch margins.
    fn default() -> Self {
        Self {
            width: 595.3,
            height: 841.9,
            top: 72.0,
            bottom: 72.0,
            left: 72.0,
            right: 72.0,
        }
    }
}

impl PageSetup {
    pub fn content_width(&self) -> f32 {
        (self.width - self.left - self.right).max(36.0)
    }

    fn content_bottom(&self) -> f32 {
        (self.height - self.bottom).max(self.top + 36.0)
    }
}

/// How a run of glyphs is drawn: the document style plus what layout resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub style: CharStyle,
    pub para: ParaStyle,
    pub face: Face,
    /// Font size in points after superscript, subscript and small caps.
    pub size: f32,
    /// Distance above the baseline (negative = below), in points.
    pub rise: f32,
}

/// One positioned character.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// Byte offset of the character in the document text.
    pub byte: usize,
    /// The character as drawn (caps applied; a tab is drawn as nothing).
    pub ch: char,
    /// Left edge in page coordinates, points.
    pub x: f32,
    pub width: f32,
    /// Index into [`Layout::looks`].
    pub look: usize,
}

/// A bullet or number drawn in a list paragraph's hanging indent.
#[derive(Debug, Clone, PartialEq)]
pub struct Marker {
    pub text: String,
    pub x: f32,
    pub look: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub page: usize,
    pub para: usize,
    /// Bytes of the line's text (without the paragraph's newline).
    pub range: Range<usize>,
    /// Top of the line box within its page.
    pub top: f32,
    pub height: f32,
    /// Baseline, measured from the top of the line box.
    pub baseline: f32,
    pub glyphs: Vec<Glyph>,
    /// Where the caret sits after the last character.
    pub end_x: f32,
    /// Where the caret sits in an empty line.
    pub start_x: f32,
    pub marker: Option<Marker>,
}

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub setup: Option<PageSetup>,
    pub pages: usize,
    pub lines: Vec<Line>,
    pub looks: Vec<Look>,
}

/// One measured character before line breaking.
struct Item {
    byte: usize,
    ch: char,
    width: f32,
    look: usize,
    ascent: f32,
    descent: f32,
    space: bool,
    tab: bool,
}

const TAB_STOP: f32 = 36.0;
/// Lists hang their marker this far left of the text.
const MARKER_GAP: f32 = 18.0;

fn twips(t: i32) -> f32 {
    t as f32 / TWIPS_PER_POINT as f32
}

fn twips_u(t: u32) -> f32 {
    t as f32 / TWIPS_PER_POINT as f32
}

/// The face and size a character gets from its own style and its paragraph's.
fn resolve(para: ParaStyle, style: &CharStyle) -> (Face, f32) {
    let size = style.size.map_or_else(|| f32::from(para.size()), f32::from);
    let face = Face {
        family: style.font.clone(),
        mono: style.code || para == ParaStyle::Code,
        bold: style.bold
            || matches!(
                para,
                ParaStyle::Heading1 | ParaStyle::Heading2 | ParaStyle::Heading3
            ),
        italic: style.italic || para == ParaStyle::Quote,
    };
    (face, size)
}

impl Layout {
    /// Lays out the whole document.
    pub fn new(doc: &RichDoc, setup: PageSetup, metrics: &dyn Metrics) -> Layout {
        let mut out = Layout {
            setup: Some(setup),
            pages: 1,
            ..Layout::default()
        };
        let runs: Vec<(CharStyle, Range<usize>)> =
            doc.styled_ranges().map(|(s, r)| (s.clone(), r)).collect();
        let paras: Vec<(Paragraph, Range<usize>)> = doc.paragraphs().collect();
        let text = doc.text();
        let mut y = setup.top;
        let mut page = 0usize;
        let mut number = 0u32;
        let mut first_para = true;

        let mut i = 0usize;
        while let Some((para, range)) = paras.get(i) {
            number = if para.style == ParaStyle::Numbered {
                number + 1
            } else {
                0
            };
            let marker = match para.style {
                ParaStyle::Bullet => Some("•".to_owned()),
                ParaStyle::Numbered => Some(format!("{number}.")),
                _ => None,
            };
            let mut lines =
                out.break_paragraph(i, para, range.clone(), text, &runs, setup, metrics, marker);
            let before = twips_u(para.before());
            let after = twips_u(para.after());
            let at_top = y <= setup.top + 0.01;

            // A page break before, or a paragraph that must stay in one piece.
            let height: f32 = lines.iter().map(|l| l.height).sum();
            let next_first = if para.keep_with_next {
                paras.get(i + 1).map_or(0.0, |(p, r)| {
                    let (face, size) = resolve(p.style, &CharStyle::default());
                    let (a, d) = metrics.vertical(&face, size);
                    let _ = r;
                    twips_u(p.before()) + a + d
                })
            } else {
                0.0
            };
            let room = setup.content_bottom() - y;
            let whole = height + before + next_first;
            let fits_on_a_page = whole <= setup.content_bottom() - setup.top;
            if !at_top
                && (para.page_break_before
                    || ((para.keep_lines_together || para.keep_with_next)
                        && whole > room
                        && fits_on_a_page))
            {
                page += 1;
                y = setup.top;
            }
            // Space before is dropped at the top of a page (but not on the first paragraph).
            if y > setup.top + 0.01 || first_para {
                y += before;
            }
            for line in &mut lines {
                if y + line.height > setup.content_bottom() && y > setup.top + 0.01 {
                    page += 1;
                    y = setup.top;
                }
                line.page = page;
                line.top = y;
                y += line.height;
            }
            y += after;
            out.lines.append(&mut lines);
            first_para = false;
            i += 1;
        }
        out.pages = page + 1;
        out
    }

    fn look_index(&mut self, look: Look) -> usize {
        if let Some(i) = self.looks.iter().position(|l| *l == look) {
            return i;
        }
        self.looks.push(look);
        self.looks.len() - 1
    }

    #[allow(clippy::too_many_arguments)]
    fn break_paragraph(
        &mut self,
        index: usize,
        para: &Paragraph,
        range: Range<usize>,
        text: &str,
        runs: &[(CharStyle, Range<usize>)],
        setup: PageSetup,
        metrics: &dyn Metrics,
        marker: Option<String>,
    ) -> Vec<Line> {
        // Measure every character with its resolved look.
        let mut items: Vec<Item> = Vec::new();
        let mut run_i = 0usize;
        let slice = text.get(range.clone()).unwrap_or_default();
        for (offset, ch) in slice.char_indices() {
            let byte = range.start + offset;
            while runs.get(run_i).is_some_and(|(_, r)| r.end <= byte) {
                run_i += 1;
            }
            let style = runs.get(run_i).map(|(s, _)| s.clone()).unwrap_or_default();
            let (face, base) = resolve(para.style, &style);
            let mut size = base;
            let mut shown = ch;
            if style.all_caps || (style.small_caps && ch.is_lowercase()) {
                let mut upper = ch.to_uppercase();
                if let (Some(u), None) = (upper.next(), upper.next()) {
                    shown = u;
                }
                if style.small_caps && !style.all_caps && ch.is_lowercase() {
                    size *= 0.8;
                }
            }
            let rise = match style.baseline {
                Baseline::Super => {
                    size *= 0.65;
                    base * 0.33
                }
                Baseline::Sub => {
                    size *= 0.65;
                    -base * 0.14
                }
                Baseline::Normal => 0.0,
            };
            let (ascent, descent) = metrics.vertical(&face, base);
            let tab = ch == '\t';
            let width = if tab {
                0.0
            } else {
                metrics.advance(&face, size, shown)
                    + f32::from(style.spacing) / TWIPS_PER_POINT as f32
            };
            let look = self.look_index(Look {
                style,
                para: para.style,
                face,
                size,
                rise,
            });
            items.push(Item {
                byte,
                ch: if tab { ' ' } else { shown },
                width,
                look,
                ascent: ascent + rise.max(0.0),
                descent: descent + (-rise).max(0.0),
                space: ch == ' ' || tab,
                tab,
            });
        }

        let left = setup.left + twips(para.effective_left());
        let right_edge = setup.left + setup.content_width() - twips(para.indent_right);
        let first_indent = twips(para.first_line);
        let (para_face, para_size) = resolve(para.style, &CharStyle::default());
        let (pa, pd) = metrics.vertical(&para_face, para_size);
        let marker_look = self.look_index(Look {
            style: CharStyle::default(),
            para: ParaStyle::Normal,
            face: Face {
                family: None,
                mono: false,
                bold: false,
                italic: false,
            },
            size: f32::from(ParaStyle::Normal.size()),
            rise: 0.0,
        });

        let mut lines = Vec::new();
        let mut start = 0usize; // index into items
        loop {
            let first = lines.is_empty();
            let x0 = left + if first { first_indent } else { 0.0 };
            let width_avail = (right_edge - x0).max(12.0);
            // Greedy fill: remember the last place a line may break.
            let mut x = 0.0f32;
            let mut end = start;
            let mut last_break: Option<usize> = None;
            while let Some(item) = items.get(end) {
                let w = if item.tab {
                    let next = ((x0 - left + x) / TAB_STOP).floor() * TAB_STOP + TAB_STOP;
                    (next - (x0 - left)) - x
                } else {
                    item.width
                };
                if x + w > width_avail && end > start && !item.space {
                    break;
                }
                x += w;
                end += 1;
                if item.space {
                    last_break = Some(end);
                }
            }
            if end < items.len()
                && let Some(b) = last_break
                && b > start
            {
                end = b;
            }
            let end = end.max(start + usize::from(start < items.len()));

            // Place the glyphs, measuring tabs again from the line start.
            let line_items = items.get(start..end).unwrap_or_default();
            let mut glyphs = Vec::with_capacity(line_items.len());
            let mut cx = x0;
            for item in line_items {
                let w = if item.tab {
                    let rel = cx - left;
                    let next = (rel / TAB_STOP).floor() * TAB_STOP + TAB_STOP;
                    next - rel
                } else {
                    item.width
                };
                glyphs.push(Glyph {
                    byte: item.byte,
                    ch: item.ch,
                    x: cx,
                    width: w,
                    look: item.look,
                });
                cx += w;
            }
            // Trailing spaces hang past the margin and don't count for alignment.
            let visible = glyphs
                .iter()
                .rev()
                .find(|g| g.ch != ' ')
                .map_or(x0, |g| g.x + g.width);
            let last_line = end >= items.len();
            let slack = (right_edge - visible).max(0.0);
            let shift = match para.align {
                Align::Center => slack / 2.0,
                Align::Right => slack,
                _ => 0.0,
            };
            if para.align == Align::Justify && !last_line {
                let spaces: Vec<usize> = glyphs
                    .iter()
                    .enumerate()
                    .filter(|(_, g)| g.ch == ' ' && g.x + g.width <= visible)
                    .map(|(k, _)| k)
                    .collect();
                if !spaces.is_empty() {
                    let extra = slack / spaces.len() as f32;
                    let mut add = 0.0;
                    for (k, g) in glyphs.iter_mut().enumerate() {
                        g.x += add;
                        if spaces.contains(&k) {
                            g.width += extra;
                            add += extra;
                        }
                    }
                }
            } else if shift > 0.0 {
                for g in &mut glyphs {
                    g.x += shift;
                }
            }

            // Line height from the tallest character and the spacing rule.
            let (ascent, descent) = line_items
                .iter()
                .fold((pa, pd), |(a, d), it| (a.max(it.ascent), d.max(it.descent)));
            let natural = ascent + descent;
            let height = match para.line_spacing {
                LineSpacing::Multiple(pct) => natural * f32::from(pct) / 100.0,
                LineSpacing::AtLeast(t) => natural.max(twips_u(t)),
                LineSpacing::Exactly(t) => twips_u(t).max(1.0),
            };
            let baseline = match para.line_spacing {
                LineSpacing::Exactly(_) => (height * 0.8).min(ascent),
                _ => ascent,
            };
            let range_start = line_items.first().map_or(range.start, |i| i.byte);
            let range_end = items.get(end).map_or(range.end, |i| i.byte);
            let end_x = glyphs.last().map_or(x0 + shift, |g| g.x + g.width);
            lines.push(Line {
                page: 0,
                para: index,
                range: range_start..range_end,
                top: 0.0,
                height,
                baseline,
                glyphs,
                end_x,
                start_x: x0 + shift,
                marker: if first {
                    marker.clone().map(|text| Marker {
                        text,
                        x: (x0 - MARKER_GAP).max(setup.left),
                        look: marker_look,
                    })
                } else {
                    None
                },
            });
            start = end;
            if start >= items.len() {
                break;
            }
        }
        lines
    }

    /// The line holding the caret at `byte` (a position between characters).
    pub fn line_at(&self, byte: usize) -> usize {
        let mut found = 0;
        for (i, line) in self.lines.iter().enumerate() {
            if line.range.start <= byte {
                found = i;
            } else {
                break;
            }
        }
        found
    }

    /// Caret position for `byte`: (page, x, line top, line height).
    pub fn caret(&self, byte: usize) -> Option<(usize, f32, f32, f32)> {
        let line = self.lines.get(self.line_at(byte))?;
        let x = line.glyphs.iter().find(|g| g.byte >= byte).map_or_else(
            || {
                if line.glyphs.is_empty() {
                    line.start_x
                } else {
                    line.end_x
                }
            },
            |g| g.x,
        );
        Some((line.page, x, line.top, line.height))
    }

    /// The text position nearest to a point on a page.
    pub fn hit(&self, page: usize, x: f32, y: f32) -> usize {
        let on_page: Vec<&Line> = self.lines.iter().filter(|l| l.page == page).collect();
        let line = on_page
            .iter()
            .find(|l| y < l.top + l.height)
            .or(on_page.last())
            .copied()
            .or_else(|| self.lines.iter().rfind(|l| l.page < page))
            .or(self.lines.first());
        let Some(line) = line else { return 0 };
        self.position_in_line(line, x)
    }

    fn position_in_line(&self, line: &Line, x: f32) -> usize {
        for g in &line.glyphs {
            if x < g.x + g.width / 2.0 {
                return g.byte;
            }
        }
        // After the last character; a wrapped line's trailing space belongs to it.
        line.range.end
    }

    /// Moving up or down a line keeps the caret's x.
    pub fn vertical_move(&self, byte: usize, x: f32, down: bool) -> Option<usize> {
        let i = self.line_at(byte);
        let target = if down {
            i.checked_add(1)?
        } else {
            i.checked_sub(1)?
        };
        let line = self.lines.get(target)?;
        Some(self.position_in_line(line, x))
    }

    /// Start and end of the visual line holding `byte`.
    pub fn line_bounds(&self, byte: usize) -> Range<usize> {
        self.lines
            .get(self.line_at(byte))
            .map_or(0..0, |l| l.range.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harshpad_rich::DocBuilder;

    /// Every character is half its size wide; lines are exactly the font size tall.
    struct Fixed;
    impl Metrics for Fixed {
        fn advance(&self, _: &Face, size: f32, _: char) -> f32 {
            size / 2.0
        }
        fn vertical(&self, _: &Face, size: f32) -> (f32, f32) {
            (size * 0.8, size * 0.2)
        }
    }

    fn doc(paras: &[(Paragraph, &str)]) -> RichDoc {
        let mut b = DocBuilder::new();
        for (p, t) in paras {
            b.paragraph(*p, &[((*t).to_owned(), CharStyle::default())]);
        }
        b.finish()
    }

    fn narrow() -> PageSetup {
        // 100 pt of text per line: 18 characters of 11 pt type (5.5 pt each).
        PageSetup {
            width: 244.0,
            height: 200.0,
            top: 20.0,
            bottom: 20.0,
            left: 72.0,
            right: 72.0,
        }
    }

    #[test]
    fn words_wrap_at_spaces_and_long_words_break() {
        let d = doc(&[(
            Paragraph::default(),
            "aaaa bbbb cccc dddd eeee ffffffffffffffffffffffffff",
        )]);
        let l = Layout::new(&d, narrow(), &Fixed);
        let texts: Vec<&str> = l
            .lines
            .iter()
            .map(|x| d.text().get(x.range.clone()).unwrap())
            .collect();
        assert_eq!(texts.first(), Some(&"aaaa bbbb cccc "));
        // The long word is split across lines rather than overflowing.
        assert!(l.lines.iter().all(|line| {
            line.glyphs
                .iter()
                .all(|g| g.ch == ' ' || g.x + g.width <= 172.01)
        }));
        assert_eq!(texts.concat(), d.text());
    }

    #[test]
    fn indents_shift_the_text() {
        let p = Paragraph {
            indent_left: 400, // 20 pt
            first_line: 200,  // +10 pt on the first line
            ..Paragraph::default()
        };
        let d = doc(&[(p, "aaaa bbbb cccc dddd eeee")]);
        let l = Layout::new(&d, narrow(), &Fixed);
        assert_eq!(
            l.lines.first().unwrap().glyphs.first().unwrap().x,
            72.0 + 30.0
        );
        assert_eq!(
            l.lines.get(1).unwrap().glyphs.first().unwrap().x,
            72.0 + 20.0
        );
    }

    #[test]
    fn alignment_centre_right_and_justify() {
        let right = Paragraph {
            align: Align::Right,
            ..Paragraph::default()
        };
        let centre = Paragraph {
            align: Align::Center,
            ..Paragraph::default()
        };
        let justify = Paragraph {
            align: Align::Justify,
            ..Paragraph::default()
        };
        let d = doc(&[
            (right, "ab"),
            (centre, "ab"),
            (justify, "aaaa bbbb cccc dddd eeee"),
        ]);
        let l = Layout::new(&d, narrow(), &Fixed);
        let first_x = |i: usize| l.lines.get(i).unwrap().glyphs.first().unwrap().x;
        assert_eq!(first_x(0), 172.0 - 11.0);
        assert_eq!(first_x(1), 72.0 + (100.0 - 11.0) / 2.0);
        // A justified line (not the last) reaches the right margin.
        let line = l.lines.get(2).unwrap();
        let last_visible = line.glyphs.iter().rev().find(|g| g.ch != ' ').unwrap();
        assert!((last_visible.x + last_visible.width - 172.0).abs() < 0.01);
    }

    #[test]
    fn spacing_and_line_spacing_move_lines_down() {
        let p = Paragraph {
            space_before: Some(0),
            space_after: Some(200), // 10 pt
            line_spacing: LineSpacing::Multiple(200),
            ..Paragraph::default()
        };
        let d = doc(&[(p, "one"), (p, "two")]);
        let l = Layout::new(&d, narrow(), &Fixed);
        let a = l.lines.first().unwrap();
        let b = l.lines.get(1).unwrap();
        assert_eq!(a.height, 22.0); // 11 pt type, double spaced
        assert_eq!(b.top, a.top + 22.0 + 10.0);
    }

    #[test]
    fn pages_fill_and_page_breaks_start_new_pages() {
        let p = Paragraph {
            space_before: Some(0),
            space_after: Some(0),
            ..Paragraph::default()
        };
        let lines: Vec<(Paragraph, &str)> = (0..30).map(|_| (p, "x")).collect();
        let l = Layout::new(&doc(&lines), narrow(), &Fixed);
        // 160 pt of content height holds 14 lines of 11 pt.
        assert_eq!(l.lines.iter().filter(|x| x.page == 0).count(), 14);
        assert_eq!(l.pages, 3);
        let broken = Paragraph {
            page_break_before: true,
            ..p
        };
        let l = Layout::new(&doc(&[(p, "a"), (broken, "b")]), narrow(), &Fixed);
        assert_eq!(l.lines.get(1).unwrap().page, 1);
        assert_eq!(l.lines.get(1).unwrap().top, 20.0);
    }

    #[test]
    fn caret_and_hit_testing_agree() {
        let d = doc(&[(Paragraph::default(), "aaaa bbbb cccc dddd eeee")]);
        let l = Layout::new(&d, narrow(), &Fixed);
        for byte in [0, 3, 7, 15, 20, d.text().len()] {
            let (page, x, top, h) = l.caret(byte).unwrap();
            assert_eq!(l.hit(page, x + 0.1, top + h / 2.0), byte, "byte {byte}");
        }
        // Down from the first line keeps the column.
        let below = l.vertical_move(2, l.caret(2).unwrap().1, true).unwrap();
        assert_eq!(below, 17);
    }

    #[test]
    fn superscript_is_smaller_and_lists_get_markers() {
        let mut b = DocBuilder::new();
        let sup = CharStyle {
            baseline: Baseline::Super,
            ..CharStyle::default()
        };
        b.paragraph(
            Paragraph::new(ParaStyle::Numbered),
            &[("x".into(), CharStyle::default()), ("2".into(), sup)],
        );
        b.paragraph(
            Paragraph::new(ParaStyle::Numbered),
            &[("y".into(), CharStyle::default())],
        );
        let d = b.finish();
        let l = Layout::new(&d, narrow(), &Fixed);
        let line = l.lines.first().unwrap();
        let two = line.glyphs.get(1).unwrap();
        assert!(two.width < line.glyphs.first().unwrap().width);
        assert!(l.looks.get(two.look).unwrap().rise > 0.0);
        assert_eq!(line.marker.as_ref().unwrap().text, "1.");
        assert_eq!(l.lines.get(1).unwrap().marker.as_ref().unwrap().text, "2.");
        // The text sits at the list indent; the marker hangs to its left.
        assert_eq!(line.glyphs.first().unwrap().x, 72.0 + 36.0);
        assert!(line.marker.as_ref().unwrap().x < 72.0 + 36.0);
    }

    #[test]
    fn empty_paragraphs_still_take_a_line() {
        let d = doc(&[(Paragraph::default(), ""), (Paragraph::default(), "")]);
        let l = Layout::new(&d, narrow(), &Fixed);
        assert_eq!(l.lines.len(), 2);
        assert!(l.caret(d.text().len()).is_some());
    }
}
