<p align="center">
  <img src="assets/icon-256.png" alt="DocShop icon: blue stacked document pages with the letters DS" width="132">
</p>

<h1 align="center">DocShop</h1>

<p align="center">
  <b>One place for all your documents.</b><br>
  Like a photo editor is the one stop for pictures, DocShop is the one stop for documents:<br>
  notes, word processing and PDFs in one window, with browser-style tabs.<br>
  No installer, no account, no telemetry. Windows, macOS and Linux.
</p>

<table align="center">
  <tr>
    <td align="center" width="25%"><img src="assets/apps/docshop.png" alt="DocShop icon" width="88"><br><b>DocShop</b><br><sub>One place for all<br>your documents</sub></td>
    <td align="center" width="25%"><img src="assets/apps/noteshop.png" alt="NoteShop icon: a spiral notepad with a pencil" width="88"><br><b>NoteShop</b><br><sub>Quick notes,<br>big ideas</sub></td>
    <td align="center" width="25%"><img src="assets/apps/wordshop.png" alt="WordShop icon: a document page with a W" width="88"><br><b>WordShop</b><br><sub>Write, edit and<br>create with ease</sub></td>
    <td align="center" width="25%"><img src="assets/apps/pdfshop.png" alt="PDFShop icon: a red PDF page" width="88"><br><b>PDFShop</b><br><sub>View, edit and<br>manage PDFs</sub></td>
  </tr>
</table>

<p align="center">
  <a href="https://github.com/hvpathak/DocShop/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/hvpathak/DocShop/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="Windows · macOS · Linux" src="https://img.shields.io/badge/Windows%20·%20macOS%20·%20Linux-native-2f7bf5?style=flat-square">
  <img alt="unsafe: forbidden" src="https://img.shields.io/badge/unsafe-forbidden-success?style=flat-square">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-3a3a3a?style=flat-square">
</p>

<p align="center"><i>Dedicated to my son, <b>Harshal Pathak</b>.</i></p>

<p align="center">
  <a href="#download"><b>⬇ Download</b></a> ·
  <a href="#the-four-shops">🛍 The four shops</a> ·
  <a href="#for-harshal-the-growing-path">📖 The growing path</a> ·
  <a href="#one-app-four-modes">🎛 Modes</a> ·
  <a href="#fast-measured">⚡ Speed</a> ·
  <a href="#roadmap">🚀 Roadmap</a>
</p>

<p align="center">
  <img src="docs/images/studio.png" alt="DocShop with a WordShop tab open: browser-style tabs in the title bar, then the full word-processor ribbon, rulers, and a page with the heading Quarterly plan" width="100%">
</p>

<p align="center">
  <img src="docs/images/splash.png" alt="The start-up card: a small window with the DocShop logo and name, the three apps NoteShop, WordShop and PDFShop, and Starting at the bottom" width="420"><br>
  <sub>DocShop opens as a small start-up card, then grows to where you left the window last time.</sub>
</p>

---

## The four shops

- **DocShop** · *One place for all your documents.* The whole suite: every tab, every app, one window.
  Open anything (text, `.docx`, `.odt`, `.rtf`, Markdown, PDF) and it lands in the right shop.
- **NoteShop** · *Quick notes, big ideas.* The fast plain-text editor: opens instantly, word wrap, find and
  replace, autosave, and crash recovery that keeps unsaved notes.
- **WordShop** · *Write, edit and create with ease.* A full word processor with the ribbon, styles, tables,
  pictures, track changes, comments and spelling; reads and writes `.docx`, `.doc`, `.odt`, `.rtf` and Markdown.
- **PDFShop** · *View, edit and manage PDFs.* Read, search, edit text, organize and combine pages, comment,
  fill and sign forms, protect and compress.

## For Harshal: the growing path

DocShop was made for my son, Harshal. It is built to grow up with him, one chapter at a time.

**Chapter 1 · First words.** It starts as a plain notepad. No fonts, no buttons he doesn't need: just a
blank page where he can type a list, a joke or a plan for tomorrow, save it, and find it again.

**Chapter 2 · Stories with style.** When he's ready, a parent unlocks the Writer. His words get titles,
headings, colours and lists: a story, a school report, a letter. The notepad stays right beside it.

**Chapter 3 · Real papers.** Later the PDF chapter opens: forms, books and certificates to read, search,
put in order and sign.

Each chapter keeps everything from the one before, and it works the same on Windows, macOS and Linux.

<p align="center">
  <img src="docs/images/chapter-1.png" alt="The Chapter 1 card: a notepad icon, 'Chapter 1 · First words · NoteShop is yours', a short story about starting with a blank page, a path showing 1. NoteShop as current and 2. Writer and 3. PDF locked, a Let's go button, and 'DocShop is dedicated to Harshal Pathak'" width="49%">
  <img src="docs/images/locked-choices.png" alt="On Chapter 1, the + menu offers Text file, while Document says 'Unlocks in Chapter 2: Writer' and PDF says 'Unlocks in Chapter 3: PDF', both greyed out with a lock" width="49%">
