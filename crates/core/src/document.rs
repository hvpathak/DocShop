//! The open text file: its contents, where it lives on disk, its encoding and line
//! endings, and whether it has unsaved edits.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Larger files are refused with a clear error instead of freezing the editor.
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Crlf,
    Lf,
}

impl LineEnding {
    /// Files with no line breaks (and new files) use Windows endings, like Notepad.
    pub fn detect(text: &str) -> Self {
        if text.contains("\r\n") || !text.contains('\n') {
            LineEnding::Crlf
        } else {
            LineEnding::Lf
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LineEnding::Crlf => "Windows (CRLF)",
            LineEnding::Lf => "Unix (LF)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    pub fn label(self) -> &'static str {
        match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 with BOM",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Document {
    /// Text as edited, always with `\n` line breaks; converted back on save.
    pub text: String,
    pub path: Option<PathBuf>,
    pub line_ending: LineEnding,
    pub encoding: Encoding,
    dirty: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl Document {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            path: None,
            line_ending: LineEnding::Crlf,
            encoding: Encoding::Utf8,
            dirty: false,
        }
    }

    /// A new, empty document that will be saved to `path` (Notepad does this
    /// when asked to open a file that does not exist).
    pub fn new_at(path: &Path) -> Self {
        Self {
            path: Some(path.to_path_buf()),
            ..Self::new()
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let size = fs::metadata(path)?.len();
        if size > MAX_FILE_BYTES {
            return Err(io::Error::other(format!(
                "the file is {} MB; DocShop opens files up to {} MB",
                size / (1024 * 1024),
                MAX_FILE_BYTES / (1024 * 1024)
            )));
        }
        Ok(Self::from_bytes(&fs::read(path)?, Some(path.to_path_buf())))
    }

    pub fn from_bytes(bytes: &[u8], path: Option<PathBuf>) -> Self {
        let (raw, encoding) = decode(bytes);
        Self {
            line_ending: LineEnding::detect(&raw),
            text: raw.replace("\r\n", "\n"),
            path,
            encoding,
            dirty: false,
        }
    }

    /// Saves to the current path. Callers ask the user for a path first if there is none.
    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.clone().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "the document has no file name yet")
        })?;
        self.save_as(path)
    }

    pub fn save_as(&mut self, path: PathBuf) -> io::Result<()> {
        write_atomically(&path, &self.encoded_for_disk())?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_changed(&mut self) {
        self.dirty = true;
    }

    pub fn display_name(&self) -> String {
        self.path.as_deref().and_then(Path::file_name).map_or_else(
            || "Untitled".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        )
    }

    pub fn line_count(&self) -> usize {
        self.text.split('\n').count()
    }

    pub fn encoded_for_disk(&self) -> Vec<u8> {
        let text = match self.line_ending {
            LineEnding::Crlf => self.text.replace('\n', "\r\n"),
            LineEnding::Lf => self.text.clone(),
        };
        encode(&text, self.encoding)
    }
}

/// Writes to a temporary file next to `path`, then renames it over the target, so
/// a failed save (full disk, crash) never leaves a half-written file behind.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "not a file path"))?;
    let mut temp_name = file_name.to_os_string();
    temp_name.push(".harshpad-save.tmp");
    let temp = path.with_file_name(temp_name);

    let result = fs::write(&temp, bytes).and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        // Best effort: the original file is untouched either way.
        let _ = fs::remove_file(&temp);
    }
    result
}

fn decode(bytes: &[u8]) -> (String, Encoding) {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return (
            String::from_utf8_lossy(rest).into_owned(),
            Encoding::Utf8Bom,
        );
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return (decode_utf16(rest, u16::from_le_bytes), Encoding::Utf16Le);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return (decode_utf16(rest, u16::from_be_bytes), Encoding::Utf16Be);
    }
    (String::from_utf8_lossy(bytes).into_owned(), Encoding::Utf8)
}

/// A trailing odd byte is dropped rather than failing the whole file.
fn decode_utf16(bytes: &[u8], unit: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| unit(*pair))
        .collect();
    String::from_utf16_lossy(&units)
}

fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf8Bom => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        Encoding::Utf16Le => {
            let mut out = vec![0xFF, 0xFE];
            out.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
            out
        }
        Encoding::Utf16Be => {
            let mut out = vec![0xFE, 0xFF];
            out.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("harshpad-test-{}-{name}", std::process::id()))
    }

    #[test]
    fn detects_line_endings() {
        assert_eq!(LineEnding::detect("a\r\nb"), LineEnding::Crlf);
        assert_eq!(LineEnding::detect("a\nb"), LineEnding::Lf);
        assert_eq!(LineEnding::detect("single line"), LineEnding::Crlf);
    }

    #[test]
    fn crlf_file_round_trips_unchanged() {
        let path = temp_file("crlf.txt");
        fs::write(&path, b"one\r\ntwo\r\n").unwrap();

        let mut doc = Document::open(&path).unwrap();
        assert_eq!(doc.text, "one\ntwo\n");
        assert_eq!(doc.line_ending, LineEnding::Crlf);
        assert_eq!(doc.line_count(), 3);

        doc.save().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"one\r\ntwo\r\n");
        fs::remove_file(path).ok();
    }

    #[test]
    fn every_encoding_round_trips() {
        for encoding in [
            Encoding::Utf8,
            Encoding::Utf8Bom,
            Encoding::Utf16Le,
            Encoding::Utf16Be,
        ] {
            let bytes = encode("héllo ✓\r\n", encoding);
            let doc = Document::from_bytes(&bytes, None);
            assert_eq!(doc.text, "héllo ✓\n", "{encoding:?}");
            assert_eq!(doc.encoding, encoding);
            assert_eq!(doc.encoded_for_disk(), bytes, "{encoding:?}");
        }
    }

    #[test]
    fn malformed_input_does_not_panic() {
        // Invalid UTF-8, a lone BOM, an odd-length UTF-16 body, and nothing at all.
        for bytes in [
            &[0xC3, 0x28, 0xFF][..],
            &[0xFF, 0xFE][..],
            &[0xFF, 0xFE, 0x41][..],
            &[0xFE, 0xFF, 0xD8, 0x00][..],
            &[][..],
        ] {
            let doc = Document::from_bytes(bytes, None);
            let _ = doc.encoded_for_disk();
        }
    }

    #[test]
    fn dirty_flag_clears_on_save_and_no_temp_file_is_left() {
        let path = temp_file("dirty.txt");
        let mut doc = Document::new();
        doc.text.push_str("hi");
        doc.mark_changed();
        assert!(doc.is_dirty());

        doc.save_as(path.clone()).unwrap();
        assert!(!doc.is_dirty());
        assert_eq!(
            doc.display_name(),
            path.file_name().unwrap().to_string_lossy()
        );
        let temp = path.with_file_name(format!(
            "{}.harshpad-save.tmp",
            path.file_name().unwrap().to_string_lossy()
        ));
        assert!(!temp.exists());
        fs::remove_file(path).ok();
    }

    #[test]
    fn failed_save_keeps_the_document_dirty() {
        let mut doc = Document::new();
        doc.mark_changed();
        let missing_dir = temp_file("no-such-dir").join("file.txt");
        assert!(doc.save_as(missing_dir).is_err());
        assert!(doc.is_dirty());
        assert!(Document::new().save().is_err());
    }
}
