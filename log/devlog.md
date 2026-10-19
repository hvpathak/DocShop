# Dev log

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