</p>

**How to use it:** top right **◧ → 📖 Start the growing path…**, choose a parent PIN, and DocShop
becomes Chapter 1. The **+** menu shows the apps still to come, locked. When it's time, a parent picks
**🔑 Unlock Chapter 2** and types the PIN; a new chapter card opens. **🚪 Leave the growing path** (also
behind the PIN) turns it off.

Where each chapter stands on its way to 100% of the projects we follow: **[Road to 100%](docs/parity.md)**.

## Three apps, one window, browser-style tabs

- **+ asks which app:** NoteShop, WordShop or PDFShop, always those three (<kbd>Ctrl+T</kbd>). One a mode
  switches off shows greyed out; on Harshal's growing path a locked one says which chapter unlocks it.
- **Tabs live in the title bar**, each with its app's colour icon. Drag to reorder, middle-click or <kbd>Ctrl+W</kbd> to
  close, <kbd>Ctrl+Tab</kbd> to cycle, <kbd>Ctrl+1</kbd>…<kbd>Ctrl+8</kbd> to jump. A dot marks unsaved work.
- **Split view, like a web browser:** right-click a tab → *Split view with…* to put two tabs of the same kind
  (two documents, two PDFs, two notes) side by side. Click a side to work in it (blue outline); drag the
  middle to resize; the × in a side's bottom-right corner takes it out of the split.
- **Close the last tab** and the window asks what to open next: NoteShop, WordShop, PDFShop or a file.
  <kbd>Ctrl+Shift+T</kbd> brings the last closed tab back, unsaved text included.
- **Indian border art** (paisleys and lotuses) runs along the title bar, gold on dark and maroon on light.
- **Each tab shows only its own app.** A NoteShop tab has NoteShop's tools; a WordShop tab has the full
  word-processor ribbon; a PDFShop tab has the PDF tools. Nothing mixes.

## One app, four modes

| Mode | What you get |
|---|---|
| **All together (DocShop)** | NoteShop, WordShop and PDFShop side by side in tabs |
| **NoteShop only** | The fast, distraction-free plain-text editor, nothing else |
| **WordShop only** | Just the word processor: perfect for students and kids |
| **PDFShop only** | Just the PDF tools |

Switch from the **◧** button at the top right. A **parent lock** keeps the app in one mode behind a 4–8 digit PIN,
so a child can use the Writer without wandering into anything else. A file whose app is switched off in the
current mode says so instead of opening.

## WordShop

<p align="center">
  <img src="docs/images/writer.png" alt="The Writer File screen inside a DocShop tab: Home, New, Open, Info, Recover, Save, Save As, Print, Export, Close and Options on the left; Good afternoon with templates for a blank document, a studio handbook, a letter and a résumé; and a Recent list with Quarterly plan.docx" width="100%">
</p>

