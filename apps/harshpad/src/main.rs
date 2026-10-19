// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod chrome;
mod doc_tab;
mod fonts;
mod recovery;
mod ribbon;
mod settings;
mod text_tab;

use std::path::PathBuf;

/// Title bar and Alt+Tab icon. The .exe file icon is embedded by build.rs.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon-256.png");

fn main() -> eframe::Result {
    // `harshpad notes.txt` or `harshpad report.docx` opens that file.
    let initial_file = std::env::args_os().nth(1).map(PathBuf::from);

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(app::APP_NAME)
        .with_app_id("harshpad")
        .with_inner_size([1100.0, 760.0])
        .with_min_inner_size([480.0, 320.0])
        .with_drag_and_drop(true);
    // Tabs live in the title bar, like a web browser. On macOS the system's
    // window buttons stay and the content extends under them.
    viewport = if cfg!(target_os = "macos") {
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
    } else {
        viewport.with_decorations(false)
    };
    // A bad icon must never stop the editor from starting.
    if let Ok(icon) = eframe::icon_data::from_png_bytes(ICON_PNG) {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        app::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(app::StudioApp::new(cc, initial_file)))),
    )
}
