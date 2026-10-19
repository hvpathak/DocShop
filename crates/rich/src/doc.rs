//! The rich document model and its editing rules.
//!
//! `text` uses `\n` between paragraphs. `runs` cover `text` exactly, in order, as
//! byte lengths with a [`CharStyle`]. `paras` has one entry per paragraph.

use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result, damaged};
use crate::style::{Align, CharStyle, ParaStyle, Paragraph};

/// Largest document text HarshPad will hold (bytes).
pub const MAX_TEXT: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub len: usize,
    pub style: CharStyle,
}

/// Character flags that toggle on and off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
}

impl Flag {
    fn get(self, s: &CharStyle) -> bool {
        match self {
            Flag::Bold => s.bold,
            Flag::Italic => s.italic,
            Flag::Underline => s.underline,
            Flag::Strike => s.strike,
            Flag::Code => s.code,
        }
    }

    pub fn set(self, s: &mut CharStyle, on: bool) {
        match self {
            Flag::Bold => s.bold = on,
            Flag::Italic => s.italic = on,
            Flag::Underline => s.underline = on,
            Flag::Strike => s.strike = on,
            Flag::Code => s.code = on,
        }
    }
}

/// What [`RichDoc::apply_edit`] did beyond following the edit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditOutcome {
    /// The document changed its own text (list ended, Markdown shortcut used);
    /// the editor must reload the text and put the cursor at `cursor` (a byte offset).
    pub adjusted: bool,
    pub cursor: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RichDoc {
    text: String,
    runs: Vec<Run>,
    paras: Vec<Paragraph>,
}

impl Default for RichDoc {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize)]
struct Stored {
    format: String,
    version: u32,
    #[serde(flatten)]
    doc: RichDoc,
}

