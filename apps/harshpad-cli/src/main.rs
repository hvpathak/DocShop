//! Headless front end on the same core as the desktop app, so scripts and agents
//! can do what the UI does.

use std::path::Path;
use std::process::ExitCode;

use harshpad_core::commands::CATALOG;
use harshpad_core::document::{Document, write_atomically};
use harshpad_core::session::Session;
use harshpad_rich::{RichDoc, RichFormat};

const USAGE: &str = "usage:
  harshpad-cli commands
  harshpad-cli info <file>
  harshpad-cli replace <file> <find> <replace-with> [--match-case]
  harshpad-cli convert <in> <out>      (.txt .md .docx .hpdoc, any to any)
  harshpad-cli words <file>            (word and character count)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["commands"] => Ok(list_commands()),
        ["info", path] => info(Path::new(path)),
        ["replace", path, needle, replacement, flags @ ..] => {
            let match_case = flags.contains(&"--match-case");
            replace(Path::new(path), needle, replacement, match_case)
        }
        ["convert", input, output] => convert(Path::new(input), Path::new(output)),
        ["words", path] => {
            let doc = read_rich(Path::new(path))?;
            Ok(format!(
                "words: {}\ncharacters: {}",
                doc.word_count(),
                doc.char_count()
            ))
        }
        _ => Err(USAGE.to_owned()),
    }
}

/// Reads any supported file as a rich document (plain text becomes plain paragraphs).
fn read_rich(path: &Path) -> Result<RichDoc, String> {
    match RichFormat::from_path(path) {
        Some(format) => {
            let bytes = std::fs::read(path)
                .map_err(|e| format!("could not open {}: {e}", path.display()))?;
            harshpad_rich::decode(&bytes, format)
                .map_err(|e| format!("could not read {}: {e}", path.display()))
        }
        None => Ok(RichDoc::from_plain(&open(path)?.text)),
    }
}

fn convert(input: &Path, output: &Path) -> Result<String, String> {
    let doc = read_rich(input)?;
    let bytes = match RichFormat::from_path(output) {
        Some(format) => harshpad_rich::encode(&doc, format).map_err(|e| e.to_string())?,
        None => doc.text().replace('\n', "\r\n").into_bytes(),
    };
    write_atomically(output, &bytes)
        .map_err(|e| format!("could not save {}: {e}", output.display()))?;
    Ok(format!(
        "wrote {} ({} paragraphs, {} words)",
        output.display(),
        doc.paras().len(),
        doc.word_count()
    ))
}

fn list_commands() -> String {
    CATALOG
        .iter()
        .map(|s| {
            let menu = s.menu.map_or("-", |m| m.label());
            let shortcut = s.shortcut.map(|k| k.label()).unwrap_or_default();
            format!("{:<20} {:<7} {:<24} {shortcut}", s.id, menu, s.label)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn info(path: &Path) -> Result<String, String> {
    let doc = open(path)?;
    Ok(format!(
        "file: {}\nencoding: {}\nline endings: {}\nlines: {}\ncharacters: {}",
        path.display(),
        doc.encoding.label(),
        doc.line_ending.label(),
        doc.line_count(),
        doc.text.chars().count()
    ))
}

fn replace(
    path: &Path,
    needle: &str,
    replacement: &str,
    match_case: bool,
) -> Result<String, String> {
    if needle.is_empty() {
        return Err("the text to find is empty".to_owned());
    }
    let mut session = Session::new(open(path)?);
    session.find.needle = needle.to_owned();
    session.find.replacement = replacement.to_owned();
    session.find.match_case = match_case;

    let count = session.replace_all();
    if count > 0 {
        session
            .doc
            .save()
            .map_err(|e| format!("could not save {}: {e}", path.display()))?;
    }
    Ok(format!("replaced {count} occurrence(s)"))
}

fn open(path: &Path) -> Result<Document, String> {
    Document::open(path).map_err(|e| format!("could not open {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn replace_and_info_work_end_to_end() {
        let path = std::env::temp_dir().join(format!("hn-cli-{}.txt", std::process::id()));
        fs::write(&path, b"Cat cat\r\ncat\r\n").unwrap();
        let p = path.to_string_lossy().into_owned();

        let out = run(&args(&["replace", &p, "cat", "dog", "--match-case"])).unwrap();
        assert_eq!(out, "replaced 2 occurrence(s)");
        assert_eq!(fs::read(&path).unwrap(), b"Cat dog\r\ndog\r\n");

        let info = run(&args(&["info", &p])).unwrap();
        assert!(info.contains("line endings: Windows (CRLF)"));
        assert!(info.contains("lines: 3"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn convert_between_every_format() {
        let dir = std::env::temp_dir();
        let pid = std::process::id();
        let md = dir.join(format!("hn-cli-{pid}.md"));
        fs::write(&md, "# Title\n\nSome **bold** text.\n\n- one\n- two\n").unwrap();
        let p = |ext: &str| {
            dir.join(format!("hn-cli-{pid}-out.{ext}"))
                .to_string_lossy()
                .into_owned()
        };
        let m = md.to_string_lossy().into_owned();
        for out in [p("docx"), p("hpdoc"), p("txt")] {
            assert!(
                run(&args(&["convert", &m, &out]))
                    .unwrap()
                    .contains("4 paragraphs"),
                "{out}"
            );
        }
        // And back from .docx to Markdown keeps the structure.
        run(&args(&["convert", &p("docx"), &p("md")])).unwrap();
        let back = fs::read_to_string(p("md")).unwrap();
        assert!(
            back.contains("# Title") && back.contains("**bold**") && back.contains("- two"),
            "{back}"
        );
        assert!(run(&args(&["words", &m])).unwrap().contains("words: 6"));
        for f in [m, p("docx"), p("hpdoc"), p("txt"), p("md")] {
            fs::remove_file(f).ok();
        }
    }

    #[test]
    fn bad_arguments_are_errors_not_panics() {
        assert!(run(&args(&[])).is_err());
        assert!(run(&args(&["info"])).is_err());
        assert!(run(&args(&["info", "Z:\\no\\such\\file.txt"])).is_err());
        assert!(run(&args(&["replace", "x.txt", "", "y"])).is_err());
        assert!(run(&args(&["commands"])).unwrap().contains("file.saveAs"));
    }
}
