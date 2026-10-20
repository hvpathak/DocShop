//! Word documents (.docx), written from the public Office Open XML standard
//! (ECMA-376): a zip with `word/document.xml`, styles, numbering and relationships.
//! Supported: paragraph styles (Title, Heading 1–3, Quote, Code, lists), alignment,
//! bold, italic, underline, strikethrough, size, colour, highlight, inline code,
//! hyperlinks. Anything else is read as plain text; nothing makes the reader panic.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use quick_xml::escape::escape;
use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

use crate::doc::{DocBuilder, RichDoc};
use crate::error::{Error, Result, damaged};
use crate::style::{Align, Baseline, CharStyle, LineSpacing, ParaStyle, Paragraph, Rgb};

/// Largest single part of a .docx we will decompress.
const MAX_PART: u64 = 64 * 1024 * 1024;

// ---- Reading ----------------------------------------------------------------------

pub fn read(bytes: &[u8]) -> Result<RichDoc> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(damaged)?;
    let document = part(&mut zip, "word/document.xml")?.ok_or_else(|| {
        Error::Format("this is not a Word document (word/document.xml is missing)".into())
    })?;
    let numbering = part(&mut zip, "word/numbering.xml")?
        .map(|x| bullet_lists(&x))
        .unwrap_or_default();
    let links = part(&mut zip, "word/_rels/document.xml.rels")?
        .map(|x| hyperlinks(&x))
        .unwrap_or_default();
    parse_document(&document, &numbering, &links)
}

fn part(zip: &mut zip::ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Option<Vec<u8>>> {
    let Ok(file) = zip.by_name(name) else {
        return Ok(None);
    };
    if file.size() > MAX_PART {
        return Err(Error::TooLarge);
    }
    let mut out = Vec::new();
    file.take(MAX_PART).read_to_end(&mut out).map_err(damaged)?;
    Ok(Some(out))
}

/// The value of an attribute by its local name (namespace prefixes vary between files).
fn attr(e: &BytesStart<'_>, name: &[u8]) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        (a.key.local_name().as_ref() == name)
            .then(|| a.unescape_value().ok().map(|v| v.into_owned()))
            .flatten()
    })
}

/// `w:b`, `w:i`… are on unless `w:val` says false.
fn on(e: &BytesStart<'_>) -> bool {
    !matches!(
        attr(e, b"val").as_deref(),
        Some("0" | "false" | "off" | "none")
    )
}

/// numId → is it a bullet list (otherwise numbered)?
fn bullet_lists(xml: &[u8]) -> HashMap<usize, bool> {
    let mut abstract_fmt: HashMap<usize, bool> = HashMap::new();
    let mut num_to_abstract: HashMap<usize, usize> = HashMap::new();
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    let (mut current_abstract, mut level0, mut current_num) = (None, false, None);
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) => match e.local_name().as_ref() {
                b"abstractNum" => {
                    current_abstract = attr(&e, b"abstractNumId").and_then(|v| v.parse().ok())
                }
                b"lvl" => level0 = attr(&e, b"ilvl").as_deref() == Some("0"),
                b"numFmt" if level0 => {
                    if let Some(id) = current_abstract {
                        abstract_fmt.insert(id, attr(&e, b"val").as_deref() == Some("bullet"));
                    }
                }
                b"num" => current_num = attr(&e, b"numId").and_then(|v| v.parse().ok()),
                b"abstractNumId" => {
                    if let (Some(num), Some(abs)) =
                        (current_num, attr(&e, b"val").and_then(|v| v.parse().ok()))
                    {
                        num_to_abstract.insert(num, abs);
                    }
                }
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    num_to_abstract
        .into_iter()
        .map(|(num, abs)| (num, abstract_fmt.get(&abs).copied().unwrap_or(true)))
        .collect()
}

/// Relationship id → external link target.
fn hyperlinks(xml: &[u8]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut reader = Reader::from_reader(xml);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e) | Event::Empty(e)) if e.local_name().as_ref() == b"Relationship" => {
                let external = attr(&e, b"TargetMode").as_deref() == Some("External");
                if let (true, Some(id), Some(target)) =
                    (external, attr(&e, b"Id"), attr(&e, b"Target"))
                {
                    out.insert(id, target);
                }
            }
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    out
}

