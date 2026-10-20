//! Browser-style window chrome, matching how a Chromium browser's tab strip behaves:
//! tabs in the title bar with fading titles, a close button on the active and hovered
//! tab, + right after the last tab, a ⌄ menu before the window controls (new tab as…,
//! search open tabs, recently closed), drag to reorder, and widths that hold still
//! while you close tabs so the next × lands under the pointer.

use eframe::egui::{
    self, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Mesh, PopupCloseBehavior, Rect,
    ResizeDirection, RichText, Sense, Shape, Stroke, ViewportCommand, pos2, vec2,
};
use harshpad_core::commands::TabKind;

use crate::icons::{self, AppIcon};

pub const BAR_H: f32 = 40.0;
const TAB_TOP: f32 = 7.0;
const MAX_TAB_W: f32 = 240.0;
const MIN_TAB_W: f32 = 56.0;
const PLUS_W: f32 = 36.0;
const CONTROL_W: f32 = 46.0;

pub struct TabInfo {
    pub title: String,
    pub icon: AppIcon,
    pub dirty: bool,
    pub tooltip: String,
    /// Tabs that can share a split view have the same family (Notepad, Writer, PDF…).
    pub family: u8,
    /// Shown in the split view right now (with the active tab).
    pub split: bool,
}

/// Whether a kind can be opened from the + menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Open,
    /// The current mode has this app switched off.
    Off,
    /// Harshal's growing path unlocks it in this chapter.
    Locked(u8),
}

/// The three apps the + menu offers (the mode and the growing path decide).
pub struct Offer {
    pub text: Choice,
    pub doc: Choice,
    pub pdf: Choice,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Select(usize),
    Close(usize),
    Move {
        from: usize,
        to: usize,
    },
    New(TabKind),
    Open,
    /// A choice the growing path hasn't unlocked yet (its chapter).
    Locked(u8),
    /// A choice the current mode has switched off.
    Off,
    /// Show these two tabs side by side (the first stays active).
    Split(usize, usize),
    /// Back to one tab at a time.
    Unsplit,
}

fn macos() -> bool {
    cfg!(target_os = "macos")
}

pub fn bar_colour(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_gray(14)
    } else {
        Color32::from_gray(218)
    }
}

/// The 3–4 choices for a new tab, shared by the + picker and the ⌄ menu.
/// One choice: big colour icon, bold name, grey description, shortcut. Returns true when
/// clicked.
fn card(ui: &mut egui::Ui, icon: AppIcon, title: &str, detail: &str, key: &str, dim: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(340.0, 50.0), Sense::click());
    let visuals = ui.visuals().clone();
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 6.0, visuals.widgets.hovered.weak_bg_fill);
    }
    let painter = ui.painter();
    icons::paint(
        painter,
        Rect::from_center_size(pos2(rect.left() + 22.0, rect.center().y), vec2(26.0, 26.0)),
        icon,
        dim,
    );
    let strong = if dim {
        visuals.weak_text_color()
    } else {
        visuals.strong_text_color()
    };
    painter.text(
        pos2(rect.left() + 46.0, rect.top() + 16.0),
        Align2::LEFT_CENTER,
        title,
        FontId::new(14.0, crate::fonts::bold(ui.ctx())),
        strong,
    );
    painter.text(
        pos2(rect.left() + 46.0, rect.top() + 35.0),
        Align2::LEFT_CENTER,
        detail,
        FontId::proportional(12.0),
        visuals.weak_text_color(),
    );
    painter.text(
        pos2(rect.right() - 10.0, rect.top() + 16.0),
        Align2::RIGHT_CENTER,
        key,
        FontId::proportional(11.5),
        visuals.weak_text_color(),
    );
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// The three apps of the suite, always all three: Notepad, Writer Studio, PDF Studio.
fn new_tab_choices(ui: &mut egui::Ui, offer: &Offer, events: &mut Vec<Event>) {
    let apps = [
        (
            offer.text,
            TabKind::Text,
            AppIcon::Notepad,
            "NoteShop",
            "Quick notes, big ideas",
            "Ctrl+N",
        ),
        (
            offer.doc,
            TabKind::Doc,
            AppIcon::Writer,
            "WordShop",
            "Write, edit and create with ease · .docx, .odt, Markdown",
            "Ctrl+Shift+N",
        ),
        (
            offer.pdf,
            TabKind::Pdf,
            AppIcon::Pdf,
            "PDFShop",
            "View, edit and manage PDFs",
            "",
        ),
    ];
    for (choice, kind, icon, title, detail, key) in apps {
        let (detail, key) = match choice {
            Choice::Open => (detail.to_owned(), key),
            Choice::Off => ("Off in this mode · switch at ◧ top right".to_owned(), ""),
            Choice::Locked(n) => (format!("🔒 Unlocks in Chapter {n}"), ""),
        };
        if card(ui, icon, title, &detail, key, choice != Choice::Open) {
            events.push(match choice {
                Choice::Open => Event::New(kind),
                Choice::Off => Event::Off,
                Choice::Locked(n) => Event::Locked(n),
            });
            egui::Popup::close_all(ui.ctx());
        }
    }
}

