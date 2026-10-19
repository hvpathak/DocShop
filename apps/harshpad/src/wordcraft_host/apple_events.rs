//! macOS open-documents and quit Apple events, as WordCraft's desktop app handles them.
//!
//! Finder double-clicks, Open With, drops on the Dock icon and `open -a HarshPad notes.docx`
//! don't pass paths on the command line: LaunchServices sends the app a `kAEOpenDocuments`
//! Apple event, which winit doesn't handle. The `fmv-macos-events` crate (which WordCraft,
//! PdfCraft and PhotoCraft use) registers a handler behind a safe API, so no `unsafe` here.

use std::path::PathBuf;

use fmv_macos_events::{Event, Inbox, Registration};

/// Keeps the handlers registered; hold it until the event loop returns.
pub struct AppleEvents {
    _registration: Registration,
    inbox: Inbox,
}

impl AppleEvents {
    /// Call on the main thread before the event loop starts, so the event that launched
    /// the app (a Finder double-click) is caught too.
    pub fn install() -> Self {
        let (registration, inbox) = Registration::install();
        Self {
            _registration: registration,
            inbox,
        }
    }

    /// The queue the app drains every frame; later events wake `ctx`.
    pub fn connect(&self, ctx: &egui::Context) -> Inbox {
        let ctx = ctx.clone();
        self.inbox.set_wake(move || ctx.request_repaint());
        self.inbox.clone()
    }
}

/// What arrived since the last frame: files to open, and whether to quit.
pub fn drain(inbox: &Inbox) -> (Vec<PathBuf>, bool) {
    let mut files = Vec::new();
    let mut quit = false;
    for e in inbox.drain() {
        match e {
            Event::Open(paths) => files.extend(paths),
            Event::Quit => quit = true,
        }
    }
    (files, quit)
}
