//! What the WordCraft editor asks of the app around it: native file dialogs, printing,
//! the rich clipboard (pictures, HTML, RTF), the system's animation setting, the GPU
//! setup that works on every Windows machine and the region for new documents. These
//! modules come from WordCraft's own desktop app (github.com/hvpathak/wordcraft, MIT OR Apache-2.0; see licenses/wordcraft).

pub mod animations;
#[cfg(target_os = "macos")]
pub mod apple_events;
pub mod clipboard;
pub mod file_dialogs;
#[cfg(any(target_os = "windows", target_os = "macos", test))]
pub mod graphics;
pub mod prefs;
pub mod print;
pub mod region;
