//! Engine for HarshPad. Everything here works without a window, so it is
//! shared by the desktop app, the CLI and tests. Nothing in this crate may depend on
//! egui, eframe, winit or rfd.

pub mod commands;
pub mod document;
pub mod find;
pub mod session;
pub mod text_pos;
