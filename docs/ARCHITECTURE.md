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
├── main.rs                    # entry — routes to --web, otherwise prints a "desktop not available" notice
├── settings.rs                # JSON settings (dirs crate → ~/InkDensityTool/settings.json)
├── core/
│   ├── models.rs              # JobConfig, ShapeData, WeightData, Ink, InkKind; STEP_LABELS_*
│   ├── session.rs             # save_session / load_session
│   └── targets.rs             # DOT_GAIN_TARGETS table + interpolate_target(step) → Option<f64>
├── export/
│   ├── report.rs              # HTML report + comparison report; orientation toggle; Pantone catalogue for spots
│   └── svg.rs                 # SVG export; multi-page A4 landscape stacked vertically
└── web/                       # web feature only; axum + tokio
    └── server.rs              # :8181 — serves embedded index.html + /api/* routes
```

> **No desktop GUI in this build.** `src/gui/` (egui/eframe) was removed — it hadn't been wired into `main.rs` for some time and had drifted out of sync with the current `JobConfig` shape. The web UI (`assets/index.html` served by `web/server.rs`) is the only live client.
>
> **No Excel/Illustrator/LibreOffice/companion export.** These were unused in practice and removed along with their assets (`build_ai_template.jsx`, `lo_uno_helper.py`, `runner.jsx`, the `.xlsx` templates) and settings keys. HTML report + SVG are the only export formats now.

## Web architecture

**Linux server** (axum, port 8181):

```
GET  /                          → embedded assets/index.html
GET/POST /api/job               → in-memory JobConfig (Arc<Mutex>)
GET/POST /api/settings          → settings JSON (includes report_orientation)
POST /api/export/report         → generate_report() → stream HTML
POST /api/export/comparison     → generate_comparison_report() → stream HTML
POST /api/export/svg            → export_svg() → stream .svg
GET  /api/version               → { "build_ts": <unix timestamp> }
```

Cloudflare tunnel: `xrite.rgb-b.com` (and legacy `xrite-export.elphiene.com`) → `:8181`. Tunnel ID `298a0402-1fa7-4a89-b7d1-3f42995dc6cb`. Config in `~/.cloudflared/config.yml`.

## Export flow

| Output | Path |
|---|---|
| HTML report | Print-ready A4, landscape or portrait (`Settings.report_orientation`, toggled in the web Settings modal). Header shows the job name as a heading plus a row of "Label: Value" tags (Job Number / Customer / Plate-Press / Print Type / Date / Set) — any empty field is skipped rather than showing a blank tag. Per-shape tables always render at `width: 100%` so two shapes with different LPI-weight counts still line up edge-to-edge. Dot-gain target column, avg/deviation for CMYK. Spot ink headers attempt Pantone colour match (catalogue embedded in `report.rs`). |
| SVG | Pages stacked vertically (842×595 pt each); ≤3 weight blocks per page. |

Exports are synchronous per web request (no background threads — that was a desktop-only concern, and the desktop app no longer exists in this build).

## Settings keys (`Settings` struct in `settings.rs`)

- `job_presets: Vec<JobPreset>` — each controls `FieldVisibility`, `GridLayout`, `ReportLayout`, and an optional default step preset
- `step_presets: Vec<StepPreset>` — named step-percentage lists
- `dot_types: Vec<String>`, `lpi_values: Vec<String>` — picker options, grow automatically as new values are entered
- `last_session_path: String`
- `report_orientation: ReportOrientation` (`Landscape` | `Portrait`, `#[serde(rename_all = "lowercase")]`) — read directly by `web/server.rs` when generating a report; no flat-key indirection
- Settings cache (`Lazy<Mutex<Option<Settings>>>`) invalidated on every `save()` call; the whole struct round-trips as JSON via `GET/POST /api/settings` — there's no separate flat key/value API anymore
