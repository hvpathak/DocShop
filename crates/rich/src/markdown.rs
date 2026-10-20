//! Markdown in and out: the bridge between plain text and rich documents.
//! Headings, quotes, code blocks, bullet and numbered lists, bold, italic,
//! strikethrough, underline (as `<u>`), inline code and links survive a round trip.
//! Colours, highlights and font sizes have no Markdown form and are dropped.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::doc::{DocBuilder, RichDoc};
use crate::style::{CharStyle, ParaStyle, Paragraph};

pub fn read(src: &str) -> RichDoc {
    let mut b = DocBuilder::new();
    let mut pieces: Vec<(String, CharStyle)> = Vec::new();
    let mut style = CharStyle::default();
    let mut heading: Option<ParaStyle> = None;
    let mut lists: Vec<bool> = Vec::new(); // true = numbered
    let mut quote = 0usize;
    let mut code_block = false;
    let mut underline_html = false;

    let context = |heading: Option<ParaStyle>, lists: &[bool], quote: usize| -> ParaStyle {
        if let Some(h) = heading {
            h
        } else if let Some(&numbered) = lists.last() {
            if numbered {
                ParaStyle::Numbered
            } else {
                ParaStyle::Bullet
            }
        } else if quote > 0 {
            ParaStyle::Quote
        } else {
            ParaStyle::Normal
        }
    };
    let flush = |b: &mut DocBuilder, pieces: &mut Vec<(String, CharStyle)>, para: ParaStyle| {
        if !pieces.is_empty() {
            b.paragraph(Paragraph::new(para), pieces);
            pieces.clear();
        }
    };

    for event in Parser::new_ext(src, Options::ENABLE_STRIKETHROUGH) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(match level {
                    HeadingLevel::H1 => ParaStyle::Heading1,
                    HeadingLevel::H2 => ParaStyle::Heading2,
                    _ => ParaStyle::Heading3,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                flush(&mut b, &mut pieces, context(heading, &lists, quote));
                heading = None;
            }
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Item) => {
                flush(&mut b, &mut pieces, context(heading, &lists, quote));
            }
            Event::Start(Tag::List(start)) => {
                flush(&mut b, &mut pieces, context(heading, &lists, quote));
                lists.push(start.is_some());
            }
            Event::End(TagEnd::List(_)) => {
                lists.pop();
            }
            Event::Start(Tag::BlockQuote(_)) => quote += 1,
            Event::End(TagEnd::BlockQuote(_)) => quote = quote.saturating_sub(1),
            Event::Start(Tag::CodeBlock(kind)) => {
                let _ = matches!(kind, CodeBlockKind::Fenced(_));
                code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => code_block = false,
            Event::Start(Tag::Strong) => style.bold = true,
            Event::End(TagEnd::Strong) => style.bold = false,
            Event::Start(Tag::Emphasis) => style.italic = true,
            Event::End(TagEnd::Emphasis) => style.italic = false,
            Event::Start(Tag::Strikethrough) => style.strike = true,
            Event::End(TagEnd::Strikethrough) => style.strike = false,
            Event::Start(Tag::Link { dest_url, .. }) => style.link = Some(dest_url.to_string()),
            Event::End(TagEnd::Link) => style.link = None,
            Event::Text(t) if code_block => {
                // Each line of a code block becomes a Code paragraph.
                for line in t.trim_end_matches('\n').split('\n') {
                    b.paragraph(
                        Paragraph::new(ParaStyle::Code),
                        &[(line.to_owned(), CharStyle::default())],
                    );
                }
            }
            Event::Text(t) => {
                let mut s = style.clone();
                s.underline = underline_html;
                pieces.push((t.to_string(), s));
            }
            Event::Code(t) => {
                let mut s = style.clone();
                s.code = true;
                pieces.push((t.to_string(), s));
            }
            Event::InlineHtml(html) => match html.trim().to_ascii_lowercase().as_str() {
                "<u>" => underline_html = true,
                "</u>" => underline_html = false,
                _ => pieces.push((html.to_string(), style.clone())),
            },
            Event::SoftBreak | Event::HardBreak => pieces.push((" ".into(), style.clone())),
            Event::Rule => {
                flush(&mut b, &mut pieces, context(heading, &lists, quote));
                b.paragraph(
                    Paragraph::default(),
                    &[("———".into(), CharStyle::default())],
                );
            }
            _ => {}
        }
    }
    flush(&mut b, &mut pieces, context(heading, &lists, quote));
    b.finish()
}

