//! The Writer's preferences, custom dictionary, added fonts and recovery copies, kept in
//! HarshPad's own folder (`<data>/HarshPad/writer`), the way WordCraft's desktop app keeps
//! them in its settings folder.

use std::path::{Path, PathBuf};

use wordcraft_ui_egui::{UiState, WordApp};

/// `<data>/HarshPad/writer`.
pub fn dir() -> Option<PathBuf> {
    crate::recovery::data_dir().map(|d| d.join("HarshPad").join("writer"))
}

fn prefs_path() -> Option<PathBuf> {
    dir().map(|d| d.join("ui.json"))
}

/// Applies saved preferences (ribbon, options, recent files…) to a new Writer.
pub fn load(app: &mut WordApp) {
    if let Some(p) = prefs_path()
        && let Ok(bytes) = std::fs::read(&p)
        && let Some(ui) = UiState::from_json(&bytes)
    {
        app.apply_prefs(ui);
    }
}

/// The preferences as JSON, to save when they change.
pub fn snapshot(app: &WordApp) -> Option<Vec<u8>> {
    serde_json::to_vec_pretty(&app.prefs()).ok()
}

/// Writes preferences atomically (a temporary file, then a rename).
pub fn save(bytes: &[u8]) {
    let Some(path) = prefs_path() else { return };
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return;
    }
    if let Err(e) = write_atomically(&path, bytes) {
        log::warn!("writer preferences: cannot save {}: {e}", path.display());
    }
}

fn write_atomically(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    let written = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

/// The words Add to Dictionary kept, shared by every Writer tab.
pub fn load_dictionary(app: &mut WordApp) {
    if let Some(p) = dir().map(|d| d.join(wordcraft_engine::Session::USER_DICT_FILE)) {
        wordcraft_proof::load_user_dictionary(&p);
        app.session.user_dict_path = Some(p);
    }
}

/// Fonts the user added in the Writer, loaded once at start.
pub fn load_user_fonts() {
    if let Some(fonts) = dir().map(|d| d.join("fonts")) {
        let (loaded, skipped) = wordcraft_fonts::FontDb::global().load_user_fonts(&fonts);
        if !loaded.is_empty() || !skipped.is_empty() {
            log::info!(
                "writer fonts: {} loaded, {} skipped",
                loaded.len(),
                skipped.len()
            );
        }
    }
}

/// Crash-recovery copies of unsaved Writer documents.
pub fn recovery_dir() -> Option<PathBuf> {
    dir().map(|d| d.join("recovery"))
}