Every WordShop tab is **[WordCraft](https://github.com/hvpathak/wordcraft)**, the open-source word
processor, running inside DocShop. That's the whole of it, not a subset: the full ribbon and command
catalog, styles and themes, tables, page layout and sections, headers and footers, footnotes,
references and citations, comments, track changes, compare, proofing, mail merge, pictures, shapes,
charts, SmartArt, equations and drawing, with native open/save dialogs, printing, the rich clipboard,
crash recovery and your own dictionary and fonts.

It opens .docx, .doc, .rtf, .odt, .html and Markdown, and saves those plus PDF. DocShop adds
the browser-style tabs, so a document, a plain-text note and a PDF sit side by side in one window.
WordCraft is pinned to an exact version on the `harshpad` branch of [our fork](https://github.com/hvpathak/wordcraft),
which adds DocShop's touches: Help and Feedback go to this project, no community or account buttons, and a
☀/☾ light/dark switch. New WordCraft releases come in with one command, `scripts/get-their-update.ps1`.

## PDFShop

<p align="center">
  <img src="docs/images/pdf.png" alt="A PDFShop tab with a red PDF icon: the PDF tools (All tools, Read, Edit, Convert, E-Sign), a tools list with Export, Fill and Sign, Edit, Create, Combine, Organize pages, Add comments, Scan and OCR, Protect, Redact, Compress, Prepare a form and Add a stamp, the page 'Harshal's Report', and side panels with zoom" width="100%">
</p>

Every PDFShop tab is **[PdfCraft](https://github.com/hvpathak/pdfcraft)**, the open-source PDF editor, running
inside DocShop: read with sharp zoom, search and copy text; edit text and images; organize, insert, extract,
split and combine pages; comment and highlight; fill and sign forms; create, export and compress PDFs; protect,
redact, OCR and more. Its own tab strip steps aside for DocShop's tabs (one PDF per tab), it keeps its light/dark
button, and Help goes to this project. It comes from the `harshpad` branch of
[our fork](https://github.com/hvpathak/pdfcraft), updated the same way as the Writer.

## NoteShop

<p align="center">
  <img src="docs/images/notepad.png" alt="A NoteShop tab: a plain-text shopping list; the ribbon has only File, Home, Insert, View and Help, and Home shows just Clipboard, Word Wrap, Date and Time, and Find, Replace, Select All; the AutoSave switch is a grey pill; the status bar reads Ln 1, Col 1, 19 words, 100%, UTF-8, Windows (CRLF)" width="100%">
</p>

Everything the classic notepad does, with guarantees it never had:

- **Never loses your text:** atomic saves, a prompt before closing unsaved work, and **crash recovery**
  (unsaved tabs are kept every 30 seconds and come back after a crash).
- **Encodings done right:** UTF-8, UTF-8 BOM, UTF-16 LE/BE; CRLF and LF kept byte for byte.
- **No crashes by design:** `unwrap`, `panic!`, raw indexing and `unsafe` are banned by the compiler in app code.
- Find/Replace, word wrap, zoom, status bar, drag and drop, the shortcuts your hands know.

## One rhythm across every tab

The same layout everywhere: **tabs · quick-access row · ribbon**, and the same status bar
(position · words · zoom · format). Each mode shows only the ribbon tabs it uses: NoteShop mode keeps
File, Home, Insert, Review, View and Help. Every action lives in one command catalog, so the same key does the
same thing in every tab and no key means two things. <kbd>Ctrl+Shift+P</kbd> opens the command palette.

## Fast, measured

On a mid-range laptop (i5-13450HX), release build:

| | |
|---|---|
| NoteShop: CLI, 200,000 replacements in a 10.7 MB file | **149 ms** |
| App size and start-up with the Writer built in | measured again for v0.6 |

## Download

| Platform | Download | Notes |
|---|---|---|
| **Windows** 10 / 11 (x64) | [DocShop-windows-x64.exe](https://github.com/hvpathak/DocShop/releases/latest/download/DocShop-windows-x64.exe) | One file, double-click. If SmartScreen warns, choose **More info → Run anyway**. |
| **macOS** 11+ (Apple Silicon **and** Intel) | [DocShop-macos-universal.zip](https://github.com/hvpathak/DocShop/releases/latest/download/DocShop-macos-universal.zip) | Unzip, drag to Applications. First launch: right-click → **Open**. |
| **Linux** x64 | [DocShop-linux-x64.tar.gz](https://github.com/hvpathak/DocShop/releases/latest/download/DocShop-linux-x64.tar.gz) | Unpack and run `./harshpad`. Needs GTK 3 (on most desktops already). |

Command-line tool: [Windows](https://github.com/hvpathak/DocShop/releases/latest/download/harshpad-cli-windows-x64.exe) ·
[macOS](https://github.com/hvpathak/DocShop/releases/latest/download/harshpad-cli-macos-universal.zip) · Linux (inside the tarball).

## The CLI

```text
harshpad-cli convert notes.md report.docx          # .txt .md .docx .hpdoc, any to any
harshpad-cli words essay.docx                      # word and character count
harshpad-cli replace notes.txt "teh" "the"         # replace all, saved atomically
harshpad-cli info notes.txt                        # encoding, line endings, lines
harshpad-cli commands                              # every command, menu and shortcut
```

## Build from source

```sh
git clone https://github.com/hvpathak/DocShop
cd DocShop
cargo run --release -p harshpad
cargo test --workspace
```

Rust 1.95+. On Linux, install GTK 3 and xkbcommon headers first
(`sudo apt install libgtk-3-dev libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev`).

## How it's built

```text
crates/core     commands (one catalog for every tab kind), modes, plain-text documents, find, undo
crates/rich     rich documents: styled-text model, .docx (from the public ECMA-376 standard), Markdown, .hpdoc
crates/layout   page layout for the built-in document view
apps/harshpad   the desktop app: browser-style tabs, NoteShop, and Writer tabs hosting WordCraft
apps/harshpad-cli  the same engines from the command line
```

- **Engine first:** all editing logic lives in libraries with no UI code, tested headlessly (50+ tests).
- **Built on WordCraft:** the Writer is WordCraft's editor and its desktop services, used under its open-source
  license (MIT OR Apache-2.0) and pinned by git revision; see [licenses](licenses/README.md).
- **Following upstream:** a weekly [radar](docs/upstream.md) watches the projects we build on, and moving to a new
  WordCraft version is a one-line change, tested on every platform.

## Roadmap

Full area-by-area tracker: **[Road to 100%](docs/parity.md)**.

- **WordShop:** done (WordCraft). Next: the agent control channel.
- **PDFShop:** done (PdfCraft).

Ideas and pull requests are welcome. The house rules: no `unwrap`/`panic`/`unsafe` outside tests,
and every change comes with a test.

## License

[MIT](LICENSE). The built-in Writer is WordCraft (MIT OR Apache-2.0); its license texts and NOTICE are in
[licenses/wordcraft](licenses/wordcraft). PDFShop is PdfCraft (MIT OR Apache-2.0); see [licenses/pdfcraft](licenses/pdfcraft).
DocShop is an independent project.
