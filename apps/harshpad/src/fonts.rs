//! Fonts the computer already has, on Windows, macOS and Linux. HarshPad ships no
//! font files; anything missing falls back to egui's built-in fonts.

use std::path::{Path, PathBuf};

use eframe::egui::{self, FontData, FontDefinitions, FontFamily};

/// Font family for bold document text.
pub const BOLD: &str = "bold";

/// The bold family, or the regular one when it isn't bound: a Writer tab (WordCraft) installs
/// its own fonts for the whole window, and egui panics on a family that isn't bound.
pub fn bold(ctx: &egui::Context) -> egui::FontFamily {
    ctx.fonts(|f| bold_in(f.definitions()))
}

fn bold_in(defs: &FontDefinitions) -> FontFamily {
    let bold = FontFamily::Name(BOLD.into());
    if defs.families.contains_key(&bold) {
        bold
    } else {
        FontFamily::Proportional
    }
}

const EDITOR: &[&str] = &[
    "consola.ttf",
    "Menlo.ttc",
    "SFNSMono.ttf",
    "DejaVuSansMono.ttf",
    "LiberationMono-Regular.ttf",
    "NotoSansMono-Regular.ttf",
];
const INTERFACE: &[&str] = &[
    "segoeui.ttf",
    "SFNS.ttf",
    "Helvetica.ttc",
    "DejaVuSans.ttf",
    "NotoSans-Regular.ttf",
    "LiberationSans-Regular.ttf",
];
const BOLD_FILES: &[&str] = &[
    "segoeuib.ttf",
    "arialbd.ttf",
    "Arial Bold.ttf",
    "DejaVuSans-Bold.ttf",
    "NotoSans-Bold.ttf",
    "LiberationSans-Bold.ttf",
];
const SYMBOLS: &[&str] = &[
    "seguisym.ttf",
    "Apple Symbols.ttf",
    "DejaVuSans.ttf",
    "NotoSansSymbols2-Regular.ttf",
];
const EMOJI: &[&str] = &["seguiemj.ttf", "NotoEmoji-Regular.ttf"];

/// HarshPad's fonts, read from the system once.
fn ours() -> &'static FontDefinitions {
    static FONTS: std::sync::OnceLock<FontDefinitions> = std::sync::OnceLock::new();
    FONTS.get_or_init(build)
}

pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(ours().clone());
}

/// A Writer or PDF tab installs its own fonts for the whole window. When HarshPad's are
/// gone (the bold family is the marker), they are added back as fallbacks after the others,
/// so symbols, emoji and bold text keep working everywhere. Cheap when nothing changed.
pub fn ensure(ctx: &egui::Context) {
    let missing = ctx.fonts(|f| {
        !f.definitions()
            .families
            .contains_key(&FontFamily::Name(BOLD.into()))
    });
    if missing {
        let mut defs = ctx.fonts(|f| f.definitions().clone());
        merge(&mut defs, ours());
        ctx.set_fonts(defs);
    }
}

/// Adds `ours` to `base`: missing font data, our faces at the end of the shared families,
/// and our named families.
fn merge(base: &mut FontDefinitions, ours: &FontDefinitions) {
    for (name, data) in &ours.font_data {
        base.font_data
            .entry(name.clone())
            .or_insert_with(|| data.clone());
    }
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        let extra = ours.families.get(&family).cloned().unwrap_or_default();
        let list = base.families.entry(family).or_default();
        for name in extra {
            if !list.contains(&name) {
                list.push(name);
            }
        }
    }
    let regular = base
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut bold: Vec<String> = ours
        .families
        .get(&FontFamily::Name(BOLD.into()))
        .map(|l| l.iter().filter(|n| *n == "bold-face").cloned().collect())
        .unwrap_or_default();
    bold.extend(regular);
    base.families.insert(FontFamily::Name(BOLD.into()), bold);
}

