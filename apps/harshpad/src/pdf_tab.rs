//! A PDF Studio tab: the PdfCraft editor (github.com/storytold/pdfcraft, MIT OR
//! Apache-2.0) hosted inside a HarshPad tab, one PDF per tab. PdfCraft draws its own
//! toolbars, panels and pages into the area the tab gives it; HarshPad's tabs replace
//! its tab strip, and its community links point to HarshPad's page (see the
//! `harshpad` branch of our fork).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui;
use pdfcraft_ui_egui::{PdfCraftApp, RecoveryStore};

use crate::writer_tab::HELP_URL;

pub struct PdfTab {
    pub app: Box<PdfCraftApp>,
    saved_prefs: Option<String>,
    prefs_checked: Instant,
}

/// `<data>/HarshPad/pdf`.
fn dir() -> Option<PathBuf> {
    crate::recovery::data_dir().map(|d| d.join("HarshPad").join("pdf"))
}

impl PdfTab {
    /// A PDF Studio tab with no document yet: PdfCraft's start page (recent files, tools).
    pub fn new() -> Self {
        let mut app = PdfCraftApp::new();
        app.embedded = true;
        app.help_url = Some(HELP_URL.into());
        app.units = pdfcraft_ui_egui::units::Unit::system();
        app.integrated_titlebar = false;
        app.window_controls = None;
        app.printer_source = Some(pdfcraft_ui_egui::system_printers());
        if let Some(json) = dir().and_then(|d| std::fs::read_to_string(d.join("prefs.json")).ok()) {
            app.restore(&json);
        }
        // Autosave of unsaved changes, shared by every PDF tab (each holds its own lock).
        if let Some(d) = dir() {
            app.enable_recovery(RecoveryStore::new(d.join("recovery")));
        }
        let saved_prefs = Some(app.persist());
        Self {
            app: Box::new(app),
            saved_prefs,
            prefs_checked: Instant::now(),
        }
    }

    pub fn open(path: &Path) -> Self {
        let mut tab = Self::new();
        tab.app.open_path(&path.to_string_lossy());
        tab
    }

    fn doc(&self) -> Option<&pdfcraft_engine::Document> {
        let (_, id) = self.app.active_ids()?;
        self.app.session.get(id)
    }

    pub fn title(&self) -> String {
        self.doc()
            .map_or_else(|| "PDF Studio".to_owned(), |d| d.display_name())
    }

    pub fn is_dirty(&self) -> bool {
        self.doc().is_some_and(|d| d.dirty)
    }

    pub fn path(&self) -> Option<&Path> {
        self.doc().and_then(|d| d.path.as_deref()).map(Path::new)
    }

    /// Runs a PdfCraft command such as `file.save` or `file.save_as`.
    pub fn run(&mut self, id: &str) -> std::io::Result<()> {
        if self.app.execute(id) {
            Ok(())
        } else {
            Err(std::io::Error::other(format!(
                "PDF Studio couldn't run {id}"
            )))
        }
    }

    pub fn is_pristine(&self) -> bool {
        self.doc().is_none()
    }

    /// Saves PdfCraft's preferences (tools, units, theme…) when they change.
    pub fn save_prefs(&mut self, force: bool) {
        if !force && self.prefs_checked.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.prefs_checked = Instant::now();
        let now = self.app.persist();
        if self.saved_prefs.as_deref() != Some(now.as_str())
            && let Some(d) = dir()
            && std::fs::create_dir_all(&d).is_ok()
        {
            let tmp = d.join("prefs.json.tmp");
            if std::fs::write(&tmp, &now).is_ok() {
                let _ = std::fs::rename(&tmp, d.join("prefs.json"));
            }
            self.saved_prefs = Some(now);
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        eframe::App::logic(&mut *self.app, &ctx, frame);
        eframe::App::ui(&mut *self.app, ui, frame);
        self.save_prefs(false);
    }
}
