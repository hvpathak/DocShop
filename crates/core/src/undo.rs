//! Undo and redo for any kind of tab: snapshots of the state before each change,
//! with quick typing grouped into one step and a memory budget.

use std::time::{Duration, Instant};

/// Typing within this long of the previous keystroke joins the same undo step.
const GROUP: Duration = Duration::from_millis(1200);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Typing,
    Other,
}

#[derive(Debug)]
pub struct UndoStack<T> {
    undo: Vec<(T, usize)>,
    redo: Vec<(T, usize)>,
    last: Option<(EditKind, Instant)>,
    bytes: usize,
    budget: usize,
}

impl<T> Default for UndoStack<T> {
    fn default() -> Self {
        Self::new(256 * 1024 * 1024)
    }
}

impl<T> UndoStack<T> {
    pub fn new(budget_bytes: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            last: None,
            bytes: 0,
            budget: budget_bytes,
        }
    }

    /// Call with the state as it was *before* a change of `size` bytes.
    pub fn record(&mut self, before: T, size: usize, kind: EditKind, now: Instant) {
        let joins = kind == EditKind::Typing
            && matches!(self.last, Some((EditKind::Typing, t)) if now.saturating_duration_since(t) < GROUP)
            && !self.undo.is_empty();
        self.redo.clear();
        self.last = Some((kind, now));
        if joins {
            return;
        }
        self.bytes += size;
        self.undo.push((before, size));
        while self.bytes > self.budget && self.undo.len() > 1 {
            let (_, s) = self.undo.remove(0);
            self.bytes = self.bytes.saturating_sub(s);
        }
    }

    /// Returns the state to go back to; `current` becomes redoable.
    pub fn undo(&mut self, current: T, size: usize) -> Option<T> {
        let (prev, s) = self.undo.pop()?;
        self.bytes = self.bytes.saturating_sub(s);
        self.redo.push((current, size));
        self.last = None;
        Some(prev)
    }

    pub fn redo(&mut self, current: T, size: usize) -> Option<T> {
        let (next, _) = self.redo.pop()?;
        self.undo.push((current, size));
        self.bytes += size;
        self.last = None;
        Some(next)
    }

    /// Ends the current typing group (cursor moved, formatting applied…).
    pub fn break_group(&mut self) {
        self.last = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typing_groups_and_undo_redo_round_trip() {
        let t0 = Instant::now();
        let mut u = UndoStack::default();
        u.record("".to_owned(), 0, EditKind::Typing, t0);
        u.record(
            "a".to_owned(),
            1,
            EditKind::Typing,
            t0 + Duration::from_millis(100),
        );
        u.record(
            "ab".to_owned(),
            2,
            EditKind::Typing,
            t0 + Duration::from_secs(5),
        );
        // "a" joined the first group; the late keystroke started a new one.
        assert_eq!(u.undo("abc".to_owned(), 3).as_deref(), Some("ab"));
        assert_eq!(u.undo("ab".to_owned(), 2).as_deref(), Some(""));
        assert!(u.undo(String::new(), 0).is_none());
        assert_eq!(u.redo(String::new(), 0).as_deref(), Some("ab"));
        assert_eq!(u.redo("ab".to_owned(), 2).as_deref(), Some("abc"));
        assert!(!u.can_redo());
    }

    #[test]
    fn other_edits_never_group_and_new_edits_clear_redo() {
        let t0 = Instant::now();
        let mut u = UndoStack::default();
        u.record(1, 1, EditKind::Other, t0);
        u.record(2, 1, EditKind::Other, t0);
        assert_eq!(u.undo(3, 1), Some(2));
        u.record(9, 1, EditKind::Other, t0);
        assert!(!u.can_redo());
    }

    #[test]
    fn budget_drops_the_oldest_steps() {
        let t0 = Instant::now();
        let mut u = UndoStack::new(10);
        for i in 0..5 {
            u.record(i, 4, EditKind::Other, t0);
        }
        let mut seen = Vec::new();
        while let Some(v) = u.undo(0, 0) {
            seen.push(v);
        }
        assert_eq!(seen, vec![4, 3]);
    }
}
