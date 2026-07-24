# xrite-export — Decisions log

## D-001 · Rust + egui (not Tauri, Electron, or web-only)

**Decided:** Build time
**Superseded:** 2026-07-24 — see D-011. The desktop client was never migrated onto the current data model and had quietly stopped being compiled; `src/gui/` was deleted rather than repaired.
**Context:** Needed a fast, native-feeling data-entry app that runs alongside Illustrator on the shop's Windows machines.
**Decision:** Rust + eframe/egui for the desktop client.
**Why:** Single binary, no runtime to install, fast startup, no Electron-tier memory footprint. egui's immediate-mode model fits the simple data-grid use case well.
**Trade-off:** egui's text-input UX isn't as polished as native widgets. Acceptable given the data-entry pattern is mostly keyboard-driven.

## D-002 · Column-major Tab order in the data grid

**Decided:** Build time
**Context:** The X-Rite eXact device, used with DataCatcher, scans a column at a time (all C readings, then all M, etc.).
**Decision:** Tab navigation in `weight_grid` goes down the C column first, then M, Y, K — not across the row.
**Why:** Matches the scanning workflow exactly. Row-major would force the user to either re-order scans or constantly reach for the mouse.
**Trade-off:** Counter-intuitive for someone who's NOT using a scanner. Documented prominently in CLAUDE.md.

## D-003 · Two transports for PDF: Illustrator (Windows) and LibreOffice UNO (Linux)

**Decided:** Build time
**Superseded:** 2026-07-24 — see D-011. Neither transport was actually in use; removed along with the Illustrator template pipeline entirely.
**Context:** PDF templates are `.ai` (Illustrator native). Real Illustrator only runs on Windows/macOS.
**Decision:** Windows → Illustrator.exe subprocess + ExtendScript (`/b` batch flag). Linux → LibreOffice headless UNO via an embedded Python helper (`lo_uno_helper.py`).
**Why:** Lets the Linux server generate PDFs without a Windows VM in the loop. LibreOffice can render `.ai` files via its PDF import filter.
**Trade-off:** LibreOffice output isn't pixel-identical to Illustrator. Acceptable for QA-style reports; users sending to print should generate on Windows.

## D-004 · Web companion EXE for browser → Illustrator bridge

**Decided:** Build time
**Superseded:** 2026-07-24 — see D-011. The companion was unused in practice; `web/companion.rs` deleted.
**Context:** Once the tool became a web app (axum on `:8181`), browser users on macOS or other non-Windows clients lost the ability to generate PDFs through Illustrator.
**Decision:** Add a `--companion` mode that runs the same EXE on a Windows machine as a local HTTP server on `:7432`. The browser probes `localhost:7432/health` on load. PDF generation requests go to the companion.
**Why:** Avoids a server-side Illustrator install (expensive, fragile). Each user runs the companion locally.
**Trade-off:** Users must install + run the companion EXE on a Windows box. Browser PNA (Private Network Access) headers required.

## D-005 · No `win32com` / `comtypes` — Illustrator driven via subprocess + ExtendScript

**Decided:** Build time
**Superseded:** 2026-07-24 — see D-011. Moot; the Illustrator export path no longer exists.
**Context:** Could have driven Illustrator from Rust via COM bindings.
**Decision:** Generate JSX text with placeholders substituted, write to a temp file, invoke `Illustrator.exe /b script.jsx` (batch mode).
**Why:** No Rust COM crate is mature. Subprocess + JSX is well-trodden, easy to debug (just inspect the generated JSX), and survives Illustrator version upgrades.
**Trade-off:** Slower than COM. Acceptable — generating a PDF is already a multi-second operation.

## D-006 · All assets embedded at compile time

**Decided:** Build time
**Partially superseded:** 2026-07-24 — the XLSX templates, JSX scripts, and Python helper were deleted along with the Excel/Illustrator/LibreOffice export paths (see D-011). `include_bytes!` for `assets/index.html` remains; the `rust-embed` crate itself turned out to be an unused dependency (nothing ever used the derive macro) and was dropped from `Cargo.toml`.
**Context:** Templates, helper scripts, web HTML need to ship with the binary.
**Decision:** `include_bytes!` for fixed-size assets (XLSX templates, JSX scripts, Python helper). `rust-embed` for the web HTML directory.
**Why:** Single-binary deploy. No "where did `runner.jsx` go" debugging in production.
**Trade-off:** Updating an asset requires a rebuild. Acceptable — these assets change rarely.

## D-007 · `set_formula()` is broken — formulas baked into the XLSX template

**Decided:** March 2026
**Superseded:** 2026-07-24 — see D-011. Excel export removed entirely; kept here for history since the umya gotcha may resurface if Excel export ever comes back.
**Context:** Cell formulas in the Excel output need to recalculate when a user opens the file.
**Decision:** Don't call umya's `set_formula()` at runtime. Bake formulas into `assets/template_extended.xlsx` once, by hand, and just fill in the data values.
**Why:** umya's `set_formula()` writes cells with `t="str"` (string type), which Excel renders as plaintext rather than evaluating.
**Trade-off:** Adding a new formula means editing the XLSX template directly. The dead code (`fix_second_table_formulas`, `clone_first_pair`) is preserved for reference but must not be called.

## D-008 · DataCatcher auto-advance with 300ms settle timer

