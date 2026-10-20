//! Settings that survive restarts: the app mode and the parent lock.

use harshpad_core::modes::{Chapter, Mode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub mode: Mode,
    /// Hash of the parent-lock PIN; `None` when the mode isn't locked.
    #[serde(default)]
    pub pin: Option<u64>,
    /// Save files that already have a name automatically, every few seconds.
    #[serde(default)]
    pub autosave: bool,
    #[serde(default = "yes")]
    pub ruler: bool,
    /// The growing path's current chapter; `None` when the path is off.
    #[serde(default)]
    pub path: Option<Chapter>,
    /// The welcome card shows at every start until `Don't show this again`.
    #[serde(default)]
    pub hide_welcome: bool,
    /// Where the window was when HarshPad last closed: x, y, width, height (points), and
    /// whether it was maximized. It comes back there after the start-up card.
    #[serde(default)]
    pub window: Option<([f32; 4], bool)>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            pin: None,
            autosave: false,
            ruler: true,
            path: None,
            hide_welcome: false,
            window: None,
        }
    }
}

fn yes() -> bool {
    true
}

impl Settings {
    fn path() -> Option<std::path::PathBuf> {
        crate::recovery::data_dir().map(|d| d.join("HarshPad").join("settings.json"))
    }

    /// Missing or damaged settings fall back to the defaults.
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read(p).ok())
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::path() else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_vec_pretty(self) {
            let _ = harshpad_core::document::write_atomically(&path, &json);
        }
    }

    pub fn locked(&self) -> bool {
        self.pin.is_some()
    }
}
