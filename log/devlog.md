# Dev log

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
