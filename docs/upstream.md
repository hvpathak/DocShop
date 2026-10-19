# Following upstream

HarshPad Studio combines a notepad, a writer and a PDF tool. Two open-source projects cover the
writer and PDF areas in depth: [WordCraft](https://github.com/storytold/wordcraft) and
[PdfCraft](https://github.com/storytold/pdfcraft). We follow what they announce so HarshPad keeps
improving, while staying **clean-room**: we read their public feature documents and release notes,
never their code.

## How updates reach HarshPad

1. **Upstream radar** (`.github/workflows/upstream-radar.yml`) runs every Monday. It snapshots their
   README, roadmap, gaps and parity documents into `.radar/`, and opens a `radar` issue listing every new
   heading, bullet, table row or release note.
2. **Triage** each item with the map below: it names the one HarshPad module the feature belongs in.
3. **Build it clean-room** from the behaviour description and public specifications (ECMA-376 for
   `.docx`, ISO 32000 for PDF), with tests, then tick the item off in the issue.
4. **Shared libraries** (for example `hayro` for PDF rendering and `krilla` for PDF writing, which
   PdfCraft and WordCraft also build on) update through Dependabot pull requests, tested by CI on
   every platform.

## Feature map

| Upstream area | HarshPad module |
|---|---|
| Document model, editing, styles | `crates/rich/src/doc.rs`, `style.rs` |
| `.docx` read/write | `crates/rich/src/docx.rs` |
| Markdown, other text formats | `crates/rich/src/markdown.rs` |
| Spelling, proofing | `crates/rich` (new module) |
| Layout, pagination, PDF export | `crates/export` (phase 3) |
| PDF parsing, rendering, text extraction | `crates/pdf` (phase 2) |
| Page organizing, combine/split | `crates/pdf` organize module (phase 2) |
| Comments, forms, signatures | `crates/pdf` annot/forms modules (phase 3) |
| Commands, shortcuts, agent control | `crates/core/src/commands.rs`, `apps/harshpad-cli` |
| Interface (ribbons, panes, views) | `apps/harshpad/src` (`doc_tab.rs`, `app.rs`) |

## Rules

- Never copy, translate or paraphrase upstream source code. Behaviour and announcements only.
- A feature that moves gets a test in the module above and a line in `log/devlog.md`.
