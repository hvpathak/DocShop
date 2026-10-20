//! Conversions between byte offsets (what Rust strings use) and character
//! positions (what the editor and the status bar show).

/// Largest char boundary at or below `index`, so slicing there cannot panic.
pub fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut i = index.min(text.len());
    while i > 0 && !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Byte offset of the character at `char_index`, or the text length past the end.
pub fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map_or(text.len(), |(i, _)| i)
}

/// Character position of `byte_index` (rounded down to a char boundary).
pub fn char_index(text: &str, byte_index: usize) -> usize {
    let end = floor_char_boundary(text, byte_index);
    text.get(..end).map_or(0, |head| head.chars().count())
}

/// 1-based line and column of the character at `char_index`, as Notepad shows them.
pub fn line_and_column(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for c in text.chars().take(char_index) {
        if c == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_line_and_column() {
        let text = "ab\ncde\n";
        assert_eq!(line_and_column(text, 0), (1, 1));
        assert_eq!(line_and_column(text, 2), (1, 3));
        assert_eq!(line_and_column(text, 3), (2, 1));
        assert_eq!(line_and_column(text, 7), (3, 1));
        assert_eq!(line_and_column(text, 999), (3, 1));
    }

    #[test]
    fn converts_between_char_and_byte_positions() {
        let text = "héllo";
        assert_eq!(byte_index(text, 2), 3);
        assert_eq!(char_index(text, 3), 2);
        assert_eq!(byte_index(text, 99), text.len());
        assert_eq!(char_index(text, 99), 5);
    }

    #[test]
    fn rounds_down_inside_multibyte_characters() {
        // Byte 2 is inside "é"; this must not panic.
        assert_eq!(floor_char_boundary("héllo", 2), 1);
        assert_eq!(char_index("héllo", 2), 1);
    }
}