/// The window with no tabs left: an almost blank page asking which app to open, with the
/// same three cards as the + menu, and "Open a file…".
pub fn start_page(ui: &mut egui::Ui, offer: &Offer) -> Vec<Event> {
    let mut events = Vec::new();
    let height = 4.0 * 50.0 + 70.0;
    ui.vertical_centered(|ui| {
        ui.add_space(((ui.available_height() - height) / 2.0 - 30.0).max(16.0));
        ui.label(
            RichText::new("What would you like to open?")
                .size(20.0)
                .strong(),
        );
        ui.add_space(4.0);
        ui.label(RichText::new("DocShop · one place for all your documents").weak());
        ui.add_space(14.0);
        new_tab_choices(ui, offer, &mut events);
        ui.add_space(6.0);
        if card(
            ui,
            AppIcon::Folder,
            "Open a file…",
            "Text, documents and PDFs",
            "Ctrl+O",
            false,
        ) {
            events.push(Event::Open);
        }
    });
    events
}

/// Where the caller's own control goes (the mode switch): `reserve` wide, just left of
/// the window buttons.
pub fn mode_slot(rect: Rect, reserve: f32) -> Rect {
    let controls = if macos() { 0.0 } else { CONTROL_W * 3.0 };
    let right = rect.right() - controls - 6.0;
    Rect::from_min_max(
        pos2(right - reserve, rect.top() + 4.0),
        pos2(right, rect.bottom()),
    )
}