**Decided:** Build time
**Context:** DataCatcher injects keystrokes very fast — a 1-byte typo can cause double-advance.
**Decision:** 300ms settle timer per field (`ui.input(|i| i.time)`). Field advance only happens after the input has been idle for 300ms AND the field is non-empty.
**Why:** Reliable scan-to-advance without dropped keystrokes.
**Trade-off:** A real human typing fast gets a 300ms pause before advance. Negligible UX impact; users rarely type in this grid.

## D-009 · `web` feature flag (not always-on)

**Decided:** Build time
**Note:** 2026-07-24 — the desktop egui app referenced below no longer exists (see D-011), so the flag's original rationale is weaker, but it's kept: a non-`--features web` build still compiles to a minimal binary with no server/HTTP deps at all, which is a reasonable thing to be able to build even with no GUI behind it yet.
**Context:** The desktop egui app shouldn't pull axum/tokio + transitive HTTP deps.
**Decision:** Web mode behind `--features web`. Desktop builds skip those deps entirely.
**Why:** Faster desktop compile, smaller binary, fewer attack-surface dependencies for the offline desktop use case.
**Trade-off:** Two build configurations to maintain. Mitigated by CI building both.

## D-010 · Migrate canonical domain to `xrite.rgb-b.com`

**Decided:** 2026-05-28
**Context:** The tool was first deployed at `xrite-export.elphiene.com`, but it's a work tool — should live under the `rgb-b` umbrella alongside the other shop tools.
**Decision:** Add `xrite.rgb-b.com` as the new canonical hostname; keep `xrite-export.elphiene.com` working during the transition; remove it once verified.
**Why:** Consistency with the rest of the shop tools (`tools.rgb-b.com`, `colour.rgb-b.com`).
**Trade-off:** Two hostnames live simultaneously for a while. Acceptable — both routes point to the same `:8181` service.

## D-011 · Remove Excel/Illustrator/LibreOffice/companion export, and the orphaned desktop GUI

**Decided:** 2026-07-24
**Context:** None of the Excel, Illustrator-PDF, LibreOffice-PDF, or companion-bridge export paths were actually being used day-to-day — only the HTML report and SVG export were. Separately, `src/gui/` (the egui desktop client) had silently stopped being compiled at all (`main.rs` never declared `mod gui;`) and had drifted out of sync with the current `JobConfig` shape (it still referenced fields like `stock_desc`/`dot_shape_type` that don't exist anymore) — it was dead weight, not a working alternate frontend.
**Decision:** Delete `src/export/excel.rs`, `illustrator.rs`, `libreoffice.rs`, `pdf_merge.rs`, `placeholders.rs`; delete `src/web/companion.rs`; delete `src/gui/` entirely; delete the now-unused template/script assets (`build_ai_template.jsx`, `lo_uno_helper.py`, `runner.jsx`, both `.xlsx` templates) and `TEMPLATE_GUIDE.md`; drop the now-dead `umya-spreadsheet`, `lopdf`, `eframe`, `egui`, `egui_extras`, `rfd`, and `rust-embed` crates from `Cargo.toml`; remove the Excel/PDF buttons, companion settings panel, and companion-probe JS from `assets/index.html`; remove the `/api/export/excel` and `/api/export/builder-script` routes and the settings fields (`illustrator_path`, `ai_template`, `ai_template_extended`) that only existed to support them.
**Why:** Less surface area to keep compiling and documenting for code paths nobody exercises. The HTML report and SVG export remain, and are now the *only* export formats — simpler mental model for anyone touching this repo next.
**Trade-off:** If Illustrator-driven PDF export is wanted again later, it's a from-scratch rebuild rather than an un-delete — this history log and D-003/D-004/D-005/D-007 above are the reference for how it worked before.

## D-012 · Report header becomes a uniform tag row; tables always render at 100% width; landscape/portrait toggle

**Decided:** 2026-07-24
**Context:** The old report header mixed a large "Customer" heading, a separate unlabeled spec-tag row (bare plate/press/print-type values with no label), and a stacked JOB/DATE/SET column — readable at a glance if you already knew the shop's shorthand, but not labeled for anyone else, and small print was hard to read for the shop's older staff. Separately, when two shapes on the same report had different numbers of LPI weight groups, their data tables ended up visibly different widths (columns sized to content, no fixed width), which looked sloppy stacked one above the other.
**Decision:** Header is now the job name as a heading, followed by a row of uniform "Label: Value" tags (Job Number, Customer, Plate/Press, Print Type, Date, Set) — any field that's empty is skipped rather than showing a blank tag. `Plate/Press` combines `plate_tech` + `press_system` + `esxr_number` (space-joined, empties dropped) into one tag rather than three separate raw fields. Every shape's `<table>` is `table-layout: fixed; width: 100%`, so it always fills its container regardless of how many LPI weight groups it has. Font sizes were bumped across the board, and each LPI group header now shows a small "LPI" tag above the ruling number instead of a bare number. A `ReportOrientation` (`Landscape` | `Portrait`) toggle, stored in `Settings.report_orientation` and set from the web Settings modal, controls `@page` orientation and whether shapes render side-by-side (landscape) or stacked (portrait, the original layout).
**Why:** Requested directly by the shop after reviewing mockups (`docs/mockup_landscape.html` / `docs/mockup_portrait.html`, gitignored — contain real customer data) built from a live job pulled off the production server. Landscape-with-shapes-side-by-side was preferred but they wanted to keep portrait available in case they change their mind.
**Trade-off:** The multi-job comparison report (`generate_comparison_report`) got the same tag treatment for consistency, but that specific redesign wasn't itself reviewed via mockup — worth a look if the shop starts using comparison reports more.
