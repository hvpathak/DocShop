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

pub const BAR_H: f32 = 40.0;
const TAB_TOP: f32 = 7.0;
const MAX_TAB_W: f32 = 240.0;
const MIN_TAB_W: f32 = 56.0;
const PLUS_W: f32 = 36.0;
const MENU_W: f32 = 36.0;
const CONTROL_W: f32 = 46.0;

pub struct TabInfo {
    pub title: String,
    pub icon: &'static str,
    pub dirty: bool,
    pub tooltip: String,
}

/// Which kinds the new-tab choices offer (the mode decides).
pub struct Offer {
    /// Ask which kind (Studio); otherwise + opens `home` straight away.
    pub ask: bool,
    pub home: TabKind,
    pub text: bool,
    pub doc: bool,
    pub pdf: bool,
    /// On the growing path: the chapter that unlocks documents / PDF.
    pub doc_locked: Option<u8>,
    pub pdf_locked: Option<u8>,
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
    PdfSoon,
    Reopen(usize),
    /// A choice the growing path hasn't unlocked yet (its chapter).
    Locked(u8),
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
fn new_tab_choices(ui: &mut egui::Ui, offer: &Offer, events: &mut Vec<Event>) {
    // Each choice is a card: big icon, bold name, grey description, shortcut.
    let mut pick = |ui: &mut egui::Ui,
                    icon: &str,
                    title: &str,
                    detail: &str,
                    key: &str,
                    event: Event,
                    dim: bool| {
        let (rect, response) = ui.allocate_exact_size(vec2(340.0, 50.0), Sense::click());
        let visuals = ui.visuals().clone();
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, 6.0, visuals.widgets.hovered.weak_bg_fill);
        }
        let painter = ui.painter();
        painter.text(
            pos2(rect.left() + 22.0, rect.center().y),
            Align2::CENTER_CENTER,
            icon,
            FontId::proportional(22.0),
            if dim {
                visuals.weak_text_color()
            } else {
                visuals.text_color()
            },
        );
        painter.text(
            pos2(rect.left() + 46.0, rect.top() + 16.0),
            Align2::LEFT_CENTER,
            title,
            FontId::new(14.0, egui::FontFamily::Name("bold".into())),
            if dim {
                visuals.weak_text_color()
            } else {
                visuals.strong_text_color()
            },
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
        if response.on_hover_cursor(CursorIcon::PointingHand).clicked() {
            events.push(event);
            ui.memory_mut(|m| m.close_popup());
        }
    };
    if offer.text {
        pick(
            ui,
            "📄",
            "Text file",
            "Plain text, like a notepad",
            "Ctrl+N",
            Event::New(TabKind::Text),
            false,
        );
    }
    if offer.doc {
        pick(
            ui,
            "📝",
            "Document",
            "Formatted writing · .docx, Markdown",
            "Ctrl+Shift+N",
            Event::New(TabKind::Doc),
            false,
        );
    } else if let Some(n) = offer.doc_locked {
        pick(
            ui,
            "🔒",
            "Document",
            &format!("Unlocks in Chapter {n}: Writer"),
            "",
            Event::Locked(n),
            true,
        );
    }
    if offer.pdf {
        pick(
            ui,
            "📕",
            "PDF",
            "Read, organize and sign · coming soon",
            "",
            Event::PdfSoon,
            false,
        );
    } else if let Some(n) = offer.pdf_locked {
        pick(
            ui,
            "🔒",
            "PDF",
            &format!("Unlocks in Chapter {n}: PDF"),
            "",
            Event::Locked(n),
            true,
        );
    }
    ui.separator();
    pick(
        ui,
        "📂",
        "Open a file…",
        "Text, documents and more",
        "Ctrl+O",
        Event::Open,
        false,
    );
}

