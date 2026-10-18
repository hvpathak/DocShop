//! An editing session: the document plus Find/Replace state. The UI passes the
//! caret position in and shows whatever range comes back as the selection.

use std::ops::Range;

use crate::document::Document;
use crate::find;

#[derive(Debug, Clone, Default)]
pub struct FindOptions {
    pub needle: String,
    pub replacement: String,
    pub match_case: bool,
}

#[derive(Debug, Default)]
pub struct Session {
    pub doc: Document,
    pub find: FindOptions,
    /// Byte range of the last match, which is what Replace acts on.
    current_match: Option<Range<usize>>,
}

impl Session {
    pub fn new(doc: Document) -> Self {
        Self {
            doc,
            ..Self::default()
        }
    }

    /// Swaps in another document, keeping the Find options.
    pub fn set_document(&mut self, doc: Document) {
        self.doc = doc;
        self.current_match = None;
    }

    pub fn text_edited(&mut self) {
        self.doc.mark_changed();
        self.current_match = None;
    }

    pub fn search_changed(&mut self) {
        self.current_match = None;
    }

    /// Clicking elsewhere drops the current match, so the next search starts at the caret.
    pub fn caret_moved(&mut self, caret: usize) {
        if self.current_match.as_ref().map(|m| m.end) != Some(caret) {
            self.current_match = None;
        }
    }

    pub fn current_match(&self) -> Option<&Range<usize>> {
        self.current_match.as_ref()
    }

    /// `caret` is a byte offset. Returns the match to select, if any.
    pub fn find_next(&mut self, caret: usize) -> Option<Range<usize>> {
        let from = self.current_match.as_ref().map_or(caret, |m| m.end);
        self.current_match = find::find_next(
            &self.doc.text,
            &self.find.needle,
            from,
            self.find.match_case,
        );
        self.current_match.clone()
    }

    pub fn find_previous(&mut self, caret: usize) -> Option<Range<usize>> {
        let before = self.current_match.as_ref().map_or(caret, |m| m.start);
        self.current_match = find::find_previous(
            &self.doc.text,
            &self.find.needle,
            before,
            self.find.match_case,
        );
        self.current_match.clone()
    }

    /// Replaces the current match (if it still matches) and moves to the next one.
    pub fn replace_next(&mut self, caret: usize) -> Option<Range<usize>> {
        if let Some(range) = self.current_match.take()
            && find::is_match(
                &self.doc.text,
                &range,
                &self.find.needle,
                self.find.match_case,
            )
        {
            // `is_match` proved `range` lies on char boundaries inside the text.
            self.doc
                .text
                .replace_range(range.clone(), &self.find.replacement);
            self.doc.mark_changed();
            let end = range.start.saturating_add(self.find.replacement.len());
            self.current_match = Some(end..end);
        }
        self.find_next(caret)
    }

    /// Returns how many matches were replaced.
    pub fn replace_all(&mut self) -> usize {
        self.current_match = None;
        match find::replace_all(
            &self.doc.text,
            &self.find.needle,
            &self.find.replacement,
            self.find.match_case,
        ) {
            Some((text, count)) if count > 0 => {
                self.doc.text = text;
                self.doc.mark_changed();
                count
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(text: &str, needle: &str, replacement: &str) -> Session {
        let mut doc = Document::new();
        doc.text = text.to_owned();
        let mut s = Session::new(doc);
        s.find.needle = needle.to_owned();
        s.find.replacement = replacement.to_owned();
        s
    }

    #[test]
    fn find_next_and_previous_step_through_matches() {
        let mut s = session("a1 a2 a3", "a", "");
        assert_eq!(s.find_next(0), Some(0..1));
        assert_eq!(s.find_next(0), Some(3..4));
        assert_eq!(s.find_previous(0), Some(0..1));
        assert_eq!(s.find_previous(0), Some(6..7));
    }

    #[test]
    fn moving_the_caret_restarts_the_search_there() {
        let mut s = session("a1 a2 a3", "a", "");
        assert_eq!(s.find_next(0), Some(0..1));
        s.caret_moved(5);
        assert_eq!(s.find_next(5), Some(6..7));
    }

    #[test]
    fn replace_next_replaces_only_the_selected_match() {
        let mut s = session("cat cat", "cat", "dog");
        // First press only selects; nothing is replaced yet.
        assert_eq!(s.replace_next(0), Some(0..3));
        assert_eq!(s.doc.text, "cat cat");
        assert_eq!(s.replace_next(0), Some(4..7));
        assert_eq!(s.doc.text, "dog cat");
        assert!(s.doc.is_dirty());
    }

    #[test]
    fn edits_invalidate_the_match() {
        let mut s = session("cat", "cat", "dog");
        s.find_next(0);
        s.doc.text = "x".to_owned();
        s.text_edited();
        assert!(s.current_match().is_none());
        assert_eq!(s.replace_next(0), None);
        assert_eq!(s.doc.text, "x");
    }

    #[test]
    fn replace_all_counts_and_marks_dirty() {
        let mut s = session("Ab ab AB", "ab", "x");
        assert_eq!(s.replace_all(), 3);
        assert_eq!(s.doc.text, "x x x");
        assert!(s.doc.is_dirty());

        let mut none = session("abc", "zzz", "x");
        assert_eq!(none.replace_all(), 0);
        assert!(!none.doc.is_dirty());
    }
}
