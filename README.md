<h1 align="center">HarshPad</h1>

<p align="center">
  <b>Notepad, rebuilt from scratch in pure Rust.</b><br>
  One 4.4 MB <code>.exe</code>. No installer. No account. No telemetry. No crashes by design.
</p>

<p align="center">
  <img alt="100% Rust" src="https://img.shields.io/badge/100%25-Rust-b7410e?style=flat-square&logo=rust">
  <img alt="Windows 10 / 11" src="https://img.shields.io/badge/Windows-10%20%7C%2011-2f7bf5?style=flat-square&logo=windows">
  <img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-3a3a3a?style=flat-square">
  <img alt="unsafe: forbidden" src="https://img.shields.io/badge/unsafe-forbidden-success?style=flat-square">
</p>

<p align="center">
  <a href="https://github.com/hvpathak/HarshPad/releases/latest"><b>⬇ Download HarshPad.exe</b></a>
</p>

<p align="center">
  <img src="https://github.com/hvpathak/HarshPad/releases/download/v0.1.0/screenshot.png" alt="HarshPad showing a welcome file, with the File, Edit, Format, View and Help menus and a status bar reading Ln 1, Col 1, 100%, Windows (CRLF), UTF-8" width="100%">
</p>

---

## Why HarshPad?

Notepad is the one app everyone opens every day. HarshPad keeps everything your
hands already know and adds the guarantees a text editor should have had all along.

| | |
|---|---|
| ⚡ **Instant** | The window is up in **~30 ms** (warm start, measured on a mid-range laptop). |
| 📦 **One file** | A single 4.4 MB `.exe` that only uses DLLs Windows already ships. Copy it to a USB stick and go. |
| 🛡️ **Never loses your text** | Saves are atomic: HarshPad writes a temp file and swaps it in, so a full disk or a power cut can't leave you with half a file. Closing with unsaved changes always asks first. |
| 🦀 **No crashes by design** | `unwrap`, `expect`, `panic!`, raw indexing and `unsafe` are **banned by the compiler** in all app code (they're allowed in tests only). Every error becomes a message you can act on. |
| 🔤 **Encodings done right** | UTF-8, UTF-8 with BOM, UTF-16 LE and UTF-16 BE. Windows (CRLF) and Unix (LF) line endings are detected and written back **byte-for-byte**. |
| ⌨️ **Zero learning curve** | Same menus and shortcuts as Notepad: Ctrl+N/O/S, Ctrl+Shift+S, Ctrl+F, F3, Shift+F3, Ctrl+H, Ctrl+Plus/Minus/0. |
| 🤖 **Scriptable** | `harshpad-cli` runs the same engine headlessly. It replaced **200,000 matches in a 10.7 MB file in 149 ms**. |
| 🔒 **Private** | No network code at all. Nothing is sent anywhere, ever. |

## Download

1. Grab **`HarshPad.exe`** from the [latest release](https://github.com/hvpathak/HarshPad/releases/latest).
2. Double-click it. That's the whole install.

Open a file directly: `HarshPad.exe notes.txt` (a missing file starts empty and saves to that name, just like Notepad).
You can also drag a file onto the window.

> Windows may show a SmartScreen warning because the `.exe` isn't code-signed yet.
> Click **More info → Run anyway**, or build it yourself from source (below).

## Shortcuts

| Action | Shortcut |
|---|---|
| New / Open / Save | Ctrl+N / Ctrl+O / Ctrl+S |
| Save As | Ctrl+Shift+S |
| Find / Replace | Ctrl+F / Ctrl+H |
| Find Next / Find Previous | F3 / Shift+F3 |
| Zoom in / out / reset | Ctrl+Plus / Ctrl+Minus / Ctrl+0 |
| Undo / Redo / Select all | Ctrl+Z / Ctrl+Y / Ctrl+A |

Word Wrap is in **Format**, and the status bar (line, column, zoom, line endings, encoding) is in **View**.

## The CLI

```text
harshpad-cli commands                                   # every command, its menu and shortcut
harshpad-cli info notes.txt                             # encoding, line endings, lines, characters
harshpad-cli replace notes.txt "teh" "the" --match-case # replace all, saved atomically
```

## Build from source

```sh
git clone https://github.com/hvpathak/HarshPad
cd HarshPad
cargo build --release      # target/release/harshpad.exe and harshpad-cli.exe
cargo test --workspace
```

Needs Rust 1.88+ (edition 2024). On the `x86_64-pc-windows-gnu` toolchain, put MinGW-w64 on `PATH`.

## How it's built

```text
crates/core        the engine: documents, encodings, find/replace, the command catalog
                   (no UI code at all, so everything is testable headlessly)
apps/harshpad      the desktop app: a thin egui shell that draws the engine's state
apps/harshpad-cli  the same engine from the command line
```

- **Everything is a command.** Each action has one entry in a catalog (`file.saveAs`, `edit.findNext`, …)
  with its label, menu and shortcut. The menu bar, keyboard shortcuts and CLI are all generated from it.
- **The UI is thin.** Editing logic lives in `crates/core`, never in the window code.
- **Tests are the gate.** Encodings round-trip byte-for-byte, malformed files can't panic, find/replace is
  tested on non-ASCII text, and saving into a missing folder fails cleanly without losing the edit.
- **Clean-room.** HarshPad was written from scratch by observing how Notepad behaves.
  No Microsoft code, assets or decompiled code are used.

## Roadmap

- [ ] Tabs
- [ ] Go To Line (Ctrl+G) and Time/Date (F5)
- [ ] Print
- [ ] App icon and code signing
- [ ] Colour emoji
- [ ] Light / dark theme switch

Ideas and pull requests are welcome. Please keep the rules: no `unwrap`/`panic`/`unsafe` outside tests,
and every change comes with a test.

## License

[MIT](LICENSE). HarshPad is an independent project and is not affiliated with or endorsed by Microsoft.
"Notepad" and "Windows" are trademarks of Microsoft Corporation.
