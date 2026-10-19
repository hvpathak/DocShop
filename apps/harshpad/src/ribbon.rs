//! The ribbon: tabs of labelled groups with large and small buttons. Buttons run
//! catalog commands or app actions; features that aren't built yet say so.

use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke, vec2};
use harshpad_core::commands::Command;
use harshpad_core::modes::{Chapter, Mode};
use harshpad_rich::{Align as TextAlign, Flag, ParaStyle, Paragraph, Rgb};

use crate::doc_tab::DocTab;

pub const TABS: [&str; 11] = [
    "File",
    "Home",
    "Insert",
    "Draw",
    "Design",
    "Layout",
    "References",
    "Mailings",
    "Review",
    "View",
    "Help",
];

/// What a ribbon button does.
#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Cmd(Command),
    /// A feature that isn't built yet (the name is shown in the status bar).
    Soon(&'static str),
    Paste,
    Cut,
    Copy,
    SelectAll,
    InsertText(String),
    InsertDate,
    HorizontalLine,
    WordCount,
    ToggleReadOnly,
    ToggleRuler,
    Shortcuts,
}

/// Everything the ribbon needs to know to draw button states.
#[derive(Debug, Clone, Default)]
pub struct State {
    /// The active tab is a rich document (formatting buttons enabled).
    pub doc: bool,
    pub flags: [bool; 5],
    pub para: Paragraph,
    pub ruler: bool,
    pub status_bar: bool,
    pub read_only: bool,
    pub word_wrap: bool,
    pub mode: Mode,
    pub path: Option<Chapter>,
}

const ACCENT: Color32 = Color32::from_rgb(30, 110, 210);
const BODY_H: f32 = 70.0;

fn soon_color(ui: &egui::Ui) -> Color32 {
    ui.visuals().weak_text_color()
}

/// A large button: icon above a (possibly two-line) label.
fn big(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool, on: bool, soon: bool) -> bool {
    let font = egui::FontId::proportional(12.0);
    let widest = label
        .lines()
        .map(|l| {
            ui.painter()
                .layout_no_wrap(l.to_owned(), font.clone(), Color32::WHITE)
                .size()
                .x
        })
        .fold(0.0_f32, f32::max);
    let size = vec2((widest + 14.0).max(56.0), BODY_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let response = response.on_hover_text(if soon {
        format!("{label} (coming soon)")
    } else {
        label.replace('\n', " ")
    });
    let visuals = ui.style().interact_selectable(&response, on);
    if response.hovered() || on {
        ui.painter().rect(
            rect.shrink(1.0),
            4.0,
            visuals.weak_bg_fill,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }
    let icon_color = if !enabled || soon {
        soon_color(ui)
    } else {
        ACCENT
    };
    let text_color = if enabled {
        ui.visuals().text_color()
    } else {
        soon_color(ui)
    };
    ui.painter().text(
        rect.center_top() + vec2(0.0, 18.0),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(24.0),
        icon_color,
    );
    // Each label line centred under the icon.
    for (k, line) in label.lines().enumerate() {
        ui.painter().text(
            rect.center_top() + vec2(0.0, 43.0 + 15.0 * k as f32),
            egui::Align2::CENTER_CENTER,
            line,
            font.clone(),
            text_color,
        );
    }
    enabled && response.clicked()
}

/// A small button: icon and label on one line.
fn small(ui: &mut egui::Ui, icon: &str, label: &str, enabled: bool, on: bool, soon: bool) -> bool {
    let text = RichText::new(format!("{icon}  {label}")).size(13.0);
    let text = if soon {
        text.color(soon_color(ui))
    } else {
        text
    };
    let button = egui::Button::new(text)
        .frame(on)
        .selected(on)
        .min_size(vec2(0.0, 21.0));
    let response = ui.add_enabled(enabled, button);
    let response = if soon {
        response.on_hover_text(format!("{label} (coming soon)"))
    } else {
        response
    };
    response.clicked()
}

/// A labelled group with a separator after it.
fn group(ui: &mut egui::Ui, name: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        let top = ui.horizontal(|ui| {
            ui.set_min_height(BODY_H);
            body(ui);
        });
        // The group name, centred under the buttons.
        let width = top.response.rect.width().max(40.0);
        let (rect, _) = ui.allocate_exact_size(vec2(width, 16.0), Sense::hover());
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            name,
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
    });
    // A short divider between groups (a Separator would grow to the panel height).
    let (rect, _) = ui.allocate_exact_size(vec2(11.0, BODY_H + 18.0), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        (rect.top() + 4.0)..=(rect.bottom() - 4.0),
        Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
}

/// A column of up to three small buttons.
fn column(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        body(ui);
    });
}

