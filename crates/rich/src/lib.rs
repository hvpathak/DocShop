//! Rich documents: text with character styles (bold, colour…) and paragraph styles
//! (headings, lists…). The model is a plain `String` plus run-length style runs and
//! one entry per paragraph, so any text editor widget can edit the text and the
//! styles follow along through [`RichDoc::apply_edit`].

pub mod doc;
pub mod docx;
pub mod error;
pub mod markdown;
pub mod style;

pub use doc::{DocBuilder, EditOutcome, Flag, RichDoc, Run};
pub use error::{Error, Result};
pub use style::{Align, CharStyle, ParaStyle, Paragraph, Rgb};

/// Rich file formats HarshPad reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RichFormat {
    Docx,
    Markdown,
    /// HarshPad's own lossless format (JSON).
    HpDoc,
}

impl RichFormat {
    pub fn from_path(path: &std::path::Path) -> Option<RichFormat> {
        let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
        Some(match ext.as_str() {
            "docx" => RichFormat::Docx,
            "md" | "markdown" => RichFormat::Markdown,
            "hpdoc" => RichFormat::HpDoc,
            _ => return None,
        })
    }

    pub fn label(self) -> &'static str {
        match self {
            RichFormat::Docx => "Word document (.docx)",
            RichFormat::Markdown => "Markdown",
            RichFormat::HpDoc => "HarshPad document",
        }
    }
}

/// Reads a rich document from bytes in the given format.
pub fn decode(bytes: &[u8], format: RichFormat) -> Result<RichDoc> {
    match format {
        RichFormat::Docx => docx::read(bytes),
        RichFormat::Markdown => Ok(markdown::read(&String::from_utf8_lossy(bytes))),
        RichFormat::HpDoc => RichDoc::from_json(bytes),
    }
}

/// Writes a rich document to bytes in the given format.
pub fn encode(doc: &RichDoc, format: RichFormat) -> Result<Vec<u8>> {
    match format {
        RichFormat::Docx => docx::write(doc),
        RichFormat::Markdown => Ok(markdown::write(doc).into_bytes()),
        RichFormat::HpDoc => doc.to_json(),
    }
}
