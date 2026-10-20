<p align="center">
  <img src="assets/icon-256.png" alt="HarshPad icon: a blue spiral notepad with the letters HP" width="132">
</p>

<h1 align="center">HarshPad Studio</h1>

<p align="center">
  <b>Notepad + Writer Studio + PDF Studio (soon): one suite, one window, browser-style tabs.</b><br>
  Writer Studio is <a href="https://github.com/storytold/wordcraft">WordCraft</a>, built in. Plain text, <code>.docx</code>, <code>.doc</code>, <code>.odt</code>, <code>.rtf</code>, Markdown.<br>
  No installer, no account, no telemetry. Windows, macOS and Linux.
</p>

<p align="center">
  <a href="https://github.com/hvpathak/HarshPad/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/hvpathak/HarshPad/actions/workflows/ci.yml/badge.svg"></a>
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="Windows · macOS · Linux" src="https://img.shields.io/badge/Windows%20·%20macOS%20·%20Linux-native-2f7bf5?style=flat-square">
  <img alt="unsafe: forbidden" src="https://img.shields.io/badge/unsafe-forbidden-success?style=flat-square">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-3a3a3a?style=flat-square">
</p>

<p align="center"><i>Dedicated to my son, <b>Harshal Pathak</b>.</i></p>

<p align="center">
  <a href="#download"><b>⬇ Download</b></a> ·
  <a href="#for-harshal-the-growing-path">📖 The growing path</a> ·
  <a href="#one-app-four-modes">🎛 Modes</a> ·
  <a href="#writer-studio">✍️ Writer Studio</a> ·
  <a href="#the-notepad">📄 Notepad</a> ·
  <a href="#fast-measured">⚡ Speed</a> ·
  <a href="#roadmap">🚀 Roadmap</a>
</p>

<p align="center">
  <img src="docs/images/studio.png" alt="HarshPad with a Writer tab open: browser-style tabs in the title bar, then the full word-processor ribbon (File, Home, Insert, Draw, Design, Layout, References, Mailings, Review, View, Developer, Zotero, Help) with Font, Paragraph, Styles and Editing groups, rulers, and a page with the heading Quarterly plan, bold and italic text, a bulleted list and a quote; the status bar reads Page 1 of 1, 25 words" width="100%">
</p>

---

## For Harshal: the growing path

HarshPad was made for my son, Harshal. It is built to grow up with him, one chapter at a time.

**Chapter 1 · First words.** It starts as a plain notepad. No fonts, no buttons he doesn't need: just a
blank page where he can type a list, a joke or a plan for tomorrow, save it, and find it again.

**Chapter 2 · Stories with style.** When he's ready, a parent unlocks the Writer. His words get titles,
headings, colours and lists: a story, a school report, a letter. The notepad stays right beside it.

**Chapter 3 · Real papers.** Later the PDF chapter opens: forms, books and certificates to read, search,
put in order and sign.

Each chapter keeps everything from the one before, and it works the same on Windows, macOS and Linux.

<p align="center">
  <img src="docs/images/chapter-1.png" alt="The Chapter 1 card: a notepad icon, 'Chapter 1 · First words · The Notepad is yours', a short story about starting with a blank page, a path showing 1. Notepad as current and 2. Writer and 3. PDF locked, a Let's go button, and 'HarshPad is dedicated to Harshal Pathak'" width="49%">
  <img src="docs/images/locked-choices.png" alt="On Chapter 1, the + menu offers Text file, while Document says 'Unlocks in Chapter 2: Writer' and PDF says 'Unlocks in Chapter 3: PDF', both greyed out with a lock" width="49%">
</p>

**How to use it:** top right **◧ → 📖 Start the growing path…**, choose a parent PIN, and HarshPad
becomes Chapter 1. The **+** menu shows the apps still to come, locked. When it's time, a parent picks
**🔑 Unlock Chapter 2** and types the PIN; a new chapter card opens. **🚪 Leave the growing path** (also
behind the PIN) turns it off.

Where each chapter stands on its way to 100% of the projects we follow: **[Road to 100%](docs/parity.md)**.

## Tabs like your browser, a ribbon like your word processor

