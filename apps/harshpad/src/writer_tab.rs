//! A Writer tab: the WordCraft editor (github.com/hvpathak/wordcraft, MIT OR
//! Apache-2.0) hosted inside a HarshPad tab. WordCraft draws its own title bar,
//! ribbon, panes, page and status bar into the area the tab gives it, and gets from
//! HarshPad everything WordCraft's own desktop app would give it: native file
//! dialogs, printing, the rich clipboard, the animation setting, preferences, the
//! custom dictionary and crash recovery.

use std::path::Path;
use std::time::{Duration, Instant};

use eframe::egui;
use serde_json::json;
use wordcraft_engine::Session;
use wordcraft_ui_egui::{Services, WordApp};

use crate::wordcraft_host::{animations, clipboard, file_dialogs, prefs, print, region};

pub struct WriterTab {
    pub app: Box<WordApp>,
    dialogs: file_dialogs::Launcher,
    /// The preferences as last saved, to write them only when they change.
    saved_prefs: Option<Vec<u8>>,
    prefs_checked: Instant,
}

/// Help in the Writer opens HarshPad's page; Feedback opens its issues.
pub const HELP_URL: &str = "https://github.com/hvpathak/DocShop#readme";
pub const FEEDBACK_URL: &str = "https://github.com/hvpathak/DocShop/issues";

fn services(file_dialog: file_dialogs::Hook, ctx: &egui::Context) -> Services {
    Services {
        file_dialog: Some(file_dialog),
        printers: Some(print::discover()),
        print_native: Some(print::submit()),
        clipboard_picture: Some(Box::new(clipboard::picture)),
        animations_enabled: Some(animations::hook(ctx)),
        clipboard_text: Some(Box::new(clipboard::text)),
        clipboard_html: Some(Box::new(clipboard::html)),
        set_clipboard_html: Some(Box::new(clipboard::set_html)),
        clipboard_rtf: Some(Box::new(clipboard::rtf)),
        clipboard_formats: Some(Box::new(clipboard::formats)),
        // HarshPad's own pages instead of WordCraft's community, and a light/dark switch
        // where WordCraft shows the account button.
        help_url: Some(HELP_URL.into()),
        community_url: Some(FEEDBACK_URL.into()),
        hide_community: true,
        theme_switch: true,
        ..Default::default()
    }
}

impl WriterTab {
    /// A new blank document on the paper of the system's region.
    pub fn new(ctx: &egui::Context) -> Self {
        let region = region::system();
        let doc = wordcraft_engine::blank_document(&region);
        let mut session = Session::new(doc);
        session.system_region = region;
        let (hook, dialogs) = file_dialogs::hook(ctx);
        let mut app = WordApp::new(session, services(hook, ctx));
        prefs::load(&mut app);
        prefs::load_dictionary(&mut app);
        // A region chosen in File › Options replaces the system's for new documents too.
        if app.session.prefs.region.is_some() {
            let doc = app.session.blank_document();
            app.session.set_document(doc);
        }
        app.watch_desktop_theme(ctx);
        if let Some(dir) = prefs::recovery_dir() {
            let repaint = ctx.clone();
            if let Err(e) = app
                .session
                .enable_recovery(dir, std::sync::Arc::new(move || repaint.request_repaint()))
            {
                log::warn!("writer recovery unavailable: {e}");
            }
        }
        app.integrated_titlebar = false;
        let saved_prefs = prefs::snapshot(&app);
        Self {
            app: Box::new(app),
            dialogs,
            saved_prefs,
            prefs_checked: Instant::now(),
        }
    }

    /// Opens a file in a new Writer tab.
    pub fn open(ctx: &egui::Context, path: &Path) -> Result<Self, String> {
        let mut tab = Self::new(ctx);
        tab.app
            .run("file.open", json!({ "path": path.to_string_lossy() }))
            .map_err(|e| e.to_string())?;
        Ok(tab)
    }

    pub fn title(&self) -> String {
        if self.app.has_document() {
            self.app.title_stem()
        } else {
            "WordShop".to_owned()
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

    /// Saves the preferences if they changed (checked every two seconds).
    pub fn save_prefs(&mut self, force: bool) {
        if !force && self.prefs_checked.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.prefs_checked = Instant::now();
        let now = prefs::snapshot(&self.app);
        if now.is_some() && now != self.saved_prefs {
            if let Some(bytes) = &now {
                prefs::save(bytes);
            }
            self.saved_prefs = now;
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, frame: &eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.app.logic(&ctx);
        self.app.ui(ui);
        // File dialogs the Writer asked for open over the window.
        self.dialogs.show(frame);
        self.save_prefs(false);
    }
}