/// Draws the body of ribbon tab `tab`. Returns what was clicked.
pub fn body(ui: &mut egui::Ui, tab: usize, s: &State, doc: Option<&mut DocTab>) -> Option<Act> {
    let mut act: Option<Act> = None;
    let mut set = |a: Act| act = Some(a);
    let d = s.doc;
    // Labels stay on one line; a narrow window scrolls the ribbon instead.
    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    // A scrollbar of its own under the groups, never on top of their names.
    ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
    ui.spacing_mut().scroll.bar_width = 4.0;
    ui.spacing_mut().scroll.bar_inner_margin = 2.0;
    egui::ScrollArea::horizontal()
        .id_salt("ribbon-body")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                match tab {
                    0 => file(ui, s, &mut set),
                    1 => home(ui, s, doc, &mut set),
                    2 => {
                        group(ui, "Pages", |ui| {
                            if big(ui, "📄", "Cover\nPage", d, false, true) {
                                set(Act::Soon("Cover page"));
                            }
                            if big(ui, "⤓", "Page\nBreak", d, false, true) {
                                set(Act::Soon("Page break"));
                            }
                        });
                        group(ui, "Tables", |ui| {
                            if big(ui, "▦", "Table", d, false, true) {
                                set(Act::Soon("Tables"));
                            }
                        });
                        group(ui, "Illustrations", |ui| {
                            if big(ui, "🖼", "Pictures", d, false, true) {
                                set(Act::Soon("Pictures"));
                            }
                            if big(ui, "◆", "Shapes", d, false, true) {
                                set(Act::Soon("Shapes"));
                            }
                            if big(ui, "📊", "Chart", d, false, true) {
                                set(Act::Soon("Charts"));
                            }
                        });
                        group(ui, "Links", |ui| {
                            if big(ui, "🔗", "Link", d, false, false) {
                                set(Act::Cmd(Command::InsertLink));
                            }
                        });
                        group(ui, "Text", |ui| {
                            if big(ui, "📅", "Date &\nTime", true, false, false) {
                                set(Act::InsertDate);
                            }
                            if big(ui, "―", "Horizontal\nLine", true, false, false) {
                                set(Act::HorizontalLine);
                            }
                            ui.menu_button(RichText::new("Ω  Symbol ▾").size(13.0), |ui| {
                                ui.set_max_width(230.0);
                                ui.horizontal_wrapped(|ui| {
                                    for c in SYMBOLS.chars() {
                                        if ui
                                            .button(RichText::new(c.to_string()).size(16.0))
                                            .clicked()
                                        {
                                            set(Act::InsertText(c.to_string()));
                                            ui.close_menu();
                                        }
                                    }
                                });
                            });
                        });
                    }
                    3 => {
                        group(ui, "Drawing Tools", |ui| {
                            for (icon, label) in [
                                ("✒", "Pen"),
                                ("🖍", "Highlighter"),
                                ("⌫", "Eraser"),
                                ("✋", "Select"),
                            ] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Drawing"));
                                }
                            }
                        });
                        group(ui, "Convert", |ui| {
                            if big(ui, "∑", "Ink to\nMath", d, false, true) {
                                set(Act::Soon("Ink to math"));
                            }
                        });
                    }
                    4 => {
                        group(ui, "Document Formatting", |ui| {
                            for (icon, label) in [
                                ("🎨", "Themes"),
                                ("Aa", "Style\nSets"),
                                ("●", "Colors"),
                                ("A", "Fonts"),
                                ("↕", "Paragraph\nSpacing"),
                            ] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Themes and style sets"));
                                }
                            }
                        });
                        group(ui, "Page Background", |ui| {
                            for (icon, label) in [
                                ("◫", "Watermark"),
                                ("🌈", "Page\nColor"),
                                ("▢", "Page\nBorders"),
                            ] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Page background"));
                                }
                            }
                        });
                    }
                    5 => {
                        group(ui, "Page Setup", |ui| {
                            for (icon, label) in [
                                ("▭", "Margins"),
                                ("⟲", "Orientation"),
                                ("📐", "Size"),
                                ("▥", "Columns"),
                            ] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Page setup"));
                                }
                            }
                            column(ui, |ui| {
                                for (icon, label) in
                                    [("⤓", "Breaks"), ("#", "Line Numbers"), ("-", "Hyphenation")]
                                {
                                    if small(ui, icon, label, d, false, true) {
                                        set(Act::Soon("Page setup"));
                                    }
                                }
                            });
                        });
                        group(ui, "Paragraph", |ui| {
                            for (c, icon, label, a) in ALIGNS {
                                if big(ui, icon, label, d, s.para.align == a, false) {
                                    set(Act::Cmd(c));
                                }
                            }
                            column(ui, |ui| {
                                for (icon, label) in [("⇥", "Indent"), ("↕", "Spacing")] {
                                    if small(ui, icon, label, d, false, true) {
                                        set(Act::Soon("Indent and spacing"));
                                    }
                                }
                            });
                        });
                        group(ui, "Arrange", |ui| {
                            for (icon, label) in
                                [("◳", "Position"), ("⤶", "Wrap\nText"), ("⧉", "Align")]
                            {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Arranging objects"));
                                }
                            }
                        });
                    }
                    6 => {
                        group(ui, "Table of Contents", |ui| {
                            if big(ui, "☰", "Table of\nContents", d, false, true) {
                                set(Act::Soon("Table of contents"));
                            }
                        });
                        group(ui, "Footnotes", |ui| {
                            if big(ui, "¹", "Insert\nFootnote", d, false, true) {
                                set(Act::Soon("Footnotes"));
                            }
                            column(ui, |ui| {
                                for label in ["Insert Endnote", "Next Footnote", "Show Notes"] {
                                    if small(ui, "ab", label, d, false, true) {
                                        set(Act::Soon("Footnotes"));
                                    }
                                }
                            });
                        });
                        group(ui, "Citations & Bibliography", |ui| {
                            for (icon, label) in [("❝", "Insert\nCitation"), ("📚", "Bibliography")]
                            {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Citations"));
                                }
                            }
                        });
                        group(ui, "Captions", |ui| {
                            if big(ui, "🏷", "Insert\nCaption", d, false, true) {
                                set(Act::Soon("Captions"));
                            }
                        });
                        group(ui, "Index", |ui| {
                            if big(ui, "🔖", "Mark\nEntry", d, false, true) {
                                set(Act::Soon("Index"));
                            }
                        });
                    }
                    7 => {
                        group(ui, "Create", |ui| {
                            for (icon, label) in [("✉", "Envelopes"), ("🏷", "Labels")] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Envelopes and labels"));
                                }
                            }
                        });
                        group(ui, "Start Mail Merge", |ui| {
                            for (icon, label) in [
                                ("📨", "Start Mail\nMerge"),
                                ("👥", "Select\nRecipients"),
                                ("✎", "Edit\nRecipients"),
                            ] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Mail merge"));
                                }
                            }
                        });
                        group(ui, "Finish", |ui| {
                            if big(ui, "✔", "Finish &\nMerge", d, false, true) {
                                set(Act::Soon("Mail merge"));
                            }
                        });
                    }
                    8 => {
                        group(ui, "Proofing", |ui| {
                            if big(ui, "abc✓", "Spelling &\nGrammar", true, false, true) {
                                set(Act::Soon("Spelling and grammar"));
                            }
                            column(ui, |ui| {
                                if small(ui, "📖", "Thesaurus", true, false, true) {
                                    set(Act::Soon("Thesaurus"));
                                }
                                if small(ui, "123", "Word Count", true, false, false) {
                                    set(Act::WordCount);
                                }
                            });
                        });
                        group(ui, "Speech", |ui| {
                            if big(ui, "🔊", "Read\nAloud", true, false, true) {
                                set(Act::Soon("Read aloud"));
                            }
                        });
                        group(ui, "Accessibility", |ui| {
                            if big(ui, "♿", "Check\nAccessibility", d, false, true) {
                                set(Act::Soon("Accessibility checker"));
                            }
                        });
                        group(ui, "Language", |ui| {
                            for (icon, label) in [("🌐", "Translate"), ("文", "Language")] {
                                if big(ui, icon, label, true, false, true) {
                                    set(Act::Soon("Translation and languages"));
                                }
                            }
                        });
                        group(ui, "Comments", |ui| {
                            if big(ui, "💬", "New\nComment", d, false, true) {
                                set(Act::Soon("Comments"));
                            }
                            column(ui, |ui| {
                                for (icon, label) in
                                    [("✖", "Delete"), ("◀", "Previous"), ("▶", "Next")]
                                {
                                    if small(ui, icon, label, d, false, true) {
                                        set(Act::Soon("Comments"));
                                    }
                                }
                            });
                            if big(ui, "🗨", "Show\nComments", d, false, true) {
                                set(Act::Soon("Comments"));
                            }
                        });
                        group(ui, "Tracking", |ui| {
                            if big(ui, "📝", "Track\nChanges", d, false, true) {
                                set(Act::Soon("Track changes"));
                            }
                            column(ui, |ui| {
                                for (icon, label) in [("☰", "All Markup"), ("▤", "Reviewing Pane")]
                                {
                                    if small(ui, icon, label, d, false, true) {
                                        set(Act::Soon("Track changes"));
                                    }
                                }
                            });
                        });
                        group(ui, "Changes", |ui| {
                            for (icon, label) in [("✔", "Accept"), ("✘", "Reject")] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Accepting and rejecting changes"));
                                }
                            }
                            column(ui, |ui| {
                                for (icon, label) in [("◀", "Previous"), ("▶", "Next")] {
                                    if small(ui, icon, label, d, false, true) {
                                        set(Act::Soon("Accepting and rejecting changes"));
                                    }
                                }
                            });
                        });
                        group(ui, "Compare", |ui| {
                            if big(ui, "⇋", "Compare", d, false, true) {
                                set(Act::Soon("Compare documents"));
                            }
                        });
                        group(ui, "Protect", |ui| {
                            if big(ui, "🚫", "Block\nAuthors", d, false, true) {
                                set(Act::Soon("Block authors"));
                            }
                            if big(ui, "🔒", "Restrict\nEditing", d, s.read_only, false) {
                                set(Act::ToggleReadOnly);
                            }
                        });
                    }
                    9 => {
                        group(ui, "Views", |ui| {
                            if big(ui, "📄", "Print\nLayout", d, d, false) {
                                set(Act::Cmd(Command::ZoomReset));
                            }
                            for (icon, label) in [("📖", "Read\nMode"), ("🌐", "Web\nLayout")] {
                                if big(ui, icon, label, d, false, true) {
                                    set(Act::Soon("Other views"));
                                }
                            }
                        });
                        group(ui, "Show", |ui| {
                            column(ui, |ui| {
                                if small(ui, "📏", "Ruler", d, s.ruler, false) {
                                    set(Act::ToggleRuler);
                                }
                                if small(ui, "▁", "Status Bar", true, s.status_bar, false) {
                                    set(Act::Cmd(Command::StatusBar));
                                }
                                if small(ui, "↩", "Word Wrap", !d, s.word_wrap, false) {
                                    set(Act::Cmd(Command::WordWrap));
                                }
                            });
                        });
                        group(ui, "Zoom", |ui| {
                            if big(ui, "🔍", "Zoom\nIn", true, false, false) {
                                set(Act::Cmd(Command::ZoomIn));
                            }
                            if big(ui, "🔎", "Zoom\nOut", true, false, false) {
                                set(Act::Cmd(Command::ZoomOut));
                            }
                            if big(ui, "1:1", "100%", true, false, false) {
                                set(Act::Cmd(Command::ZoomReset));
                            }
                        });
                        group(ui, "Mode", |ui| {
                            for (m, c, icon) in [
                                (Mode::Studio, Command::ModeStudio, "◧"),
                                (Mode::Notepad, Command::ModeNotepad, "📄"),
                                (Mode::Writer, Command::ModeWriter, "📝"),
                                (Mode::Pdf, Command::ModePdf, "📕"),
                            ] {
                                if big(
                                    ui,
                                    icon,
                                    m.short(),
                                    m.is_available(),
                                    s.mode == m,
                                    !m.is_available(),
                                ) {
                                    set(Act::Cmd(c));
                                }
                            }
                            if big(ui, "🔒", "Parent\nLock", s.path.is_none(), false, false) {
                                set(Act::Cmd(Command::ParentLock));
                            }
                            let (label, command) = match s.path {
                                Some(c) if c.next().is_some() => {
                                    ("Unlock Next\nChapter", Command::UnlockNextChapter)
                                }
                                Some(_) => ("Leave\nPath", Command::LeavePath),
                                None => ("Growing\nPath", Command::StartPath),
                            };
                            if big(ui, "📖", label, true, s.path.is_some(), false) {
                                set(Act::Cmd(command));
                            }
                        });
                        group(ui, "Window", |ui| {
                            for (icon, label) in [("🗗", "New\nWindow"), ("⊟", "Split")] {
                                if big(ui, icon, label, true, false, true) {
                                    set(Act::Soon("Multiple windows"));
                                }
                            }
                        });
                    }
                    10 => {
                        group(ui, "Help", |ui| {
                            if big(ui, "⌨", "Keyboard\nShortcuts", true, false, false) {
                                set(Act::Shortcuts);
                            }
                            if big(ui, "⌘", "Command\nPalette", true, false, false) {
                                set(Act::Cmd(Command::CommandPalette));
                            }
                            if big(ui, "ℹ", "About", true, false, false) {
                                set(Act::Cmd(Command::About));
                            }
                            if big(ui, "✉", "Feedback", true, false, true) {
                                set(Act::Soon("Feedback"));
                            }
                        });
                    }
                    _ => {}
                }
            });
        });
    act
}

