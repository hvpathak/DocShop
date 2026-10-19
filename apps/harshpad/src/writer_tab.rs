//! A Writer tab: the WordCraft editor (github.com/storytold/wordcraft, MIT OR
//! Apache-2.0) hosted inside a HarshPad tab. WordCraft draws its own title bar,
//! ribbon, panes, page and status bar into the area the tab gives it.

use std::path::Path;

use eframe::egui;
use serde_json::json;
use wordcraft_engine::Session;
use wordcraft_ui_egui::{Services, WordApp};

pub struct WriterTab {
    pub app: Box<WordApp>,
}

impl WriterTab {
    /// A new blank document on the system region's paper.
    pub fn new() -> Self {
        let region = wordcraft_ui_egui::i18n::system_region();
        let doc = wordcraft_engine::blank_document(&region);
        let mut session = Session::new(doc);
        session.system_region = region;
        let app = WordApp::new(session, Services::default());
        Self { app: Box::new(app) }
    }

    /// Opens a file in a new Writer tab.
    pub fn open(path: &Path) -> Result<Self, String> {
        let mut tab = Self::new();
        tab.app
            .run("file.open", json!({ "path": path.to_string_lossy() }))
            .map_err(|e| e.to_string())?;
        Ok(tab)
    }

    pub fn title(&self) -> String {
        if self.app.has_document() {
            self.app.title_stem()
        } else {
            "Writer".to_owned()
        }
    }

    pub fn is_dirty(&self) -> bool {
        self.app.session.dirty
    }

    pub fn path(&self) -> Option<&Path> {
        self.app.session.path.as_deref()
    }

    /// An untouched new document, which opening a file may replace.
    pub fn is_pristine(&self) -> bool {
        !self.is_dirty() && self.path().is_none()
    }

    /// Runs a WordCraft command such as `file.save` or `file.saveAs`.
    pub fn run(&mut self, id: &str) -> std::io::Result<()> {
        self.app
            .run(id, json!({}))
            .map(|_| ())
            .map_err(std::io::Error::other)
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.app.logic(&ctx);
        self.app.ui(ui);
    }
}