<p align="center">
  <img src="docs/images/new-tab.png" alt="Clicking + next to the tabs opens a menu titled 'Open a new tab as…' with four cards: Text file (plain text, like a notepad, Ctrl+N), Document (formatted writing, .docx and Markdown, Ctrl+Shift+N), PDF (coming soon) and Open a file… (Ctrl+O)" width="100%">
</p>

- **Tabs live in the title bar.** Drag to reorder, middle-click or <kbd>Ctrl+W</kbd> to close, <kbd>Ctrl+Tab</kbd>
  to cycle, <kbd>Ctrl+1</kbd>…<kbd>Ctrl+8</kbd> to jump, <kbd>Ctrl+9</kbd> for the last tab. A dot marks unsaved work. Titles fade
  instead of being cut off, and tab widths hold still while you close tabs, so the next × is right under your pointer.
- **+ asks what you want:** a text file, a document, or a PDF (<kbd>Ctrl+T</kbd>). In a single-app mode it just opens one.
- **⌄ menu:** new tab as…, search your open tabs, and recently closed tabs. <kbd>Ctrl+Shift+T</kbd> brings the last one back,
  unsaved text included.
- **The ribbon:** File · Home · Insert · Draw · Design · Layout · References · Mailings · Review · View · Help, with
  big and small buttons in named groups. Double-click a tab to fold the ribbon away.
  Working today: clipboard, fonts and styles gallery, paragraph and alignment, find and replace, links, date and time,
  symbols, horizontal line, Word Count, Restrict Editing (read-only), ruler, status bar, zoom, modes, parent lock,
  keyboard shortcuts. Buttons for features still being built are shown greyed out and say **coming soon**.
- **Quick-access row:** an **AutoSave** switch (named files save themselves every 20 seconds), Save, Undo, Redo, the
  classic menus under **⋯**, the document name with *Saved* / *Edited*, and a **command search** box.

## One app, four modes

| Mode | What you get |
|---|---|
| **All together (Studio)** | Text files and rich documents side by side in tabs, with one-click conversion between them |
| **Notepad only** | The fast, distraction-free plain-text editor, nothing else |
| **Writer only** | Just the document writer: perfect for students and kids |
| **PDF only** | Coming in the next release |

<p align="center">
  <img src="docs/images/writer-mode.png" alt="Writer-only mode: the mode switch reads Writer, + opens a new document straight away, and the ribbon shows the writing tools above a formatted page" width="100%">
</p>

Switch from the **◧** button at the top right (or *View → Mode* in the ribbon). A **parent lock** keeps the app in one
mode behind a 4–8 digit PIN, so a child can use the Writer without wandering into anything else.
Files of another kind open as a new copy in the current mode, so nothing on disk is ever overwritten
in a different format.

## Writer Studio

<p align="center">
  <img src="docs/images/writer.png" alt="The Writer File screen inside a HarshPad tab: Home, New, Open, Info, Recover, Save, Save As, Print, Export, Close and Options on the left; Good afternoon with templates for a blank document, a studio handbook, a letter and a résumé; and a Recent list with Quarterly plan.docx" width="100%">
</p>

