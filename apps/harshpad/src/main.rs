// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use std::path::PathBuf;

/// Title bar and Alt+Tab icon. The .exe file icon is embedded by build.rs.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon-256.png");

fn main() -> eframe::Result {
    // Like notepad.exe: `harshpad.exe some-file.txt` opens that file.
    let initial_file = std::env::args_os().nth(1).map(PathBuf::from);

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(app::APP_NAME)
        .with_inner_size([900.0, 620.0])
        .with_min_inner_size([320.0, 200.0])
        .with_drag_and_drop(true);
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
        Box::new(move |cc| Ok(Box::new(app::NotepadApp::new(cc, initial_file)))),
    )
}
