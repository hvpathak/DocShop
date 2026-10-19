//! Crash recovery: unsaved tabs are copied to the app-data folder every 30 seconds.
//! If HarshPad closes unexpectedly, the next start brings them back as tabs.
//! Choosing "Don't save" on exit, or saving, removes the copies.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

const EVERY: Duration = Duration::from_secs(30);

#[derive(Serialize, Deserialize)]
pub struct Saved {
    /// "text" or "doc".
    pub kind: String,
    pub title: String,
    pub path: Option<PathBuf>,
    /// Plain text, or a document in HarshPad's JSON format.
    pub content: String,
}

pub struct Recovery {
    dir: Option<PathBuf>,
    last: Instant,
}

impl Recovery {
    pub fn new() -> Self {
        Self {
            dir: data_dir().map(|d| d.join("HarshPad").join("recovery")),
            last: Instant::now(),
        }
    }

    /// Copies left behind by a previous session that didn't close cleanly.
    pub fn load(&self) -> Vec<Saved> {
        let Some(dir) = &self.dir else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for entry in entries.flatten().take(100) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Ok(bytes) = std::fs::read(&path)
                && let Ok(saved) = serde_json::from_slice::<Saved>(&bytes)
            {
                out.push(saved);
            }
        }
        out
    }

    /// Called every frame; writes at most every 30 seconds.
    pub fn tick(&mut self, dirty: impl FnOnce() -> Vec<(u64, Saved)>) {
        if self.last.elapsed() < EVERY {
            return;
        }
        self.last = Instant::now();
        self.write(dirty());
    }

    /// Replaces the stored copies with these tabs (an empty list clears them).
    pub fn write(&self, tabs: Vec<(u64, Saved)>) {
        let Some(dir) = &self.dir else { return };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        for (id, saved) in tabs {
            if let Ok(json) = serde_json::to_vec(&saved) {
                let _ = harshpad_core::document::write_atomically(
                    &dir.join(format!("tab-{id}.json")),
                    &json,
                );
            }
        }
    }

    pub fn clear(&self) {
        self.write(Vec::new());
    }
}

/// Where per-user app data lives on each system.
pub fn data_dir() -> Option<PathBuf> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Some(PathBuf::from(appdata));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library").join("Application Support"));
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home.map(|h| h.join(".local").join("share")))
}