/// Draws the bar and returns what the user did.
/// `open_picker` opens the new-tab choices (Ctrl+T); `search` is the ⌄ menu's filter.
pub fn title_bar(
    ui: &mut egui::Ui,
    tabs: &[TabInfo],
    active: usize,
    offer: &Offer,
    open_picker: bool,
    search: &mut String,
    closed: &[String],
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

    let left = rect.left() + if macos() { 78.0 } else { 8.0 };
    let controls = if macos() { 0.0 } else { CONTROL_W * 3.0 };
    let n = tabs.len().max(1) as f32;
    let room = (rect.right() - controls - MENU_W - PLUS_W - 60.0 - left).max(MIN_TAB_W);
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
        let shape = CornerRadius {
            nw: 9,
            ne: 9,
            sw: 0,
            se: 0,
        };
        let fill = if is_active {
            active_fill
        } else if response.hovered() {
            hover_fill
        } else {
            bar
        };
        if is_active || response.hovered() {
            painter.rect_filled(
                r.shrink2(vec2(if is_active { 0.0 } else { 2.0 }, 0.0)),
                shape,
                fill,
            );
        }
        let next_active = i + 1 == active;
        if !is_active && !next_active && i + 1 < tabs.len() && !response.hovered() {
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
        painter.text(
            pos2(r.left() + 13.0, r.center().y),
            Align2::LEFT_CENTER,
            tab.icon,
            FontId::proportional(14.0),
            text_colour,
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
    if plus_response.clicked() && !offer.ask {
        events.push(Event::New(offer.home));
    } else if plus_response.clicked() {
        ctx.memory_mut(|m| m.toggle_popup(picker));
    }
    if open_picker && offer.ask {
        ctx.memory_mut(|m| m.open_popup(picker));
    }
    egui::popup::popup_below_widget(
        ui,
        picker,
        &plus_response,
        PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(340.0);
            ui.add_space(4.0);
            ui.label(RichText::new("Open a new tab as…").strong());
            ui.add_space(4.0);
            new_tab_choices(ui, offer, &mut events);
        },
    );

    // ---- ⌄ menu before the window controls -----------------------------------------
    let menu = Rect::from_center_size(
        pos2(
            rect.right() - controls - MENU_W / 2.0 - 4.0,
            rect.center().y + 2.0,
        ),
        vec2(28.0, 28.0),
    );
    let menu_response = ui
        .interact(menu, Id::new("chrome-menu"), Sense::click())
        .on_hover_text("New tab as… · search tabs");
    if menu_response.hovered() {
        painter.circle_filled(menu.center(), 14.0, hover_fill);
    }
    let c = menu.center();
    let chevron = Stroke::new(1.6_f32, text_colour);
    painter.line_segment([pos2(c.x - 5.0, c.y - 2.5), pos2(c.x, c.y + 2.5)], chevron);
    painter.line_segment([pos2(c.x, c.y + 2.5), pos2(c.x + 5.0, c.y - 2.5)], chevron);
    let menu_id = Id::new("tab-menu");
    if menu_response.clicked() {
        ctx.memory_mut(|m| m.toggle_popup(menu_id));
    }
    egui::popup::popup_below_widget(
        ui,
        menu_id,
        &menu_response,
        PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(340.0);
            ui.add_space(4.0);
            ui.label(RichText::new("Open a new tab as…").strong());
            ui.add_space(2.0);
            new_tab_choices(ui, offer, &mut events);
            ui.separator();
            ui.add(
                egui::TextEdit::singleline(search)
                    .id(Id::new("tab-search"))
                    .hint_text("🔍  Search tabs")
                    .desired_width(310.0),
            );
            let query = search.to_lowercase();
            ui.label(RichText::new("Open tabs").small().color(weak));
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .id_salt("tab-menu-list")
                .show(ui, |ui| {
                    for (i, tab) in tabs.iter().enumerate() {
                        if !query.is_empty() && !tab.title.to_lowercase().contains(&query) {
                            continue;
                        }
                        let label = format!(
                            "{}  {}{}",
                            tab.icon,
                            tab.title,
                            if tab.dirty { "  ●" } else { "" }
                        );
                        if ui
                            .add(
                                egui::Button::new(label)
                                    .selected(i == active)
                                    .min_size(vec2(310.0, 26.0)),
                            )
                            .clicked()
                        {
                            events.push(Event::Select(i));
                            ui.memory_mut(|m| m.close_popup());
                        }
                    }
                });
            if !closed.is_empty() {
                ui.label(RichText::new("Recently closed").small().color(weak));
                for (i, title) in closed.iter().enumerate().rev().take(8) {
                    if ui
                        .add(
                            egui::Button::new(format!("↺  {title}"))
                                .shortcut_text(if i + 1 == closed.len() {
                                    "Ctrl+Shift+T"
                                } else {
                                    ""
                                })
                                .min_size(vec2(310.0, 24.0)),
                        )
                        .clicked()
                    {
                        events.push(Event::Reopen(i));
                        ui.memory_mut(|m| m.close_popup());
                    }
                }
            }
        },
    );

    // ---- Drag area and window controls -------------------------------------------------
    let drag = Rect::from_min_max(
        pos2(plus.right() + 6.0, rect.top()),
        pos2(menu.left() - 6.0, rect.bottom()),
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
    let screen = ctx.screen_rect();
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
    let colour = if ctx.style().visuals.dark_mode {
        Color32::from_gray(70)
    } else {
        Color32::from_gray(150)
    };
    painter.rect_stroke(
        ctx.screen_rect().shrink(0.5),
        0.0,
        Stroke::new(1.0_f32, colour),
        egui::StrokeKind::Inside,
    );
}
