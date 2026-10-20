//! Split view: two tabs of the same kind side by side, like a web browser's split view.
//! The pane you click is the active one (blue outline); the other stays visible. Each pane has
//! a × in its bottom-right corner that takes it out of the split (the tab stays open).

use eframe::egui::{Rect, pos2, vec2};

/// Neither side gets narrower than this share of the window.
pub const MIN_SIDE: f32 = 0.2;
/// Space around the panes and between them (the divider).
pub const MARGIN: f32 = 6.0;
pub const GAP: f32 = 10.0;
/// Room under the panes for each side's × (so it never covers the editor's own controls).
pub const BOTTOM: f32 = 24.0;

/// The two tabs (by id) in the split, left and right, and where the divider sits (0–1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Split {
    pub left: u64,
    pub right: u64,
    pub ratio: f32,
}

impl Split {
    pub fn new(left: u64, right: u64) -> Self {
        Self {
            left,
            right,
            ratio: 0.5,
        }
    }

    pub fn contains(&self, id: u64) -> bool {
        self.left == id || self.right == id
    }

    /// Moves the divider, keeping both sides usable; nonsense (NaN) puts it back in the middle.
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = if ratio.is_finite() {
            ratio.clamp(MIN_SIDE, 1.0 - MIN_SIDE)
        } else {
            0.5
        };
    }
}

/// The left pane, the right pane and the divider between them, inside `full`.
pub fn panes(full: Rect, ratio: f32) -> (Rect, Rect, Rect) {
    let ratio = if ratio.is_finite() {
        ratio.clamp(MIN_SIDE, 1.0 - MIN_SIDE)
    } else {
        0.5
    };
    let x = full.left() + full.width() * ratio;
    let top = full.top() + MARGIN * 0.5;
    let bottom = (full.bottom() - BOTTOM).max(top);
    let left = Rect::from_min_max(
        pos2(full.left() + MARGIN, top),
        pos2((x - GAP / 2.0).max(full.left() + MARGIN), bottom),
    );
    let right = Rect::from_min_max(
        pos2((x + GAP / 2.0).min(full.right() - MARGIN), top),
        pos2(full.right() - MARGIN, bottom),
    );
    let divider = Rect::from_min_max(pos2(left.right(), top), pos2(right.left(), bottom));
    (left, right, divider.expand2(vec2(2.0, 0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_split_holds_its_two_tabs() {
        let s = Split::new(3, 8);
        assert!(s.contains(3) && s.contains(8) && !s.contains(5));
        assert_eq!(s.ratio, 0.5);
    }

    #[test]
    fn the_divider_keeps_both_sides_usable() {
        let mut s = Split::new(1, 2);
        s.set_ratio(0.01);
        assert_eq!(s.ratio, MIN_SIDE);
        s.set_ratio(0.99);
        assert_eq!(s.ratio, 1.0 - MIN_SIDE);
        s.set_ratio(f32::NAN);
        assert_eq!(s.ratio, 0.5);
    }

    #[test]
    fn panes_sit_side_by_side_with_a_gap() {
        let full = Rect::from_min_size(pos2(0.0, 100.0), vec2(1000.0, 600.0));
        let (left, right, divider) = panes(full, 0.5);
        assert!(left.right() < right.left());
        assert_eq!(right.left() - left.right(), GAP);
        assert!((left.width() - right.width()).abs() < 0.01);
        assert!(divider.contains(pos2(500.0, 400.0)));
        assert!(full.contains_rect(left) && full.contains_rect(right));
        // A window too small for both still gives valid (possibly empty) rects.
        let tiny = Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 4.0));
        let (l, r, _) = panes(tiny, f32::INFINITY);
        assert!(l.width() >= 0.0 && r.width() >= 0.0 && l.height() >= 0.0);
    }
}
