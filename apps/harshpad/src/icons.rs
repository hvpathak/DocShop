//! The suite's app icons: DocShop (the whole suite), NoteShop, WordShop and PDFShop, from the
//! artwork in assets/apps, plus a drawn folder for "Open a file…". If a picture can't be
//! loaded, a simple drawn icon stands in, so a tab never shows a blank.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Stroke, TextureFilter,
    TextureHandle, TextureOptions, pos2,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppIcon {
    /// DocShop: the whole suite.
    Suite,
    /// NoteShop.
    Notepad,
    /// WordShop.
    Writer,
    /// PDFShop.
    Pdf,
    Folder,
}

/// The artwork for each app (128 px squares with rounded, transparent corners).
fn artwork(icon: AppIcon) -> Option<(&'static str, &'static [u8])> {
    Some(match icon {
        AppIcon::Suite => (
            "docshop",
            include_bytes!("../../../assets/apps/docshop.png"),
        ),
        AppIcon::Notepad => (
            "noteshop",
            include_bytes!("../../../assets/apps/noteshop.png"),
        ),
        AppIcon::Writer => (
            "wordshop",
            include_bytes!("../../../assets/apps/wordshop.png"),
        ),
        AppIcon::Pdf => (
            "pdfshop",
            include_bytes!("../../../assets/apps/pdfshop.png"),
        ),
        AppIcon::Folder => return None,
    })
}

/// The icon's texture, decoded once and kept for the session. Mipmaps keep it smooth when
/// drawn as small as a tab icon.
fn texture(ctx: &egui::Context, icon: AppIcon) -> Option<TextureHandle> {
    let (name, png) = artwork(icon)?;
    let id = Id::new(("app-icon", name));
    if let Some(t) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return Some(t);
    }
    let decoded = eframe::icon_data::from_png_bytes(png).ok()?;
    let image = egui::ColorImage::from_rgba_unmultiplied(
        [decoded.width as usize, decoded.height as usize],
        &decoded.rgba,
    );
    let options = TextureOptions::LINEAR.with_mipmap_mode(Some(TextureFilter::Linear));
    let t = ctx.load_texture(name, image, options);
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
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
    if let Some(t) = texture(painter.ctx(), icon) {
        let side = rect.width().min(rect.height());
        let tint = if dim {
            Color32::from_white_alpha(90)
        } else {
            Color32::WHITE
        };
        painter.image(
            t.id(),
            Rect::from_center_size(rect.center(), egui::vec2(side, side)),
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            tint,
        );
        return;
    }
    paint_drawn(painter, rect, icon, dim);
}

/// The drawn stand-in, used when the artwork can't be loaded.
fn paint_drawn(painter: &Painter, rect: Rect, icon: AppIcon, dim: bool) {
    let s = rect.width().min(rect.height()) / 16.0;
    let o = rect.center() - egui::vec2(8.0 * s, 8.0 * s);
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
                egui::StrokeKind::Inside,
            );
            // A red margin and blue ruled lines.
            line(p(5.0, 2.0), p(5.0, 14.0), 0.8, RED);
            for y in [5.0, 7.7, 10.4] {
                line(p(6.5, y), p(11.5, y), 1.1, LIGHT_BLUE);
            }
        }
        AppIcon::Writer | AppIcon::Suite => {
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
            painter.add(egui::Shape::convex_polygon(
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
