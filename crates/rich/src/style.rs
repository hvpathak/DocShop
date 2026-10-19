//! Character and paragraph styles.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// "RRGGBB", as Word files and the web write colours.
    pub fn to_hex(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.0, self.1, self.2)
    }

    pub fn from_hex(s: &str) -> Option<Rgb> {
        let s = s.trim_start_matches('#');
        if s.len() != 6 || !s.is_ascii() {
            return None;
        }
        let byte = |i: usize| s.get(i..i + 2).and_then(|h| u8::from_str_radix(h, 16).ok());
        Some(Rgb(byte(0)?, byte(2)?, byte(4)?))
    }
}

/// How a run of characters looks. `None` means "use the paragraph's default".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct CharStyle {
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub underline: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,
    /// Font size in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight: Option<Rgb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// Inline code.
    #[serde(default, skip_serializing_if = "is_false")]
    pub code: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum ParaStyle {
    #[default]
    Normal,
    Title,
    Heading1,
    Heading2,
    Heading3,
    Quote,
    Code,
    Bullet,
    Numbered,
}

impl ParaStyle {
    pub const ALL: [ParaStyle; 9] = [
        ParaStyle::Normal,
        ParaStyle::Title,
        ParaStyle::Heading1,
        ParaStyle::Heading2,
        ParaStyle::Heading3,
        ParaStyle::Quote,
        ParaStyle::Code,
        ParaStyle::Bullet,
        ParaStyle::Numbered,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ParaStyle::Normal => "Normal",
            ParaStyle::Title => "Title",
            ParaStyle::Heading1 => "Heading 1",
            ParaStyle::Heading2 => "Heading 2",
            ParaStyle::Heading3 => "Heading 3",
            ParaStyle::Quote => "Quote",
            ParaStyle::Code => "Code",
            ParaStyle::Bullet => "Bulleted list",
            ParaStyle::Numbered => "Numbered list",
        }
    }

    /// Default font size in points.
    pub fn size(self) -> u16 {
        match self {
            ParaStyle::Title => 26,
            ParaStyle::Heading1 => 20,
            ParaStyle::Heading2 => 16,
            ParaStyle::Heading3 => 13,
            ParaStyle::Code => 10,
            _ => 11,
        }
    }

    pub fn is_heading(self) -> bool {
        matches!(
            self,
            ParaStyle::Title | ParaStyle::Heading1 | ParaStyle::Heading2 | ParaStyle::Heading3
        )
    }

    pub fn is_list(self) -> bool {
        matches!(self, ParaStyle::Bullet | ParaStyle::Numbered)
    }

    /// The style a new paragraph gets after pressing Enter at the end of this one:
    /// lists, quotes and code continue; headings return to Normal.
    pub fn next(self) -> ParaStyle {
        match self {
            ParaStyle::Bullet | ParaStyle::Numbered | ParaStyle::Quote | ParaStyle::Code => self,
            _ => ParaStyle::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

impl Align {
    pub fn label(self) -> &'static str {
        match self {
            Align::Left => "Left",
            Align::Center => "Center",
            Align::Right => "Right",
            Align::Justify => "Justify",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Paragraph {
    pub style: ParaStyle,
    #[serde(default)]
    pub align: Align,
}

impl Paragraph {
    pub fn new(style: ParaStyle) -> Self {
        Self {
            style,
            align: Align::Left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colours_round_trip_and_reject_junk() {
        assert_eq!(Rgb(255, 0, 128).to_hex(), "FF0080");
        assert_eq!(Rgb::from_hex("#ff0080"), Some(Rgb(255, 0, 128)));
        assert_eq!(Rgb::from_hex("auto"), None);
        assert_eq!(Rgb::from_hex("ééé"), None);
    }

    #[test]
    fn enter_continues_lists_but_not_headings() {
        assert_eq!(ParaStyle::Bullet.next(), ParaStyle::Bullet);
        assert_eq!(ParaStyle::Heading1.next(), ParaStyle::Normal);
    }
}
