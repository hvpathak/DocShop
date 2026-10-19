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
| 2 · Writer | **18%** | `████░░░░░░░░░░░░░░░░` |
| 3 · PDF | **0%** (next) | `░░░░░░░░░░░░░░░░░░░░` |

## Chapter 1 · Notepad

| Area | Score | Have | Missing |
|---|---|---|---|
| Editing, undo, find/replace, go to | 95% | Typing, grouped undo/redo, find/replace with match case, word wrap, zoom, status bar | Go to line, find with wrap-around toggle |
| Files and encodings | 100% | UTF-8 / BOM / UTF-16 LE/BE, CRLF/LF kept, atomic saves, recovery, drag and drop, AutoSave | |
| Tabs and windows | 90% | Browser-style tabs, reopen closed, search tabs | Tear a tab off into its own window |
| Printing | 0% | | Print, page setup |

## Chapter 2 · Writer

| Area | Weight | Score | Have | Missing |
|---|---|---|---|---|
| Typing, selection, clipboard, undo, find/replace | 12 | 55% | Typing with style carry-over, undo, clipboard, find/replace keeping formatting | Paste special, advanced find, column select, clipboard pane |
| Character and paragraph formatting | 11 | 35% | Bold, italic, underline, strike, code, size, colour, highlight, alignment, clear; the model, layout and .docx already carry fonts, super/subscript, caps, letter spacing, indents, spacing and line spacing | Ribbon controls and Font/Paragraph dialogs for those, borders, shading |
| Styles and themes | 8 | 20% | Styles gallery (Normal, Title, Heading 1–3, Quote, Code) | Custom styles, style pane, style sets, themes |
| Lists | 6 | 35% | Bullets, numbering, Enter continues, empty item ends, Markdown shortcuts | Multilevel lists, restart, list styles |
| Tables | 6 | 0% | | Insert, merge/split, styles, sort, formulas |
| Page layout and sections | 7 | 15% | Own layout engine: real pages, one-inch margins, page breaks, keep-with-next, keep lines together, ruler | Margins/size/orientation UI, columns, section breaks |
| Headers, footers, page numbers, fields | 5 | 0% | | All |
| Footnotes and endnotes | 3 | 0% | | All |
| References (contents, citations, captions, index) | 5 | 0% | | All |
| Review (comments, track changes, compare, protect) | 6 | 5% | Restrict Editing (read-only) | Comments, track changes, compare, editing restrictions |
| Proofing (spelling, grammar, thesaurus) | 5 | 5% | Word Count | Spelling, grammar, thesaurus, languages |
| Mailings | 2 | 0% | | Mail merge, envelopes, labels |
| Pictures, shapes, text boxes | 4 | 0% | | All |
| Equations | 1 | 0% | | All |
| Charts, diagrams, drawing | 4 | 0% | | All |
| View (modes, zoom, panes) | 3 | 25% | Zoom, ruler, status bar, print layout with every page | Read mode, web layout, navigation pane, outline |
| Backstage, files, options | 3 | 30% | New/open/save, .docx/.md/.hpdoc, AutoSave, recovery | Print, export PDF, more formats (ODT, RTF), options |
| Right-to-left scripts | 1 | 0% | | All |
| East Asian typography | 1 | 0% | | All |
| Accessibility | 1 | 10% | Keyboard shortcuts for every command | Screen-reader text, checker, alt text |
| **Writer total** | **100** | **18%** | | |

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

The Writer reaches 100% first, then the PDF chapter begins.

1. **Writer, by weight of the gap:** character and paragraph formatting (fonts, spacing, indents, dialogs),
   styles and themes, tables, page layout and sections, headers/footers/page numbers, lists, pictures and
   shapes, review (comments, track changes), proofing, references, footnotes, view modes, the rest.
2. **Chapter 3, phase 2:** PDF reader and page organizer (areas A, B, D).
3. **Chapter 3, phase 3 onward:** comments, forms, signatures, export, then every remaining PDF area.
4. New items from the weekly radar join the list in the area they belong to.