fn para_style_from_id(id: &str) -> ParaStyle {
    let id = id.to_ascii_lowercase().replace(' ', "");
    match id.as_str() {
        "title" => ParaStyle::Title,
        "heading1" => ParaStyle::Heading1,
        "heading2" => ParaStyle::Heading2,
        s if s.starts_with("heading") => ParaStyle::Heading3,
        "quote" | "intensequote" => ParaStyle::Quote,
        "code" | "sourcecode" | "htmlpreformatted" => ParaStyle::Code,
        _ => ParaStyle::Normal,
    }
}

fn named_highlight(name: &str) -> Option<Rgb> {
    Some(match name {
        "yellow" => Rgb(255, 255, 0),
        "green" => Rgb(0, 255, 0),
        "cyan" => Rgb(0, 255, 255),
        "magenta" => Rgb(255, 0, 255),
        "blue" => Rgb(0, 0, 255),
        "red" => Rgb(255, 0, 0),
        "darkBlue" => Rgb(0, 0, 139),
        "darkCyan" => Rgb(0, 139, 139),
        "darkGreen" => Rgb(0, 100, 0),
        "darkMagenta" => Rgb(139, 0, 139),
        "darkRed" => Rgb(139, 0, 0),
        "darkYellow" => Rgb(128, 128, 0),
        "darkGray" => Rgb(169, 169, 169),
        "lightGray" => Rgb(211, 211, 211),
        "black" => Rgb(0, 0, 0),
        "white" => Rgb(255, 255, 255),
        _ => return None,
    })
}

#[derive(Default)]
struct ParaState {
    style_id: Option<String>,
    num_id: Option<usize>,
    props: Paragraph,
    pieces: Vec<(String, CharStyle)>,
}