impl RichDoc {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            runs: Vec::new(),
            paras: vec![Paragraph::default()],
        }
    }

    /// A document from plain text: every line becomes a Normal paragraph.
    pub fn from_plain(text: &str) -> Self {
        let text = text.replace("\r\n", "\n");
        let mut doc = Self::new();
        doc.paras = vec![Paragraph::default(); text.matches('\n').count() + 1];
        if !text.is_empty() {
            doc.runs = vec![Run {
                len: text.len(),
                style: CharStyle::default(),
            }];
        }
        doc.text = text;
        doc
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    pub fn paras(&self) -> &[Paragraph] {
        &self.paras
    }

    pub fn word_count(&self) -> usize {
        self.text.split_whitespace().count()
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().filter(|c| *c != '\n').count()
    }

    /// Paragraphs as (paragraph, byte range of its text without the newline).
    pub fn paragraphs(&self) -> impl Iterator<Item = (Paragraph, Range<usize>)> + '_ {
        let mut start = 0;
        self.text
            .split('\n')
            .zip(self.paras.iter().copied())
            .map(move |(line, p)| {
                let r = start..start + line.len();
                start = r.end + 1;
                (p, r)
            })
    }

    /// Runs as (style, byte range).
    pub fn styled_ranges(&self) -> impl Iterator<Item = (&CharStyle, Range<usize>)> + '_ {
        let mut start = 0;
        self.runs.iter().map(move |r| {
            let range = start..start + r.len;
            start = range.end;
            (&r.style, range)
        })
    }

    /// Index of the paragraph containing byte `at`.
    pub fn para_index_at(&self, at: usize) -> usize {
        let at = floor_boundary(&self.text, at);
        self.text
            .get(..at)
            .map_or(0, |head| head.matches('\n').count())
    }

    pub fn para_at(&self, at: usize) -> Paragraph {
        self.paras
            .get(self.para_index_at(at))
            .copied()
            .unwrap_or_default()
    }

    /// The style a character typed at byte `at` gets: that of the character before it
    /// (or after it, at the very start).
    pub fn style_at(&self, at: usize) -> CharStyle {
        let mut start = 0;
        let mut last = None;
        for run in &self.runs {
            let end = start + run.len;
            if at > start && at <= end {
                return run.style.clone();
            }
            if at == 0 && start == 0 {
                return run.style.clone();
            }
            last = Some(&run.style);
            start = end;
        }
        last.cloned().unwrap_or_default()
    }

    // ---- Following edits ------------------------------------------------------------

    /// Makes the document match `new_text` (as edited by the user), moving styles with
    /// the text. Inserted text gets `typing` if given, else the style before the cursor.
    pub fn apply_edit(&mut self, new_text: &str, typing: Option<&CharStyle>) -> EditOutcome {
        if new_text == self.text || new_text.len() > MAX_TEXT {
            return EditOutcome::default();
        }
        let old = &self.text;
        let mut p = old
            .bytes()
            .zip(new_text.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !old.is_char_boundary(p) || !new_text.is_char_boundary(p) {
            p -= 1;
        }
        let max_s = old.len().min(new_text.len()) - p;
        let mut s = old
            .bytes()
            .rev()
            .zip(new_text.bytes().rev())
            .take(max_s)
            .take_while(|(a, b)| a == b)
            .count();
        while !old.is_char_boundary(old.len() - s) || !new_text.is_char_boundary(new_text.len() - s)
        {
            s -= 1;
        }
        let deleted = old.get(p..old.len() - s).unwrap_or_default().to_owned();
        let inserted = new_text
            .get(p..new_text.len() - s)
            .unwrap_or_default()
            .to_owned();

        let style = typing.cloned().unwrap_or_else(|| self.style_at(p));
        let k = self.para_index_at(p);
        let para_k = self.paras.get(k).copied().unwrap_or_default();
        let para_was_empty = self.paragraph_text(k).is_empty();

        self.delete_runs(p, deleted.len());
        self.insert_runs(p, inserted.len(), style);
        self.text = new_text.to_owned();

        // Paragraph list: merged ones disappear, split ones are added after `k`.
        let removed = deleted.matches('\n').count();
        let added = inserted.matches('\n').count();
        let from = (k + 1).min(self.paras.len());
        let to = (k + 1 + removed).min(self.paras.len());
        self.paras.drain(from..to);
        if added > 0 {
            // Splitting at the end of a paragraph starts the "next" style (heading → Normal);
            // splitting in the middle keeps the style on both halves.
            let after = p + inserted.len();
            let rest_empty = self
                .text
                .get(after..)
                .is_none_or(|t| t.starts_with('\n') || t.is_empty());
            let style = if rest_empty {
                para_k.style.next()
            } else {
                para_k.style
            };
            let new_para = Paragraph {
                style,
                align: para_k.align,
            };
            let at = (k + 1).min(self.paras.len());
            for _ in 0..added {
                self.paras.insert(at, new_para);
            }
        }
        self.repair_paras();
        self.merge_runs();

        // Enter on an empty list item ends the list instead of adding another item.
        if inserted == "\n" && deleted.is_empty() && para_k.style.is_list() && para_was_empty {
            return self.end_list(k, p);
        }
        // Markdown-style shortcuts typed at the start of a Normal paragraph.
        if inserted == " " && deleted.is_empty() && para_k.style == ParaStyle::Normal {
            return self.autoformat(k, p + 1);
        }
        EditOutcome::default()
    }

    fn end_list(&mut self, k: usize, newline_at: usize) -> EditOutcome {
        // Remove the newline just typed and make the empty item a Normal paragraph.
        self.delete_runs(newline_at, 1);
        self.text.replace_range(newline_at..newline_at + 1, "");
        if k + 1 < self.paras.len() {
            self.paras.remove(k + 1);
        }
        if let Some(p) = self.paras.get_mut(k) {
            p.style = ParaStyle::Normal;
        }
        self.repair_paras();
        EditOutcome {
            adjusted: true,
            cursor: newline_at,
        }
    }

    fn autoformat(&mut self, k: usize, cursor: usize) -> EditOutcome {
        let line = self.paragraph_range(k);
        let Some(typed) = self.text.get(line.start..cursor) else {
            return EditOutcome::default();
        };
        let style = match typed {
            "- " | "* " => ParaStyle::Bullet,
            "1. " => ParaStyle::Numbered,
            "# " => ParaStyle::Heading1,
            "## " => ParaStyle::Heading2,
            "### " => ParaStyle::Heading3,
            "> " => ParaStyle::Quote,
            _ => return EditOutcome::default(),
        };
        let marker = typed.len();
        self.delete_runs(line.start, marker);
        self.text.replace_range(line.start..line.start + marker, "");
        if let Some(p) = self.paras.get_mut(k) {
            p.style = style;
        }
        self.merge_runs();
        EditOutcome {
            adjusted: true,
            cursor: line.start,
        }
    }

    fn delete_runs(&mut self, at: usize, len: usize) {
        if len == 0 {
            return;
        }
        let end = at + len;
        let mut start = 0;
        for run in &mut self.runs {
            let (rs, re) = (start, start + run.len);
            start = re;
            let overlap = re.min(end).saturating_sub(rs.max(at));
            run.len -= overlap;
        }
        self.runs.retain(|r| r.len > 0);
    }

    fn insert_runs(&mut self, at: usize, len: usize, style: CharStyle) {
        if len == 0 {
            return;
        }
        let mut start = 0;
        for i in 0..self.runs.len() {
            let Some(run) = self.runs.get_mut(i) else {
                break;
            };
            let end = start + run.len;
            if at >= start && at <= end {
                if run.style == style {
                    run.len += len;
                    return;
                }
                // Split the run and put the new text between the halves.
                let tail = end - at;
                run.len = at - start;
                let tail_style = run.style.clone();
                let mut add = vec![Run { len, style }];
                if tail > 0 {
                    add.push(Run {
                        len: tail,
                        style: tail_style,
                    });
                }
                let pos = i + 1;
                self.runs.splice(pos..pos, add);
                self.runs.retain(|r| r.len > 0);
                return;
            }
            start = end;
        }
        self.runs.push(Run { len, style });
    }

    fn merge_runs(&mut self) {
        let mut merged: Vec<Run> = Vec::with_capacity(self.runs.len());
        for run in self.runs.drain(..) {
            if run.len == 0 {
                continue;
            }
            match merged.last_mut() {
                Some(last) if last.style == run.style => last.len += run.len,
                _ => merged.push(run),
            }
        }
        self.runs = merged;
    }

    /// Keeps one paragraph entry per line, whatever happened.
    fn repair_paras(&mut self) {
        let want = self.text.matches('\n').count() + 1;
        self.paras.resize(want, Paragraph::default());
    }

    fn paragraph_range(&self, k: usize) -> Range<usize> {
        self.paragraphs()
            .nth(k)
            .map_or(self.text.len()..self.text.len(), |(_, r)| r)
    }

    fn paragraph_text(&self, k: usize) -> &str {
        let r = self.paragraph_range(k);
        self.text.get(r).unwrap_or_default()
    }

    // ---- Formatting ----------------------------------------------------------------

    /// Applies `f` to the character style of every byte in `range`.
    pub fn set_char_style(&mut self, range: Range<usize>, f: impl Fn(&mut CharStyle)) {
        let start = floor_boundary(&self.text, range.start);
        let end = floor_boundary(&self.text, range.end.max(range.start));
        if start >= end {
            return;
        }
        self.split_at(start);
        self.split_at(end);
        let mut pos = 0;
        for run in &mut self.runs {
            let (rs, re) = (pos, pos + run.len);
            pos = re;
            if rs >= start && re <= end {
                f(&mut run.style);
            }
        }
        self.merge_runs();
    }

    /// Turns a flag on for the whole range, or off if it was already on everywhere.
    pub fn toggle(&mut self, range: Range<usize>, flag: Flag) {
        let on = !self.all_have(range.clone(), flag);
        self.set_char_style(range, |s| flag.set(s, on));
    }

    /// True when every character in `range` has `flag` (toolbar button state).
    pub fn all_have(&self, range: Range<usize>, flag: Flag) -> bool {
        if range.is_empty() {
            return flag.get(&self.style_at(range.start));
        }
        self.styled_ranges()
            .filter(|(_, r)| r.start < range.end && r.end > range.start)
            .all(|(s, _)| flag.get(s))
    }

    fn split_at(&mut self, at: usize) {
        let mut start = 0;
        for i in 0..self.runs.len() {
            let Some(run) = self.runs.get_mut(i) else {
                return;
            };
            let end = start + run.len;
            if at > start && at < end {
                let tail = Run {
                    len: end - at,
                    style: run.style.clone(),
                };
                run.len = at - start;
                self.runs.insert(i + 1, tail);
                return;
            }
            start = end;
        }
    }

    /// Paragraph indices touched by a byte range (at least the one at `range.start`).
    pub fn para_indices(&self, range: Range<usize>) -> Range<usize> {
        let first = self.para_index_at(range.start);
        let last = self.para_index_at(range.end.max(range.start));
        first..last + 1
    }

    pub fn set_para_style(&mut self, range: Range<usize>, style: ParaStyle) {
        for i in self.para_indices(range) {
            if let Some(p) = self.paras.get_mut(i) {
                p.style = style;
            }
        }
    }

    /// Lists toggle: on for all touched paragraphs, or back to Normal if all had it.
    pub fn toggle_list(&mut self, range: Range<usize>, style: ParaStyle) {
        let indices = self.para_indices(range.clone());
        let all = indices
            .clone()
            .all(|i| self.paras.get(i).is_some_and(|p| p.style == style));
        self.set_para_style(range, if all { ParaStyle::Normal } else { style });
    }

    pub fn set_align(&mut self, range: Range<usize>, align: Align) {
        for i in self.para_indices(range) {
            if let Some(p) = self.paras.get_mut(i) {
                p.align = align;
            }
        }
    }

    /// Removes character formatting from a range (keeps links).
    pub fn clear_formatting(&mut self, range: Range<usize>) {
        self.set_char_style(range, |s| {
            let link = s.link.take();
            *s = CharStyle {
                link,
                ..CharStyle::default()
            };
        });
    }

    // ---- Storing ------------------------------------------------------------------------

    pub fn to_json(&self) -> Result<Vec<u8>> {
        let stored = Stored {
            format: "harshpad-doc".into(),
            version: 1,
            doc: self.clone(),
        };
        serde_json::to_vec_pretty(&stored).map_err(damaged)
    }

    pub fn from_json(bytes: &[u8]) -> Result<RichDoc> {
        if bytes.len() > MAX_TEXT * 2 {
            return Err(Error::TooLarge);
        }
        let stored: Stored = serde_json::from_slice(bytes).map_err(damaged)?;
        if stored.format != "harshpad-doc" || stored.version != 1 {
            return Err(Error::Format(
                "this document was made by a newer HarshPad".into(),
            ));
        }
        let doc = stored.doc;
        let total: usize = doc.runs.iter().map(|r| r.len).sum();
        let boundaries_ok = doc
            .styled_ranges()
            .all(|(_, r)| doc.text.is_char_boundary(r.start) && doc.text.is_char_boundary(r.end));
        if total != doc.text.len()
            || !boundaries_ok
            || doc.paras.len() != doc.text.matches('\n').count() + 1
        {
            return Err(damaged("the styles don't match the text"));
        }
        Ok(doc)
    }
}