const SYMBOLS: &str = "©®™€£¥¢°±×÷≠≈≤≥∞√∑πΩµ§¶•…—–«»‹›←→↑↓↔✓✗★☆♥☺☼½¼¾²³¹‰";

const ALIGNS: [(Command, &str, &str, TextAlign); 4] = [
    (Command::AlignLeft, "⫷", "Left", TextAlign::Left),
    (Command::AlignCenter, "≡", "Center", TextAlign::Center),
    (Command::AlignRight, "⫸", "Right", TextAlign::Right),
    (Command::AlignJustify, "☰", "Justify", TextAlign::Justify),
];

fn file(ui: &mut egui::Ui, s: &State, set: &mut dyn FnMut(Act)) {
    let studio = s.mode == Mode::Studio;
    group(ui, "New", |ui| {
        if studio {
            if big(ui, "➕", "New Tab…", true, false, false) {
                set(Act::Cmd(Command::NewTab));
            }
            if big(ui, "📄", "Text\nFile", true, false, false) {
                set(Act::Cmd(Command::NewText));
            }
            if big(ui, "📝", "Document", true, false, false) {
                set(Act::Cmd(Command::NewDocument));
            }
            if big(ui, "📕", "PDF", true, false, true) {
                set(Act::Soon("PDF tools"));
            }
        } else if big(ui, "➕", "New", true, false, false) {
            set(Act::Cmd(Command::NewText));
        }
    });
    group(ui, "Open & Save", |ui| {
        if big(ui, "📂", "Open", true, false, false) {
            set(Act::Cmd(Command::Open));
        }
        if big(ui, "💾", "Save", true, false, false) {
            set(Act::Cmd(Command::Save));
        }
        if big(ui, "🖫", "Save\nAs…", true, false, false) {
            set(Act::Cmd(Command::SaveAs));
        }
        if big(ui, "⭳", "Export\nPDF", true, false, true) {
            set(Act::Soon("Export to PDF"));
        }
        if big(ui, "🖶", "Print", true, false, true) {
            set(Act::Soon("Printing"));
        }
    });
    if studio {
        group(ui, "Convert", |ui| {
            if big(ui, "📝", "Open as\nDocument", !s.doc, false, false) {
                set(Act::Cmd(Command::ConvertToDocument));
            }
            if big(ui, "📄", "Open as\nText", s.doc, false, false) {
                set(Act::Cmd(Command::ConvertToText));
            }
        });
    }
    group(ui, "Tabs", |ui| {
        if big(ui, "↺", "Reopen\nClosed", true, false, false) {
            set(Act::Cmd(Command::ReopenClosedTab));
        }
        if big(ui, "✖", "Close\nTab", true, false, false) {
            set(Act::Cmd(Command::CloseTab));
        }
        if big(ui, "⏻", "Exit", true, false, false) {
            set(Act::Cmd(Command::Exit));
        }
    });
}

