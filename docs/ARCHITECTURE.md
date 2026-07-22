# xrite-export — Architecture

## Data model

`JobConfig` → `shapes: Vec<ShapeData>` → `weights: Vec<WeightData>`

- **`JobConfig`** — `preset_name`, metadata strings (`job_name`, `job_number`, `customer`, `plate_tech`, `press_system`, `esxr_number`, `print_type`, `date`, `set_number`), `inks: Vec<Ink>`, `step_labels: Vec<String>`, `shapes: Vec<ShapeData>`
- **`Ink`** — `kind: InkKind` (Cyan/Magenta/Yellow/Black/White/Spot) + `name: String`. Default set is CMYK. `in_deviation_average()` filters to CMYK only (excludes White and Spot).
- **`ShapeData`** — `dot_type` (e.g. "CRS") + `dot_number` (e.g. "501"); `display_name()` → "CRS 501"
- **`WeightData`** — `lpi: String`, `density: Vec<f64>` (max density per ink), `steps: Vec<Vec<f64>>` (step_idx × ink_idx). `resize_steps()` pads/trims to match ink and step counts.
- **Step presets:** `STEP_LABELS_14` (100→1) and `STEP_LABELS_16` (100→0.4)

Session files are JSON, forward-compatible via pad/trim on load.

### Session forward compatibility (`core/session.rs`)

Manual deserialization ensures old sessions load cleanly:
- Old `colour_names` array maps to new `inks` structure
- Old `"name"` / `"label"` fields map to `dot_type` / `lpi`
- Step/ink count mismatches handled by `WeightData::resize_steps()` and `pad_f64_vec()`

## Module layout

```
src/
├── main.rs                    # entry — routes to --web, --companion, or egui desktop
├── settings.rs                # JSON settings (dirs crate → ~/InkDensityTool/settings.json)
├── core/
│   ├── models.rs              # JobConfig, ShapeData, WeightData, Ink, InkKind; STEP_LABELS_*
│   ├── session.rs             # save_session / load_session
│   └── targets.rs             # DOT_GAIN_TARGETS table + interpolate_target(step) → Option<f64>
├── gui/                       # eframe/egui desktop only
│   ├── app.rs                 # InkDensityApp (eframe::App); top-level layout + menus
│   ├── job_config.rs          # left panel: metadata fields + weight label chips
│   ├── shape_tabs.rs          # shape tab switcher + inner weight tabs
│   └── weight_grid.rs         # data-entry grid; column-major Tab order
├── export/
│   ├── placeholders.rs        # <<PLACEHOLDER>> dict builder; shared by all PDF paths
│   ├── illustrator.rs         # JSX substitution → Illustrator.exe subprocess (Windows)
│   ├── libreoffice.rs         # LibreOffice headless UNO bridge (Linux)
│   ├── excel.rs               # umya-spreadsheet template fill + sheet selection
│   ├── pdf_merge.rs           # lopdf PdfWriter merge
│   ├── report.rs              # HTML report + comparison report; Pantone catalogue for spots
│   └── svg.rs                 # SVG export; multi-page A4 landscape stacked vertically
└── web/                       # web feature only; axum + tokio
    ├── server.rs              # :8181 — serves embedded index.html + /api/* routes
    └── companion.rs           # :7432 — Illustrator PDF bridge with PNA CORS headers
```

## Web architecture

**Linux server** (axum, port 8181):

```
GET  /                          → embedded assets/index.html
GET/POST /api/job               → in-memory JobConfig (Arc<Mutex>)
GET/POST /api/settings          → settings JSON
POST /api/export/excel          → umya-spreadsheet → stream .xlsx
POST /api/export/report         → generate_report() → stream HTML
POST /api/export/comparison     → generate_comparison_report() → stream HTML
POST /api/export/svg            → export_svg() → stream .svg
GET  /api/export/builder-script → download build_ai_template.jsx
GET  /api/version               → { "build_ts": <unix timestamp> }
```

Cloudflare tunnel: `xrite.rgb-b.com` (and legacy `xrite-export.elphiene.com`) → `:8181`. Tunnel ID `298a0402-1fa7-4a89-b7d1-3f42995dc6cb`. Config in `~/.cloudflared/config.yml`.

