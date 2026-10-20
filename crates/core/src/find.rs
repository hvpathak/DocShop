//! Text search for Find / Replace. Positions are byte offsets into the text.

use std::ops::Range;

use crate::text_pos::floor_char_boundary;

/// Next match at or after `from`, wrapping to the top like Notepad's "Wrap around".
pub fn find_next(text: &str, needle: &str, from: usize, match_case: bool) -> Option<Range<usize>> {
    if needle.is_empty() {
        return None;
    }
    let from = floor_char_boundary(text, from);
    find_from(text, needle, from, match_case).or_else(|| find_from(text, needle, 0, match_case))
}

/// Last match starting before `before`, wrapping to the bottom.
pub fn find_previous(
    text: &str,
    needle: &str,
    before: usize,
    match_case: bool,
) -> Option<Range<usize>> {
    let mut last_before = None;
    let mut last = None;
    for found in matches(text, needle, match_case) {
        if found.start < before {
            last_before = Some(found.clone());
        }
        last = Some(found);
    }
    last_before.or(last)
}

/// Replaces every match. Returns the new text and the count, or `None` if the
/// text could not be rebuilt (it is then left untouched).
pub fn replace_all(
    text: &str,
    needle: &str,
    replacement: &str,
    match_case: bool,
) -> Option<(String, usize)> {
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut pos = 0;
    for found in matches(text, needle, match_case) {
        out.push_str(text.get(pos..found.start)?);
        out.push_str(replacement);
        pos = found.end;
        count += 1;
    }
    out.push_str(text.get(pos..)?);
    Some((out, count))
}

/// True when `text[range]` is exactly one match for `needle`.
pub fn is_match(text: &str, range: &Range<usize>, needle: &str, match_case: bool) -> bool {
    text.get(range.clone())
        .and_then(|slice| match_len(slice, needle, match_case))
        .is_some_and(|len| len == range.len())
}

/// All non-overlapping matches from the start of the text.
pub fn matches<'a>(
    text: &'a str,
    needle: &'a str,
    match_case: bool,
) -> impl Iterator<Item = Range<usize>> + 'a {
    let mut pos = 0;
    std::iter::from_fn(move || {
        if needle.is_empty() {
            return None;
        }
        let found = find_from(text, needle, pos, match_case)?;
        pos = found.end;
        Some(found)
    })
}

fn find_from(text: &str, needle: &str, from: usize, match_case: bool) -> Option<Range<usize>> {
    let tail = text.get(from..)?;
    if match_case {
        let start = from.checked_add(tail.find(needle)?)?;
        return Some(start..start.checked_add(needle.len())?);
    }
    tail.char_indices().find_map(|(offset, _)| {
        let start = from.checked_add(offset)?;
        let len = match_len(text.get(start..)?, needle, false)?;
        Some(start..start.checked_add(len)?)
    })
}

/// If `haystack` starts with `needle`, how many bytes of `haystack` matched.
/// Case-insensitive matching can change byte lengths (for example "É" vs "é").
fn match_len(haystack: &str, needle: &str, match_case: bool) -> Option<usize> {
    let mut hay = haystack.char_indices();
    for n in needle.chars() {
        let (_, h) = hay.next()?;
        let same = h == n || (!match_case && h.to_lowercase().eq(n.to_lowercase()));
        if !same {
            return None;
        }
    }
    Some(hay.next().map_or(haystack.len(), |(i, _)| i))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_and_wraps_around() {
        let text = "cat dog cat";
        assert_eq!(find_next(text, "cat", 0, true), Some(0..3));
        assert_eq!(find_next(text, "cat", 1, true), Some(8..11));
        assert_eq!(find_next(text, "cat", 9, true), Some(0..3));
        assert_eq!(find_next(text, "bird", 0, true), None);
        assert_eq!(find_next(text, "", 0, true), None);
        assert_eq!(find_next(text, "cat", 9999, true), Some(0..3));
    }

    #[test]
    fn finds_previous_and_wraps_around() {
        let text = "cat dog cat";
        assert_eq!(find_previous(text, "cat", 11, true), Some(8..11));
        assert_eq!(find_previous(text, "cat", 8, true), Some(0..3));
        assert_eq!(find_previous(text, "cat", 0, true), Some(8..11));
        assert_eq!(find_previous(text, "", 5, true), None);
    }

    #[test]
    fn respects_match_case() {
        assert_eq!(find_next("Hello hello", "hello", 0, true), Some(6..11));
        assert_eq!(find_next("Hello hello", "hello", 0, false), Some(0..5));
    }

    #[test]
    fn handles_non_ascii_text() {
        let text = "naïve ÉCOLE école";
        let found = find_next(text, "école", 0, false).unwrap();
        assert_eq!(&text[found], "ÉCOLE");
        // A start offset inside a multi-byte character must not panic.
        assert!(find_next(text, "école", 3, true).is_some());
    }

    #[test]
    fn replaces_all_matches() {
        assert_eq!(
            replace_all("a-b-c", "-", "+", true),
            Some(("a+b+c".to_owned(), 2))
        );
        assert_eq!(
            replace_all("Aaa", "a", "b", false),
            Some(("bbb".to_owned(), 3))
        );
        assert_eq!(
            replace_all("abc", "", "x", true),
            Some(("abc".to_owned(), 0))
        );
        // A replacement containing the needle must not loop forever.
        assert_eq!(
            replace_all("aa", "a", "aa", true),
            Some(("aaaa".to_owned(), 2))
        );
    }

    #[test]
    fn checks_selection_matches() {
        assert!(is_match("say Hi", &(4..6), "hi", false));
        assert!(!is_match("say Hi", &(4..6), "hi", true));
        assert!(!is_match("say Hi", &(3..6), "hi", false));
        assert!(!is_match("say Hi", &(4..60), "hi", false));
    }
}