fn parse_document(
    xml: &[u8],
    numbering: &HashMap<usize, bool>,
    links: &HashMap<String, String>,
) -> Result<RichDoc> {
    let mut b = DocBuilder::new();
    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut para: Option<ParaState> = None;
    let mut run = CharStyle::default();
    let (mut in_ppr, mut in_rpr, mut in_t) = (false, false, false);
    let mut deleted = 0usize;
    let mut link: Option<String> = None;

    loop {
        let event = reader.read_event_into(&mut buf).map_err(damaged)?;
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                let empty = matches!(event, Event::Empty(_));
                match e.local_name().as_ref() {
                    b"p" if !empty => para = Some(ParaState::default()),
                    b"pPr" if !empty => in_ppr = true,
                    b"rPr" if !empty => in_rpr = true,
                    b"del" if !empty => deleted += 1,
                    b"hyperlink" if !empty => {
                        link = attr(e, b"id").and_then(|id| links.get(&id).cloned());
                    }
                    b"r" if !empty => {
                        run = CharStyle {
                            link: link.clone(),
                            ..CharStyle::default()
                        }
                    }
                    b"t" if !empty => in_t = deleted == 0,
                    b"pStyle" if in_ppr => {
                        if let Some(p) = para.as_mut() {
                            p.style_id = attr(e, b"val");
                        }
                    }
                    b"numId" if in_ppr => {
                        if let Some(p) = para.as_mut() {
                            p.num_id = attr(e, b"val")
                                .and_then(|v| v.parse().ok())
                                .filter(|&n| n != 0);
                        }
                    }
                    b"jc" if in_ppr => {
                        if let Some(p) = para.as_mut() {
                            p.props.align = match attr(e, b"val").as_deref() {
                                Some("center") => Align::Center,
                                Some("right" | "end") => Align::Right,
                                Some("both" | "distribute") => Align::Justify,
                                _ => Align::Left,
                            };
                        }
                    }
                    b"ind" if in_ppr && !in_rpr => {
                        if let Some(p) = para.as_mut() {
                            let num = |names: &[&[u8]]| {
                                names
                                    .iter()
                                    .find_map(|n| attr(e, n))
                                    .and_then(|v| v.parse::<i32>().ok())
                            };
                            p.props.indent_left = num(&[b"left", b"start"]).unwrap_or(0);
                            p.props.indent_right = num(&[b"right", b"end"]).unwrap_or(0);
                            p.props.first_line = match (num(&[b"firstLine"]), num(&[b"hanging"])) {
                                (_, Some(h)) => -h,
                                (Some(f), None) => f,
                                (None, None) => 0,
                            };
                        }
                    }
                    b"spacing" if in_ppr && !in_rpr => {
                        if let Some(p) = para.as_mut() {
                            let num = |n: &[u8]| attr(e, n).and_then(|v| v.parse::<u32>().ok());
                            if let Some(v) = num(b"before") {
                                p.props.space_before = Some(v);
                            }
                            if let Some(v) = num(b"after") {
                                p.props.space_after = Some(v);
                            }
                            if let Some(line) = num(b"line") {
                                p.props.line_spacing = match attr(e, b"lineRule").as_deref() {
                                    Some("exact") => LineSpacing::Exactly(line),
                                    Some("atLeast") => LineSpacing::AtLeast(line),
                                    // "auto": 240 is single spacing.
                                    _ => LineSpacing::Multiple(
                                        u16::try_from(line * 100 / 240)
                                            .unwrap_or(100)
                                            .clamp(50, 500),
                                    ),
                                };
                            }
                        }
                    }
                    b"pageBreakBefore" | b"keepNext" | b"keepLines" if in_ppr && !in_rpr => {
                        if let Some(p) = para.as_mut() {
                            let value = on(e);
                            match e.local_name().as_ref() {
                                b"pageBreakBefore" => p.props.page_break_before = value,
                                b"keepNext" => p.props.keep_with_next = value,
                                _ => p.props.keep_lines_together = value,
                            }
                        }
                    }
                    // Run properties (skip the paragraph-mark properties inside pPr).
                    name if in_rpr && !in_ppr => match name {
                        b"b" => run.bold = on(e),
                        b"i" => run.italic = on(e),
                        b"u" => run.underline = on(e),
                        b"strike" => run.strike = on(e),
                        b"dstrike" => run.double_strike = on(e),
                        b"caps" => run.all_caps = on(e),
                        b"smallCaps" => run.small_caps = on(e),
                        b"vertAlign" => {
                            run.baseline = match attr(e, b"val").as_deref() {
                                Some("superscript") => Baseline::Super,
                                Some("subscript") => Baseline::Sub,
                                _ => Baseline::Normal,
                            }
                        }
                        b"spacing" => {
                            run.spacing = attr(e, b"val")
                                .and_then(|v| v.parse::<i32>().ok())
                                .map_or(0, |v| v.clamp(-2000, 2000) as i16);
                        }
                        b"sz" => {
                            run.size = attr(e, b"val")
                                .and_then(|v| v.parse::<u32>().ok())
                                .map(|half| (half / 2).clamp(1, 400) as u16);
                        }
                        b"color" => run.color = attr(e, b"val").and_then(|v| Rgb::from_hex(&v)),
                        b"highlight" => {
                            run.highlight = attr(e, b"val").and_then(|v| named_highlight(&v))
                        }
                        b"shd" => {
                            if let Some(fill) = attr(e, b"fill").and_then(|v| Rgb::from_hex(&v)) {
                                run.highlight = Some(fill);
                            }
                        }
                        b"rFonts" => {
                            let name = attr(e, b"ascii").or_else(|| attr(e, b"hAnsi"));
                            let font = name.clone().unwrap_or_default().to_ascii_lowercase();
                            run.code = ["consolas", "courier", "mono", "menlo"]
                                .iter()
                                .any(|m| font.contains(m));
                            // Inline code keeps its own look; other fonts are kept by name.
                            if !run.code {
                                run.font = name.filter(|n| !n.is_empty());
                            }
                        }
                        _ => {}
                    },
                    b"tab" if !in_ppr && deleted == 0 => {
                        if let Some(p) = para.as_mut() {
                            p.pieces.push(("\t".into(), run.clone()));
                        }
                    }
                    b"br" | b"cr" if !in_ppr && deleted == 0 => {
                        if let Some(p) = para.as_mut() {
                            p.pieces.push((" ".into(), run.clone()));
                        }
                    }
                    _ => {}
                }
            }
            Event::Text(t) if in_t => {
                if let (Some(p), Ok(text)) = (para.as_mut(), t.unescape()) {
                    p.pieces.push((text.into_owned(), run.clone()));
                }
            }
            Event::End(e) => match e.local_name().as_ref() {
                b"p" => {
                    if let Some(p) = para.take() {
                        let mut style = p
                            .style_id
                            .as_deref()
                            .map_or(ParaStyle::Normal, para_style_from_id);
                        if let Some(num) = p.num_id {
                            style = if numbering.get(&num).copied().unwrap_or(true) {
                                ParaStyle::Bullet
                            } else {
                                ParaStyle::Numbered
                            };
                        }
                        b.paragraph(Paragraph { style, ..p.props }, &p.pieces);
                    }
                }
                b"pPr" => in_ppr = false,
                b"rPr" => in_rpr = false,
                b"t" => in_t = false,
                b"del" => deleted = deleted.saturating_sub(1),
                b"hyperlink" => link = None,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    Ok(b.finish())
}

// ---- Writing ----------------------------------------------------------------------

const NS: &str = r#"xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships""#;

pub fn write(doc: &RichDoc) -> Result<Vec<u8>> {
    let mut links: Vec<String> = Vec::new();
    let mut body = String::new();
    for (para, range) in doc.paragraphs() {
        body.push_str("<w:p><w:pPr>");
        let style_id = match para.style {
            ParaStyle::Normal => None,
            ParaStyle::Title => Some("Title"),
            ParaStyle::Heading1 => Some("Heading1"),
            ParaStyle::Heading2 => Some("Heading2"),
            ParaStyle::Heading3 => Some("Heading3"),
            ParaStyle::Quote => Some("Quote"),
            ParaStyle::Code => Some("Code"),
            ParaStyle::Bullet | ParaStyle::Numbered => Some("ListParagraph"),
        };
        if let Some(id) = style_id {
            body.push_str(&format!(r#"<w:pStyle w:val="{id}"/>"#));
        }
        for (on, tag) in [
            (para.keep_with_next, "<w:keepNext/>"),
            (para.keep_lines_together, "<w:keepLines/>"),
            (para.page_break_before, "<w:pageBreakBefore/>"),
        ] {
            if on {
                body.push_str(tag);
            }
        }
        match para.style {
            ParaStyle::Bullet => {
                body.push_str(r#"<w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr>"#)
            }
            ParaStyle::Numbered => {
                body.push_str(r#"<w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr>"#)
            }
            _ => {}
        }
        body.push_str(&spacing_xml(&para));
        body.push_str(&indent_xml(&para));
        let jc = match para.align {
            Align::Left => None,
            Align::Center => Some("center"),
            Align::Right => Some("right"),
            Align::Justify => Some("both"),
        };
        if let Some(jc) = jc {
            body.push_str(&format!(r#"<w:jc w:val="{jc}"/>"#));
        }
        body.push_str("</w:pPr>");
        for (s, r) in doc.styled_ranges() {
            let (start, end) = (r.start.max(range.start), r.end.min(range.end));
            let Some(text) = doc.text().get(start..end).filter(|t| !t.is_empty()) else {
                continue;
            };
            let run = run_xml(text, s);
            match &s.link {
                Some(url) => {
                    let n = links.iter().position(|l| l == url).unwrap_or_else(|| {
                        links.push(url.clone());
                        links.len() - 1
                    });
                    body.push_str(&format!(
                        r#"<w:hyperlink r:id="rIdLink{n}">{run}</w:hyperlink>"#
                    ));
                }
                None => body.push_str(&run),
            }
        }
        body.push_str("</w:p>");
    }
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document {NS}><w:body>{body}<w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="708" w:footer="708" w:gutter="0"/></w:sectPr></w:body></w:document>"#
    );
    let mut rels = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rIdNumbering" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering" Target="numbering.xml"/>"#,
    );
    for (n, url) in links.iter().enumerate() {
        rels.push_str(&format!(
            r#"<Relationship Id="rIdLink{n}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="{}" TargetMode="External"/>"#,
            escape(url)
        ));
    }
    rels.push_str("</Relationships>");

    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let parts: [(&str, &str); 6] = [
        ("[Content_Types].xml", CONTENT_TYPES),
        ("_rels/.rels", PACKAGE_RELS),
        ("word/document.xml", &document),
        ("word/_rels/document.xml.rels", &rels),
        ("word/styles.xml", STYLES),
        ("word/numbering.xml", NUMBERING),
    ];
    for (name, content) in parts {
        zip.start_file(name, opts).map_err(damaged)?;
        zip.write_all(content.as_bytes()).map_err(damaged)?;
    }
    Ok(zip.finish().map_err(damaged)?.into_inner())
}

fn spacing_xml(p: &Paragraph) -> String {
    let mut attrs = String::new();
    if let Some(v) = p.space_before {
        attrs.push_str(&format!(r#" w:before="{v}""#));
    }
    if let Some(v) = p.space_after {
        attrs.push_str(&format!(r#" w:after="{v}""#));
    }
    match p.line_spacing {
        LineSpacing::Multiple(100) => {}
        LineSpacing::Multiple(pct) => {
            attrs.push_str(&format!(
                r#" w:line="{}" w:lineRule="auto""#,
                u32::from(pct) * 240 / 100
            ));
        }
        LineSpacing::AtLeast(t) => {
            attrs.push_str(&format!(r#" w:line="{t}" w:lineRule="atLeast""#))
        }
        LineSpacing::Exactly(t) => attrs.push_str(&format!(r#" w:line="{t}" w:lineRule="exact""#)),
    }
    if attrs.is_empty() {
        String::new()
    } else {
        format!("<w:spacing{attrs}/>")
    }
}

fn indent_xml(p: &Paragraph) -> String {
    let mut attrs = String::new();
    if p.indent_left != 0 {
        attrs.push_str(&format!(r#" w:left="{}""#, p.indent_left));
    }
    if p.indent_right != 0 {
        attrs.push_str(&format!(r#" w:right="{}""#, p.indent_right));
    }
    if p.first_line > 0 {
        attrs.push_str(&format!(r#" w:firstLine="{}""#, p.first_line));
    } else if p.first_line < 0 {
        attrs.push_str(&format!(r#" w:hanging="{}""#, -p.first_line));
    }
    if attrs.is_empty() {
        String::new()
    } else {
        format!("<w:ind{attrs}/>")
    }
}

fn run_xml(text: &str, s: &CharStyle) -> String {
    let mut props = String::new();
    if s.code {
        props.push_str(r#"<w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/>"#);
    } else if let Some(font) = &s.font {
        let f = escape(font);
        props.push_str(&format!(
            r#"<w:rFonts w:ascii="{f}" w:hAnsi="{f}" w:cs="{f}"/>"#
        ));
    }
    if s.bold {
        props.push_str("<w:b/>");
    }
    if s.italic {
        props.push_str("<w:i/>");
    }
    if s.all_caps {
        props.push_str("<w:caps/>");
    }
    if s.small_caps {
        props.push_str("<w:smallCaps/>");
    }
    if s.strike {
        props.push_str("<w:strike/>");
    }
    if s.double_strike {
        props.push_str("<w:dstrike/>");
    }
    if let Some(c) = s.color {
        props.push_str(&format!(r#"<w:color w:val="{}"/>"#, c.to_hex()));
    }
    if s.spacing != 0 {
        props.push_str(&format!(r#"<w:spacing w:val="{}"/>"#, s.spacing));
    }
    if let Some(size) = s.size {
        props.push_str(&format!(
            r#"<w:sz w:val="{0}"/><w:szCs w:val="{0}"/>"#,
            u32::from(size) * 2
        ));
    }
    if s.underline || s.link.is_some() {
        props.push_str(r#"<w:u w:val="single"/>"#);
    }
    if let Some(h) = s.highlight {
        props.push_str(&format!(
            r#"<w:shd w:val="clear" w:color="auto" w:fill="{}"/>"#,
            h.to_hex()
        ));
    }
    match s.baseline {
        Baseline::Super => props.push_str(r#"<w:vertAlign w:val="superscript"/>"#),
        Baseline::Sub => props.push_str(r#"<w:vertAlign w:val="subscript"/>"#),
        Baseline::Normal => {}
    }
    let mut out = String::from("<w:r>");
    if !props.is_empty() {
        out.push_str(&format!("<w:rPr>{props}</w:rPr>"));
    }
    for (i, piece) in text.split('\t').enumerate() {
        if i > 0 {
            out.push_str("<w:tab/>");
        }
        if !piece.is_empty() {
            out.push_str(&format!(
                r#"<w:t xml:space="preserve">{}</w:t>"#,
                escape(piece)
            ));
        }
    }
    out.push_str("</w:r>");
    out
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/><Override PartName="/word/numbering.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml"/></Types>"#;

const PACKAGE_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#;

const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:ascii="Calibri" w:hAnsi="Calibri" w:cs="Calibri"/><w:sz w:val="22"/><w:szCs w:val="22"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr><w:spacing w:after="160" w:line="259" w:lineRule="auto"/></w:pPr></w:pPrDefault></w:docDefaults><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Title"><w:name w:val="Title"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:rPr><w:sz w:val="52"/><w:szCs w:val="52"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="360" w:after="80"/><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="40"/><w:szCs w:val="40"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading2"><w:name w:val="heading 2"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="160" w:after="80"/><w:outlineLvl w:val="1"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Heading3"><w:name w:val="heading 3"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:keepNext/><w:spacing w:before="160" w:after="80"/><w:outlineLvl w:val="2"/></w:pPr><w:rPr><w:b/><w:sz w:val="26"/><w:szCs w:val="26"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Quote"><w:name w:val="Quote"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="720" w:right="720"/></w:pPr><w:rPr><w:i/><w:color w:val="595959"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="Code"><w:name w:val="Code"/><w:basedOn w:val="Normal"/><w:pPr><w:spacing w:after="0"/></w:pPr><w:rPr><w:rFonts w:ascii="Consolas" w:hAnsi="Consolas" w:cs="Consolas"/><w:sz w:val="20"/><w:szCs w:val="20"/></w:rPr></w:style><w:style w:type="paragraph" w:styleId="ListParagraph"><w:name w:val="List Paragraph"/><w:basedOn w:val="Normal"/><w:pPr><w:ind w:left="720"/><w:contextualSpacing/></w:pPr></w:style></w:styles>"#;

const NUMBERING: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="•"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl></w:abstractNum><w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num><w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num></w:numbering>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::Flag;

    fn sample() -> RichDoc {
        let mut b = DocBuilder::new();
        b.paragraph(
            Paragraph::new(ParaStyle::Title),
            &[("Report".into(), CharStyle::default())],
        );
        b.paragraph(
            Paragraph {
                align: Align::Center,
                ..Paragraph::new(ParaStyle::Normal)
            },
            &[
                ("Plain, ".into(), CharStyle::default()),
                (
                    "bold".into(),
                    CharStyle {
                        bold: true,
                        ..Default::default()
                    },
                ),
                (
                    " & <big> ".into(),
                    CharStyle {
                        size: Some(18),
                        color: Some(Rgb(200, 0, 0)),
                        ..Default::default()
                    },
                ),
                (
                    "marked".into(),
                    CharStyle {
                        highlight: Some(Rgb(255, 255, 0)),
                        italic: true,
                        ..Default::default()
                    },
                ),
                (
                    "\tsite".into(),
                    CharStyle {
                        link: Some("https://example.com/a?b=1&c=2".into()),
                        ..Default::default()
                    },
                ),
            ],
        );
        b.paragraph(
            Paragraph::new(ParaStyle::Bullet),
            &[("dot".into(), CharStyle::default())],
        );
        b.paragraph(
            Paragraph::new(ParaStyle::Numbered),
            &[(
                "one".into(),
                CharStyle {
                    strike: true,
                    ..Default::default()
                },
            )],
        );
        b.paragraph(
            Paragraph::new(ParaStyle::Code),
            &[("let x = 1;".into(), CharStyle::default())],
        );
        b.paragraph(
            Paragraph::new(ParaStyle::Heading2),
            &[("héllo ✓".into(), CharStyle::default())],
        );
        b.finish()
    }

    #[test]
    fn round_trip_keeps_text_paragraphs_and_styles() {
        let doc = sample();
        let back = read(&write(&doc).unwrap()).unwrap();
        assert_eq!(back.text(), doc.text());
        assert_eq!(back.paras(), doc.paras());
        let at = |needle: &str| back.text().find(needle).unwrap();
        assert!(back.all_have(at("bold")..at("bold") + 4, Flag::Bold));
        let big = back.style_at(at("big") + 1);
        assert_eq!((big.size, big.color), (Some(18), Some(Rgb(200, 0, 0))));
        let marked = back.style_at(at("marked") + 1);
        assert_eq!(marked.highlight, Some(Rgb(255, 255, 0)));
        assert!(marked.italic);
        assert_eq!(
            back.style_at(at("site") + 1).link.as_deref(),
            Some("https://example.com/a?b=1&c=2")
        );
        assert!(back.all_have(at("one")..at("one") + 3, Flag::Strike));
    }

    #[test]
    fn damaged_files_are_errors_not_panics() {
        assert!(read(b"").is_err());
        assert!(read(b"PK\x03\x04 not really").is_err());
        // A real zip without word/document.xml.
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("hello.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"hi").unwrap();
        assert!(read(&zip.finish().unwrap().into_inner()).is_err());
        // Broken XML inside.
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            "word/document.xml",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"<w:document><w:body><w:p><w:r><w:t>ok</w:t></w:r></w:p></w:bod")
            .unwrap();
        let _ = read(&zip.finish().unwrap().into_inner());
    }

    #[test]
    fn tracked_deletions_are_skipped() {
        let xml = br#"<w:document xmlns:w="w"><w:body><w:p><w:r><w:t>keep</w:t></w:r><w:del><w:r><w:delText>gone</w:delText><w:t>gone</w:t></w:r></w:del></w:p></w:body></w:document>"#;
        let doc = parse_document(xml, &HashMap::new(), &HashMap::new()).unwrap();
        assert_eq!(doc.text(), "keep");
    }

    #[test]
    fn paragraph_and_character_details_round_trip() {
        let para = Paragraph {
            indent_left: 720,
            indent_right: 360,
            first_line: -360,
            space_before: Some(240),
            space_after: Some(0),
            line_spacing: LineSpacing::Multiple(150),
            page_break_before: true,
            keep_with_next: true,
            keep_lines_together: true,
            ..Paragraph::new(ParaStyle::Normal)
        };
        let fancy = CharStyle {
            font: Some("Georgia".into()),
            baseline: Baseline::Super,
            all_caps: true,
            small_caps: true,
            double_strike: true,
            spacing: 40,
            ..CharStyle::default()
        };
        let exact = Paragraph {
            line_spacing: LineSpacing::Exactly(360),
            align: Align::Justify,
            ..Paragraph::default()
        };
        let mut b = DocBuilder::new();
        b.paragraph(
            para,
            &[
                ("E = mc".into(), CharStyle::default()),
                ("2".into(), fancy.clone()),
            ],
        );
        b.paragraph(exact, &[("Second".into(), CharStyle::default())]);
        let doc = b.finish();
        for back in [
            read(&write(&doc).unwrap()).unwrap(),
            RichDoc::from_json(&doc.to_json().unwrap()).unwrap(),
        ] {
            assert_eq!(back.paras().first(), Some(&para));
            assert_eq!(back.paras().get(1), Some(&exact));
            // style_at reads the character before a position, like a caret.
            assert_eq!(back.style_at(back.text().find('2').unwrap() + 1), fancy);
        }
    }
}