Every Writer Studio tab is **[WordCraft](https://github.com/storytold/wordcraft)**, the open-source word
processor, running inside HarshPad. That's the whole of it, not a subset: the full ribbon and command
catalog, styles and themes, tables, page layout and sections, headers and footers, footnotes,
references and citations, comments, track changes, compare, proofing, mail merge, pictures, shapes,
charts, SmartArt, equations and drawing, with native open/save dialogs, printing, the rich clipboard,
crash recovery and your own dictionary and fonts.

It opens .docx, .doc, .rtf, .odt, .html and Markdown, and saves those plus PDF. HarshPad adds
the browser-style tabs, so a document, a plain-text note and (soon) a PDF sit side by side in one window.
WordCraft is pinned to an exact version on the `harshpad` branch of [our fork](https://github.com/hvpathak/wordcraft),
which adds HarshPad's touches: Help and Feedback go to this project, no community or account buttons, and a
☀/☾ light/dark switch. New WordCraft releases come in with one command, `scripts/get-their-update.ps1`.
## The Notepad

<p align="center">
  <img src="docs/images/notepad.png" alt="Notepad-only mode: a plain-text shopping list; the ribbon keeps only File, Home, Insert, Review, View and Help, and Home shows just Clipboard, Word Wrap, Date and Time, and Find, Replace, Select All; the status bar reads Ln 1, Col 1, 19 words, 100%, UTF-8, Windows (CRLF)" width="100%">
</p>

Everything the classic notepad does, with guarantees it never had:

- **Never loses your text:** atomic saves, a prompt before closing unsaved work, and **crash recovery**
  (unsaved tabs are kept every 30 seconds and come back after a crash).
- **Encodings done right:** UTF-8, UTF-8 BOM, UTF-16 LE/BE; CRLF and LF kept byte for byte.
- **No crashes by design:** `unwrap`, `panic!`, raw indexing and `unsafe` are banned by the compiler in app code.
- Find/Replace, word wrap, zoom, status bar, drag and drop, the shortcuts your hands know.

## One rhythm across every tab

The same layout everywhere: **tabs · quick-access row · ribbon**, and the same status bar
(position · words · zoom · format). Each mode shows only the ribbon tabs it uses: Notepad mode keeps
File, Home, Insert, Review, View and Help. Every action lives in one command catalog, so the same key does the
same thing in every tab and no key means two things. <kbd>Ctrl+Shift+P</kbd> opens the command palette.

## Fast, measured

On a mid-range laptop (i5-13450HX), release build:

| | |
|---|---|
| Notepad: CLI, 200,000 replacements in a 10.7 MB file | **149 ms** |
| App size and start-up with the Writer built in | measured again for v0.6 |

## Download

| Platform | Download | Notes |
|---|---|---|
| **Windows** 10 / 11 (x64) | [HarshPad-windows-x64.exe](https://github.com/hvpathak/HarshPad/releases/latest/download/HarshPad-windows-x64.exe) | One file, double-click. If SmartScreen warns, choose **More info → Run anyway**. |
| **macOS** 11+ (Apple Silicon **and** Intel) | [HarshPad-macos-universal.zip](https://github.com/hvpathak/HarshPad/releases/latest/download/HarshPad-macos-universal.zip) | Unzip, drag to Applications. First launch: right-click → **Open**. |
| **Linux** x64 | [HarshPad-linux-x64.tar.gz](https://github.com/hvpathak/HarshPad/releases/latest/download/HarshPad-linux-x64.tar.gz) | Unpack and run `./harshpad`. Needs GTK 3 (on most desktops already). |

Command-line tool: [Windows](https://github.com/hvpathak/HarshPad/releases/latest/download/harshpad-cli-windows-x64.exe) ·
[macOS](https://github.com/hvpathak/HarshPad/releases/latest/download/harshpad-cli-macos-universal.zip) · Linux (inside the tarball).

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
git clone https://github.com/hvpathak/HarshPad
cd HarshPad
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
apps/harshpad   the desktop app: browser-style tabs, the Notepad, and Writer tabs hosting WordCraft
apps/harshpad-cli  the same engines from the command line
```

- **Engine first:** all editing logic lives in libraries with no UI code, tested headlessly (50+ tests).
- **Built on WordCraft:** the Writer is WordCraft's editor and its desktop services, used under its open-source
  license (MIT OR Apache-2.0) and pinned by git revision; see [licenses](licenses/README.md).
- **Following upstream:** a weekly [radar](docs/upstream.md) watches the projects we build on, and moving to a new
  WordCraft version is a one-line change, tested on every platform.

## Roadmap

Full area-by-area tracker: **[Road to 100%](docs/parity.md)**.

- **Writer:** done (WordCraft). Next for it: the agent control channel, opening from Finder on macOS.
- **PDF** (next): the PdfCraft editor in PDF tabs, built in the same way; read with sharp zoom, thumbnails, search, copy text; rotate, delete, reorder, merge, split pages.
- **Comment & sign:** highlights, notes, ink, signatures; export any document to PDF with page numbers.

Ideas and pull requests are welcome. The house rules: no `unwrap`/`panic`/`unsafe` outside tests,
and every change comes with a test.

## License

[MIT](LICENSE). The built-in Writer is WordCraft (MIT OR Apache-2.0); its license texts and NOTICE are in
[licenses/wordcraft](licenses/wordcraft). HarshPad is an independent project.
