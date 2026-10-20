//! The start-up card: a small centred window with the logo, the suite's name and
//! "Starting…", shown while Writer Studio and PDF Studio load underneath it. Then the
//! window grows to its working size. Follows the system's light or dark theme.

use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Id, Pos2, Rect, Sense, Stroke, TextureHandle,
    ViewportCommand, pos2, vec2,
};

use crate::icons::{self, AppIcon};

/// The splash window's size, and the working window it grows into.
/// The card's size, like the installed word processor's (500×310 pixels at 125%).
pub const SPLASH_SIZE: [f32; 2] = [400.0, 248.0];
pub const WINDOW_SIZE: [f32; 2] = [1180.0, 800.0];
/// Shown at least this long, so it never flickers.
const MIN_SHOWN: Duration = Duration::from_millis(1100);
/// A few frames, so the card is on screen before the window grows.
const MIN_FRAMES: u32 = 3;

pub struct Splash {
    /// When the card was first on screen (the editors load before the first frame).
    started: Option<Instant>,
    frames: u32,
    logo: Option<TextureHandle>,
}

impl Splash {
    pub fn new(ctx: &egui::Context, logo_png: &[u8]) -> Self {
        let logo = eframe::icon_data::from_png_bytes(logo_png)
            .ok()
            .map(|icon| {
                let image = egui::ColorImage::from_rgba_unmultiplied(
                    [icon.width as usize, icon.height as usize],
                    &icon.rgba,
                );
                ctx.load_texture("harshpad-logo", image, egui::TextureOptions::LINEAR)
            });
        Self {
            started: None,
            frames: 0,
            logo,
        }
    }

    /// Draws the card over everything. Returns true when it's done (the window has been
    /// asked to grow), so the caller can drop it.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        status: &str,
        last: Option<([f32; 4], bool)>,
    ) -> bool {
        let ctx = &ui.ctx().clone();
        self.frames += 1;
        let started = *self.started.get_or_insert_with(Instant::now);
        let done = self.frames >= MIN_FRAMES && started.elapsed() >= MIN_SHOWN;
        if done {
            grow(ctx, last);
            return true;
        }
        ctx.request_repaint();
        let dark = ctx
            .system_theme()
            .map_or(ctx.global_style().visuals.dark_mode, |t| {
                t == egui::Theme::Dark
            });
        let (bg, text, muted, pill) = if dark {
            (
                Color32::from_rgb(32, 33, 36),
                Color32::from_gray(240),
                Color32::from_gray(160),
                Color32::from_rgb(40, 62, 102),
            )
        } else {
            (
                Color32::WHITE,
                Color32::from_gray(25),
                Color32::from_gray(105),
                Color32::from_rgb(220, 232, 252),
            )
        };
        let screen = ctx.content_rect();
        {
            {
                // The whole window: drag it by any spot. Painted straight into the window (an
                // Area would stay hidden on its first frame, showing what's underneath).
                let r = ui.interact(screen, Id::new("splash"), Sense::click_and_drag());
                if r.drag_started() {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                let p = ui.painter();
                p.rect_filled(screen, CornerRadius::ZERO, bg);
                p.rect_stroke(
                    screen.shrink(0.5),
                    CornerRadius::ZERO,
                    Stroke::new(1.0_f32, muted.gamma_multiply(0.4)),
                    egui::StrokeKind::Inside,
                );
                // Version pill, top left.
                let label = format!("DocShop {}", env!("CARGO_PKG_VERSION"));
                let galley = p.layout_no_wrap(label, FontId::proportional(13.0), text);
                let pill_rect = Rect::from_min_size(
                    screen.min + vec2(18.0, 16.0),
                    galley.size() + vec2(20.0, 10.0),
                );
                p.rect_filled(pill_rect, 6.0, pill);
                p.galley(pill_rect.min + vec2(10.0, 5.0), galley, text);
                // The logo, the name and the three apps.
                let c = screen.center() - vec2(0.0, 26.0);
                let logo_rect = Rect::from_center_size(c, vec2(78.0, 78.0));
                match &self.logo {
                    Some(t) => {
                        p.image(
                            t.id(),
                            logo_rect,
                            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    None => icons::paint(p, logo_rect, AppIcon::Suite, false),
                }
                p.text(
                    c + vec2(0.0, 58.0),
                    Align2::CENTER_CENTER,
                    "DocShop",
                    FontId::proportional(24.0),
                    text,
                );
                // Notepad · Writer Studio · PDF Studio, with their icons.
                let apps = [
                    (AppIcon::Notepad, "NoteShop"),
                    (AppIcon::Writer, "WordShop"),
                    (AppIcon::Pdf, "PDFShop"),
                ];
                let font = FontId::proportional(12.5);
                let widths: Vec<f32> = apps
                    .iter()
                    .map(|(_, n)| {
                        p.layout_no_wrap((*n).to_owned(), font.clone(), muted)
                            .size()
                            .x
                    })
                    .collect();
                let total: f32 = widths.iter().sum::<f32>() + 3.0 * 20.0 + 2.0 * 18.0;
                let mut x = c.x - total / 2.0;
                let y = c.y + 86.0;
                for ((icon, name), w) in apps.iter().zip(widths) {
                    icons::paint(
                        p,
                        Rect::from_center_size(pos2(x + 8.0, y), vec2(16.0, 16.0)),
                        *icon,
                        false,
                    );
                    p.text(
                        pos2(x + 20.0, y),
                        Align2::LEFT_CENTER,
                        *name,
                        font.clone(),
                        muted,
                    );
                    x += 20.0 + w + 18.0;
                }
                // Status, bottom left, with moving dots.
                let dots = ".".repeat(1 + (started.elapsed().as_millis() / 300 % 3) as usize);
                p.text(
                    pos2(screen.left() + 18.0, screen.bottom() - 20.0),
                    Align2::LEFT_CENTER,
                    format!("{status}{dots}"),
                    FontId::proportional(12.5),
                    muted,
                );
                p.text(
                    pos2(screen.right() - 18.0, screen.bottom() - 20.0),
                    Align2::RIGHT_CENTER,
                    "Dedicated to Harshal",
                    FontId::proportional(11.0),
                    muted.gamma_multiply(0.8),
                );
            }
        }
        false
    }
}

/// Grows the window to where it was last time (like the installed word processor, which
/// reopens at its last size and place), or to the working size centred on the monitor.
fn grow(ctx: &egui::Context, last: Option<([f32; 4], bool)>) {
    let monitor = ctx.input(|i| i.viewport().monitor_size);
    let fits = |[x, y, w, h]: [f32; 4]| {
        w >= 560.0
            && h >= 380.0
            && monitor
                .is_none_or(|m| x > -w / 2.0 && y >= -10.0 && x < m.x - 80.0 && y < m.y - 80.0)
    };
    if let Some((geo, maximized)) = last.filter(|(g, _)| fits(*g)) {
        let [x, y, w, h] = geo;
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(vec2(w, h)));
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos2(x, y)));
        if maximized {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(true));
        }
        return;
    }
    let size = vec2(WINDOW_SIZE[0], WINDOW_SIZE[1]);
    let size = monitor.map_or(size, |m| size.min(m - vec2(40.0, 60.0)));
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
    if let Some(m) = monitor {
        let pos = ((m - size) / 2.0).max(vec2(0.0, 0.0));
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos2(pos.x, pos.y)));
    }
}