fn build() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let mut load = |name: &str, candidates: &[&str]| -> bool {
        let Some(bytes) = find(candidates).and_then(|p| std::fs::read(p).ok()) else {
            return false;
        };
        fonts.font_data.insert(
            name.to_owned(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        true
    };
    let editor = load("editor", EDITOR);
    let interface = load("interface", INTERFACE);
    let bold = load("bold-face", BOLD_FILES);
    let symbols = load("symbols", SYMBOLS);
    let emoji = load("emoji", EMOJI);

    let mut prepend = |family: FontFamily, name: &str, present: bool| {
        if present {
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(0, name.to_owned());
        }
    };
    prepend(FontFamily::Monospace, "editor", editor);
    prepend(FontFamily::Proportional, "interface", interface);
    for family in [FontFamily::Monospace, FontFamily::Proportional] {
        let list = fonts.families.entry(family).or_default();
        if symbols {
            list.push("symbols".to_owned());
        }
        if emoji {
            list.push("emoji".to_owned());
        }
    }
    // The bold family must always exist (egui panics on unknown families), so it
    // falls back to the regular interface fonts when no bold face is installed.
    let mut bold_list = Vec::new();
    if bold {
        bold_list.push("bold-face".to_owned());
    }
    bold_list.extend(
        fonts
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default(),
    );
    fonts
        .families
        .insert(FontFamily::Name(BOLD.into()), bold_list);
    fonts
}

fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    for d in [
        "/System/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
        "/Library/Fonts",
        "/usr/share/fonts",
        "/usr/local/share/fonts",
    ] {
        dirs.push(PathBuf::from(d));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join("Library").join("Fonts"));
        dirs.push(home.join(".local").join("share").join("fonts"));
        dirs.push(home.join(".fonts"));
    }
    dirs
}

/// The first candidate found anywhere under the font folders (Linux keeps fonts in
/// sub-folders, so the search goes a few levels deep, with a budget).
fn find(candidates: &[&str]) -> Option<PathBuf> {
    fn search(dir: &Path, name: &str, depth: u32, budget: &mut u32) -> Option<PathBuf> {
        let direct = dir.join(name);
        if direct.is_file() {
            return Some(direct);
        }
        if depth == 0 {
            return None;
        }
        for entry in std::fs::read_dir(dir).ok()?.flatten() {
            *budget += 1;
            if *budget > 20_000 {
                return None;
            }
            let path = entry.path();
            if path.is_dir()
                && let Some(found) = search(&path, name, depth - 1, budget)
            {
                return Some(found);
            }
        }
        None
    }
    let dirs = font_dirs();
    candidates
        .iter()
        .find_map(|name| dirs.iter().find_map(|d| search(d, name, 4, &mut 0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_falls_back_when_another_app_replaced_the_fonts() {
        // A Writer tab installs WordCraft's fonts, which have no `bold` family.
        let mut defs = FontDefinitions::default();
        assert_eq!(bold_in(&defs), FontFamily::Proportional);
        defs.families
            .insert(FontFamily::Name(BOLD.into()), Vec::new());
        assert_eq!(bold_in(&defs), FontFamily::Name(BOLD.into()));
    }

    #[test]
    fn merging_adds_our_fonts_after_the_others() {
        let mut ours = FontDefinitions::default();
        ours.font_data.insert(
            "symbols".into(),
            std::sync::Arc::new(FontData::from_static(&[])),
        );
        ours.families
            .entry(FontFamily::Proportional)
            .or_default()
            .push("symbols".into());
        let mut base = FontDefinitions::default();
        let first = base
            .families
            .get(&FontFamily::Proportional)
            .and_then(|l| l.first().cloned());
        merge(&mut base, &ours);
        let list = base
            .families
            .get(&FontFamily::Proportional)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            list.first().cloned(),
            first,
            "the other app's fonts stay first"
        );
        assert_eq!(list.last().map(String::as_str), Some("symbols"));
        assert!(base.font_data.contains_key("symbols"));
        assert!(base.families.contains_key(&FontFamily::Name(BOLD.into())));
        // Merging twice adds nothing more.
        merge(&mut base, &ours);
        assert_eq!(
            base.families.get(&FontFamily::Proportional).map(Vec::len),
            Some(list.len())
        );
    }
}
