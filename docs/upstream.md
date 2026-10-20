# Following upstream

HarshPad Studio combines a notepad, a writer and a PDF tool. Two open-source projects cover the
writer and PDF areas in depth: [WordCraft](https://github.com/storytold/wordcraft) and
[PdfCraft](https://github.com/storytold/pdfcraft). We follow what they announce so HarshPad keeps
improving, while staying **clean-room**: we read their public feature documents and release notes,
never their code.

## How updates reach HarshPad

Writer Studio *is* WordCraft, and PDF Studio will be PdfCraft. Both are used under their open-source
licenses from forks on our GitHub ([hvpathak/wordcraft](https://github.com/hvpathak/wordcraft),
[hvpathak/pdfcraft](https://github.com/hvpathak/pdfcraft)), on a `harshpad` branch that adds only
HarshPad's touches: Help and Feedback go to this project, no community or account buttons, and a
light/dark switch.

**"Get their update"** = `pwsh scripts/get-their-update.ps1` (`-Project pdfcraft` for PDF Studio):

1. merges their latest `main` into our `harshpad` branch;
2. **guards it**: if their update added a community, Discord, account or upstream-website link, the
   script stops and lists them, so they are hidden before anything ships
   (the known ones are in `scripts/upstream-guard-<project>.json`);
3. pushes the fork and moves HarshPad's pinned revision to it.

Then `cargo test --workspace`, CI on Windows, macOS and Linux, and a release. GitHub Actions is
switched off on the forks, so their own workflows never run (or email) from our account.

The weekly upstream radar still watches their feature notes and opens an issue when there's news.

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