/// Builds a document paragraph by paragraph (used by the importers).
#[derive(Default)]
pub struct DocBuilder {
    text: String,
    runs: Vec<Run>,
    paras: Vec<Paragraph>,
}

impl DocBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends one paragraph made of styled pieces. Line breaks inside pieces become spaces.
    pub fn paragraph(&mut self, para: Paragraph, pieces: &[(String, CharStyle)]) {
        if !self.paras.is_empty() {
            self.text.push('\n');
            let style = self
                .runs
                .last()
                .map(|r| r.style.clone())
                .unwrap_or_default();
            self.runs.push(Run { len: 1, style });
        }
        self.paras.push(para);
        for (text, style) in pieces {
            let clean = text.replace(['\n', '\r'], " ");
            if !clean.is_empty() && self.text.len() + clean.len() <= MAX_TEXT {
                self.runs.push(Run {
                    len: clean.len(),
                    style: style.clone(),
                });
                self.text.push_str(&clean);
            }
        }
    }

    pub fn finish(self) -> RichDoc {
        let mut doc = RichDoc {
            text: self.text,
            runs: self.runs,
            paras: self.paras,
        };
        doc.merge_runs();
        doc.repair_paras();
        doc
    }
}

fn floor_boundary(text: &str, at: usize) -> usize {
    let mut i = at.min(text.len());
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bold() -> CharStyle {
        CharStyle {
            bold: true,
            ..CharStyle::default()
        }
    }

    /// The invariants every operation must keep.
    fn check(doc: &RichDoc) {
        let total: usize = doc.runs().iter().map(|r| r.len).sum();
        assert_eq!(total, doc.text().len(), "runs must cover the text");
        assert_eq!(doc.paras().len(), doc.text().matches('\n').count() + 1);
        assert!(doc.runs().iter().all(|r| r.len > 0));
    }

    #[test]
    fn typing_follows_the_style_before_the_cursor() {
        let mut d = RichDoc::from_plain("hello world");
        d.set_char_style(0..5, |s| s.bold = true);
        d.apply_edit("helloX world", None);
        check(&d);
        assert_eq!(d.style_at(6), bold());
        assert!(!d.style_at(8).bold);
    }

    #[test]
    fn typing_style_overrides_and_deletions_shrink_runs() {
        let mut d = RichDoc::from_plain("ab");
        d.apply_edit("aXb", Some(&bold()));
        check(&d);
        assert_eq!(d.runs().len(), 3);
        d.apply_edit("ab", None);
        check(&d);
        assert_eq!(d.runs().len(), 1);
    }

    #[test]
    fn enter_splits_paragraphs_with_the_right_styles() {
        let mut d = RichDoc::from_plain("Title");
        d.set_para_style(0..0, ParaStyle::Heading1);
        d.apply_edit("Title\n", None);
        check(&d);
        assert_eq!(d.paras()[1].style, ParaStyle::Normal);
        // Splitting in the middle keeps the heading on both halves.
        let mut e = RichDoc::from_plain("Title");
        e.set_para_style(0..0, ParaStyle::Heading1);
        e.apply_edit("Ti\ntle", None);
        assert_eq!(e.paras()[1].style, ParaStyle::Heading1);
    }

    #[test]
    fn backspace_across_paragraphs_merges_them() {
        let mut d = RichDoc::from_plain("one\ntwo\nthree");
        d.set_para_style(4..4, ParaStyle::Quote);
        d.apply_edit("onetwo\nthree", None);
        check(&d);
        assert_eq!(d.paras().len(), 2);
        assert_eq!(d.paras()[0].style, ParaStyle::Normal);
    }

    #[test]
    fn lists_continue_and_end_on_an_empty_item() {
        let mut d = RichDoc::from_plain("item");
        d.set_para_style(0..0, ParaStyle::Bullet);
        d.apply_edit("item\n", None);
        assert_eq!(d.paras()[1].style, ParaStyle::Bullet);
        let out = d.apply_edit("item\n\n", None);
        check(&d);
        assert!(out.adjusted);
        assert_eq!(d.text(), "item\n");
        assert_eq!(d.paras()[1].style, ParaStyle::Normal);
    }

    #[test]
    fn markdown_shortcuts_format_the_paragraph() {
        for (typed, style) in [
            ("- ", ParaStyle::Bullet),
            ("1. ", ParaStyle::Numbered),
            ("## ", ParaStyle::Heading2),
            ("> ", ParaStyle::Quote),
        ] {
            let mut d = RichDoc::from_plain(typed.trim_end());
            let out = d.apply_edit(typed, None);
            check(&d);
            assert!(out.adjusted, "{typed}");
            assert_eq!(d.text(), "");
            assert_eq!(d.paras()[0].style, style);
        }
        // Not at the start of a line: nothing happens.
        let mut d = RichDoc::from_plain("a -");
        assert!(!d.apply_edit("a - ", None).adjusted);
    }

    #[test]
    fn toggles_apply_to_the_whole_selection() {
        let mut d = RichDoc::from_plain("hello");
        d.toggle(0..5, Flag::Bold);
        assert!(d.all_have(0..5, Flag::Bold));
        d.toggle(1..3, Flag::Bold);
        assert!(!d.all_have(0..5, Flag::Bold));
        d.toggle(0..5, Flag::Bold);
        assert!(d.all_have(0..5, Flag::Bold));
        check(&d);
    }

    #[test]
    fn non_ascii_edits_keep_boundaries() {
        let mut d = RichDoc::from_plain("héllo wörld ✓");
        d.set_char_style(1..9, |s| s.italic = true);
        d.apply_edit("hé✓llo wörld ✓", None);
        d.apply_edit("✓", None);
        d.apply_edit("", None);
        check(&d);
        // Ranges inside a character are rounded down, never a panic.
        let mut e = RichDoc::from_plain("é");
        e.set_char_style(1..2, |s| s.bold = true);
        check(&e);
    }

    #[test]
    fn json_round_trip_and_damage_detection() {
        let mut d = RichDoc::from_plain("Hello\nworld");
        d.set_para_style(0..0, ParaStyle::Title);
        d.set_char_style(6..11, |s| s.color = Some(crate::Rgb(200, 0, 0)));
        let bytes = d.to_json().unwrap();
        assert_eq!(RichDoc::from_json(&bytes).unwrap(), d);
        let broken = String::from_utf8(bytes)
            .unwrap()
            .replace("\"len\": 6", "\"len\": 60");
        assert!(RichDoc::from_json(broken.as_bytes()).is_err());
        assert!(RichDoc::from_json(b"{}").is_err());
    }

    #[test]
    fn builder_makes_documents() {
        let mut b = DocBuilder::new();
        b.paragraph(
            Paragraph::new(ParaStyle::Title),
            &[("Hi".into(), CharStyle::default())],
        );
        b.paragraph(
            Paragraph::default(),
            &[("a ".into(), CharStyle::default()), ("b".into(), bold())],
        );
        let d = b.finish();
        check(&d);
        assert_eq!(DocBuilder::new().finish().paras().len(), 1);
        assert_eq!(d.text(), "Hi\na b");
        assert_eq!(d.paras()[0].style, ParaStyle::Title);
        assert_eq!(d.word_count(), 3);
    }
}