**Windows companion** (`ink-density-tool.exe --companion`, port 7432):

```
GET  /health       → liveness probe
POST /export/pdf   → Illustrator.exe → stream PDF
GET/POST /settings → companion-relevant settings subset
```

**PDF from browser:** frontend probes `http://localhost:7432/health` on load. PNA CORS (`Access-Control-Allow-Private-Network: true`) is required because the page is served over HTTPS.

## GUI data flow (desktop)

1. `InkDensityApp.update()` — single egui frame callback; polls `Arc<Mutex<ExportStatus>>` for background-thread results
2. Left panel (`job_config`) → `JobConfigState` holds metadata + weight label list; broadcasts changes to `ShapeNotebookState`
3. Right panel (`shape_tabs`) — outer tabs = shapes, inner tabs = weights per shape
4. Data grid (`weight_grid`) — column-major Tab order (all C rows → M → Y → K) to match X-Rite eXact scan sequence; 100% row is read-only
5. **DataCatcher auto-advance:** 300ms settle timer per field via `ui.input(|i| i.time)`; advances focus automatically when field is non-empty

## Export flow

| Output | Path |
|---|---|
| PDF | `placeholders::build_placeholders` builds `<<W1_DC>>`, `<<W1_R01_C>>`, etc. Weights chunked into ≤3 per page (W1/W2/W3 slots). Windows → Illustrator.exe subprocess; Linux → LibreOffice UNO via embedded `lo_uno_helper.py`. Chunks merged via `pdf_merge`. |
| Excel | `template_extended.xlsx` (16-step) or `template_standard.xlsx` (14-step). 4 sheets — 0/1 = standard pair (single/dual), 2/3 = extended pair. `sheet_offset=2` for 16-step. Unused sheets stripped before save. |
| HTML report | Print-ready A4. Per-shape tables, dot-gain target column, avg/deviation for CMYK. Spot ink headers attempt Pantone colour match (catalogue embedded in `report.rs`). |
| SVG | Pages stacked vertically (842×595 pt each); ≤3 weight blocks per page. |

Desktop: exports on `std::thread::spawn` daemon threads. Web: exports synchronous per request.

## Excel template layout (extended, 16-step)

**Single sheet** (weight[0]): rows 1–20 (title/date → headers → density → 16 steps → label)

**Dual sheet** (weight[1] + weight[2]): table 1 at rows 1–20, table 2 at rows 21–40

Cell-mapping defaults (`xcm_*` settings keys):
- `xcm_step_start_row = 4`, `xcm_gap_t1_to_t2 = 4`, `xcm_density_row_offset = 1`, `xcm_title_t2_row_offset = 3`
- Data cols: `B=C`, `C=M`, `D=Y`, `E=K`; `F=Average%`, `G=Difference`, `H=Ideal%`

**Critical:** `fix_second_table_formulas` and `clone_first_pair` in `excel.rs` are dead code — **do not call**. The template formulas were fixed directly in `assets/template_extended.xlsx` (March 2026). umya's `set_formula()` produces `t="str"` cells that Excel renders as plaintext rather than evaluating.

**Temp files for Excel export must use `.xlsx` suffix:** `tempfile::Builder::new().suffix(".xlsx").tempfile()` — umya panics on missing extension.

## Settings keys

- `illustrator_path`, `ai_template`, `ai_template_extended`, `excel_template`, `libreoffice_path`
- `default_weight_labels`, `default_step_labels`, `last_session_path`
- Cell mapping (`xcm_*`): `xcm_title_col`, `xcm_date_col`, `xcm_step_start_row`, `xcm_data_col_c/m/y/k`, `xcm_label_col`, `xcm_dot_shape_col`, `xcm_gap_t1_to_t2`, `xcm_density_row_offset`, `xcm_title_t2_row_offset`
- Dropdown options: `default_print_types`, `default_finishes`, `default_dot_shape_types`
- Settings cache (`Lazy<Mutex<Option<HashMap>>>`) invalidated on every `set()` call; web server cache updated live via `POST /api/settings`

## Tools menu (desktop)

- **Tools → Export Builder Script** — extracts embedded `build_ai_template.jsx` to disk; run in Illustrator (**File → Scripts → Other Script**) to generate `.ai` templates from scratch
