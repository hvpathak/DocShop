// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use std::path::PathBuf;

fn main() -> eframe::Result {
    // Like notepad.exe: `harshpad.exe some-file.txt` opens that file.
    let initial_file = std::env::args_os().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(app::APP_NAME)
            .with_inner_size([900.0, 620.0])
            .with_min_inner_size([320.0, 200.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        app::APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(app::NotepadApp::new(cc, initial_file)))),
    )
}
