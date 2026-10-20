# Road to 100%

HarshPad grows in three chapters: **Notepad → Writer → PDF**. The Writer and PDF chapters aim for the
full feature set of the two open-source projects we follow, [WordCraft](https://github.com/hvpathak/wordcraft)
and [PdfCraft](https://github.com/hvpathak/pdfcraft). This page tracks the distance, area by area.

- **How it's measured:** each area gets a percentage for *depth*, not just presence: does it behave
  fully, with the options people expect? The chapter score is the weighted sum (weight = share of a typical
  user's time in that area; weights sum to 100).
- **How it moves:** the weekly [upstream radar](upstream.md) opens an issue when either project announces
  something new. Each item is mapped to a row here, built clean-room (behaviour only, never their code),
  tested, and the row's number goes up in the same pull request.
- **Honest scale:** the parent projects are large (tens of thousands of lines each) and still on their own
  way to 100%. Getting HarshPad there is a long road travelled chapter by chapter, not one release.

## Overall

| Chapter | Score | |
|---|---|---|
| 1 · Notepad | **95%** | `███████████████████░` |
| 2 · Writer | **100% of WordCraft** | `████████████████████` |
| 3 · PDF | **100% of PdfCraft** | `████████████████████` |

## Chapter 1 · Notepad

| Area | Score | Have | Missing |
|---|---|---|---|
| Editing, undo, find/replace, go to | 95% | Typing, grouped undo/redo, find/replace with match case, word wrap, zoom, status bar | Go to line, find with wrap-around toggle |
| Files and encodings | 100% | UTF-8 / BOM / UTF-16 LE/BE, CRLF/LF kept, atomic saves, recovery, drag and drop, AutoSave | |
| Tabs and windows | 90% | Browser-style tabs, reopen closed, search tabs | Tear a tab off into its own window |
| Printing | 0% | | Print, page setup |

## Chapter 2 · Writer

Since v0.6 every Writer tab **is the WordCraft editor** (github.com/hvpathak/wordcraft, MIT OR
Apache-2.0), pinned to an exact upstream commit and hosted inside HarshPad's tabs. So the Writer has
**100% of WordCraft's features**: its full ribbon and command catalog, styles, tables, page layout and
sections, headers/footers, footnotes, references and citations, review (comments, track changes,
compare, protect), proofing, mailings, pictures, shapes, charts, SmartArt, equations, drawing,
views, and every file format it reads and writes (.docx, .doc, .rtf, .odt, .html, .md and more).

| What WordCraft's own app adds around the editor | In HarshPad |
|---|---|
| Native open/save dialogs (non-blocking) | ✔ |
| Printing through the system spooler | ✔ |
| Rich clipboard: pictures, HTML, RTF, MathML | ✔ |
| System animation setting, desktop theme | ✔ |
| GPU setup that works on every Windows machine | ✔ |
| Preferences, recent files, custom dictionary, added fonts | ✔ (kept in HarshPad's own folder) |
| Crash recovery of unsaved documents | ✔ |
| Region-based paper size for new documents | ✔ |
| Agent control channel (control port) | next |
| Open files from Finder (macOS), default app for documents | next |

**How updates arrive:** when you say *get their update*, the pinned commit in Cargo.toml moves to
WordCraft's latest, and HarshPad is rebuilt, tested on all platforms and released.

## Chapter 3 · PDF

Since v0.7 every PDF Studio tab **is the PdfCraft editor** (github.com/hvpathak/pdfcraft, MIT OR
Apache-2.0), from the `harshpad` branch of [our fork](https://github.com/hvpathak/pdfcraft), with its patched
PDF renderer, parser and windowing applied the way its own workspace does. So PDF Studio has **100% of
PdfCraft's features**: all 14 areas of its checklist (viewing, editing, organizing, comments, forms and
JavaScript, protect and redact, signatures, OCR, create and export, optimize, print, accessibility,
automation). One PDF per HarshPad tab; its own Home, tabs and Open button step aside, its community links,
About and update checks are hidden, and Help goes to this project.

## Order of work

1. **Writer Studio: done** (v0.6, WordCraft). **PDF Studio: done** (v0.7, PdfCraft).
2. Left: the agent control channel and default-app registration for documents and PDFs.
3. New upstream features arrive with each *get their update* (`scripts/get-their-update.ps1`).
