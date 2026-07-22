# xrite-export — Decisions log

## D-001 · Rust + egui (not Tauri, Electron, or web-only)

**Decided:** Build time
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
**Context:** PDF templates are `.ai` (Illustrator native). Real Illustrator only runs on Windows/macOS.
**Decision:** Windows → Illustrator.exe subprocess + ExtendScript (`/b` batch flag). Linux → LibreOffice headless UNO via an embedded Python helper (`lo_uno_helper.py`).
**Why:** Lets the Linux server generate PDFs without a Windows VM in the loop. LibreOffice can render `.ai` files via its PDF import filter.
**Trade-off:** LibreOffice output isn't pixel-identical to Illustrator. Acceptable for QA-style reports; users sending to print should generate on Windows.

## D-004 · Web companion EXE for browser → Illustrator bridge

**Decided:** Build time
**Context:** Once the tool became a web app (axum on `:8181`), browser users on macOS or other non-Windows clients lost the ability to generate PDFs through Illustrator.
**Decision:** Add a `--companion` mode that runs the same EXE on a Windows machine as a local HTTP server on `:7432`. The browser probes `localhost:7432/health` on load. PDF generation requests go to the companion.
**Why:** Avoids a server-side Illustrator install (expensive, fragile). Each user runs the companion locally.
**Trade-off:** Users must install + run the companion EXE on a Windows box. Browser PNA (Private Network Access) headers required.

## D-005 · No `win32com` / `comtypes` — Illustrator driven via subprocess + ExtendScript

**Decided:** Build time
**Context:** Could have driven Illustrator from Rust via COM bindings.
**Decision:** Generate JSX text with placeholders substituted, write to a temp file, invoke `Illustrator.exe /b script.jsx` (batch mode).
**Why:** No Rust COM crate is mature. Subprocess + JSX is well-trodden, easy to debug (just inspect the generated JSX), and survives Illustrator version upgrades.
**Trade-off:** Slower than COM. Acceptable — generating a PDF is already a multi-second operation.

## D-006 · All assets embedded at compile time

**Decided:** Build time
**Context:** Templates, helper scripts, web HTML need to ship with the binary.
**Decision:** `include_bytes!` for fixed-size assets (XLSX templates, JSX scripts, Python helper). `rust-embed` for the web HTML directory.
**Why:** Single-binary deploy. No "where did `runner.jsx` go" debugging in production.
**Trade-off:** Updating an asset requires a rebuild. Acceptable — these assets change rarely.

## D-007 · `set_formula()` is broken — formulas baked into the XLSX template

**Decided:** March 2026
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
