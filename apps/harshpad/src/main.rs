// A window app: no console window behind it on Windows, in any build.
#![windows_subsystem = "windows"]

mod app;
mod chrome;
mod doc_tab;
mod fonts;
mod icons;
mod page;
mod pdf_tab;
mod recovery;
mod ribbon;
mod settings;
mod text_tab;
mod wordcraft_host;
mod writer_tab;

use std::path::PathBuf;

/// Title bar and Alt+Tab icon. The .exe file icon is embedded by build.rs.
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon-256.png");

fn main() -> eframe::Result {
    // `harshpad notes.txt report.docx form.pdf` opens each file in its own tab.
    let initial_files: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();

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

    // Changed below on Windows and macOS only.
    #[allow(unused_mut)]
    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    // The GPU setup WordCraft uses: DirectX 12 where it works, OpenGL otherwise (Windows),
    // and no indirect validation on Intel Macs.
    #[cfg(target_os = "windows")]
    wordcraft_host::graphics::configure(&mut options, eframe::wgpu::Backends::from_env());
    #[cfg(target_os = "macos")]
    wordcraft_host::graphics::configure_indirect_validation(
        &mut options,
        wordcraft_host::graphics::skips_indirect_validation(true, cfg!(target_arch = "x86_64")),
    );
    // Fonts added in the Writer, before any Writer tab lays out text.
    wordcraft_host::prefs::load_user_fonts();
    // Finder's open-document events, registered before the event loop so the one that
    // launched the app is caught too (macOS).
    #[cfg(target_os = "macos")]
    let finder = wordcraft_host::apple_events::AppleEvents::install();
    #[cfg(target_os = "macos")]
    let finder = &finder;
    eframe::run_native(
        app::APP_NAME,
        options,
        Box::new(move |cc| {
            #[allow(unused_mut)]
            let mut app = app::StudioApp::new(cc, initial_files);
            #[cfg(target_os = "macos")]
            app.connect_finder(finder.connect(&cc.egui_ctx));
            Ok(Box::new(app))
        }),
    )
}
