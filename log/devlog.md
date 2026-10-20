# Dev log

## 2026-10-20: v0.8.0, DocShop
- The suite is **DocShop** ("one place for all your documents"), with NoteShop (quick notes, big ideas),
  WordShop (write, edit and create with ease) and PDFShop (view, edit and manage PDFs). The four icons are
  the owner's artwork (assets/apps, 128 px with rounded transparent corners); DocShop's is the app icon.
  Release files are now DocShop-*. Settings, recovery and the repo keep the HarshPad name, so nothing is lost.
- Split view (right-click a tab), Indian title-bar art, no ⌄ menu, one PDF toolbar row, start page when the
  last tab closes, mode switch on every tab.
- Build sources come only from hvpathak forks (craft-libs, winprint.rs added); CI enforces it.
- Split view: each side's × sits in a gap under the pane (it used to cover the editor's zoom control).
  The window redraws right after the start-up card (it stayed blank until the mouse moved). DocShop opens in
  the sun/moon light/dark choice, and NoteShop tabs and the start page get it back after a PDF tab.
- README: every picture recaptured full screen, plus split-view videos for WordShop and PDFShop.

## 2026-10-20: v0.7.0, PDF Studio and Writer Studio
- Suite names: Notepad + Writer Studio + PDF Studio, with colour icons drawn as shapes.
- Forks hvpathak/wordcraft and hvpathak/pdfcraft, each with a small `harshpad` branch: embedding options so
  Help/Feedback go to HarshPad's GitHub, community/Discord/About/account links are hidden, and (WordCraft) a
  sun/moon light-dark switch replaces the account button. Actions are off on the forks.
- PDF Studio: PdfCraft in HarshPad tabs (one PDF per tab), with its patched hayro/lopdf/winit/egui-wgpu/vello.
  New TabKind::Pdf; PDF-only mode; Chapter 3 of the growing path unlocks it.
- + always offers exactly three apps; ones a mode switches off are greyed, path-locked ones say their chapter.
- Each tab shows only its own app: Notepad tabs get File/Home/Insert/View/Help without document tools.
- Fonts: a Writer or PDF tab replaces the window's fonts; HarshPad merges its symbols, emoji and bold back
  in (fixed a startup crash and boxes instead of symbols). AutoSave switch visible in light mode.
- scripts/get-their-update.ps1 with upstream-guard baselines; Dependabot patch-only; actions bumped.
- Opens every file given on the command line.

## 2026-10-19: v0.6.0, the Writer is WordCraft
- User's decision: build on WordCraft's code (MIT OR Apache-2.0) instead of rewriting it, for 100% of its features.
- HarshPad moved to egui/eframe 0.36 on wgpu, WordCraft's version, so its editor runs inside a HarshPad tab
  (its panels draw into the Ui the tab gives it).
- Writer tabs host `wordcraft_ui_egui::WordApp`, pinned by git rev, with WordCraft's patched krilla.
  New documents and .docx/.doc/.rtf/.odt/.html/.md open there; .txt stays in the Notepad; our bars step aside.
- WordCraft's desktop services ported into `wordcraft_host`: native file dialogs, printing, rich clipboard,
  animations, GPU setup, region; preferences, dictionary, fonts and recovery in `<data>/HarshPad/writer`.
- Welcome card at start until "Don't show this again"; no console window in test builds.
- Earlier the same day: own layout engine and page editor (crates/layout), kept for the built-in .hpdoc view.

## 2026-10-19: v0.5.0, the growing path (for Harshal)
- Growing path: Chapter 1 Notepad → Chapter 2 Writer → Chapter 3 PDF, unlocked one by one with the parent
  PIN; each chapter keeps what came before. Chapter cards at unlock and at start; locked choices stay visible
  in the + menu; the mode menu becomes the chapter menu. Off by default. Core logic and tests in modes.rs.
- Dedication to Harshal Pathak in the README, the About box and the chapter cards.
- docs/parity.md: Road to 100%, every area of the writer and PDF projects we follow, scored; Writer first.
- Ribbon: narrower style cards, thin scrollbar.

## 2026-10-19: v0.4.0, browser-style tabs and the ribbon
- Tabs moved into the title bar, modelled on a Chromium browser's tab strip (studied from a screenshot only):
  fading titles, close on active/hovered tab, unsaved dot, drag to reorder, middle-click close, widths frozen
  while closing, own window controls and edge resizing (macOS keeps its traffic lights).
- + asks "Open a new tab as…": Text file / Document / PDF (soon) / Open a file; Ctrl+T. Single-app modes skip the question.
- ⌄ menu: new tab as…, search open tabs, recently closed (Ctrl+Shift+T restores unsaved text too). Ctrl+1…8, Ctrl+9.
- Quick-access row: AutoSave switch (named files every 20 s), Save/Undo/Redo, classic menus under ⋯, name + Saved/Edited,
  command search.
- Ribbon with 11 tabs and named groups; real: clipboard, font, paragraph, styles gallery, editing, link, date/time,
  symbols, horizontal line, Word Count, Restrict Editing, ruler, view toggles, zoom, modes, lock, shortcuts list.
  Unbuilt features greyed out with "coming soon". Notepad mode shows only the ribbon tabs plain text uses.
- Upstream radar now redacts vendor brand names in snapshots and issues.

## 2026-10-19: v0.3.0, HarshPad Studio (phase 1: writer)
- Tabs mixing plain text and rich documents; one toolbar layout and status bar for both.
- crates/rich: styled-text model with edit diffing (styles follow typing), Enter continues/ends lists,
  Markdown shortcuts, .docx reader/writer from ECMA-376, Markdown in/out, .hpdoc.
- Command catalog with per-kind availability; modes (Studio / Notepad / Writer / PDF soon) with a parent
  PIN lock; files of other kinds open as new copies in the current mode.
- Undo stack with typing groups and a memory budget for every tab; crash-recovery autosave every 30 s.
- Upstream radar workflow (docs only) + Dependabot.
- Numbers: window 30 ms warm, 76 MB, app 5.7 MB. 50+ tests.
- Still open: PDF tabs (phase 2), export to PDF (phase 3), tables/images in documents.
