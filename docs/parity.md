# Road to 100%

HarshPad grows in three chapters: **Notepad → Writer → PDF**. The Writer and PDF chapters aim for the
full feature set of the two open-source projects we follow, [WordCraft](https://github.com/storytold/wordcraft)
and [PdfCraft](https://github.com/storytold/pdfcraft). This page tracks the distance, area by area.

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
| 3 · PDF | **0%** (next) | `░░░░░░░░░░░░░░░░░░░░` |

## Chapter 1 · Notepad

| Area | Score | Have | Missing |
|---|---|---|---|
| Editing, undo, find/replace, go to | 95% | Typing, grouped undo/redo, find/replace with match case, word wrap, zoom, status bar | Go to line, find with wrap-around toggle |
| Files and encodings | 100% | UTF-8 / BOM / UTF-16 LE/BE, CRLF/LF kept, atomic saves, recovery, drag and drop, AutoSave | |
| Tabs and windows | 90% | Browser-style tabs, reopen closed, search tabs | Tear a tab off into its own window |
| Printing | 0% | | Print, page setup |

## Chapter 2 · Writer

Since v0.6 every Writer tab **is the WordCraft editor** (github.com/storytold/wordcraft, MIT OR
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

Feature counts are the size of each area as listed in PdfCraft's public checklist.

| Area | Features | Score | Plan |
|---|---|---|---|
| A · Core model and fidelity | 56 | 0% | Phase 2: open, save, repair, undo |
| B · View and navigation | 92 | 0% | Phase 2: pages, zoom, thumbnails, find, copy text |
| C · Content editing | 61 | 0% | Later |
| D · Organize pages | 76 | 0% | Phase 2: rotate, delete, insert, reorder, merge, split, extract |
| E · Comments and review | 65 | 0% | Phase 3 |
| F · Forms | 79 | 0% | Phase 3: fill simple forms |
| G · Protect, redact, sanitize | 41 | 0% | Later |
| H · Digital signatures | 45 | 0% | Phase 3: visible signature |
| I · Scan and OCR | 26 | 0% | Later |
| J · Create and export | 34 | 0% | Phase 3: Text/Doc → PDF |
| K · Optimize and standards | 58 | 0% | Later |
| L · Print | 39 | 0% | Later |
| M · Accessibility | 61 | 0% | Later |
| N · Automation and preferences | 75 | 0% | CLI commands alongside every feature |
| **PDF total** | **808** | **0%** | |

## Order of work

1. **Writer: done** (v0.6, WordCraft inside HarshPad). Left: the agent control channel, Finder
   open on macOS, and registering HarshPad as the default app for documents.
2. **Chapter 3 · PDF next:** the PdfCraft editor in PDF tabs, the same way (its patched PDF renderer
   and parser included), then every PdfCraft area above reaches 100% with it.
3. New upstream features arrive with each *get their update*.