pub fn write(doc: &RichDoc) -> String {
    let mut out = String::new();
    let mut number = 0u32;
    let mut prev: Option<ParaStyle> = None;
    for (para, range) in doc.paragraphs() {
        let style = para.style;
        // Blank lines between blocks, but not inside a list or a code block.
        if let Some(p) = prev {
            let same_block = (p == style && (style.is_list() || style == ParaStyle::Code))
                || (p.is_list() && style.is_list());
            if p == ParaStyle::Code && style != ParaStyle::Code {
                out.push_str("```\n");
            }
            out.push_str(if same_block { "" } else { "\n" });
        }
        if style == ParaStyle::Code {
            if prev != Some(ParaStyle::Code) {
                out.push_str("```\n");
            }
            out.push_str(doc.text().get(range).unwrap_or_default());
            out.push('\n');
            prev = Some(style);
            continue;
        }
        number = if style == ParaStyle::Numbered {
            number + 1
        } else {
            0
        };
        let prefix = match style {
            ParaStyle::Title | ParaStyle::Heading1 => "# ".to_owned(),
            ParaStyle::Heading2 => "## ".to_owned(),
            ParaStyle::Heading3 => "### ".to_owned(),
            ParaStyle::Quote => "> ".to_owned(),
            ParaStyle::Bullet => "- ".to_owned(),
            ParaStyle::Numbered => format!("{number}. "),
            _ => String::new(),
        };
        out.push_str(&prefix);
        let mut line = String::new();
        for (s, r) in doc.styled_ranges() {
            let start = r.start.max(range.start);
            let end = r.end.min(range.end);
            if start < end {
                line.push_str(&inline(doc.text().get(start..end).unwrap_or_default(), s));
            }
        }
        // A Normal line that looks like Markdown syntax must not turn into it.
        if style == ParaStyle::Normal {
            out.push_str(&protect(&line));
        } else {
            out.push_str(&line);
        }
        out.push('\n');
        prev = Some(style);
    }
    if prev == Some(ParaStyle::Code) {
        out.push_str("```\n");
    }
    out
}

/// Escapes the start of a plain line that Markdown would read as a heading, quote,
/// list item or code fence.
fn protect(line: &str) -> String {
    let t = line.trim_start();
    let indent = line.get(..line.len() - t.len()).unwrap_or_default();
    if t.starts_with(['#', '>', '-', '+']) {
        return format!("{indent}\\{t}");
    }
    if let Some((n, rest)) = t.split_once(". ")
        && !n.is_empty()
        && n.chars().all(|c| c.is_ascii_digit())
    {
        return format!("{indent}{n}\\. {rest}");
    }
    line.to_owned()
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '[' | ']' | '~' | '<') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// One styled piece as Markdown, with spaces kept outside the markers
/// (`** bold **` is not bold in Markdown).
fn inline(text: &str, s: &CharStyle) -> String {
    let core = text.trim();
    if core.is_empty() {
        return text.to_owned();
    }
    let lead = text
        .get(..text.len() - text.trim_start().len())
        .unwrap_or_default();
    let trail = text.get(text.trim_end().len()..).unwrap_or_default();
    let mut body = if s.code {
        format!("`{core}`")
    } else {
        escape(core)
    };
    if !s.code {
        if s.strike {
            body = format!("~~{body}~~");
        }
        if s.italic {
            body = format!("*{body}*");
        }
        if s.bold {
            body = format!("**{body}**");
        }
        if s.underline {
            body = format!("<u>{body}</u>");
        }
    }
    if let Some(url) = &s.link {
        body = format!("[{body}]({})", url.replace(' ', "%20").replace(')', "%29"));
    }
    format!("{lead}{body}{trail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Flag;

    #[test]
    fn reads_common_markdown() {
        let d = read(
            "# Title\n\nSome **bold** and *it* and ~~gone~~ and `code`.\n\n- a\n- b\n\n1. one\n2. two\n\n> wise\n\n```\nfn x() {}\nlet y;\n```\n\n[link](https://example.com)\n",
        );
        let styles: Vec<ParaStyle> = d.paras().iter().map(|p| p.style).collect();
        assert_eq!(
            styles,
            vec![
                ParaStyle::Heading1,
                ParaStyle::Normal,
                ParaStyle::Bullet,
                ParaStyle::Bullet,
                ParaStyle::Numbered,
                ParaStyle::Numbered,
                ParaStyle::Quote,
                ParaStyle::Code,
                ParaStyle::Code,
                ParaStyle::Normal
            ]
        );
        let bold_at = d.text().find("bold").unwrap();
        assert!(d.all_have(bold_at..bold_at + 4, Flag::Bold));
        let link_at = d.text().find("link").unwrap();
        assert_eq!(
            d.style_at(link_at + 1).link.as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn round_trip_keeps_text_and_styles() {
        let src = "## Plan\n\nWe **ship** *today* with <u>care</u>.\n\n- first\n- second\n\n```\nlet x = 1;\n```\n\n> done\n";
        let d = read(src);
        let again = read(&write(&d));
        assert_eq!(again.text(), d.text());
        assert_eq!(again.paras(), d.paras());
        assert_eq!(again.runs(), d.runs());
    }

    #[test]
    fn plain_lines_that_look_like_syntax_stay_plain() {
        let d = RichDoc::from_plain("# not a heading\n1. not a list\n*stars*");
        let back = read(&write(&d));
        assert_eq!(back.text(), d.text());
        assert!(back.paras().iter().all(|p| p.style == ParaStyle::Normal));
    }

    #[test]
    fn garbage_does_not_panic() {
        for src in [
            "", "\u{0}", "**", "[](", "```", "> > > -", "1.", "<u>", "\n\n\n",
        ] {
            let d = read(src);
            let _ = write(&d);
        }
    }
}
