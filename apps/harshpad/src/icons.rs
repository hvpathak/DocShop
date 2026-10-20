//! Colour icons for the three apps of the suite (Notepad, Writer Studio, PDF Studio)
//! and for opening a file. Drawn as shapes on a 16-point grid, so they stay sharp at
//! any size and look the same on every platform.

use eframe::egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, pos2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIcon {
    Notepad,
    Writer,
    Pdf,
    Folder,
}

const BLUE: Color32 = Color32::from_rgb(37, 99, 235);
const LIGHT_BLUE: Color32 = Color32::from_rgb(59, 130, 246);
const RED: Color32 = Color32::from_rgb(220, 38, 38);
const ORANGE: Color32 = Color32::from_rgb(245, 158, 11);
const AMBER: Color32 = Color32::from_rgb(251, 191, 36);
const PAPER: Color32 = Color32::from_rgb(246, 248, 251);
const EDGE: Color32 = Color32::from_rgb(148, 160, 178);

/// Paints `icon` filling `rect` (square). `dim` greys it out (a locked choice).
pub fn paint(painter: &Painter, rect: Rect, icon: AppIcon, dim: bool) {
    let s = rect.width().min(rect.height()) / 16.0;
    let o = rect.center() - eframe::egui::vec2(8.0 * s, 8.0 * s);
    let p = |x: f32, y: f32| -> Pos2 { pos2(o.x + x * s, o.y + y * s) };
    let c = |col: Color32| if dim { col.gamma_multiply(0.35) } else { col };
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(p(x0, y0), p(x1, y1));
    let line = |a: Pos2, b: Pos2, w: f32, col: Color32| {
        painter.line_segment([a, b], Stroke::new(w * s, c(col)));
    };
    let page = r(2.5, 1.0, 13.5, 15.0);
    let round = CornerRadius::same((1.6 * s).round().clamp(1.0, 255.0) as u8);
    match icon {
        AppIcon::Notepad => {
            painter.rect_filled(page, round, c(PAPER));
            painter.rect_stroke(
                page,
                round,
                Stroke::new(0.9 * s, c(EDGE)),
                eframe::egui::StrokeKind::Inside,
            );
            // A red margin and blue ruled lines.
            line(p(5.0, 2.0), p(5.0, 14.0), 0.8, RED);
            for y in [5.0, 7.7, 10.4] {
                line(p(6.5, y), p(11.5, y), 1.1, LIGHT_BLUE);
            }
        }
        AppIcon::Writer => {
            painter.rect_filled(page, round, c(BLUE));
            for (y, end) in [(4.8, 11.0), (7.3, 11.0), (9.8, 8.6)] {
                line(p(5.0, y), p(end, y), 1.2, Color32::WHITE);
            }
            // An orange pen across the corner.
            line(p(8.8, 14.2), p(14.6, 8.4), 2.2, ORANGE);
            line(
                p(8.0, 15.0),
                p(8.8, 14.2),
                1.2,
                Color32::from_rgb(55, 65, 81),
            );
        }
        AppIcon::Pdf => {
            painter.rect_filled(page, round, c(RED));
            // A folded corner and a white band.
            painter.add(eframe::egui::Shape::convex_polygon(
                vec![p(10.0, 1.0), p(13.5, 4.5), p(10.0, 4.5)],
                c(Color32::from_rgb(254, 202, 202)),
                Stroke::NONE,
            ));
            let band = r(2.5, 8.6, 13.5, 12.6);
            painter.rect_filled(band, 0.0, c(Color32::WHITE));
            if rect.height() >= 20.0 {
                painter.text(
                    band.center(),
                    Align2::CENTER_CENTER,
                    "PDF",
                    FontId::new(3.4 * s, crate::fonts::bold(painter.ctx())),
                    c(RED),
                );
            } else {
                line(p(4.5, 10.6), p(11.5, 10.6), 1.1, RED);
            }
        }
        AppIcon::Folder => {
            let back = r(1.0, 3.0, 15.0, 13.5);
            painter.rect_filled(r(1.0, 2.0, 7.0, 5.0), round, c(ORANGE));
            painter.rect_filled(back, round, c(ORANGE));
            painter.rect_filled(r(1.0, 5.5, 15.0, 13.5), round, c(AMBER));
        }
    }
}