fn home(ui: &mut egui::Ui, s: &State, doc: Option<&mut DocTab>, set: &mut dyn FnMut(Act)) {
    let d = s.doc;
    group(ui, "Clipboard", |ui| {
        if big(ui, "📋", "Paste", true, false, false) {
            set(Act::Paste);
        }
        column(ui, |ui| {
            if small(ui, "✂", "Cut", true, false, false) {
                set(Act::Cut);
            }
            if small(ui, "⧉", "Copy", true, false, false) {
                set(Act::Copy);
            }
            if small(ui, "🖌", "Format Painter", d, false, true) {
                set(Act::Soon("Format painter"));
            }
        });
    });
    if !d {
        text_tools(ui, s, set);
    } else {
        group(ui, "Font", |ui| {
            column(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (i, (flag, text, c)) in [
                        (Flag::Bold, RichText::new("B").strong(), Command::Bold),
                        (Flag::Italic, RichText::new("I").italics(), Command::Italic),
                        (
                            Flag::Underline,
                            RichText::new("U").underline(),
                            Command::Underline,
                        ),
                        (
                            Flag::Strike,
                            RichText::new("ab").strikethrough(),
                            Command::Strike,
                        ),
                        (
                            Flag::Code,
                            RichText::new("</>").monospace(),
                            Command::InlineCode,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let on = s.flags.get(i).copied().unwrap_or(false);
                        let _ = flag;
                        if ui
                            .add_enabled(
                                d,
                                egui::Button::new(text.size(15.0))
                                    .selected(on)
                                    .min_size(vec2(28.0, 26.0)),
                            )
                            .clicked()
                        {
                            set(Act::Cmd(c));
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (label, c) in [
                        ("A▲", Command::BiggerText),
                        ("A▼", Command::SmallerText),
                        ("A⌫", Command::ClearFormatting),
                    ] {
                        if ui
                            .add_enabled(
                                d,
                                egui::Button::new(RichText::new(label).size(13.0))
                                    .min_size(vec2(36.0, 24.0)),
                            )
                            .clicked()
                        {
                            set(Act::Cmd(c));
                        }
                    }
                    if let Some(doc) = doc {
                        let cs = doc.char_style();
                        let mut color = cs.color.map_or([32, 32, 32], |c| [c.0, c.1, c.2]);
                        ui.label(
                            RichText::new("A")
                                .strong()
                                .color(Color32::from_rgb(color[0], color[1], color[2])),
                        );
                        if egui::color_picker::color_edit_button_srgb(ui, &mut color)
                            .on_hover_text("Text colour")
                            .changed()
                        {
                            doc.set_color(Some(Rgb(color[0], color[1], color[2])));
                        }
                        let mut mark = cs.highlight.map_or([255, 241, 118], |c| [c.0, c.1, c.2]);
                        ui.label("🖍");
                        if egui::color_picker::color_edit_button_srgb(ui, &mut mark)
                            .on_hover_text("Highlight colour")
                            .changed()
                        {
                            doc.set_highlight(Some(Rgb(mark[0], mark[1], mark[2])));
                        }
                        if cs.highlight.is_some()
                            && ui
                                .small_button("✕")
                                .on_hover_text("Remove highlight")
                                .clicked()
                        {
                            doc.set_highlight(None);
                        }
                    }
                });
            });
        });
        group(ui, "Paragraph", |ui| {
            column(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if ui
                        .add_enabled(
                            d,
                            egui::Button::new("• List").selected(s.para.style == ParaStyle::Bullet),
                        )
                        .clicked()
                    {
                        set(Act::Cmd(Command::BulletList));
                    }
                    if ui
                        .add_enabled(
                            d,
                            egui::Button::new("1. List")
                                .selected(s.para.style == ParaStyle::Numbered),
                        )
                        .clicked()
                    {
                        set(Act::Cmd(Command::NumberedList));
                    }
                    if ui
                        .add_enabled(d, egui::Button::new("🔗"))
                        .on_hover_text("Link (Ctrl+K)")
                        .clicked()
                    {
                        set(Act::Cmd(Command::InsertLink));
                    }
                });
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (c, icon, label, a) in ALIGNS {
                        if ui
                            .add_enabled(
                                d,
                                egui::Button::new(RichText::new(icon).size(15.0))
                                    .selected(s.para.align == a)
                                    .min_size(vec2(28.0, 24.0)),
                            )
                            .on_hover_text(label)
                            .clicked()
                        {
                            set(Act::Cmd(c));
                        }
                    }
                });
            });
        });
        group(ui, "Styles", |ui| {
            for (style, c) in [
                (ParaStyle::Normal, Command::StyleNormal),
                (ParaStyle::Title, Command::StyleTitle),
                (ParaStyle::Heading1, Command::StyleHeading1),
                (ParaStyle::Heading2, Command::StyleHeading2),
                (ParaStyle::Heading3, Command::StyleHeading3),
                (ParaStyle::Quote, Command::StyleQuote),
                (ParaStyle::Code, Command::StyleCode),
            ] {
                let sample = match style {
                    ParaStyle::Title => RichText::new("AaBb").size(19.0),
                    ParaStyle::Heading1 => RichText::new("AaBb")
                        .size(17.0)
                        .strong()
                        .color(Color32::from_rgb(40, 100, 180)),
                    ParaStyle::Heading2 => RichText::new("AaBb")
                        .size(15.0)
                        .strong()
                        .color(Color32::from_rgb(40, 100, 180)),
                    ParaStyle::Heading3 => RichText::new("AaBb")
                        .size(13.0)
                        .strong()
                        .color(Color32::from_rgb(40, 100, 180)),
                    ParaStyle::Quote => RichText::new("AaBb").size(13.0).italics(),
                    ParaStyle::Code => RichText::new("AaBb").size(12.0).monospace(),
                    _ => RichText::new("AaBb").size(13.0),
                };
                let on = s.para.style == style;
                let (rect, response) =
                    ui.allocate_exact_size(vec2(56.0, BODY_H - 6.0), Sense::click());
                let response = response.on_hover_text(style.label());
                let fill = if on {
                    ui.visuals().selection.bg_fill
                } else if response.hovered() {
                    ui.visuals().widgets.hovered.weak_bg_fill
                } else {
                    ui.visuals().extreme_bg_color
                };
                ui.painter().rect(
                    rect,
                    4.0,
                    fill,
                    ui.visuals().widgets.noninteractive.bg_stroke,
                    egui::StrokeKind::Inside,
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(rect.shrink(4.0))
                        .layout(Layout::top_down(Align::Center)),
                );
                child.add_space(6.0);
                child.label(sample);
                child.label(RichText::new(style.label()).size(10.0));
                if d && response.clicked() {
                    set(Act::Cmd(c));
                }
            }
        });
    }
    group(ui, "Editing", |ui| {
        column(ui, |ui| {
            if small(ui, "🔍", "Find", true, false, false) {
                set(Act::Cmd(Command::Find));
            }
            if small(ui, "⇄", "Replace", true, false, false) {
                set(Act::Cmd(Command::Replace));
            }
            if small(ui, "⬚", "Select All", true, false, false) {
                set(Act::SelectAll);
            }
        });
    });
}
/// Home for plain-text tabs: no formatting, just the text tools.
fn text_tools(ui: &mut egui::Ui, s: &State, set: &mut dyn FnMut(Act)) {
    group(ui, "Text", |ui| {
        if big(ui, "↩", "Word\nWrap", true, s.word_wrap, false) {
            set(Act::Cmd(Command::WordWrap));
        }
        if big(ui, "📅", "Date &\nTime", true, false, false) {
            set(Act::InsertDate);
        }
        if s.mode == Mode::Studio && big(ui, "📝", "Open as\nDocument", true, false, false) {
            set(Act::Cmd(Command::ConvertToDocument));
        }
    });
}

/// Ribbon tabs each mode shows: Notepad keeps only what plain text uses.
pub fn shows(tab: usize, mode: Mode) -> bool {
    mode != Mode::Notepad || matches!(tab, 0 | 1 | 2 | 8 | 9 | 10)
}