/// Draws the bar and returns what the user did. `reserve` is kept free for [`mode_slot`].
/// `open_picker` opens the new-tab choices (Ctrl+T).
pub fn title_bar(
    ui: &mut egui::Ui,
    reserve: f32,
    tabs: &[TabInfo],
    active: usize,
    offer: &Offer,
    open_picker: bool,
) -> Vec<Event> {
    let mut events = Vec::new();
    let rect = ui.max_rect();
    let ctx = ui.ctx().clone();
    let painter = ui.painter().clone();
    let dark = ui.visuals().dark_mode;
    let bar = bar_colour(ui);
    let active_fill = ui.visuals().panel_fill;
    let hover_fill = if dark {
        Color32::from_gray(40)
    } else {
        Color32::from_gray(228)
    };
    let text_colour = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    painter.rect_filled(rect, 0.0, bar);
    // Indian border art across the right of the bar, right to the edge behind the mode
    // switch and window buttons: soft gold on dark, deep maroon on light.
    let art = Rect::from_min_max(
        pos2(
            (rect.right() - 820.0).max(rect.left() + rect.width() * 0.4),
            rect.top(),
        ),
        rect.right_bottom(),
    );
    flourish(
        &painter,
        art,
        if dark {
            Color32::from_rgba_unmultiplied(226, 186, 96, 70)
        } else {
            Color32::from_rgba_unmultiplied(140, 36, 48, 56)
        },
    );

    let left = rect.left() + if macos() { 78.0 } else { 8.0 };
    let controls = if macos() { 0.0 } else { CONTROL_W * 3.0 };
    let n = tabs.len().max(1) as f32;
    let room = (rect.right() - controls - reserve - PLUS_W - 60.0 - left).max(MIN_TAB_W);
    let natural = (room / n).clamp(MIN_TAB_W, MAX_TAB_W);
    // Like a browser: after closing a tab with the mouse, widths hold still until
    // the pointer leaves the strip.
    let frozen_id = Id::new("chrome-frozen-width");
    let pointer_in_bar = ctx
        .input(|i| i.pointer.hover_pos())
        .is_some_and(|p| rect.contains(p));
    let frozen: Option<f32> = ctx.data(|d| d.get_temp(frozen_id));
    let tab_w = match frozen {
        Some(w) if pointer_in_bar => w,
        _ => {
            ctx.data_mut(|d| d.remove::<f32>(frozen_id));
            natural
        }
    };

    // ---- Tabs -----------------------------------------------------------------------
    let dragging: Option<usize> = ctx.data(|d| d.get_temp(Id::new("chrome-dragging")));
    for (i, tab) in tabs.iter().enumerate() {
        let x = left + tab_w * i as f32;
        let r = Rect::from_min_max(
            pos2(x, rect.top() + TAB_TOP),
            pos2(x + tab_w, rect.bottom()),
        );
        let response = ui
            .interact(r, Id::new(("chrome-tab", i)), Sense::click_and_drag())
            .on_hover_text(&tab.tooltip);
        let is_active = i == active;
        // Both halves of a split view look selected, joined like one wide tab.
        let shown = is_active || tab.split;
        let shape = CornerRadius {
            nw: 9,
            ne: 9,
            sw: 0,
            se: 0,
        };
        let fill = if shown {
            active_fill
        } else if response.hovered() {
            hover_fill
        } else {
            bar
        };
        if shown || response.hovered() {
            painter.rect_filled(
                r.shrink2(vec2(if shown { 0.0 } else { 2.0 }, 0.0)),
                shape,
                fill,
            );
        }
        let next_shown = i + 1 == active || tabs.get(i + 1).is_some_and(|t| t.split);
        if !shown && !next_shown && i + 1 < tabs.len() && !response.hovered() {
            painter.line_segment(
                [
                    pos2(r.right(), r.top() + 9.0),
                    pos2(r.right(), r.bottom() - 9.0),
                ],
                Stroke::new(1.0_f32, weak.gamma_multiply(0.5)),
            );
        }
        let show_close = is_active || response.hovered() || tab_w >= 120.0 && tab.dirty;
        let close = Rect::from_center_size(pos2(r.right() - 17.0, r.center().y), vec2(18.0, 18.0));
        // Favicon-style icon, then the title clipped with a fade at the right.
        icons::paint(
            &painter,
            Rect::from_center_size(pos2(r.left() + 21.0, r.center().y), vec2(16.0, 16.0)),
            tab.icon,
            false,
        );
        let text_left = r.left() + 33.0;
        let text_right = if show_close {
            close.left() - 3.0
        } else {
            r.right() - 8.0
        };
        if text_right > text_left {
            let clip = Rect::from_min_max(pos2(text_left, r.top()), pos2(text_right, r.bottom()));
            let galley = painter.layout_no_wrap(
                tab.title.clone(),
                FontId::proportional(13.0),
                if is_active { text_colour } else { weak },
            );
            let fades = galley.size().x > clip.width();
            painter.with_clip_rect(clip).galley(
                pos2(text_left, r.center().y - galley.size().y / 2.0),
                galley,
                text_colour,
            );
            if fades {
                fade_right(
                    &painter,
                    Rect::from_min_max(
                        pos2(clip.right() - 22.0, r.top() + 4.0),
                        pos2(clip.right(), r.bottom() - 2.0),
                    ),
                    fill,
                );
            }
        }
        let close_response = ui
            .interact(close, Id::new(("chrome-close", i)), Sense::click())
            .on_hover_text("Close tab (Ctrl+W)");
        if show_close || close_response.hovered() {
            if close_response.hovered() {
                painter.circle_filled(close.center(), 9.0, hover_fill.gamma_multiply(1.8));
            }
            let dot = tab.dirty && !close_response.hovered() && !response.hovered();
            let (glyph, size) = if dot { ("●", 10.0) } else { ("×", 15.0) };
            painter.text(
                close.center(),
                Align2::CENTER_CENTER,
                glyph,
                FontId::proportional(size),
                text_colour,
            );
        } else if tab.dirty {
            painter.text(
                close.center(),
                Align2::CENTER_CENTER,
                "●",
                FontId::proportional(9.0),
                weak,
            );
        }

        // Right-click: split view with another tab of the same kind, like a web browser.
        response.context_menu(|ui| {
            if tab.split {
                if ui.button("◫  Exit split view").clicked() {
                    events.push(Event::Unsplit);
                    ui.close();
                }
            } else {
                ui.label(RichText::new("Split view with").strong());
                let mut any = false;
                for (j, other) in tabs.iter().enumerate() {
                    if j == i || other.family != tab.family {
                        continue;
                    }
                    any = true;
                    if ui.button(format!("◫  {}", other.title)).clicked() {
                        events.push(Event::Split(i, j));
                        ui.close();
                    }
                }
                if !any {
                    ui.label(
                        RichText::new(
                            "Open another tab of the same kind to show them side by side",
                        )
                        .weak(),
                    );
                }
            }
            ui.separator();
            if ui.button("Close tab").clicked() {
                events.push(Event::Close(i));
                ui.close();
            }
        });
        if close_response.clicked() || response.middle_clicked() {
            events.push(Event::Close(i));
            ctx.data_mut(|d| d.insert_temp(frozen_id, tab_w));
        } else if response.drag_started() {
            ctx.data_mut(|d| d.insert_temp(Id::new("chrome-dragging"), i));
            events.push(Event::Select(i));
        } else if response.clicked() {
            events.push(Event::Select(i));
        }
        if response.drag_stopped()
            && let Some(p) = ctx.input(|inp| inp.pointer.interact_pos())
        {
            let to = (((p.x - left) / tab_w).floor().max(0.0) as usize).min(tabs.len() - 1);
            if to != i {
                events.push(Event::Move { from: i, to });
            }
            ctx.data_mut(|d| d.remove::<usize>(Id::new("chrome-dragging")));
        }
    }
    // Where a dragged tab will land.
    if let (Some(_), Some(p)) = (dragging, ctx.input(|i| i.pointer.interact_pos()))
        && ctx.input(|i| i.pointer.primary_down())
    {
        let slot = (((p.x - left) / tab_w).round().max(0.0) as usize).min(tabs.len());
        let x = left + tab_w * slot as f32;
        painter.line_segment(
            [pos2(x, rect.top() + 10.0), pos2(x, rect.bottom() - 4.0)],
            Stroke::new(2.0_f32, Color32::from_rgb(30, 110, 210)),
        );
    }

    // ---- + right after the last tab: asks what to open -----------------------------
    let tabs_end = left + tab_w * tabs.len() as f32;
    let plus = Rect::from_center_size(
        pos2(
            tabs_end + PLUS_W / 2.0 + 2.0,
            rect.top() + TAB_TOP + (rect.height() - TAB_TOP) / 2.0,
        ),
        vec2(28.0, 28.0),
    );
    let plus_response = ui
        .interact(plus, Id::new("chrome-plus"), Sense::click())
        .on_hover_text("New tab (Ctrl+T)");
    if plus_response.hovered() {
        painter.circle_filled(plus.center(), 14.0, hover_fill);
    }
    painter.text(
        plus.center(),
        Align2::CENTER_CENTER,
        "+",
        FontId::proportional(20.0),
        text_colour,
    );
    let picker = Id::new("new-tab-picker");
    // + always asks which of the three apps to open.
    if plus_response.clicked() {
        egui::Popup::toggle_id(&ctx, picker);
    }
    if open_picker {
        egui::Popup::open_id(&ctx, picker);
    }
    egui::Popup::from_response(&plus_response)
        .id(picker)
        .open_memory(None)
        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_min_width(340.0);
            ui.add_space(4.0);
            ui.label(RichText::new("Open a new tab as…").strong());
            ui.add_space(4.0);
            new_tab_choices(ui, offer, &mut events);
        });

    // ---- Drag area and window controls -------------------------------------------------
    let drag = Rect::from_min_max(
        pos2(plus.right() + 6.0, rect.top()),
        pos2(rect.right() - controls - reserve - 10.0, rect.bottom()),
    );
    if drag.width() > 0.0 {
        let response = ui.interact(drag, Id::new("chrome-drag"), Sense::click_and_drag());
        if response.double_clicked() {
            toggle_maximized(&ctx);
        } else if response.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
    }
    if !macos() {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        for (k, kind) in ["min", "max", "close"].into_iter().enumerate() {
            let r = Rect::from_min_size(
                pos2(rect.right() - controls + CONTROL_W * k as f32, rect.top()),
                vec2(CONTROL_W, BAR_H),
            );
            let response = ui.interact(r, Id::new(("chrome-control", kind)), Sense::click());
            let hot = response.hovered();
            if hot {
                let fill = if kind == "close" {
                    Color32::from_rgb(232, 17, 35)
                } else {
                    hover_fill
                };
                painter.rect_filled(r, 0.0, fill);
            }
            let colour = if hot && kind == "close" {
                Color32::WHITE
            } else {
                text_colour
            };
            let stroke = Stroke::new(1.0_f32, colour);
            let c = r.center();
            match kind {
                "min" => {
                    painter.line_segment([pos2(c.x - 5.0, c.y), pos2(c.x + 5.0, c.y)], stroke);
                }
                "max" if maximized => {
                    painter.rect_stroke(
                        Rect::from_center_size(c + vec2(-1.5, 1.5), vec2(8.0, 8.0)),
                        0.0,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                    painter.line_segment(
                        [pos2(c.x - 3.0, c.y - 4.5), pos2(c.x + 4.5, c.y - 4.5)],
                        stroke,
                    );
                    painter.line_segment(
                        [pos2(c.x + 4.5, c.y - 4.5), pos2(c.x + 4.5, c.y + 3.0)],
                        stroke,
                    );
                }
                "max" => {
                    painter.rect_stroke(
                        Rect::from_center_size(c, vec2(10.0, 10.0)),
                        0.0,
                        stroke,
                        egui::StrokeKind::Middle,
                    );
                }
                _ => {
                    painter.line_segment(
                        [pos2(c.x - 5.0, c.y - 5.0), pos2(c.x + 5.0, c.y + 5.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [pos2(c.x + 5.0, c.y - 5.0), pos2(c.x - 5.0, c.y + 5.0)],
                        stroke,
                    );
                }
            }
            if response.clicked() {
                match kind {
                    "min" => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                    "max" => toggle_maximized(&ctx),
                    _ => ctx.send_viewport_cmd(ViewportCommand::Close),
                }
            }
        }
    }
    events
}

/// Indian-style border art (like mehndi or a sari border): paisleys (kairi) and lotus
/// flowers on a flowing vine, with rangoli dots and dotted edges. Fades in from the left of
/// `area` so it starts softly behind the tabs.
fn flourish(painter: &egui::Painter, area: Rect, colour: Color32) {
    if area.width() < 80.0 {
        return;
    }
    let h = area.height();
    let mid = area.center().y + 1.0;
    let unit = 112.0;
    let count = (area.width() / unit).ceil() as usize + 1;
    let fade_at = |x: f32| ((x - area.left()) / 160.0).clamp(0.0, 1.0);
    let painter = painter.with_clip_rect(area);
    // Dotted edges, top and bottom, like the woven border of a sari.
    let mut x = area.left() + 3.0;
    while x < area.right() {
        let c = colour.gamma_multiply(fade_at(x) * 0.8);
        painter.circle_filled(pos2(x, area.top() + 4.0), 0.9, c);
        painter.circle_filled(pos2(x, area.bottom() - 4.0), 0.9, c);
        x += 7.0;
    }
    for i in 0..count {
        let x0 = area.left() + i as f32 * unit;
        let c = colour.gamma_multiply(fade_at(x0 + unit / 2.0));
        if c.a() == 0 {
            continue;
        }
        let up = if i % 2 == 0 { -1.0 } else { 1.0 };
        // The vine: one gentle wave per unit, the paisley sits on its crest.
        bezier(
            &painter,
            [
                pos2(x0, mid),
                pos2(x0 + unit * 0.25, mid + up * h * 0.2),
                pos2(x0 + unit * 0.5, mid + up * h * 0.2),
                pos2(x0 + unit * 0.75, mid),
            ],
            Stroke::new(1.0_f32, c.gamma_multiply(0.8)),
        );
        bezier(
            &painter,
            [
                pos2(x0 + unit * 0.75, mid),
                pos2(x0 + unit * 0.85, mid - up * h * 0.06),
                pos2(x0 + unit * 0.95, mid - up * h * 0.06),
                pos2(x0 + unit, mid),
            ],
            Stroke::new(1.0_f32, c.gamma_multiply(0.8)),
        );
        // Small leaves along the vine.
        for (lx, side) in [(0.12, 1.0), (0.62, -1.0)] {
            let base = pos2(x0 + unit * lx, mid + up * h * 0.1);
            petal(
                &painter,
                base,
                base + vec2(7.0, side * up * -6.0),
                2.6,
                Stroke::new(0.9_f32, c),
            );
        }
        paisley(
            &painter,
            pos2(x0 + unit * 0.37, mid - up * h * 0.02),
            h * 0.3,
            up,
            c,
        );
        lotus(&painter, pos2(x0 + unit * 0.86, mid), h * 0.26, c);
    }
}

fn bezier(painter: &egui::Painter, points: [egui::Pos2; 4], stroke: Stroke) {
    painter.add(egui::epaint::CubicBezierShape::from_points_stroke(
        points,
        false,
        Color32::TRANSPARENT,
        stroke,
    ));
}

/// A leaf or petal from `base` to `tip`, `width` wide at its middle.
fn petal(painter: &egui::Painter, base: egui::Pos2, tip: egui::Pos2, width: f32, stroke: Stroke) {
    let along = tip - base;
    let side = vec2(-along.y, along.x).normalized() * width;
    bezier(
        painter,
        [
            base,
            base + along * 0.3 + side,
            tip - along * 0.25 + side * 0.6,
            tip,
        ],
        stroke,
    );
    bezier(
        painter,
        [
            base,
            base + along * 0.3 - side,
            tip - along * 0.25 - side * 0.6,
            tip,
        ],
        stroke,
    );
}

/// The kairi (mango) paisley: a rounded bulb whose tip curls over, with an inner echo and a
/// dot. `flip` turns it upside down for the alternating pattern.
fn paisley(painter: &egui::Painter, centre: egui::Pos2, size: f32, flip: f32, colour: Color32) {
    let p = |x: f32, y: f32, k: f32| pos2(centre.x + x * size * k, centre.y + y * size * k * -flip);
    for (k, width) in [(1.0, 1.2_f32), (0.55, 0.9)] {
        let stroke = Stroke::new(width, colour);
        // Tip, down the outer side, round the bulb, back up the inner side.
        bezier(
            painter,
            [
                p(0.55, -0.95, k),
                p(1.05, -0.45, k),
                p(0.85, 0.75, k),
                p(0.0, 0.8, k),
            ],
            stroke,
        );
        bezier(
            painter,
            [
                p(0.0, 0.8, k),
                p(-0.85, 0.82, k),
                p(-0.9, -0.05, k),
                p(-0.4, -0.4, k),
            ],
            stroke,
        );
        bezier(
            painter,
            [
                p(-0.4, -0.4, k),
                p(-0.1, -0.62, k),
                p(0.25, -0.62, k),
                p(0.55, -0.95, k),
            ],
            stroke,
        );
    }
    // The curl at the tip.
    bezier(
        painter,
        [
            p(0.55, -0.95, 1.0),
            p(0.5, -1.25, 1.0),
            p(0.15, -1.2, 1.0),
            p(0.2, -0.98, 1.0),
        ],
        Stroke::new(1.0_f32, colour),
    );
    painter.circle_filled(p(-0.05, 0.25, 1.0), size * 0.1, colour);
    // Tiny dots around the bulb, as in mehndi.
    for (x, y) in [
        (-1.15, 0.2),
        (-0.8, 1.05),
        (0.0, 1.2),
        (0.85, 1.0),
        (1.2, 0.2),
    ] {
        painter.circle_filled(p(x, y, 1.0), 0.9, colour.gamma_multiply(0.8));
    }
}

/// A lotus: five petals opening upwards from a small bowl, with rangoli dots beneath.
fn lotus(painter: &egui::Painter, centre: egui::Pos2, size: f32, colour: Color32) {
    let stroke = Stroke::new(1.1_f32, colour);
    let base = centre + vec2(0.0, size * 0.55);
    for (dx, dy, w) in [
        (0.0, -1.15, 0.32),
        (-0.55, -0.85, 0.28),
        (0.55, -0.85, 0.28),
        (-0.95, -0.25, 0.22),
        (0.95, -0.25, 0.22),
    ] {
        petal(
            painter,
            base,
            base + vec2(dx * size, dy * size),
            w * size,
            stroke,
        );
    }
    // The bowl under the petals.
    bezier(
        painter,
        [
            base + vec2(-size * 0.75, -size * 0.05),
            base + vec2(-size * 0.4, size * 0.35),
            base + vec2(size * 0.4, size * 0.35),
            base + vec2(size * 0.75, -size * 0.05),
        ],
        stroke,
    );
    for k in -1..=1 {
        painter.circle_filled(
            base + vec2(k as f32 * size * 0.35, size * 0.55),
            if k == 0 { 1.4 } else { 1.0 },
            colour.gamma_multiply(0.9),
        );
    }
}

/// A horizontal fade from transparent to `fill`, over the end of a clipped title.
fn fade_right(painter: &egui::Painter, r: Rect, fill: Color32) {
    let clear = Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), 0);
    let mut mesh = Mesh::default();
    mesh.colored_vertex(r.left_top(), clear);
    mesh.colored_vertex(r.right_top(), fill);
    mesh.colored_vertex(r.right_bottom(), fill);
    mesh.colored_vertex(r.left_bottom(), clear);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

fn toggle_maximized(ctx: &egui::Context) {
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
}

/// Without system decorations the window edges must be resizable by hand.
pub fn resize_edges(ctx: &egui::Context) {
    if macos() || ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let screen = ctx.content_rect();
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else {
        return;
    };
    let m = 5.0;
    let (w, e, n, s) = (
        p.x <= screen.left() + m,
        p.x >= screen.right() - m,
        p.y <= screen.top() + m,
        p.y >= screen.bottom() - m,
    );
    let dir = match (n, s, w, e) {
        (true, _, true, _) => Some((ResizeDirection::NorthWest, CursorIcon::ResizeNorthWest)),
        (true, _, _, true) => Some((ResizeDirection::NorthEast, CursorIcon::ResizeNorthEast)),
        (_, true, true, _) => Some((ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest)),
        (_, true, _, true) => Some((ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast)),
        (true, ..) => Some((ResizeDirection::North, CursorIcon::ResizeNorth)),
        (_, true, ..) => Some((ResizeDirection::South, CursorIcon::ResizeSouth)),
        (_, _, true, _) => Some((ResizeDirection::West, CursorIcon::ResizeWest)),
        (_, _, _, true) => Some((ResizeDirection::East, CursorIcon::ResizeEast)),
        _ => None,
    };
    if let Some((direction, cursor)) = dir {
        ctx.set_cursor_icon(cursor);
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
        }
    }
}

/// A hairline border, since the window has no system frame.
pub fn border(ctx: &egui::Context) {
    if macos() || ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        Id::new("window-border"),
    ));
    let colour = if ctx.global_style().visuals.dark_mode {
        Color32::from_gray(70)
    } else {
        Color32::from_gray(150)
    };
    painter.rect_stroke(
        ctx.content_rect().shrink(0.5),
        0.0,
        Stroke::new(1.0_f32, colour),
        egui::StrokeKind::Inside,
    );
}
